use ordyn_ast::{Declaration, EntityMember, PrimitiveType, Program, TypeRef};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum WitError {
    #[error("Unsupported type in WIT generation: {0}")]
    UnsupportedType(String),
}

/// Generates standard WebAssembly Component Model WIT (WebAssembly Interface Types) specifications.
pub struct WitGenerator;

impl WitGenerator {
    /// Convert an ORDYN program into a complete, standard WIT document.
    pub fn generate_wit(
        program: &Program,
        package_namespace: &str,
        package_name: &str,
    ) -> Result<String, WitError> {
        let mut out = String::new();
        out.push_str(&format!(
            "package {}:{}@0.1.0;\n\n",
            package_namespace, package_name
        ));

        // 1. Emit Capabilities and Entities as WIT records/resources
        for decl in &program.declarations {
            match decl {
                Declaration::Capability(cap) => {
                    let kebab_name = to_kebab_case(&cap.name.name);
                    out.push_str(&format!("/// Sovereign Capability: {}\n", cap.name.name));
                    out.push_str(&format!("record {} {{\n", kebab_name));
                    for f in &cap.fields {
                        let field_kebab = to_kebab_case(&f.name.name);
                        let wit_ty = Self::map_type_to_wit(&f.ty)?;
                        out.push_str(&format!("    {}: {},\n", field_kebab, wit_ty));
                    }
                    out.push_str("}\n\n");
                }
                Declaration::Entity(entity) => {
                    let kebab_name = to_kebab_case(&entity.name.name);
                    out.push_str(&format!("/// Sovereign Entity: {}\n", entity.name.name));
                    out.push_str(&format!("record {} {{\n", kebab_name));
                    for member in &entity.members {
                        if let EntityMember::Field(f) = member {
                            let field_kebab = to_kebab_case(&f.name.name);
                            let wit_ty = Self::map_type_to_wit(&f.ty)?;
                            out.push_str(&format!("    {}: {},\n", field_kebab, wit_ty));
                        }
                    }
                    out.push_str("}\n\n");
                }
                _ => {}
            }
        }

        // 2. Emit Interfaces
        for decl in &program.declarations {
            if let Declaration::Interface(iface) = decl {
                let kebab_name = to_kebab_case(&iface.name.name);
                out.push_str(&format!("interface {} {{\n", kebab_name));
                for m in &iface.methods {
                    let m_kebab = to_kebab_case(&m.name.name);
                    out.push_str(&format!("    {}: func(", m_kebab));
                    for (i, p) in m.params.iter().enumerate() {
                        if i > 0 {
                            out.push_str(", ");
                        }
                        let p_kebab = to_kebab_case(&p.name.name);
                        let p_ty = Self::map_type_to_wit(&p.ty)?;
                        out.push_str(&format!("{}: {}", p_kebab, p_ty));
                    }
                    out.push(')');

                    if let Some(ret) = &m.return_type {
                        let ret_ty = Self::map_type_to_wit(ret)?;
                        out.push_str(&format!(" -> {}", ret_ty));
                    }
                    out.push_str(";\n");
                }
                out.push_str("}\n\n");
            }
        }

        // 3. Emit Boundaries as WIT Worlds
        for decl in &program.declarations {
            if let Declaration::Boundary(b) = decl {
                let kebab_name = to_kebab_case(&b.name.name);
                out.push_str(&format!("/// Sovereign Sandbox World: {}\n", b.name.name));
                out.push_str(&format!("world {} {{\n", kebab_name));

                // Add standard WASI baseline imports
                out.push_str("    import wasi:clocks/monotonic-clock@0.2.0;\n");
                out.push_str("    import wasi:io/streams@0.2.0;\n");

                // Export bound interfaces
                for binding in &b.bindings {
                    let binding_kebab = to_kebab_case(&binding.name);
                    out.push_str(&format!("    export {};\n", binding_kebab));
                }
                out.push_str("}\n\n");
            }
        }

        Ok(out)
    }

    fn map_type_to_wit(ty: &TypeRef) -> Result<String, WitError> {
        match ty {
            TypeRef::Primitive(prim, _) => Ok(match prim {
                PrimitiveType::Int => "s64".to_string(),
                PrimitiveType::Float => "float64".to_string(),
                PrimitiveType::Bool => "bool".to_string(),
                PrimitiveType::String => "string".to_string(),
                PrimitiveType::Bytes => "list<u8>".to_string(),
                PrimitiveType::Unit => "()".to_string(),
            }),
            TypeRef::Named(ident) => match ident.name.as_str() {
                "Int" => Ok("s64".to_string()),
                "Float" => Ok("float64".to_string()),
                "Bool" => Ok("bool".to_string()),
                "String" => Ok("string".to_string()),
                "Bytes" => Ok("list<u8>".to_string()),
                "Unit" => Ok("()".to_string()),
                other => Ok(to_kebab_case(other)),
            },
            TypeRef::Generic(ident, args, _) => match ident.name.as_str() {
                "Option" => {
                    let inner = Self::map_type_to_wit(&args[0])?;
                    Ok(format!("option<{}>", inner))
                }
                "Result" => {
                    let ok_ty = Self::map_type_to_wit(&args[0])?;
                    let err_ty = Self::map_type_to_wit(&args[1])?;
                    Ok(format!("result<{}, {}>", ok_ty, err_ty))
                }
                "List" => {
                    let inner = Self::map_type_to_wit(&args[0])?;
                    Ok(format!("list<{}>", inner))
                }
                other => Err(WitError::UnsupportedType(other.to_string())),
            },
        }
    }
}

fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('-');
            }
            result.push(c.to_ascii_lowercase());
        } else if c == '_' {
            result.push('-');
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use ordyn_parser::Parser;

    #[test]
    fn test_wit_generation_from_ordyn_program() {
        let source = r#"
            capability HttpConnect {
                host: String;
                port: Int;
            }

            interface PaymentProcessor {
                fn process_payment(target: String, amount: Int) -> Result<Int, String>;
            }

            entity Wallet {
                let wallet_id: String;
                mut balance: Int;
            }

            boundary BankingBoundary {
                bind Wallet;
            }
        "#;

        let mut parser = Parser::from_source(source).unwrap();
        let program = parser.parse_program().unwrap();

        let wit_output = WitGenerator::generate_wit(&program, "ordyn", "banking").unwrap();

        assert!(wit_output.contains("package ordyn:banking@0.1.0;"));
        assert!(wit_output.contains("record http-connect {"));
        assert!(wit_output.contains("host: string,"));
        assert!(wit_output.contains("port: s64,"));
        assert!(wit_output.contains("interface payment-processor {"));
        assert!(wit_output.contains(
            "process-payment: func(target: string, amount: s64) -> result<s64, string>;"
        ));
        assert!(wit_output.contains("world banking-boundary {"));
        assert!(wit_output.contains("export wallet;"));
    }
}
