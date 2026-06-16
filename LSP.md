# Fox Language Server Protocol (LSP) Implementation Plan

This document outlines a structured, step-by-step plan to implement a Language Server Protocol (LSP) server for the Fox programming language. The LSP server will be integrated directly into the Rust compiler binary under a new `lsp` subcommand (`fox lsp`), enabling IDE features such as real-time diagnostics, go-to-definition, hover information, and outline/document symbols.

Each step in this plan is designed to be self-contained and modular, ensuring that the existing compiler and Vitest integration suites compile and pass successfully after every milestone.

---

## Technical Context: Span & Coordinate Mapping

A critical part of any LSP implementation is converting between coordinate systems. Fox's compiler and LSP use different conventions:

| Attribute | Fox Compiler (`Span`) | LSP (`Position`) |
| :--- | :--- | :--- |
| **Line Indexing** | **1-based** (e.g., Line 1 is the first line) | **0-based** (e.g., Line 0 is the first line) |
| **Column Indexing** | **1-based** UTF-8 character count | **0-based** UTF-16 code unit offset |
| **Offset Tracking** | `start` and `end` byte offsets in UTF-8 source | N/A (Uses Line/Character pairs) |

### Conversion Utility Specification
To prevent cursor mismatches, the following utility should be implemented early:

```rust
pub struct PositionConverter {
    // Stores byte offset of the start of each line
    line_offsets: Vec<usize>,
    source: String,
}

impl PositionConverter {
    pub fn new(source: &str) -> Self {
        let mut line_offsets = vec![0];
        for (i, c) in source.char_indices() {
            if c == '\n' {
                line_offsets.push(i + 1);
            }
        }
        Self {
            line_offsets,
            source: source.to_string(),
        }
    }

    /// Converts an LSP Position (0-indexed, UTF-16 characters) to a Fox byte offset.
    pub fn lsp_to_offset(&self, pos: &lsp_types::Position) -> Option<usize> {
        let line = pos.line as usize;
        if line >= self.line_offsets.len() {
            return None;
        }
        let line_start_offset = self.line_offsets[line];
        let line_end_offset = if line + 1 < self.line_offsets.len() {
            self.line_offsets[line + 1]
        } else {
            self.source.len()
        };
        
        let line_str = &self.source[line_start_offset..line_end_offset];
        let mut utf16_count = 0;
        let mut byte_offset = 0;
        
        for c in line_str.chars() {
            if utf16_count >= pos.character as usize {
                break;
            }
            utf16_count += c.len_utf16();
            byte_offset += c.len_utf8();
        }
        
        Some(line_start_offset + byte_offset)
    }

    /// Converts a Fox byte offset to an LSP Position.
    pub fn offset_to_lsp(&self, offset: usize) -> lsp_types::Position {
        let mut line = 0;
        for (i, &line_start) in self.line_offsets.iter().enumerate() {
            if offset >= line_start {
                line = i;
            } else {
                break;
            }
        }
        
        let line_start_offset = self.line_offsets[line];
        let line_str = &self.source[line_start_offset..offset.min(self.source.len())];
        let character = line_str.chars().map(|c| c.len_utf16()).sum::<usize>();
        
        lsp_types::Position::new(line as u32, character as u32)
    }
}
```

---

## Phase 1: LSP Server Infrastructure (Status: Completed ✅)

### Step 1.1: Add LSP Cargo Dependencies (Completed)
- **Goal**: Integrate necessary crates for implementing the LSP backend in Rust.
- **Tasks**:
  1. Open `Cargo.toml`.
  2. Add the following dependencies to the `[dependencies]` section:
     ```toml
     tower-lsp = "0.20.0"
     tokio = { version = "1.0", features = ["full"] }
     ```
  3. Run `cargo check` to fetch and compile the dependencies.
- **Verification & Safety**: 
  - Run `cargo test` and `npm test`. Both must pass with zero issues.
  - Adding dependencies does not affect the runtime code paths, ensuring 100% test compatibility.

### Step 1.2: Add Subcommand to Command Line Interface (Completed)
- **Goal**: Define the CLI structure to support launching the LSP server using `fox lsp`.
- **Tasks**:
  1. Open `src/main.rs`.
  2. Update the argument parser logic inside `fn main()` to recognize `lsp` as a command/argument.
  3. If `lsp` is specified, direct control to a placeholder async entry point `run_lsp()`.
  4. Example CLI integration:
     ```rust
     #[tokio::main]
     async fn main() {
         let args: Vec<String> = std::env::args().collect();
         if args.len() > 1 && args[1] == "lsp" {
             // Entry point for LSP server
             if let Err(e) = run_lsp().await {
                 eprintln!("LSP server error: {:?}", e);
                 std::process::exit(1);
             }
             return;
         }
         // ... existing compiler invocation logic ...
     }
     
     async fn run_lsp() -> Result<(), Box<dyn std::error::Error>> {
         Ok(())
     }
     ```
- **Verification & Safety**:
  - Run `cargo test` to ensure unit tests run correctly.
  - Run `npm test` to ensure integration tests pass (which execute `cargo run -- <file_path>` and do not invoke the `lsp` argument).
  - Run `cargo run -- lsp` manually; the process should terminate instantly and cleanly.

### Step 1.3: LSP Connection Loop & Lifecycle Handshake (Completed)
- **Goal**: Implement standard LSP connection initiation and shutdown protocol.
- **Tasks**:
  1. Implement a `FoxLanguageServer` struct that implements `tower_lsp::LanguageServer`.
  2. Stub the standard lifecycle methods (`initialize`, `initialized`, `shutdown`):
     ```rust
     use tower_lsp::jsonrpc::Result;
     use tower_lsp::lsp_types::*;
     use tower_lsp::{Client, LanguageServer, LspService, Server};

     struct Backend {
         client: Client,
     }

     #[tower_lsp::async_trait]
     impl LanguageServer for Backend {
         async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
             Ok(InitializeResult {
                 capabilities: ServerCapabilities {
                     text_document_sync: Some(TextDocumentSyncCapability::Kind(
                         TextDocumentSyncKind::FULL,
                     )),
                     ..Default::default()
                 },
                 ..Default::default()
             })
         }

         async fn initialized(&self, _: InitializedParams) {
             self.client.log_message(MessageType::INFO, "Fox Language Server initialized!").await;
         }

         async fn shutdown(&self) -> Result<()> {
             Ok(())
         }
     }
     ```
  3. Implement the async connection loop inside `run_lsp()` using `tokio::io::stdin()` and `tokio::io::stdout()`.
- **Verification & Safety**:
  - Run `npm test` to ensure normal compilation operations remain functional.
  - Test LSP handshake: Execute `cargo run -- lsp` and pipe a valid JSON-RPC initialization request into standard input. The server should respond with an `initialize` JSON-RPC response containing capabilities, then exit gracefully when sent a shutdown message.

---

## Phase 2: Document Sync & Diagnostics (Status: Completed ✅)

### Step 2.1: In-Memory Document Sync (Completed)
- **Goal**: Listen to document sync lifecycle notifications to cache working document buffers.
- **Tasks**:
  1. Add a thread-safe map (e.g., using `std::sync::Mutex` or `dashmap` crate) to `Backend` to store document texts associated with their `Url` identifiers.
  2. Implement `did_open`, `did_change`, and `did_close` callbacks on `Backend`.
  3. Ensure the document map is updated on open and change, and items are deleted on close.
- **Verification & Safety**:
  - Ensure all existing unit and integration tests compile and pass.
  - Verify that standard Rust tests can be added targeting `Backend` directly, asserting that simulated editor edits correctly update the in-memory document state.

### Step 2.2: Live Compiler Diagnostics Reporting (Completed)
- **Goal**: Parse and type-check files on change to emit visual errors to the IDE.
- **Tasks**:
  1. Intercept/adapt the compiler entry points (`parse_file` from `src/main.rs`) to load from the in-memory document cache instead of reading from disk.
  2. Provide a run pipeline that executes the parser and type-checker without performing code generation (`codegen`) to ensure minimal overhead.
  3. Collect compilation diagnostics from `DIAGNOSTICS` thread-local list (defined in `src/diagnostics.rs`).
  4. Convert compiler spans to LSP-compatible ranges using the `PositionConverter` utility.
  5. Publish these diagnostics back to the IDE client via `client.publish_diagnostics` inside the `did_open` and `did_change` event loops.
- **Verification & Safety**:
  - Keep diagnostics compiler integration clean so it doesn't modify any compiler internals used for code generation.
  - Verify `npm test` passes, including the compiler error tests (which compile erroneous files directly from disk).
  - Simulate a `did_change` event in a custom unit test and assert that an LSP diagnostic notification is published with exact, correct coordinates.

---

## Phase 3: Core IDE Features (Status: Completed ✅)

### Step 3.1: Go to Definition (Completed)
- **Goal**: Allow navigating to variable declarations, struct definitions, functions, and trait definitions.
- **Tasks**:
  1. Extend the compiler's semantic analysis phase to collect references and their corresponding definition sites in a mapping (`Span` -> `Span`).
  2. Implement the `goto_definition` request on the `Backend`.
  3. Retrieve the document's content, locate the cursor position using `PositionConverter`, find the AST node under the cursor, look up its definition span, and return the matching location.
- **Verification & Safety**:
  - Ensure `npm test` passes.
  - Verify manually or via a unit test by launching the server, feeding a mock source code with a function call, and asserting that querying the definition at the call-site returns the span of the function declaration.

### Step 3.2: Hover Support (Completed)
- **Goal**: Display type signatures and definitions when pointing the cursor at a symbol.
- **Tasks**:
  1. During the type-checking phase, construct a map of symbol spans to type strings (e.g., `i32`, `Option<T>`, or custom struct types).
  2. Implement the `hover` request on the `Backend`.
  3. When queried, look up the symbol at the cursor location and return a formatted markdown snippet representing its type signature and any documentation.
- **Verification & Safety**:
  - All compiler tests pass.
  - Unit tests verify that hovering over a variable returns its annotated or inferred type.

### Step 3.3: Document Symbols (Outline View) (Completed)
- **Goal**: Enable the outline view in editors, allowing users to quickly jump between structs, traits, constants, and functions.
- **Tasks**:
  1. Implement `document_symbol` request on the `Backend`.
  2. Traverse the parsed AST of the document and extract top-level items (`StructDef`, `Function`, `TraitDef`, `ConstDef`, `ImplDef`).
  3. Return a list of `DocumentSymbol` objects containing the symbol's name, kind (e.g. `SymbolKind::Struct` or `SymbolKind::Function`), range, and selection range.
- **Verification & Safety**:
  - Standard test suite passes.
  - Verify by sending a `textDocument/documentSymbol` JSON-RPC message and asserting that the JSON output lists all top-level symbols.

---

## Phase 4: Client Integration (Status: Completed ✅)

### Step 4.1: Integrate LSP Client into VS Code Extension (Completed)
- **Goal**: Update the VS Code extension to spawn and communicate with `fox lsp`.
- **Tasks**:
  1. Open `vscode-extension/package.json`.
  2. Add `@vscode/vsce` and `vscode-languageclient` dependencies.
  3. Create an extension activation file `vscode-extension/src/extension.ts` (or `.js`):
     ```typescript
     import * as path from 'path';
     import { workspace, ExtensionContext } from 'vscode';
     import { LanguageClient, LanguageClientOptions, ServerOptions } from 'vscode-languageclient/node';

     let client: LanguageClient;

     export function activate(context: ExtensionContext) {
         // Path to the compiler binary
         let serverExe = process.env.FOX_PATH ? path.join(process.env.FOX_PATH, 'target', 'debug', 'fox') : 'fox';
         
         let serverOptions: ServerOptions = {
             run: { command: serverExe, args: ['lsp'] },
             debug: { command: serverExe, args: ['lsp'] }
         };

         let clientOptions: LanguageClientOptions = {
             documentSelector: [{ scheme: 'file', language: 'fox' }],
             synchronize: {
                 fileEvents: workspace.createFileSystemWatcher('**/*.fox')
             }
         };

         client = new LanguageClient(
             'foxLsp',
             'Fox Language Server',
             serverOptions,
             clientOptions
         );

         client.start();
     }

     export function deactivate(): Thenable<void> | undefined {
         if (!client) {
             return undefined;
         }
         return client.stop();
     }
     ```
  4. Update `vscode-extension/package.json` to declare activation events and point to the main entry script.
- **Verification & Safety**:
  - Run the `vsce package` script to compile the extension.
  - Launch VS Code with the extension installed, load a `.fox` file, and verify the status bar, log outputs, and diagnostics highlight correctly.
  - Verify that standard compiler and Vitest integration suites run successfully and do not depend on the VS Code extension.
