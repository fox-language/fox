import { execSync } from 'child_process';
import fs from 'fs';
import path from 'path';
import { Bench } from 'tinybench';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
process.env.FOX_PATH = __dirname;


function findFiles(dir, suffix, results = []) {
    if (!fs.existsSync(dir)) return results;
    for (const file of fs.readdirSync(dir)) {
        if (file === 'node_modules' || file === 'target' || file.startsWith('.')) continue;
        const fullPath = path.join(dir, file);
        const stat = fs.statSync(fullPath);
        if (stat.isDirectory()) {
            findFiles(fullPath, suffix, results);
        } else if (file.endsWith(suffix)) {
            results.push(fullPath);
        }
    }
    return results;
}

const stdDir = path.join(__dirname, 'std');
const benchDir = path.join(__dirname, 'benchmarks');
const outDir = path.join(__dirname, '.fox-benchs');

if (!fs.existsSync(outDir)) {
    fs.mkdirSync(outDir, { recursive: true });
}

const pkgPath = path.join(outDir, 'package.json');
if (!fs.existsSync(pkgPath)) {
    fs.writeFileSync(pkgPath, '{ "type": "module" }\n');
}

const WASM_OPT_FLAGS = ['-O3', '--strip-debug', '--strip-producers', '--vacuum', '--converge', '--enable-gc', '--enable-reference-types', '--enable-exception-handling', '--enable-multimemory', '--enable-bulk-memory', '--enable-sign-ext', '--enable-nontrapping-float-to-int', '--enable-mutable-globals', '--enable-tail-call', '--enable-multivalue'];

function wasmOptInstalled() {
    try {
        execSync('wasm-opt --version', { stdio: ['ignore', 'pipe', 'pipe'] });
        return true;
    } catch {
        return false;
    }
}

function optimizeWasm(wasmPath) {
    execSync(`wasm-opt ${WASM_OPT_FLAGS.join(' ')} "${wasmPath}" -o "${wasmPath}"`, { stdio: 'inherit' });
}

if (!wasmOptInstalled()) {
    console.error('wasm-opt is required for fair wasm size comparisons but was not found on PATH.');
    console.error('Install binaryen: https://github.com/WebAssembly/binaryen');
    process.exit(1);
}

const foxFiles = [
    ...findFiles(stdDir, '.bench.fox'),
    ...findFiles(benchDir, '.bench.fox'),
];

console.log('Building fox compiler...');
execSync('cargo build', { stdio: 'inherit' });

const bench = new Bench();
const tasks = [];

function fmtBytes(n) {
    if (n == null) return '—';
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(2)} KiB`;
    return `${(n / 1024 / 1024).toFixed(2)} MiB`;
}

function fmtHz(hz) {
    if (!isFinite(hz) || hz <= 0) return '—';
    if (hz >= 1e6) return `${(hz / 1e6).toFixed(2)} Mops/s`;
    if (hz >= 1e3) return `${(hz / 1e3).toFixed(2)} kops/s`;
    return `${hz.toFixed(2)} ops/s`;
}

function fmtMs(ms) {
    if (!isFinite(ms) || ms <= 0) return '—';
    if (ms < 1e-3) return `${(ms * 1e6).toFixed(2)} ns`;
    if (ms < 1) return `${(ms * 1e3).toFixed(2)} µs`;
    return `${ms.toFixed(2)} ms`;
}

async function loadFoxModule(foxFile) {
    const baseName = path.basename(foxFile, '.fox');
    execSync(`cargo run --release -- "${foxFile}" -o "${outDir}"`, {
        stdio: 'inherit',
    });
    const jsPath = path.join(outDir, `${baseName}.js`);
    const wasmPath = path.join(outDir, `${baseName}.wasm`);
    optimizeWasm(wasmPath);
    const wasmBuffer = fs.readFileSync(wasmPath);
    const size = wasmBuffer.byteLength;
    const { fox } = await import(`file://${jsPath}`);
    const { exports } = await fox(wasmBuffer);
    return { exports, size };
}

function wasmTargetInstalled() {
    try {
        const out = execSync('rustup target list --installed', { stdio: ['ignore', 'pipe', 'ignore'] }).toString();
        return out.split('\n').some(l => l.trim() === 'wasm32-unknown-unknown');
    } catch {
        try {
            const out = execSync('rustc --print target-libdir --target wasm32-unknown-unknown', { stdio: ['ignore', 'pipe', 'pipe'] }).toString();
            return out.trim().length > 0;
        } catch {
            return false;
        }
    }
}

async function loadRustModule(cargoDir, crateName) {
    if (!wasmTargetInstalled()) {
        console.error(`\nSkipping Rust baseline for ${crateName}: the wasm32-unknown-unknown target is not installed.`);
        console.error(`Install it with: rustup target add wasm32-unknown-unknown\n`);
        return null;
    }
    console.log(`Building Rust baseline: ${crateName}...`);
    try {
        execSync('cargo build --target wasm32-unknown-unknown --release', {
            stdio: 'inherit',
            cwd: cargoDir,
        });
    } catch (e) {
        console.error(`\nFailed to build Rust baseline for ${crateName}. See output above.`);
        return null;
    }
    const wasmPath = path.join(cargoDir, 'target', 'wasm32-unknown-unknown', 'release', `${crateName}.wasm`);
    if (!fs.existsSync(wasmPath)) {
        console.error(`\nExpected wasm at ${wasmPath} after cargo build.`);
        return null;
    }
    optimizeWasm(wasmPath);
    const wasmBuffer = fs.readFileSync(wasmPath);
    const size = wasmBuffer.byteLength;
    const module = await WebAssembly.compile(wasmBuffer);
    const instance = await WebAssembly.instantiate(module, {});
    return { exports: instance.exports, size };
}

async function loadJsModule(jsFile) {
    const size = fs.statSync(jsFile).size;
    const mod = await import(`file://${jsFile}`);
    return { exports: mod, size };
}

function registerBenchExports(mod, sourceLabel) {
    for (const name of Object.keys(mod.exports)) {
        if (!name.startsWith('bench_')) continue;
        if (typeof mod.exports[name] !== 'function') continue;
        const fn = mod.exports[name];
        const taskName = `${sourceLabel}:${name}`;
        bench.add(taskName, () => fn());
        tasks.push({ name, source: sourceLabel, size: mod.size });
    }
}

for (const foxFile of foxFiles) {
    const stem = path.basename(foxFile, '.bench.fox');
    const parentDir = path.dirname(foxFile);
    const cargoToml = path.join(parentDir, 'Cargo.toml');
    const jsFile = path.join(parentDir, `${stem}.bench.mjs`);

    console.log(`Compiling ${path.relative(__dirname, foxFile)}...`);
    const foxMod = await loadFoxModule(foxFile);
    registerBenchExports(foxMod, 'fox');

    if (fs.existsSync(jsFile)) {
        const jsMod = await loadJsModule(jsFile);
        registerBenchExports(jsMod, 'js');
    }

    if (fs.existsSync(cargoToml)) {
        const crateName = path.basename(parentDir);
        const rustMod = await loadRustModule(parentDir, crateName);
        if (rustMod) registerBenchExports(rustMod, 'rust');
    }
}

if (tasks.length === 0) {
    console.log('\nNo bench_* exports found.');
    process.exit(0);
}

console.log(`\nRunning ${tasks.length} benchmark task(s)...\n`);
await bench.run();

const resultsByName = new Map();
for (const t of bench.tasks) {
    const r = t.result;
    if (!r) continue;
    const idx = t.name.indexOf(':');
    const source = t.name.slice(0, idx);
    const fnName = t.name.slice(idx + 1);
    const entry = resultsByName.get(fnName) || {};
    entry[source] = r;
    resultsByName.set(fnName, entry);
}

const rows = [];
for (const [name, entry] of resultsByName) {
    const foxTask = tasks.find(t => t.name === name && t.source === 'fox');
    const rustTask = tasks.find(t => t.name === name && t.source === 'rust');
    const jsTask = tasks.find(t => t.name === name && t.source === 'js');
    rows.push({
        name,
        fox: entry.fox,
        foxSize: foxTask?.size,
        rust: entry.rust,
        rustSize: rustTask?.size,
        js: entry.js,
        jsSize: jsTask?.size,
    });
}

const hasJs = rows.some(r => r.js);
const hasRust = rows.some(r => r.rust);

function buildHeaders(primaryLabel, ratioLabel) {
    const headers = ['benchmark', `fox ${primaryLabel}`];
    if (hasRust) headers.push(`rust ${primaryLabel}`);
    if (hasJs) headers.push(`js ${primaryLabel}`, 'fox/js');
    if (hasJs && hasRust) headers.push(ratioLabel);
    return headers;
}

function speedRowOf(row) {
    const cells = [row.name, row.fox ? fmtMs(row.fox.mean) : '—'];
    if (hasRust) cells.push(row.rust ? fmtMs(row.rust.mean) : '—');
    if (hasJs) {
        cells.push(row.js ? fmtMs(row.js.mean) : '—');
        cells.push((row.fox && row.js) ? `${(row.fox.hz / row.js.hz).toFixed(2)}×` : '—');
        if (hasRust) {
            cells.push((row.rust && row.js) ? `${(row.rust.hz / row.js.hz).toFixed(2)}×` : '—');
        }
    }
    return cells;
}

function sizeRowOf(row) {
    const cells = [row.name, row.foxSize != null ? fmtBytes(row.foxSize) : '—'];
    if (hasRust) cells.push(row.rustSize != null ? fmtBytes(row.rustSize) : '—');
    if (hasJs) {
        cells.push(row.jsSize != null ? fmtBytes(row.jsSize) : '—');
        cells.push((row.foxSize != null && row.jsSize != null) ? `${(row.foxSize / row.jsSize).toFixed(2)}×` : '—');
        if (hasRust) {
            cells.push((row.rustSize != null && row.jsSize != null) ? `${(row.rustSize / row.jsSize).toFixed(2)}×` : '—');
        }
    }
    return cells;
}

function geomean(xs) {
    if (xs.length === 0) return null;
    let sum = 0;
    for (const x of xs) sum += Math.log(x);
    return Math.exp(sum / xs.length);
}

function renderTable(title, headers, rowOf, foxRatioIdx, rustRatioIdx, foxRatio, rustRatio) {
    const widths = headers.map(h => h.length);
    const renderRow = (cells) => cells.map((c, i) => c.padEnd(widths[i])).join('  ');
    for (const row of rows) {
        const cells = rowOf(row);
        cells.forEach((c, i) => { widths[i] = Math.max(widths[i], c.length); });
    }
    const summaryCells = new Array(headers.length).fill('—');
    if (rows.length > 1 && hasJs) {
        summaryCells[0] = 'geomean';
        if (foxRatio != null) summaryCells[foxRatioIdx] = `${foxRatio.toFixed(2)}×`;
        if (hasRust && rustRatio != null) summaryCells[rustRatioIdx] = `${rustRatio.toFixed(2)}×`;
        summaryCells.forEach((c, i) => { widths[i] = Math.max(widths[i], c.length); });
    }
    console.log(title);
    console.log(renderRow(headers));
    console.log(widths.map(w => '-'.repeat(w)).join('  '));
    for (const row of rows) console.log(renderRow(rowOf(row)));
    if (rows.length > 1 && hasJs) console.log(renderRow(summaryCells));
}

const foxJsSpeedRatios = rows
    .filter(r => r.fox && r.js && r.fox.hz > 0 && r.js.hz > 0)
    .map(r => r.fox.hz / r.js.hz);
const rustJsSpeedRatios = rows
    .filter(r => r.rust && r.js && r.rust.hz > 0 && r.js.hz > 0)
    .map(r => r.rust.hz / r.js.hz);
const foxJsSizeRatios = rows
    .filter(r => r.foxSize != null && r.jsSize != null && r.foxSize > 0 && r.jsSize > 0)
    .map(r => r.foxSize / r.jsSize);
const rustJsSizeRatios = rows
    .filter(r => r.rustSize != null && r.jsSize != null && r.rustSize > 0 && r.jsSize > 0)
    .map(r => r.rustSize / r.jsSize);

const speedHeaders = buildHeaders('mean', 'rust/js');
const sizeHeaders = buildHeaders('size', 'rust/js size');

renderTable(
    'Speed (mean time, lower is better)',
    speedHeaders,
    speedRowOf,
    speedHeaders.indexOf('fox/js'),
    speedHeaders.indexOf('rust/js'),
    geomean(foxJsSpeedRatios),
    geomean(rustJsSpeedRatios),
);
console.log();
renderTable(
    'Size (bytes, lower is better)',
    sizeHeaders,
    sizeRowOf,
    sizeHeaders.indexOf('fox/js'),
    sizeHeaders.indexOf('rust/js size'),
    geomean(foxJsSizeRatios),
    geomean(rustJsSizeRatios),
);
