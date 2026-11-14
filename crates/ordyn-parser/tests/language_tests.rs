use ordyn_ast::*;
use ordyn_parser::Parser;
use std::fs;

#[test]
fn test_canonical_example_parses_completely() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let root_path = std::path::Path::new(&manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let file_path = root_path.join("examples").join("hello_boundary.ordyn");

    let source = fs::read_to_string(&file_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", file_path.display(), e));

    let mut parser = Parser::from_source(&source).expect("lexer should succeed on canonical example");
    let program = parser.parse_program().expect("parser should succeed on canonical example");

    assert_eq!(program.declarations.len(), 5);

    match &program.declarations[0] {
        Declaration::Capability(cap) => {
            assert_eq!(cap.name.name, "NetConnect");
            assert_eq!(cap.fields.len(), 2);
        }
        _ => panic!("Expected Capability declaration"),
    }

    match &program.declarations[1] {
        Declaration::Capability(cap) => {
            assert_eq!(cap.name.name, "AuditLog");
            assert_eq!(cap.fields.len(), 1);
        }
        _ => panic!("Expected Capability declaration"),
    }

    match &program.declarations[2] {
        Declaration::Interface(iface) => {
            assert_eq!(iface.name.name, "PaymentProcessor");
            assert_eq!(iface.methods.len(), 1);
            assert_eq!(iface.methods[0].effects.len(), 2);
        }
        _ => panic!("Expected Interface declaration"),
    }

    match &program.declarations[3] {
        Declaration::Entity(entity) => {
            assert_eq!(entity.name.name, "BankAccount");
            assert_eq!(entity.members.len(), 4);
        }
        _ => panic!("Expected Entity declaration"),
    }

    match &program.declarations[4] {
        Declaration::Boundary(b) => {
            assert_eq!(b.name.name, "SecureBankingBoundary");
            assert_eq!(b.grants.len(), 2);
            assert!(b.quota.is_some());
            assert_eq!(b.bindings.len(), 1);
            assert_eq!(b.bindings[0].name, "BankAccount");
        }
        _ => panic!("Expected Boundary declaration"),
    }
}
