# Agents

- Always delete temporary files when done if they are not part of the source code.

# Benchmarks

- Files ending in `.bench.fox` are picked up by `npm run bench` (which runs `fox.bench.mjs`).
- Exported functions whose name starts with `bench_` are registered as benchmark tasks (everything else is ignored).
- Generated artifacts are written to `.fox-benchs/`.
- `std/global/performance.fox` exposes `Performance::now(): f64`, a host wrapper that returns `performance.now()` (or `Date.now()` as a fallback).
- For head-to-head comparisons, place a `.bench.fox` file and a Rust crate in the same `benchmarks/<name>/` directory. The driver will compile both and report `rust/fox` throughput ratio. Requires `rustup target add wasm32-unknown-unknown`.
