# ORDYN Master Roadmap

**Baseline:** v0.1.0  
**Status:** Core Ecosystem Delivered & Verified  
**Engineering Methodology:** Incremental, Test-Driven, Security-First  

---

## Milestone Delivery Matrix

| Milestone | Title | Scope & Deliverables | Status | Test Evidence |
|---|---|---|---|---|
| **M1** | **Language Foundations** | Formal syntax, EBNF grammar, lexer, AST, parser, basic types, type checker, tests. | **Complete & Verified** | 10 passing tests |
| **M2** | **Core Semantics** | Six constructs (`entity`, `capability`, `contract`, `flow`, `boundary`, `interface`), effect analysis (`ordyn-effects`), TIR SSA form (`ordyn-ir`). | **Complete & Verified** | 4 passing tests |
| **M3** | **Security Foundations** | Security Kernel (`ordyn-guard`), capability table, monotonic attenuation lattice, cascading revocation, audit engine. | **Complete & Verified** | 3 passing tests |
| **M4** | **Compiler & Execution** | WebAssembly binary emitter, sandboxed runtime (`ordyn-runtime`), contract assertion enforcement, fuel governor. | **Complete & Verified** | 3 passing tests |
| **M5** | **Application Development** | Standard library (`ordyn-stdlib`), transactional ACID key-value persistence, HTTP client/transport, secure banking application. | **Complete & Verified** | 3 passing tests |
| **M6** | **Developer Tooling & Forge**| Developer CLI (`ordyn check`, `build`, `run`, `verify`, `package`, `inspect`), package manager (`ordyn-forge`) with zero-ambient authority. | **Complete & Verified** | 2 passing tests |
| **M7** | **Language Server (LSP)** | Language service (`ordyn-lsp`), diagnostics, document symbols, hover tooltips for IDE integration. | **Complete & Verified** | 3 passing tests |
| **M8** | **Security Validation & Red Lab**| Offensive security validation suite (`ordyn-red-lab`), capability forgery tests, attenuation widening attacks, fuel exhaustion tests. | **Complete & Verified** | 4 passing tests |
| **M9** | **Public Ecosystem Release** | Open-source public repository, signed manifests, comprehensive architecture documentation, formal risk assessment. | **Complete & Verified** | Published to GitHub |
