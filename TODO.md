# Fox Compiler & Standard Library TODO List

This document outlines prioritized action items to groom the Fox compiler codebase, harden the type system, improve user feedback with precise compiler diagnostics, and expand the standard library to make Fox a production-ready WebAssembly language.

---

## Priority 1: Standard Library Expansion (Application Capabilities)

The standard library (`std/`) lacks several vital modules needed to build complete applications.

- [ ] **Iterator combinators (`std::iter`)**
  - Introduce a standard `Iterator` trait and generic combinators such as `.map()`, `.filter()`, `.fold()`, `.zip()`, and `.enumerate()`.
- [ ] **File System & Paths (`std::fs`, `std::path`)**
  - Add path parsing (e.g. extension, basename, join).
  - Add basic file I/O operations (backed by WASI or JS host calls depending on the environment).
- [x] **Time Manipulation (`std::time`)**
  - Support high-level Date/Time formatting and parsing operations.
- [ ] **Regular Expressions (`std::regex`)**
  - Add regex search, replace, and match utilities utilizing JS-RegExp bindings.
- [ ] **Common Encoders (`std::encoding::base64`, `std::encoding::hex`)**
  - Implement base64 encoding/decoding and hex utilities.

---

## Priority 2: Testing & CI Infrastructure (Quality Assurance)

Currently, testing relies entirely on JS integration tests via Vitest. There are no unit tests for the compiler components in Rust.

- [ ] **Add Rust Unit Tests**
  - Implement unit tests for `src/lexer.rs` (verifying token stream matches source) and `src/parser.rs` (verifying AST shapes).
- [ ] **Introduce Compiler UI Testing**
  - Add tests that compile invalid source code and compare the diagnostic output against saved snapshot files to guarantee compiler errors remain descriptive and clean.
