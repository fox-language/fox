# Fox for VS Code

Syntax highlighting for the [Fox](https://github.com/anomalyco/fox) language. No LSP required.

## Features

- Token-level syntax highlighting for `.fox` files (including `.bench.fox` and `.test.fox`)
- Bracket pair colorization
- Auto-closing pairs (`{`, `[`, `(`, `"`)
- Comment toggling with `//`
- Scope names follow TextMate conventions, so any theme works out of the box

## Highlighting

| Construct           | Scope                              |
| ------------------- | ---------------------------------- |
| Keywords (`fn`, `let`, `if`, `match`, ...) | `keyword.*.fox`        |
| Primitive types (`i32`, `str`, `bool`, ...) | `storage.type.primitive.fox` |
| Strings, numbers, booleans | `string.*` / `constant.*`       |
| Function declarations | `entity.name.function.fox`        |
| Struct / trait / impl names | `entity.name.type.*.fox`        |
| `self`              | `variable.language.self.fox`       |
| `Some` / `None` / `Ok` / `Err` | `variable.other.enummember.fox` |
| `//` comments       | `comment.line.double-slash.fox`    |

## Install (local development)

1. Open the `vscode-extension/` folder in VS Code
2. Press `F5` to launch an Extension Development Host with the extension loaded
3. Open any `.fox` file to see highlighting

## Package for the marketplace

```sh
npm install -g @vscode/vsce
cd vscode-extension
vsce package
```

This produces a `fox-0.1.0.vsix` you can install with `code --install-extension fox-0.1.0.vsix`.
