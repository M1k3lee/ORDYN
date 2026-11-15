# ORDYN Implementation Status Matrix

**Last Updated:** Milestone 3 Foundations (Security Kernel & Capability Engine)  
**Reference Specification:** ORDYN Language Specification v0.1  

---

## 1. Feature & Component Matrix

| Subsystem | Feature | Status | Automated Test Coverage | Verification Status | Notes |
|---|---|---|---|---|---|
| **Language Specs** | EBNF Formal Grammar | Implemented | Verified via parser | Documented | `docs/language/GRAMMAR.md` |
| **Language Specs** | Type System Spec | Implemented | Specification complete | Documented | `docs/language/TYPE_SYSTEM.md` |
| **Language Specs** | Effect System Spec | Implemented | Specification complete | Documented | `docs/language/EFFECTS.md` |
| **Language Specs** | Capability Model | Implemented | Specification complete | Documented | `docs/language/CAPABILITIES.md` |
| **Language Specs** | Contract Model | Implemented | Specification complete | Documented | `docs/language/CONTRACTS.md` |
| **Compiler (Core)** | Diagnostics (`ordyn-diagnostics`) | Implemented | Integration verified | Tested | Source spans, miette diagnostics |
| **Compiler (Core)** | AST (`ordyn-ast`) | Implemented | Integration verified | Tested | All 6 constructs represented |
| **Compiler (Core)** | Lexer (`ordyn-lexer`) | Implemented | 3 unit tests passing | Tested | Zero-copy tokens, spans, keywords |
| **Compiler (Core)** | Parser (`ordyn-parser`)| Implemented | 3 unit/integration tests | Tested | Recursive descent, Pratt expressions |
| **Compiler (Core)** | Type Checker (`ordyn-types`)| Implemented | 4 unit tests passing | Tested | Scopes, type inference, contracts, mutability |
| **Compiler (Core)** | Effect Analyzer (`ordyn-effects`)| Implemented | 3 unit tests passing | Tested | Algebraic effects & caller propagation |
| **Compiler (Core)** | Typed IR (`ordyn-ir`) | Implemented | 1 unit test passing | Tested | SSA form, basic blocks, ContractAssert |
| **Security (Guard)**| Security Kernel (`ordyn-guard`) | Implemented | 3 unit tests passing | Tested | Opaque handles, monotonic attenuation, revocation |
| **Security (Guard)**| Capability Table | Implemented | Table lookup verified | Tested | Unforgeable handle directory in kernel memory |
| **Security (Guard)**| Audit Engine | Implemented | Audit events recorded | Tested | Monotonic timestamped binary records |
| **CLI Tooling** | `ordyn` CLI (`ordyn-cli`)| Implemented | Verified on canonical example | Tested | `check`, `verify`, `new` commands |
| **Compiler (Core)** | WASM Backend | Planned (M4) | Planned | Planned | Deterministic WASM output |
| **Runtime** | Wasmtime Host | Planned (M4) | Planned | Planned | WASM sandbox isolation |
| **Runtime** | Fuel / Memory Limits | Planned (M4) | Planned | Planned | DoS prevention |

---

## 2. Test Evidence Log

- `ordyn_lexer::tests::test_lex_basic_tokens`: PASSED
- `ordyn_lexer::tests::test_lex_keywords_and_symbols`: PASSED
- `ordyn_lexer::tests::test_unterminated_string_error`: PASSED
- `ordyn_parser::tests::test_parse_entity_and_contracts`: PASSED
- `ordyn_parser::tests::test_parse_all_six_constructs`: PASSED
- `ordyn_parser::tests::language_tests::test_canonical_example_parses_completely`: PASSED
- `ordyn_types::tests::test_valid_program_typechecks`: PASSED
- `ordyn_types::tests::test_immutable_assignment_rejected`: PASSED
- `ordyn_types::tests::test_contract_non_boolean_rejected`: PASSED
- `ordyn_types::tests::test_type_mismatch_rejected`: PASSED
- `ordyn_effects::tests::test_pure_function_calling_pure_function_allowed`: PASSED
- `ordyn_effects::tests::test_pure_function_calling_effectful_function_rejected`: PASSED
- `ordyn_effects::tests::test_effectful_caller_with_matching_effects_allowed`: PASSED
- `ordyn_ir::tests::test_lower_function_with_contracts`: PASSED
- `ordyn_guard::tests::test_valid_capability_authorization`: PASSED
- `ordyn_guard::tests::test_forged_handle_trapped`: PASSED
- `ordyn_guard::tests::test_monotonic_attenuation_and_cascading_revocation`: PASSED
- CLI verification: `ordyn check examples/hello_boundary.ordyn` -> `TIR Lowering succeeded: generated 0 functions, 1 entities.`
- CLI verification: `ordyn verify examples/hello_boundary.ordyn` -> `Static verification baseline passed for 5 declarations.`
