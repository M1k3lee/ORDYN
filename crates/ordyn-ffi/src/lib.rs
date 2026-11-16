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
}
