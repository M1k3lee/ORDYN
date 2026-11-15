use miette::Diagnostic;
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Error, Diagnostic, Debug, Clone, PartialEq)]
pub enum KernelError {
    #[error("Capability forgery / invalid handle: {handle}")]
    #[diagnostic(code(ordyn::guard::invalid_handle))]
    InvalidHandle { handle: u32 },

    #[error("Capability handle {handle} has been revoked or expired")]
    #[diagnostic(code(ordyn::guard::capability_revoked))]
    CapabilityRevoked { handle: u32 },

    #[error("Unauthorized operation `{operation}` for capability `{capability}` on resource `{resource}`")]
    #[diagnostic(code(ordyn::guard::unauthorized_operation))]
    UnauthorizedOperation {
        capability: String,
        operation: String,
        resource: String,
    },

    #[error("Illegal authority expansion during attenuation: `{details}`")]
    #[diagnostic(code(ordyn::guard::illegal_attenuation))]
    IllegalAttenuation { details: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CapHandle(pub u32);

#[derive(Debug, Clone, PartialEq)]
pub struct CapabilityRecord {
    pub handle: CapHandle,
    pub name: String,
    pub operations: HashSet<String>,
    pub resource_prefix: String,
    pub generation: u64,
    pub is_revoked: bool,
    pub parent: Option<CapHandle>,
    pub children: Vec<CapHandle>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuditDecision {
    Granted,
    Denied(String),
    Trapped(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct AuditRecord {
    pub timestamp_micros: u64,
    pub caller_entity: String,
    pub handle: u32,
    pub operation: String,
    pub resource: String,
    pub decision: AuditDecision,
}

pub struct AuditEngine {
    records: Vec<AuditRecord>,
}

impl Default for AuditEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditEngine {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    pub fn log(
        &mut self,
        caller: &str,
        handle: u32,
        operation: &str,
        resource: &str,
        decision: AuditDecision,
    ) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_micros() as u64)
            .unwrap_or(0);

        self.records.push(AuditRecord {
            timestamp_micros: timestamp,
            caller_entity: caller.to_string(),
            handle,
            operation: operation.to_string(),
            resource: resource.to_string(),
            decision,
        });
    }

    pub fn records(&self) -> &[AuditRecord] {
        &self.records
    }
}

pub struct SecurityKernel {
    next_handle_id: u32,
    current_generation: u64,
    capability_table: HashMap<CapHandle, CapabilityRecord>,
    audit_engine: AuditEngine,
}

impl Default for SecurityKernel {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityKernel {
    pub fn new() -> Self {
        Self {
            next_handle_id: 1,
            current_generation: 1,
            capability_table: HashMap::new(),
            audit_engine: AuditEngine::new(),
        }
    }

    pub fn issue_root_capability(
        &mut self,
        name: &str,
        operations: &[&str],
        resource_prefix: &str,
    ) -> CapHandle {
        let handle = CapHandle(self.next_handle_id);
        self.next_handle_id += 1;

        let ops_set: HashSet<String> = operations.iter().map(|s| s.to_string()).collect();

        self.capability_table.insert(
            handle,
            CapabilityRecord {
                handle,
                name: name.to_string(),
                operations: ops_set,
                resource_prefix: resource_prefix.to_string(),
                generation: self.current_generation,
                is_revoked: false,
                parent: None,
                children: Vec::new(),
            },
        );

        handle
    }

    pub fn attenuate(
        &mut self,
        parent_handle: CapHandle,
        restricted_ops: &[&str],
        restricted_prefix: &str,
    ) -> Result<CapHandle, KernelError> {
        let parent_record =
            self.capability_table
                .get(&parent_handle)
                .ok_or(KernelError::InvalidHandle {
                    handle: parent_handle.0,
                })?;

        if parent_record.is_revoked {
            return Err(KernelError::CapabilityRevoked {
                handle: parent_handle.0,
            });
        }

        let new_ops: HashSet<String> = restricted_ops.iter().map(|s| s.to_string()).collect();

        // Monotonic Reduction Check: New operations must be a subset of parent operations
        for op in &new_ops {
            if !parent_record.operations.contains(op) {
                return Err(KernelError::IllegalAttenuation {
                    details: format!("Operation `{}` is not in parent authority", op),
                });
            }
        }

        // Monotonic Reduction Check: Resource scope can only be narrowed (must start with parent prefix)
        if !restricted_prefix.starts_with(&parent_record.resource_prefix) {
            return Err(KernelError::IllegalAttenuation {
                details: format!(
                    "Resource prefix `{}` does not narrow parent `{}`",
                    restricted_prefix, parent_record.resource_prefix
                ),
            });
        }

        let child_handle = CapHandle(self.next_handle_id);
        self.next_handle_id += 1;

        let child_record = CapabilityRecord {
            handle: child_handle,
            name: parent_record.name.clone(),
            operations: new_ops,
            resource_prefix: restricted_prefix.to_string(),
            generation: self.current_generation,
            is_revoked: false,
            parent: Some(parent_handle),
            children: Vec::new(),
        };

        self.capability_table.insert(child_handle, child_record);

        // Track parent-child hierarchy for cascading revocation
        if let Some(parent) = self.capability_table.get_mut(&parent_handle) {
            parent.children.push(child_handle);
        }

        Ok(child_handle)
    }

    pub fn revoke(&mut self, handle: CapHandle) -> Result<usize, KernelError> {
        let mut to_revoke = vec![handle];
        let mut revoked_count = 0;

        while let Some(current) = to_revoke.pop() {
            if let Some(record) = self.capability_table.get_mut(&current) {
                if !record.is_revoked {
                    record.is_revoked = true;
                    revoked_count += 1;
                    to_revoke.extend(record.children.iter().copied());
                }
            } else {
                return Err(KernelError::InvalidHandle { handle: handle.0 });
            }
        }

        Ok(revoked_count)
    }

    pub fn authorize(
        &mut self,
        caller: &str,
        handle: CapHandle,
        operation: &str,
        resource: &str,
    ) -> Result<(), KernelError> {
        let record = match self.capability_table.get(&handle) {
            Some(r) => r,
            None => {
                self.audit_engine.log(
                    caller,
                    handle.0,
                    operation,
                    resource,
                    AuditDecision::Trapped("Invalid capability handle".to_string()),
                );
                return Err(KernelError::InvalidHandle { handle: handle.0 });
            }
        };

        if record.is_revoked {
            self.audit_engine.log(
                caller,
                handle.0,
                operation,
                resource,
                AuditDecision::Denied("Capability has been revoked".to_string()),
            );
            return Err(KernelError::CapabilityRevoked { handle: handle.0 });
        }

        if !record.operations.contains(operation) || !resource.starts_with(&record.resource_prefix)
        {
            self.audit_engine.log(
                caller,
                handle.0,
                operation,
                resource,
                AuditDecision::Denied("Operation or resource out of capability scope".to_string()),
            );
            return Err(KernelError::UnauthorizedOperation {
                capability: record.name.clone(),
                operation: operation.to_string(),
                resource: resource.to_string(),
            });
        }

        self.audit_engine.log(
            caller,
            handle.0,
            operation,
            resource,
            AuditDecision::Granted,
        );
        Ok(())
    }

    pub fn audit_records(&self) -> &[AuditRecord] {
        self.audit_engine.records()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_capability_authorization() {
        let mut kernel = SecurityKernel::new();
        let cap = kernel.issue_root_capability("Storage", &["Read", "Write"], "/var/data");

        let auth_res = kernel.authorize("PaymentEntity", cap, "Read", "/var/data/ledger.db");
        assert!(auth_res.is_ok());

        assert_eq!(kernel.audit_records().len(), 1);
        assert_eq!(kernel.audit_records()[0].decision, AuditDecision::Granted);
    }

    #[test]
    fn test_forged_handle_trapped() {
        let mut kernel = SecurityKernel::new();
        let forged = CapHandle(9999);

        let err = kernel
            .authorize("MaliciousEntity", forged, "Read", "/etc/passwd")
            .unwrap_err();
        assert!(matches!(err, KernelError::InvalidHandle { .. }));

        assert_eq!(kernel.audit_records().len(), 1);
        assert!(matches!(
            kernel.audit_records()[0].decision,
            AuditDecision::Trapped(_)
        ));
    }

    #[test]
    fn test_monotonic_attenuation_and_cascading_revocation() {
        let mut kernel = SecurityKernel::new();
        let root_cap = kernel.issue_root_capability("Net", &["Connect", "Listen"], "10.0.");

        // Attenuate to only Connect and a narrower IP range
        let child_cap = kernel.attenuate(root_cap, &["Connect"], "10.0.1.").unwrap();

        // Attempting to expand authority fails
        let invalid_attenuate = kernel.attenuate(child_cap, &["Connect", "Listen"], "10.0.1.");
        assert!(invalid_attenuate.is_err());

        // Child can connect within narrowed scope
        assert!(kernel
            .authorize("ChildEntity", child_cap, "Connect", "10.0.1.5:8080")
            .is_ok());

        // Revoking root cascades down and revokes child
        let revoked_count = kernel.revoke(root_cap).unwrap();
        assert_eq!(revoked_count, 2);

        let err = kernel
            .authorize("ChildEntity", child_cap, "Connect", "10.0.1.5:8080")
            .unwrap_err();
        assert!(matches!(err, KernelError::CapabilityRevoked { .. }));
    }
}
