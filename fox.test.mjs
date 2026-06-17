import { execSync } from 'child_process';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { describe, test } from 'vitest';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
process.env.FOX_PATH = __dirname;


// Find all test files
function findTests(dir) {
    let results = [];
    const list = fs.readdirSync(dir);
    for (let file of list) {
        const fullPath = path.join(dir, file);
        const stat = fs.statSync(fullPath);
        if (stat && stat.isDirectory()) {
            results = results.concat(findTests(fullPath));
        } else if (file.endsWith('.test.fox')) {
            results.push(fullPath);
        }
    }
    return results;
}

const stdDir = path.join(__dirname, 'std');
const testFiles = findTests(stdDir);
const outDir = path.join(__dirname, '.fox-tests');

if (!fs.existsSync(outDir)) {
    fs.mkdirSync(outDir, { recursive: true });
}

// Build the fox compiler first to ensure it's up to date
execSync('cargo build', { stdio: 'inherit' });

for (const testFile of testFiles) {
    const relativePath = path.relative(__dirname, testFile);
    
    // Compile the test file
    const baseName = path.basename(testFile, '.fox'); // map.test
    execSync(`cargo run -- "${testFile}" -o "${outDir}"`, { 
        stdio: 'inherit',
    });

    const jsPath = path.join(outDir, `${baseName}.js`);
    const wasmPath = path.join(outDir, `${baseName}.wasm`);

    // Import the compiled wrapper
    const { fox } = await import(`file://${jsPath}?cb=${Date.now()}`);
    
    // Read the wasm buffer
    const wasmBuffer = fs.readFileSync(wasmPath);
    
    // Instantiate
    const instance = await fox(wasmBuffer);
    
    // Inject mock window for DOM tests
    global.window = {
        document: {
            createElement: (tag) => ({ tag, setAttribute: () => {}, appendChild: () => {} }),
            querySelector: (sel) => ({ sel, setAttribute: () => {}, appendChild: () => {}, textContent: '' })
        },
        alert: () => {}
    };
    global.document = global.window.document;

    // Inject mock fetch for HTTP tests
    global.fetch = async (url, options = {}) => {
        if (url.startsWith("https://example.com/")) {
            const path = url.substring("https://example.com/".length);
            if (path === "get") {
                return {
                    status: 200,
                    statusText: "OK",
                    text: async () => "hello get"
                };
            }
            if (path === "post") {
                return {
                    status: 200,
                    statusText: "OK",
                    text: async () => "hello post: " + (options.body || "")
                };
            }
            if (path === "headers") {
                return {
                    status: 200,
                    statusText: "OK",
                    text: async () => JSON.stringify(options.headers || {})
                };
            }
            if (path === "error") {
                throw new Error("mock connection error");
            }
        }
        throw new Error("mock fetch not matched: " + url);
    };

    const isAsyncSupport = typeof WebAssembly !== 'undefined' && typeof WebAssembly.promising === 'function';
    if (relativePath === 'std/global/task.test.fox') {
        console.log(`[JSPI] Executing task.test.fox. JSPI active: ${isAsyncSupport}, execArgv: ${JSON.stringify(process.execArgv)}`);
    }

    describe(relativePath, () => {
        // Find exported test functions
        for (const exportName of Object.keys(instance.exports)) {
            if (exportName.startsWith('test_') && typeof instance.exports[exportName] === 'function') {
                test(exportName, () => {
                    const fn = isAsyncSupport
                        ? WebAssembly.promising(instance.instance.exports[exportName])
                        : instance.exports[exportName];
                    return fn();
                });
            }
        }
    });
}

describe("Compiler Errors", () => {
    test("fails when type is inferred (let a = 5;)", () => {
        const errorTestFile = path.join(outDir, "error_test.fox");
        fs.writeFileSync(errorTestFile, `
pub fn test_error(): void {
    let a = 5;
}
`);
        try {
            execSync(`cargo run -- "${errorTestFile}" -o "${outDir}"`, {
                stdio: 'pipe',
            });
            throw new Error("Compilation should have failed but succeeded");
        } catch (err) {
            const stderr = err.stderr ? err.stderr.toString() : err.message;
            if (!stderr.includes("Type annotation is required for numeric literal 'a'")) {
                throw new Error(`Unexpected compiler output: ${stderr}`);
            }
        } finally {
            if (fs.existsSync(errorTestFile)) {
                fs.unlinkSync(errorTestFile);
            }
        }
    });

    test("fails on type mismatch between i32 and i64", () => {
        const errorTestFile = path.join(outDir, "error_test.fox");
        fs.writeFileSync(errorTestFile, `
pub fn test_error(): void {
    let a: i64 = 5;
    let b: i32 = 10;
    a = b;
}
`);
        try {
            execSync(`cargo run -- "${errorTestFile}" -o "${outDir}"`, {
                stdio: 'pipe',
            });
            throw new Error("Compilation should have failed but succeeded");
        } catch (err) {
            const stderr = err.stderr ? err.stderr.toString() : err.message;
            if (!stderr.includes("Type mismatch: expected 'i64', found 'i32'")) {
                throw new Error(`Unexpected compiler output: ${stderr}`);
            }
        } finally {
            if (fs.existsSync(errorTestFile)) {
                fs.unlinkSync(errorTestFile);
            }
        }
    });

    test("fails when user-defined extern function uses protected prefix", () => {
        const errorTestFile = path.join(outDir, "error_test.fox");
        fs.writeFileSync(errorTestFile, `
extern fn __fox_foo(): void;
pub fn test_error(): void {
}
`);
        try {
            execSync(`cargo run -- "${errorTestFile}" -o "${outDir}"`, {
                stdio: 'pipe',
            });
            throw new Error("Compilation should have failed but succeeded");
        } catch (err) {
            const stderr = err.stderr ? err.stderr.toString() : err.message;
            if (!stderr.includes("User-defined extern function '__fox_foo' cannot use the protected standard library prefix '__fox_'")) {
                throw new Error(`Unexpected compiler output: ${stderr}`);
            }
        } finally {
            if (fs.existsSync(errorTestFile)) {
                fs.unlinkSync(errorTestFile);
            }
        }
    });

    test("fails when user-defined constant uses protected prefix", () => {
        const errorTestFile = path.join(outDir, "error_test.fox");
        fs.writeFileSync(errorTestFile, `
const __fox_bar: i32 = 42;
pub fn test_error(): void {
}
`);
        try {
            execSync(`cargo run -- "${errorTestFile}" -o "${outDir}"`, {
                stdio: 'pipe',
            });
            throw new Error("Compilation should have failed but succeeded");
        } catch (err) {
            const stderr = err.stderr ? err.stderr.toString() : err.message;
            if (!stderr.includes("User-defined constant '__fox_bar' cannot use the protected standard library prefix '__fox_'")) {
                throw new Error(`Unexpected compiler output: ${stderr}`);
            }
        } finally {
            if (fs.existsSync(errorTestFile)) {
                fs.unlinkSync(errorTestFile);
            }
        }
    });

    test("does not treat capitalized identifier in condition as struct init", () => {
        const testFile = path.join(outDir, "parser_test.fox");
        fs.writeFileSync(testFile, `
const PI: f64 = 3.141592653589793;
pub fn test_parser(x: f64): f64 {
    if x < PI {
        return PI;
    }
    return x;
}
`);
        try {
            execSync(`cargo run -- "${testFile}" -o "${outDir}"`, {
                stdio: 'pipe',
            });
        } finally {
            if (fs.existsSync(testFile)) {
                fs.unlinkSync(testFile);
            }
        }
    });

    test("fails when if condition is not a boolean", () => {
        const errorTestFile = path.join(outDir, "error_test.fox");
        fs.writeFileSync(errorTestFile, `
pub fn test_error(): void {
    if "asdf" {
        return;
    }
}
`);
        try {
            execSync(`cargo run -- "${errorTestFile}" -o "${outDir}"`, {
                stdio: 'pipe',
            });
            throw new Error("Compilation should have failed but succeeded");
        } catch (err) {
            const stderr = err.stderr ? err.stderr.toString() : err.message;
            if (!stderr.includes("if condition must be a boolean")) {
                throw new Error(`Unexpected compiler output: ${stderr}`);
            }
        } finally {
            if (fs.existsSync(errorTestFile)) {
                fs.unlinkSync(errorTestFile);
            }
        }
    });

    test("fails when while condition is not a boolean", () => {
        const errorTestFile = path.join(outDir, "error_test.fox");
        fs.writeFileSync(errorTestFile, `
pub fn test_error(): void {
    while 1 {
        return;
    }
}
`);
        try {
            execSync(`cargo run -- "${errorTestFile}" -o "${outDir}"`, {
                stdio: 'pipe',
            });
            throw new Error("Compilation should have failed but succeeded");
        } catch (err) {
            const stderr = err.stderr ? err.stderr.toString() : err.message;
            if (!stderr.includes("while condition must be a boolean")) {
                throw new Error(`Unexpected compiler output: ${stderr}`);
            }
        } finally {
            if (fs.existsSync(errorTestFile)) {
                fs.unlinkSync(errorTestFile);
            }
        }
    });

    test("allows inherent impl on a user-defined type", () => {
        const errorTestFile = path.join(outDir, "error_test.fox");
        fs.writeFileSync(errorTestFile, `
struct Foo {
    x: i32;
}

impl Foo {
    pub fn bar(): i32 {
        return 42;
    }
}

pub fn test_error(): void {
}
`);
        try {
            execSync(`cargo run -- "${errorTestFile}" -o "${outDir}"`, {
                stdio: 'pipe',
            });
        } catch (err) {
            throw new Error(`Inherent impl on user-defined type should be allowed, but got: ${err.stderr ? err.stderr.toString() : err.message}`);
        } finally {
            if (fs.existsSync(errorTestFile)) {
                fs.unlinkSync(errorTestFile);
            }
        }
    });

    test("fails when user file declares inherent impl on a builtin outside FOX_PATH", () => {
        const errorTestFile = path.join(outDir, "error_test.fox");
        fs.writeFileSync(errorTestFile, `
impl str {
    pub compiler fn my_method(): i32;
}

pub fn test_error(): void {
}
`);
        try {
            execSync(`cargo run -- "${errorTestFile}" -o "${outDir}"`, {
                stdio: 'pipe',
            });
            throw new Error("Compilation should have failed but succeeded");
        } catch (err) {
            const stderr = err.stderr ? err.stderr.toString() : err.message;
            if (!stderr.includes("Inherent impl `impl str { ... }` is only allowed inside the standard library (FOX_PATH)")) {
                throw new Error(`Unexpected compiler output: ${stderr}`);
            }
        } finally {
            if (fs.existsSync(errorTestFile)) {
                fs.unlinkSync(errorTestFile);
            }
        }
    });

    test("fails when matching Ok/Err on Option type", () => {
        const errorTestFile = path.join(outDir, "error_test.fox");
        fs.writeFileSync(errorTestFile, `
use std::collections::{Option};
pub fn test_error(): void {
    let opt: Option<i32> = Option<i32>::none();
    match opt {
        Ok(v) => {},
        Err(e) => {},
    };
}
`);
        try {
            execSync(`cargo run -- "${errorTestFile}" -o "${outDir}"`, {
                stdio: 'pipe',
            });
            throw new Error("Compilation should have failed but succeeded");
        } catch (err) {
            const stderr = err.stderr ? err.stderr.toString() : err.message;
            if (!stderr.includes("Cannot match Result patterns (Ok/Err) on non-Result type")) {
                throw new Error(`Unexpected compiler output: ${stderr}`);
            }
        } finally {
            if (fs.existsSync(errorTestFile)) {
                fs.unlinkSync(errorTestFile);
            }
        }
    });

    test("fails when matching Some/None on Result type", () => {
        const errorTestFile = path.join(outDir, "error_test.fox");
        fs.writeFileSync(errorTestFile, `
use std::collections::{Result};
pub fn test_error(): void {
    let res: Result<i32, str> = Result<i32, str>::err("err");
    match res {
        Some(v) => {},
        None() => {},
    };
}
`);
        try {
            execSync(`cargo run -- "${errorTestFile}" -o "${outDir}"`, {
                stdio: 'pipe',
            });
            throw new Error("Compilation should have failed but succeeded");
        } catch (err) {
            const stderr = err.stderr ? err.stderr.toString() : err.message;
            if (!stderr.includes("Cannot match Option patterns (Some/None) on non-Option type")) {
                throw new Error(`Unexpected compiler output: ${stderr}`);
            }
        } finally {
            if (fs.existsSync(errorTestFile)) {
                fs.unlinkSync(errorTestFile);
            }
        }
    });

    test("fails when variable is named default", () => {
        const errorTestFile = path.join(outDir, "error_test.fox");
        fs.writeFileSync(errorTestFile, `
pub fn test_error(): void {
    let default: i32 = 42;
}
`);
        try {
            execSync(`cargo run -- "${errorTestFile}" -o "${outDir}"`, {
                stdio: 'pipe',
            });
            throw new Error("Compilation should have failed but succeeded");
        } catch (err) {
            const stderr = err.stderr ? err.stderr.toString() : err.message;
            if (!stderr.includes("Expected var name, got Default")) {
                throw new Error(`Unexpected compiler output: ${stderr}`);
            }
        } finally {
            if (fs.existsSync(errorTestFile)) {
                fs.unlinkSync(errorTestFile);
            }
        }
    });

    test("fails when match expression is non-exhaustive", () => {
        const errorTestFile = path.join(outDir, "error_test.fox");
        fs.writeFileSync(errorTestFile, `
use std::collections::{Option};
pub fn test_error(): void {
    let opt: Option<i32> = Option<i32>::none();
    match opt {
        Some(v) => {},
    };
}
`);
        try {
            execSync(`cargo run -- "${errorTestFile}" -o "${outDir}"`, {
                stdio: 'pipe',
            });
            throw new Error("Compilation should have failed but succeeded");
        } catch (err) {
            const stderr = err.stderr ? err.stderr.toString() : err.message;
            if (!stderr.includes("Match expression must be exhaustive: missing None")) {
                throw new Error(`Unexpected compiler output: ${stderr}`);
            }
        } finally {
            if (fs.existsSync(errorTestFile)) {
                fs.unlinkSync(errorTestFile);
            }
        }
    });

    test("fails when enum variant is declared without parentheses", () => {
        const errorTestFile = path.join(outDir, "error_test.fox");
        fs.writeFileSync(errorTestFile, `
enum Shape {
    Circle(i32);
    Point;
}
`);
        try {
            execSync(`cargo run -- "${errorTestFile}" -o "${outDir}"`, {
                stdio: 'pipe',
            });
            throw new Error("Compilation should have failed but succeeded");
        } catch (err) {
            const stderr = err.stderr ? err.stderr.toString() : err.message;
            if (!stderr.includes("Expected LParen")) {
                throw new Error(`Unexpected compiler output: ${stderr}`);
            }
        } finally {
            if (fs.existsSync(errorTestFile)) {
                fs.unlinkSync(errorTestFile);
            }
        }
    });

    test("allows struct instantiation to miss fields (defaulting to zero values)", () => {
        const testFile = path.join(outDir, "struct_defaults_test.fox");
        fs.writeFileSync(testFile, `
struct Point {
    x: i32;
    y: i32;
}

pub fn test_defaults(): void {
    let p = Point { x: 42 };
}
`);
        try {
            execSync(`cargo run -- "${testFile}" -o "${outDir}"`, {
                stdio: 'pipe',
            });
        } finally {
            if (fs.existsSync(testFile)) {
                fs.unlinkSync(testFile);
            }
        }
    });

    test("fails when struct instantiation has an unknown field", () => {
        const errorTestFile = path.join(outDir, "error_test.fox");
        fs.writeFileSync(errorTestFile, `
struct Point {
    x: i32;
}

pub fn test_error(): void {
    let p = Point { x: 42, y: 100 };
}
`);
        try {
            execSync(`cargo run -- "${errorTestFile}" -o "${outDir}"`, {
                stdio: 'pipe',
            });
            throw new Error("Compilation should have failed but succeeded");
        } catch (err) {
            const stderr = err.stderr ? err.stderr.toString() : err.message;
            if (!stderr.includes("Unknown field 'y' in instantiation of struct 'Point'")) {
                throw new Error(`Unexpected compiler output: ${stderr}`);
            }
        } finally {
            if (fs.existsSync(errorTestFile)) {
                fs.unlinkSync(errorTestFile);
            }
        }
    });

    test("fails when raw array types are used outside standard library", () => {
        const errorTestFile = path.join(outDir, "error_test.fox");
        fs.writeFileSync(errorTestFile, `
pub fn test_error(): void {
    let a: []i32 = default;
}
`);
        try {
            execSync(`cargo run -- "${errorTestFile}" -o "${outDir}"`, {
                stdio: 'pipe',
            });
            throw new Error("Compilation should have failed but succeeded");
        } catch (err) {
            const stderr = err.stderr ? err.stderr.toString() : err.message;
            if (!stderr.includes("Raw array types '[]T' are not allowed outside the standard library and benchmarks")) {
                throw new Error(`Unexpected compiler output: ${stderr}`);
            }
        } finally {
            if (fs.existsSync(errorTestFile)) {
                fs.unlinkSync(errorTestFile);
            }
        }
    });

    test("fails when raw array allocations are used outside standard library", () => {
        const errorTestFile = path.join(outDir, "error_test.fox");
        fs.writeFileSync(errorTestFile, `
pub fn test_error(): void {
    let a = new [10]i32;
}
`);
        try {
            execSync(`cargo run -- "${errorTestFile}" -o "${outDir}"`, {
                stdio: 'pipe',
            });
            throw new Error("Compilation should have failed but succeeded");
        } catch (err) {
            const stderr = err.stderr ? err.stderr.toString() : err.message;
            if (!stderr.includes("Raw array allocations are not allowed outside the standard library and benchmarks")) {
                throw new Error(`Unexpected compiler output: ${stderr}`);
            }
        } finally {
            if (fs.existsSync(errorTestFile)) {
                fs.unlinkSync(errorTestFile);
            }
        }
    });

    test("fails when struct instantiation has a field type mismatch", () => {
        const errorTestFile = path.join(outDir, "error_test.fox");
        fs.writeFileSync(errorTestFile, `
struct Point {
    x: i32;
}

pub fn test_error(): void {
    let p = Point { x: "hello" };
}
`);
        try {
            execSync(`cargo run -- "${errorTestFile}" -o "${outDir}"`, {
                stdio: 'pipe',
            });
            throw new Error("Compilation should have failed but succeeded");
        } catch (err) {
            const stderr = err.stderr ? err.stderr.toString() : err.message;
            if (!stderr.includes("Type mismatch for field 'x'")) {
                throw new Error(`Unexpected compiler output: ${stderr}`);
            }
        } finally {
            if (fs.existsSync(errorTestFile)) {
                fs.unlinkSync(errorTestFile);
            }
        }
    });

    test("fails when passing invalid type to method call argument", () => {
        const errorTestFile = path.join(outDir, "error_test.fox");
        fs.writeFileSync(errorTestFile, `
use std::global::{Element, Document};

pub struct Div {
    ref: Element;
}

pub fn test_error(): void {
    let app_el = Document::create_element("div");
    let d = Div { ref: Document::create_element("div") };
    app_el.append_child(d);
}
`);
        try {
            execSync(`cargo run -- "${errorTestFile}" -o "${outDir}"`, {
                stdio: 'pipe',
            });
            throw new Error("Compilation should have failed but succeeded");
        } catch (err) {
            const stderr = err.stderr ? err.stderr.toString() : err.message;
            if (!stderr.includes("Type mismatch: expected 'dom::Element', found 'Div'")) {
                throw new Error(`Unexpected compiler output: ${stderr}`);
            }
        } finally {
            if (fs.existsSync(errorTestFile)) {
                fs.unlinkSync(errorTestFile);
            }
        }
    });

    test("fails when accessing tuple using f-prefixed names", () => {
        const errorTestFile = path.join(outDir, "error_test.fox");
        fs.writeFileSync(errorTestFile, `
pub fn test_error(): void {
    let t: (i32, str) = (42, "test");
    let val = t.f0;
}
`);
        try {
            execSync(`cargo run -- "${errorTestFile}" -o "${outDir}"`, {
                stdio: 'pipe',
            });
            throw new Error("Compilation should have failed but succeeded");
        } catch (err) {
            const stderr = err.stderr ? err.stderr.toString() : err.message;
            if (!stderr.includes("Tuple elements can only be accessed by index (e.g. '.0'), found '.f0'")) {
                throw new Error(`Unexpected compiler output: ${stderr}`);
            }
        } finally {
            if (fs.existsSync(errorTestFile)) {
                fs.unlinkSync(errorTestFile);
            }
        }
    });

    test("fails when let type annotation mismatches initializer (string to int)", () => {
        const errorTestFile = path.join(outDir, "error_test.fox");
        fs.writeFileSync(errorTestFile, `
pub fn test_error(): void {
    let foo: i32 = "asdf";
}
`);
        try {
            execSync(`cargo run -- "${errorTestFile}" -o "${outDir}"`, {
                stdio: 'pipe',
            });
            throw new Error("Compilation should have failed but succeeded");
        } catch (err) {
            const stderr = err.stderr ? err.stderr.toString() : err.message;
            if (!stderr.includes("Type mismatch: expected 'i32', found 'str'")) {
                throw new Error(`Unexpected compiler output: ${stderr}`);
            }
        } finally {
            if (fs.existsSync(errorTestFile)) {
                fs.unlinkSync(errorTestFile);
            }
        }
    });

    test("fails when let type annotation mismatches initializer (string to bool)", () => {
        const errorTestFile = path.join(outDir, "error_test.fox");
        fs.writeFileSync(errorTestFile, `
pub fn test_error(): void {
    let bar: bool = "hello";
}
`);
        try {
            execSync(`cargo run -- "${errorTestFile}" -o "${outDir}"`, {
                stdio: 'pipe',
            });
            throw new Error("Compilation should have failed but succeeded");
        } catch (err) {
            const stderr = err.stderr ? err.stderr.toString() : err.message;
            if (!stderr.includes("Type mismatch: expected 'bool', found 'str'")) {
                throw new Error(`Unexpected compiler output: ${stderr}`);
            }
        } finally {
            if (fs.existsSync(errorTestFile)) {
                fs.unlinkSync(errorTestFile);
            }
        }
    });

    test("succeeds when let type annotation matches initializer", () => {
        const errorTestFile = path.join(outDir, "error_test.fox");
        fs.writeFileSync(errorTestFile, `
pub fn test_error(): void {
    let foo: i32 = 42;
    let bar: str = "hello";
    let baz: f64 = 3.14;
}
`);
        try {
            execSync(`cargo run -- "${errorTestFile}" -o "${outDir}"`, {
                stdio: 'pipe',
            });
        } catch (err) {
            const stderr = err.stderr ? err.stderr.toString() : err.message;
            throw new Error(`Compilation should have succeeded but failed: ${stderr}`);
        } finally {
            if (fs.existsSync(errorTestFile)) {
                fs.unlinkSync(errorTestFile);
            }
        }
    });
});
