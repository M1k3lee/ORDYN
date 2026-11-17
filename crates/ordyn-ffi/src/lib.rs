use ordyn_guard::{CapHandle, SecurityKernel};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum FfiError {
    #[error("Capability violation: caller '{caller}' is not authorized to invoke foreign symbol '{symbol}'")]
    UnauthorizedForeignSymbol { caller: String, symbol: String },

    #[error("Foreign library not found: '{name}'")]
    LibraryNotFound { name: String },

    #[error("Type marshalling mismatch for symbol '{symbol}': expected {expected}, got {got}")]
    TypeMismatch {
        symbol: String,
        expected: String,
        got: String,
    },

    #[error("Foreign execution trap: {0}")]
    ExecutionTrap(String),
}

/// Primitive types supported over the C-ABI FFI boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FfiType {
    Int32,
    Int64,
    UInt32,
    UInt64,
    Float32,
    Float64,
    Pointer,
    Void,
}

/// Foreign function declaration and signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForeignFunctionSig {
    pub symbol: String,
    pub params: Vec<FfiType>,
    pub return_type: FfiType,
    pub library: String,
}

/// Dynamic Foreign Capability descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfiCapability {
    pub library_name: String,
    pub allowed_symbols: HashSet<String>,
}

pub type ForeignHandlerFn = fn(&[i64]) -> Result<i64, String>;

/// The ORDYN FFI Bridge: mediates all cross-boundary C-ABI calls through the Security Kernel.
pub struct FfiBridge {
    registered_functions: HashMap<String, ForeignFunctionSig>,
    ffi_capabilities: HashMap<CapHandle, FfiCapability>,
    mock_handlers: HashMap<String, ForeignHandlerFn>,
}

impl Default for FfiBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl FfiBridge {
    pub fn new() -> Self {
        Self {
            registered_functions: HashMap::new(),
            ffi_capabilities: HashMap::new(),
            mock_handlers: HashMap::new(),
        }
    }

    /// Register a foreign C-ABI function signature.
    pub fn register_foreign_function(&mut self, sig: ForeignFunctionSig) {
        self.registered_functions.insert(sig.symbol.clone(), sig);
    }

    /// Grant an unforgeable FFI capability handle in the Security Kernel for specific symbols.
    pub fn grant_ffi_capability(
        &mut self,
        kernel: &mut SecurityKernel,
        _caller_boundary: &str,
        library_name: &str,
        symbols: &[&str],
    ) -> CapHandle {
        let handle = kernel.issue_root_capability(
            "FfiInvoke",
            &["CallForeign"],
            &format!("lib:{}", library_name),
        );

        let mut allowed = HashSet::new();
        for s in symbols {
            allowed.insert(s.to_string());
        }

        self.ffi_capabilities.insert(
            handle,
            FfiCapability {
                library_name: library_name.to_string(),
                allowed_symbols: allowed,
            },
        );

        handle
    }

    /// Register a simulated/native execution handler for a foreign symbol.
    pub fn register_symbol_handler(&mut self, symbol: &str, handler: ForeignHandlerFn) {
        self.mock_handlers.insert(symbol.to_string(), handler);
    }

    /// Invoke a foreign function through the capability gate.
    /// Zero ambient authority: requires valid CapHandle registered for this library and symbol.
    pub fn invoke_foreign(
        &self,
        kernel: &mut SecurityKernel,
        caller: &str,
        handle: CapHandle,
        symbol: &str,
        args: &[i64],
    ) -> Result<i64, FfiError> {
        let sig =
            self.registered_functions
                .get(symbol)
                .ok_or_else(|| FfiError::LibraryNotFound {
                    name: symbol.to_string(),
                })?;

        // 1. Verify capability handle in Security Kernel
        kernel
            .authorize(
                caller,
                handle,
                "CallForeign",
                &format!("lib:{}", sig.library),
            )
            .map_err(|_| FfiError::UnauthorizedForeignSymbol {
                caller: caller.to_string(),
                symbol: symbol.to_string(),
            })?;

        // 2. Verify symbol whitelist in FFI capability
        let ffi_cap = self.ffi_capabilities.get(&handle).ok_or_else(|| {
            FfiError::UnauthorizedForeignSymbol {
                caller: caller.to_string(),
                symbol: symbol.to_string(),
            }
        })?;

        if !ffi_cap.allowed_symbols.contains(symbol) {
            return Err(FfiError::UnauthorizedForeignSymbol {
                caller: caller.to_string(),
                symbol: symbol.to_string(),
            });
        }

        // 3. Arity check
        if args.len() != sig.params.len() {
            return Err(FfiError::TypeMismatch {
                symbol: symbol.to_string(),
                expected: format!("{} arguments", sig.params.len()),
                got: format!("{} arguments", args.len()),
            });
        }

        // 4. Dispatch foreign handler
        if let Some(handler) = self.mock_handlers.get(symbol) {
            handler(args).map_err(FfiError::ExecutionTrap)
        } else {
            // Default safe stub for foreign calls without explicit dynamic linker attachment
            Ok(0)
        }
    }
}

pub struct CHeaderGenerator;

impl CHeaderGenerator {
    pub fn generate(program: &ordyn_ast::Program, module_name: &str) -> String {
        let mut out = String::new();
        let guard_name = format!(
            "ORDYN_FFI_{}_H",
            module_name.to_uppercase().replace('-', "_")
        );

        out.push_str(
            "/* =========================================================================\n",
        );
        out.push_str(&format!(
            " * Automatically generated by ORDYN Sovereign C-ABI Bridge\n * Module: {}\n",
            module_name
        ));
        out.push_str(
            " * ========================================================================= */\n\n",
        );
        out.push_str(&format!(
            "#ifndef {}\n#define {}\n\n",
            guard_name, guard_name
        ));
        out.push_str("#ifdef __cplusplus\nextern \"C\" {\n#endif\n\n");
        out.push_str("#include <stdint.h>\n#include <stdbool.h>\n#include <stddef.h>\n\n");

        out.push_str("/* ORDYN Core Status Codes */\n");
        out.push_str("typedef enum {\n");
        out.push_str("    ORDYN_OK = 0,\n");
        out.push_str("    ORDYN_ERR_UNAUTHORIZED = 1,\n");
        out.push_str("    ORDYN_ERR_CONTRACT_VIOLATION = 2,\n");
        out.push_str("    ORDYN_ERR_FUEL_EXHAUSTED = 3,\n");
        out.push_str("    ORDYN_ERR_PANIC = 4\n");
        out.push_str("} ordyn_status_t;\n\n");

        out.push_str("/* Sovereign Capability & Boundary Handles */\n");
        out.push_str("typedef uint64_t ordyn_cap_handle_t;\n");
        out.push_str("typedef struct ordyn_boundary_s* ordyn_boundary_t;\n\n");

        out.push_str("/* --- ORDYN Entities (C Struct Mappings) --- */\n");
        for decl in &program.declarations {
            if let ordyn_ast::Declaration::Entity(e) = decl {
                out.push_str(&format!("typedef struct ordyn_{}_s {{\n", e.name.name));
                for member in &e.members {
                    if let ordyn_ast::EntityMember::Field(f) = member {
                        let c_ty = Self::map_type_to_c(&f.ty);
                        out.push_str(&format!("    {} {};\n", c_ty, f.name.name));
                    }
                }
                out.push_str(&format!("}} ordyn_{}_t;\n\n", e.name.name));
            }
        }

        out.push_str("/* --- ORDYN Interfaces (C Function Declarations) --- */\n");
        for decl in &program.declarations {
            if let ordyn_ast::Declaration::Interface(iface) = decl {
                out.push_str(&format!("/* Interface: {} */\n", iface.name.name));
                for method in &iface.methods {
                    let ret_ty = method
                        .return_type
                        .as_ref()
                        .map(Self::map_type_to_c)
                        .unwrap_or_else(|| "void".to_string());
                    out.push_str(&format!(
                        "ordyn_status_t ordyn_{}_{}(\n    ordyn_cap_handle_t cap_handle",
                        iface.name.name, method.name.name
                    ));
                    for p in &method.params {
                        out.push_str(&format!(
                            ",\n    {} {}",
                            Self::map_type_to_c(&p.ty),
                            p.name.name
                        ));
                    }
                    if ret_ty != "void" {
                        out.push_str(&format!(",\n    {}* out_result", ret_ty));
                    }
                    out.push_str("\n);\n\n");
                }
            }
        }

        out.push_str("/* --- Sovereign Boundary Lifecycle --- */\n");
        for decl in &program.declarations {
            if let ordyn_ast::Declaration::Boundary(b) = decl {
                out.push_str(&format!(
                    "ordyn_status_t ordyn_enter_{}(uint64_t fuel_limit, ordyn_boundary_t* out_boundary);\n",
                    b.name.name
                ));
                out.push_str(&format!(
                    "ordyn_status_t ordyn_leave_{}(ordyn_boundary_t boundary);\n\n",
                    b.name.name
                ));
            }
        }

        out.push_str("#ifdef __cplusplus\n}\n#endif\n\n");
        out.push_str(&format!("#endif /* {} */\n", guard_name));

        out
    }

    fn map_type_to_c(ty: &ordyn_ast::TypeRef) -> String {
        match ty {
            ordyn_ast::TypeRef::Primitive(p, _) => match p {
                ordyn_ast::PrimitiveType::Int => "int64_t".to_string(),
                ordyn_ast::PrimitiveType::Float => "double".to_string(),
                ordyn_ast::PrimitiveType::Bool => "bool".to_string(),
                ordyn_ast::PrimitiveType::String => "const char*".to_string(),
                ordyn_ast::PrimitiveType::Bytes => "const uint8_t*".to_string(),
                ordyn_ast::PrimitiveType::Unit => "void".to_string(),
            },
            ordyn_ast::TypeRef::Named(id) => format!("ordyn_{}_t", id.name),
            ordyn_ast::TypeRef::Generic(id, _, _) => format!("ordyn_{}_t", id.name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_c_crypto_hash(args: &[i64]) -> Result<i64, String> {
        let input = args[0];
        // Simulate SHA-like scalar hash
        Ok(input.wrapping_mul(0x517cc1b727220a95u64 as i64) ^ 0x6a09e667f3bcc908u64 as i64)
    }

    #[test]
    fn test_valid_ffi_invocation_with_capability() {
        let mut kernel = SecurityKernel::new();
        let mut bridge = FfiBridge::new();

        bridge.register_foreign_function(ForeignFunctionSig {
            symbol: "crypto_sha256_scalar".to_string(),
            params: vec![FfiType::Int64],
            return_type: FfiType::Int64,
            library: "libcrypto.so".to_string(),
        });
        bridge.register_symbol_handler("crypto_sha256_scalar", sample_c_crypto_hash);

        let cap = bridge.grant_ffi_capability(
            &mut kernel,
            "SecurityGateway",
            "libcrypto.so",
            &["crypto_sha256_scalar"],
        );

        let res = bridge.invoke_foreign(
            &mut kernel,
            "SecurityGateway",
            cap,
            "crypto_sha256_scalar",
            &[42],
        );

        assert!(res.is_ok());
        assert_ne!(res.unwrap(), 0);
    }

    #[test]
    fn test_ffi_call_without_valid_capability_is_rejected() {
        let mut kernel = SecurityKernel::new();
        let mut bridge = FfiBridge::new();

        bridge.register_foreign_function(ForeignFunctionSig {
            symbol: "crypto_sha256_scalar".to_string(),
            params: vec![FfiType::Int64],
            return_type: FfiType::Int64,
            library: "libcrypto.so".to_string(),
        });

        // Use forged handle (CapHandle(9999))
        let forged_handle = CapHandle(9999);

        let res = bridge.invoke_foreign(
            &mut kernel,
            "UntrustedCaller",
            forged_handle,
            "crypto_sha256_scalar",
            &[42],
        );

        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            FfiError::UnauthorizedForeignSymbol {
                caller: "UntrustedCaller".to_string(),
                symbol: "crypto_sha256_scalar".to_string(),
            }
        );
    }

    #[test]
    fn test_ffi_call_to_unwhitelisted_symbol_is_blocked() {
        let mut kernel = SecurityKernel::new();
        let mut bridge = FfiBridge::new();

        bridge.register_foreign_function(ForeignFunctionSig {
            symbol: "system_exec".to_string(),
            params: vec![FfiType::Pointer],
            return_type: FfiType::Int32,
            library: "libc.so".to_string(),
        });

        // Grant capability only for "malloc", not "system_exec"
        let cap =
            bridge.grant_ffi_capability(&mut kernel, "RestrictedBoundary", "libc.so", &["malloc"]);

        let res = bridge.invoke_foreign(
            &mut kernel,
            "RestrictedBoundary",
            cap,
            "system_exec",
            &[0x1000],
        );

        assert!(res.is_err());
        match res.unwrap_err() {
            FfiError::UnauthorizedForeignSymbol { symbol, .. } => {
                assert_eq!(symbol, "system_exec");
            }
            other => panic!("Unexpected error: {:?}", other),
        }
    }

    #[test]
    fn test_c_header_generation() {
        let source = r#"
entity UserSession {
    let user_id: Int;
    let token: String;
}

interface AuthService {
    fn validate(session: UserSession) -> Bool with [CapCheck];
}

boundary AppGateway {
    quota { memory_limit: 64; fuel_limit: 100000; }
}
"#;
        let mut parser = ordyn_parser::Parser::from_source(source).unwrap();
        let program = parser.parse_program().unwrap();

        let header = CHeaderGenerator::generate(&program, "auth_module");
        assert!(header.contains("#ifndef ORDYN_FFI_AUTH_MODULE_H"));
        assert!(header.contains("typedef struct ordyn_UserSession_s {"));
        assert!(header.contains("int64_t user_id;"));
        assert!(header.contains("const char* token;"));
        assert!(header.contains("ordyn_status_t ordyn_AuthService_validate("));
        assert!(header.contains("ordyn_status_t ordyn_enter_AppGateway("));
        assert!(header.contains("ordyn_status_t ordyn_leave_AppGateway("));
    }
}
