# ORDYN Language Support for VS Code & Cursor

Official Visual Studio Code and Cursor extension for **ORDYN** — the security-first sovereign programming language.

## Features

- **Full Syntax Highlighting:** Native TextMate grammar covering all 6 foundational constructs (`entity`, `capability`, `contract`, `flow`, `boundary`, `interface`).
- **Contract & Effect Semantics:** Distinct highlight rules for `requires`, `ensures`, `invariant`, and `with [...]` effect sets.
- **Language Server Integration (LSP):** Real-time diagnostics, capability boundary warnings, and document symbols powered by `ordyn lsp`.
- **Bracket Matching & Auto-closing:** Smart indentation and pair closing for scopes, contracts, and capability blocks.

## Installation

### Method 1: Install from VSIX package
```bash
code --install-extension editors/code/ordyn-language-0.1.0.vsix
```

For Cursor:
```bash
cursor --install-extension editors/code/ordyn-language-0.1.0.vsix
```

### Method 2: Manual Link
Copy the `editors/code` folder into your VS Code extensions directory:
- **Windows:** `%USERPROFILE%\.vscode\extensions\ordyn-language`
- **Linux / macOS:** `~/.vscode/extensions/ordyn-language`

## Language Server (LSP) Setup

Ensure the `ordyn` binary is in your system `PATH`:
```bash
cargo install --path crates/ordyn-cli
ordyn lsp
```
