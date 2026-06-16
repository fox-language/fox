# Fox Programming Language

Fox is a statically-typed, garbage-collected programming language designed to target WebAssembly natively. It compiles to WebAssembly Text (WAT) and utilizes standard WebAssembly Garbage Collection (Wasm GC) and JavaScript-String integration (`wasm:js-string` built-ins) for efficient execution and low-overhead memory management.

## Key Features

- **Strict Static Typing:** Strict type system without unsafe type inference; type annotations are required for numeric literals.
- **WebAssembly GC & JS-String Integration:** Native support for garbage-collected arrays, structs, and strings leveraging browser/host capabilities.
- **Custom Enums & Pattern Matching:** Powerful algebraic data types (enums with payloads) matched via exhaustive `match` expressions/statements, plus conditional bindings with `if let` and `while let`.
- **Tuples:** Support for heterogeneous tuple types, tuple destructuring, and element access (e.g. `t.0`).
- **Generics & Traits:** Polymorphism via generic structs, functions, and trait-based constraints.
- **Modules & Visibility:** File-based modules with strict explicit exports (`pub`) and imports (`use` / `as`).

## Compiler Overview

The Fox compiler is implemented in Rust. When compiling a `.fox` source file, it generates:
1. A `.wasm` WebAssembly binary.
2. A `.js` companion helper file which handles Wasm GC allocations, JS-String environment imports, and exports proxying.

## Getting Started

### Prerequisites

- Rust (Cargo)
- Node.js (with npm)
- A WebAssembly runtime/browser that supports Wasm GC and JS-String built-ins (e.g., modern Chrome, V8, or Node/V8 flag-enabled environments).

### Installation & Compilation

1. Build the compiler executable:
   ```bash
   cargo build --release
   ```

2. Compile a Fox program:
   ```bash
   target/release/fox path/to/file.fox -o output_dir/
   ```
   This will output `file.wasm` and `file.js` inside `output_dir/`.

## Running Tests

Fox comes with a comprehensive test suite for both compiler internals (in Rust) and standard library/integration features (written in Fox and run in Node.js).

- **Rust Unit Tests:**
  ```bash
  cargo test
  ```

- **Integration Tests (Vitest):**
  ```bash
  npm run test
  ```
  This builds the compiler, compiles all tests in `std/` to WebAssembly, and executes them within a mocked JS host environment under Vitest.

## Benchmarks

Benchmark tasks compare execution performance of Fox and Rust.

- **Running Benchmarks:**
  ```bash
  npm run bench
  ```
  This runs `fox.bench.mjs`, which scans for files ending with `.bench.fox`, registers exported functions prefixed with `bench_`, and runs them.

- **Head-to-Head Comparisons:**
  To compare Fox directly against Rust:
  1. Add the `.bench.fox` file and a Rust crate inside a folder under `benchmarks/<name>/`.
  2. Run the benchmarks (`npm run bench`). The driver will compile both and report the throughput ratio.
  *(Note: Requires the WebAssembly target installed: `rustup target add wasm32-unknown-unknown`)*

## Project Structure

- `src/` - Rust implementation of the compiler (lexer, parser, type checker, optimizer, and WAT/JS codegen).
- `std/` - The Fox standard library:
  - `std::testing` - Assertions (`assert_eq`, `assert_approx_eq`).
  - `std::global` - Environment APIs (`dom`, `console`, `performance`, `task`).
  - `std::collections` - Generic collections (`Option`, `Result`, `Vec`, `Map`, `Set`).
  - `std::math` - Mathematical utilities.
  - `std::string` - String extensions.
  - `std::crypto` - Hashing and random utilities.
  - `std::fmt` - String formatting (`sprintf`).
- `examples/` - Illustrative Fox source examples.
- `vscode-extension/` - VS Code syntax highlighting support for Fox files.
- `SPEC.md` - Technical specification of the Fox programming language.
