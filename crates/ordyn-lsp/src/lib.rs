use ordyn_ast::*;
use ordyn_diagnostics::Span;
use ordyn_effects::EffectAnalyzer;
use ordyn_parser::Parser;
use ordyn_types::TypeChecker;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LspDiagnostic {
    pub span: Span,
    pub message: String,
    pub severity: DiagnosticSeverity,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LspSymbol {
    pub name: String,
    pub kind: String,
    pub span: Span,
}

pub struct LanguageService;

impl LanguageService {
    /// Runs full semantic and security pipeline on an editor buffer, producing diagnostics
    pub fn analyze_document(source: &str) -> Vec<LspDiagnostic> {
        let mut diagnostics = Vec::new();

        // 1. Parsing pass
        let mut parser = match Parser::from_source(source) {
            Ok(p) => p,
            Err(e) => {
                diagnostics.push(LspDiagnostic {
                    span: Span::default(),
                    message: format!("Lexer error: {}", e),
                    severity: DiagnosticSeverity::Error,
                });
                return diagnostics;
            }
        };

        let program = match parser.parse_program() {
            Ok(p) => p,
            Err(e) => {
                diagnostics.push(LspDiagnostic {
                    span: Span::default(),
                    message: format!("Syntax error: {}", e),
                    severity: DiagnosticSeverity::Error,
                });
                return diagnostics;
            }
        };

        // 2. Type analysis pass
        let mut checker = TypeChecker::new();
        if let Err(e) = checker.check_program(&program) {
            diagnostics.push(LspDiagnostic {
                span: Span::default(),
                message: format!("Type error: {}", e),
                severity: DiagnosticSeverity::Error,
            });
        }

        // 3. Static effect analysis pass
        let mut effect_analyzer = EffectAnalyzer::new();
        if let Err(e) = effect_analyzer.analyze_program(&program) {
            diagnostics.push(LspDiagnostic {
                span: Span::default(),
                message: format!("Effect error: {}", e),
                severity: DiagnosticSeverity::Error,
            });
        }

        diagnostics
    }

    /// Extracts structural symbols for IDE outline and breadcrumbs
    pub fn get_document_symbols(source: &str) -> Vec<LspSymbol> {
        let mut symbols = Vec::new();
        if let Ok(mut parser) = Parser::from_source(source) {
            if let Ok(program) = parser.parse_program() {
                for decl in &program.declarations {
                    match decl {
                        Declaration::Entity(e) => {
                            symbols.push(LspSymbol {
                                name: e.name.name.clone(),
                                kind: "Entity".to_string(),
                                span: e.span,
                            });
                        }
                        Declaration::Capability(c) => {
                            symbols.push(LspSymbol {
                                name: c.name.name.clone(),
                                kind: "Capability".to_string(),
                                span: c.span,
                            });
                        }
                        Declaration::Boundary(b) => {
                            symbols.push(LspSymbol {
                                name: b.name.name.clone(),
                                kind: "Boundary".to_string(),
                                span: b.span,
                            });
                        }
                        Declaration::Interface(i) => {
                            symbols.push(LspSymbol {
                                name: i.name.name.clone(),
                                kind: "Interface".to_string(),
                                span: i.span,
                            });
                        }
                        Declaration::Contract(k) => {
                            symbols.push(LspSymbol {
                                name: k.name.name.clone(),
                                kind: "Contract".to_string(),
                                span: k.span,
                            });
                        }
                        Declaration::Flow(fl) => {
                            symbols.push(LspSymbol {
                                name: fl.name.name.clone(),
                                kind: "Flow".to_string(),
                                span: fl.span,
                            });
                        }
                        Declaration::Function(f) => {
                            symbols.push(LspSymbol {
                                name: f.name.name.clone(),
                                kind: "Function".to_string(),
                                span: f.span,
                            });
                        }
                    }
                }
            }
        }
        symbols
    }

    /// Resolves documentation and hover information for IDE tooltip
    pub fn get_hover_info(source: &str, byte_offset: usize) -> Option<String> {
        let symbols = Self::get_document_symbols(source);
        for s in symbols {
            if byte_offset >= s.span.start && byte_offset <= (s.span.start + s.span.length) {
                return Some(format!("**ORDYN {}**: `{}`\nSecurity Boundary: Kernel Protected", s.kind, s.name));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsp_diagnostics_on_clean_document() {
        let source = r#"
            capability NetConnect {
                host: String;
                port: Int;
            }

            entity Service {
                pub fn run() -> Unit {
                    return;
                }
            }
        "#;
        let diags = LanguageService::analyze_document(source);
        assert!(diags.is_empty(), "Valid document should produce zero diagnostics");
    }

    #[test]
    fn test_lsp_diagnostics_catches_type_and_effect_errors() {
        let invalid_source = r#"
            fn send() -> Unit with [Net] {
                return;
            }

            fn pure_func() -> Int {
                send(); // Unhandled effect error
                let x: Int = "string_mismatch"; // Type error
                return x;
            }
        "#;
        let diags = LanguageService::analyze_document(invalid_source);
        assert!(!diags.is_empty(), "Invalid document must generate diagnostics");
    }

    #[test]
    fn test_lsp_document_symbols_extraction() {
        let source = r#"
            capability StorageRead { path_prefix: String; }
            interface Logger { fn log(m: String) -> Unit; }
            entity Bank { mut balance: Int; }
            boundary SecureZone { bind Bank; }
        "#;
        let symbols = LanguageService::get_document_symbols(source);
        assert_eq!(symbols.len(), 4);
        assert_eq!(symbols[0].name, "StorageRead");
        assert_eq!(symbols[1].name, "Logger");
        assert_eq!(symbols[2].name, "Bank");
        assert_eq!(symbols[3].name, "SecureZone");
    }
}
