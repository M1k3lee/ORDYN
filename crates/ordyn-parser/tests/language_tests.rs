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

    let mut parser =
        Parser::from_source(&source).expect("lexer should succeed on canonical example");
    let program = parser
        .parse_program()
        .expect("parser should succeed on canonical example");

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

#[test]
fn test_while_loop_and_control_flow() {
    let source = r#"
fn compute_sum(max: Int) -> Int {
    mut sum = 0;
    mut i = 0;
    while i < max {
        if i == 5 {
            i = i + 1;
            continue;
        }
        if i > 10 {
            break;
        }
        sum = sum + i;
        i = i + 1;
    }
    return sum;
}
"#;
    let mut parser = Parser::from_source(source).unwrap();
    let program = parser.parse_program().unwrap();
    assert_eq!(program.declarations.len(), 1);

    if let Declaration::Function(f) = &program.declarations[0] {
        assert_eq!(f.name.name, "compute_sum");
        assert_eq!(f.body.statements.len(), 4);
        assert!(matches!(f.body.statements[2], Stmt::While { .. }));
    } else {
        panic!("Expected function declaration");
    }
}

#[test]
fn test_enum_and_match_expressions() {
    let source = r#"
enum TransactionState {
    Pending,
    Approved(Int),
    Rejected(String)
}

fn handle_state(state: TransactionState) -> String {
    let result = match state {
        TransactionState::Pending => "Waiting",
        TransactionState::Approved(code) => "Success",
        _ => "Failed"
    };
    return result;
}
"#;
    let mut parser = Parser::from_source(source).unwrap();
    let program = parser.parse_program().unwrap();
    assert_eq!(program.declarations.len(), 2);

    if let Declaration::Enum(e) = &program.declarations[0] {
        assert_eq!(e.name.name, "TransactionState");
        assert_eq!(e.variants.len(), 3);
        assert_eq!(e.variants[0].name.name, "Pending");
        assert!(e.variants[0].payload.is_none());
        assert_eq!(e.variants[1].name.name, "Approved");
        assert!(e.variants[1].payload.is_some());
    } else {
        panic!("Expected enum declaration");
    }
}
