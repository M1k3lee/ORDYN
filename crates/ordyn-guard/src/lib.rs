use miette::Diagnostic;
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Error, Diagnostic, Debug, Clone, PartialEq)]
pub enum KernelError {
    #[error("Capability forgery / invalid handle: {handle:#x}")]
    #[diagnostic(code(ordyn::guard::invalid_handle))]
    InvalidHandle { handle: u64 },

    #[error("Capability handle {handle:#x} has been revoked")]
    #[diagnostic(code(ordyn::guard::capability_revoked))]
    CapabilityRevoked { handle: u64 },

    #[error("Capability handle {handle:#x} has expired at {expired_at}µs (current time: {current_time}µs)")]
    #[diagnostic(code(ordyn::guard::capability_expired))]
    CapabilityExpired {
        handle: u64,
        expired_at: u64,
        current_time: u64,
    },

    #[error("Unauthorized caller `{caller}`: capability {handle:#x} is owned by `{owner}`")]
    #[diagnostic(code(ordyn::guard::unauthorized_caller))]
    UnauthorizedCaller {
        caller: String,
        owner: String,
        handle: u64,
    },

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

    #[error("Capability handle {handle:#x} is non-delegable")]
    #[diagnostic(code(ordyn::guard::non_delegable))]
    NonDelegableCapability { handle: u64 },

    #[error("Delegation depth limit exceeded for capability handle {handle:#x}")]
    #[diagnostic(code(ordyn::guard::delegation_depth_exceeded))]
    DelegationDepthExceeded { handle: u64 },

    #[error(
        "Linear capability handle {handle:#x} has already been consumed and cannot be re-used"
    )]
    #[diagnostic(code(ordyn::guard::capability_consumed))]
    CapabilityConsumed { handle: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CapHandle(pub u64);

impl From<u32> for CapHandle {
    fn from(val: u32) -> Self {
        CapHandle(val as u64)
    }
}

impl From<u64> for CapHandle {
    fn from(val: u64) -> Self {
        CapHandle(val)
    }
}

impl From<CapHandle> for u64 {
    fn from(h: CapHandle) -> Self {
        h.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CapabilityRecord {
    pub handle: CapHandle,
    pub name: String,
    pub owner_boundary: Option<String>,
    pub allowed_delegates: HashSet<String>,
    pub operations: HashSet<String>,
    pub resource_prefix: String,
    pub generation: u64,
    pub is_revoked: bool,
    pub expires_at_micros: Option<u64>,
    pub delegable: bool,
    pub delegation_depth: u32,
    pub is_linear: bool,
    pub is_consumed: bool,
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
    pub sequence_number: u64,
    pub timestamp_micros: u64,
    pub caller_entity: String,
    pub handle: u64,
    pub operation: String,
    pub resource: String,
    pub decision: AuditDecision,
    pub prev_hash: String,
    pub record_hash: String,
}

pub struct AuditEngine {
    records: Vec<AuditRecord>,
    current_head_hash: String,
    next_sequence: u64,
}

impl Default for AuditEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditEngine {
    pub fn new() -> Self {
        let genesis_hash = compute_sha256(b"ORDYN_AUDIT_GENESIS_BLOCK_v1");
        Self {
            records: Vec::new(),
            current_head_hash: genesis_hash,
            next_sequence: 1,
        }
    }

    pub fn log(
        &mut self,
        timestamp: u64,
        caller: &str,
        handle: u64,
        operation: &str,
        resource: &str,
        decision: AuditDecision,
    ) {
        let seq = self.next_sequence;
        self.next_sequence += 1;

        let payload = format!(
            "{}:{}:{}:{}:{}:{}:{:?}",
            self.current_head_hash, seq, timestamp, caller, handle, operation, decision
        );
        let record_hash = compute_sha256(payload.as_bytes());

        let record = AuditRecord {
            sequence_number: seq,
            timestamp_micros: timestamp,
            caller_entity: caller.to_string(),
            handle,
            operation: operation.to_string(),
            resource: resource.to_string(),
            decision,
            prev_hash: self.current_head_hash.clone(),
            record_hash: record_hash.clone(),
        };

        self.current_head_hash = record_hash;
        self.records.push(record);
    }

    pub fn records(&self) -> &[AuditRecord] {
        &self.records
    }

    pub fn records_mut(&mut self) -> &mut Vec<AuditRecord> {
        &mut self.records
    }

    pub fn head_hash(&self) -> &str {
        &self.current_head_hash
    }

    pub fn verify_integrity(&self) -> Result<(), (usize, String)> {
        let genesis = compute_sha256(b"ORDYN_AUDIT_GENESIS_BLOCK_v1");
        let mut expected_prev = genesis;

        for (idx, record) in self.records.iter().enumerate() {
            if record.sequence_number != (idx as u64 + 1) {
                return Err((
                    idx,
                    format!(
                        "Sequence discontinuity: expected {}, found {}",
                        idx + 1,
                        record.sequence_number
                    ),
                ));
            }
            if record.prev_hash != expected_prev {
                return Err((
                    idx,
                    format!(
                        "Hash chain broken: expected prev_hash {}, found {}",
                        expected_prev, record.prev_hash
                    ),
                ));
            }
            let payload = format!(
                "{}:{}:{}:{}:{}:{}:{:?}",
                record.prev_hash,
                record.sequence_number,
                record.timestamp_micros,
                record.caller_entity,
                record.handle,
                record.operation,
                record.decision
            );
            let computed_hash = compute_sha256(payload.as_bytes());
            if computed_hash != record.record_hash {
                return Err((
                    idx,
                    format!(
                        "Record payload tampered: computed hash {}, stored hash {}",
                        computed_hash, record.record_hash
                    ),
                ));
            }
            expected_prev = record.record_hash.clone();
        }

        Ok(())
    }
}

pub struct SecurityKernel {
    capability_table: HashMap<CapHandle, CapabilityRecord>,
    audit_engine: AuditEngine,
    current_generation: u64,
    rng_state: u64,
    next_handle_counter: u64,
    simulated_clock_micros: Option<u64>,
}

impl Default for SecurityKernel {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityKernel {
    pub fn new() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let initial_seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_micros() as u64)
            .unwrap_or(0xCAFE_BABE_DEAD_BEEF);
        #[cfg(target_arch = "wasm32")]
        let initial_seed = 0xCAFE_BABE_DEAD_BEEF;

        Self {
            capability_table: HashMap::new(),
            audit_engine: AuditEngine::new(),
            current_generation: 1,
            rng_state: initial_seed ^ 0x5555_AAAA_5555_AAAA,
            next_handle_counter: 1,
            simulated_clock_micros: None,
        }
    }

    fn generate_secure_handle(&mut self) -> CapHandle {
        self.rng_state = self
            .rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let random_bits = self.rng_state.rotate_right(16);
        let handle_val =
            (random_bits & 0x00FF_FFFF_FFFF_FFFF) | ((self.next_handle_counter & 0xFF) << 56);
        self.next_handle_counter += 1;
        CapHandle(handle_val)
    }

    pub fn set_simulated_time(&mut self, time_micros: u64) {
        self.simulated_clock_micros = Some(time_micros);
    }

    pub fn advance_time(&mut self, delta_micros: u64) {
        let cur = self.current_time();
        self.simulated_clock_micros = Some(cur.saturating_add(delta_micros));
    }

    pub fn current_time(&self) -> u64 {
        if let Some(t) = self.simulated_clock_micros {
            return t;
        }
        #[cfg(not(target_arch = "wasm32"))]
        return SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_micros() as u64)
            .unwrap_or(0);
        #[cfg(target_arch = "wasm32")]
        return 1_000_000;
    }

    pub fn issue_root_capability(
        &mut self,
        name: &str,
        operations: &[&str],
        resource_prefix: &str,
    ) -> CapHandle {
        self.issue_root_capability_scoped(None, name, operations, resource_prefix, None, true, 8)
    }

    pub fn issue_root_capability_for(
        &mut self,
        owner_boundary: &str,
        name: &str,
        operations: &[&str],
        resource_prefix: &str,
    ) -> CapHandle {
        self.issue_root_capability_scoped(
            Some(owner_boundary),
            name,
            operations,
            resource_prefix,
            None,
            true,
            8,
        )
    }

    pub fn issue_time_limited_capability(
        &mut self,
        owner_boundary: &str,
        name: &str,
        operations: &[&str],
        resource_prefix: &str,
        ttl_micros: u64,
    ) -> CapHandle {
        let now = self.current_time();
        let expires = now.saturating_add(ttl_micros);
        self.issue_root_capability_scoped(
            Some(owner_boundary),
            name,
            operations,
            resource_prefix,
            Some(expires),
            true,
            8,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn issue_root_capability_scoped(
        &mut self,
        owner_boundary: Option<&str>,
        name: &str,
        operations: &[&str],
        resource_prefix: &str,
        expires_at_micros: Option<u64>,
        delegable: bool,
        delegation_depth: u32,
    ) -> CapHandle {
        let handle = self.generate_secure_handle();
        let ops_set: HashSet<String> = operations.iter().map(|s| s.to_string()).collect();

        self.capability_table.insert(
            handle,
            CapabilityRecord {
                handle,
                name: name.to_string(),
                owner_boundary: owner_boundary.map(|s| s.to_string()),
                allowed_delegates: HashSet::new(),
                operations: ops_set,
                resource_prefix: resource_prefix.to_string(),
                generation: self.current_generation,
                is_revoked: false,
                expires_at_micros,
                delegable,
                delegation_depth,
                is_linear: false,
                is_consumed: false,
                parent: None,
                children: Vec::new(),
            },
        );

        handle
    }

    pub fn issue_linear_capability(
        &mut self,
        owner_boundary: Option<&str>,
        name: &str,
        operations: &[&str],
        resource_prefix: &str,
    ) -> CapHandle {
        let handle = self.generate_secure_handle();
        let ops_set: HashSet<String> = operations.iter().map(|s| s.to_string()).collect();

        self.capability_table.insert(
            handle,
            CapabilityRecord {
                handle,
                name: name.to_string(),
                owner_boundary: owner_boundary.map(|s| s.to_string()),
                allowed_delegates: HashSet::new(),
                operations: ops_set,
                resource_prefix: resource_prefix.to_string(),
                generation: self.current_generation,
                is_revoked: false,
                expires_at_micros: None,
                delegable: false,
                delegation_depth: 0,
                is_linear: true,
                is_consumed: false,
                parent: None,
                children: Vec::new(),
            },
        );

        handle
    }

    pub fn grant_delegate(
        &mut self,
        handle: CapHandle,
        delegate_boundary: &str,
    ) -> Result<(), KernelError> {
        let record = self
            .capability_table
            .get_mut(&handle)
            .ok_or(KernelError::InvalidHandle { handle: handle.0 })?;
        record
            .allowed_delegates
            .insert(delegate_boundary.to_string());
        Ok(())
    }

    pub fn attenuate(
        &mut self,
        parent_handle: CapHandle,
        restricted_ops: &[&str],
        restricted_prefix: &str,
    ) -> Result<CapHandle, KernelError> {
        self.attenuate_scoped(
            None,
            parent_handle,
            None,
            restricted_ops,
            restricted_prefix,
            None,
        )
    }

    pub fn attenuate_scoped(
        &mut self,
        caller: Option<&str>,
        parent_handle: CapHandle,
        new_owner: Option<&str>,
        restricted_ops: &[&str],
        restricted_prefix: &str,
        ttl_micros: Option<u64>,
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

        let now = self.current_time();
        if let Some(parent_exp) = parent_record.expires_at_micros {
            if now > parent_exp {
                return Err(KernelError::CapabilityExpired {
                    handle: parent_handle.0,
                    expired_at: parent_exp,
                    current_time: now,
                });
            }
        }

        if let Some(c) = caller {
            if let Some(ref owner) = parent_record.owner_boundary {
                if owner != c && !parent_record.allowed_delegates.contains(c) {
                    return Err(KernelError::UnauthorizedCaller {
                        caller: c.to_string(),
                        owner: owner.clone(),
                        handle: parent_handle.0,
                    });
                }
            }
        }

        if !parent_record.delegable {
            return Err(KernelError::NonDelegableCapability {
                handle: parent_handle.0,
            });
        }

        if parent_record.delegation_depth == 0 {
            return Err(KernelError::DelegationDepthExceeded {
                handle: parent_handle.0,
            });
        }

        let new_ops: HashSet<String> = restricted_ops.iter().map(|s| s.to_string()).collect();

        // Monotonic Reduction: New operations must be a subset of parent operations
        for op in &new_ops {
            if !parent_record.operations.contains(op) {
                return Err(KernelError::IllegalAttenuation {
                    details: format!("Operation `{}` is not in parent authority", op),
                });
            }
        }

        // Monotonic Reduction: Resource scope can only be narrowed
        if !restricted_prefix.starts_with(&parent_record.resource_prefix) {
            return Err(KernelError::IllegalAttenuation {
                details: format!(
                    "Resource prefix `{}` does not narrow parent `{}`",
                    restricted_prefix, parent_record.resource_prefix
                ),
            });
        }

        let child_expires = match (parent_record.expires_at_micros, ttl_micros) {
            (Some(p_exp), Some(ttl)) => {
                let requested = now.saturating_add(ttl);
                Some(requested.min(p_exp))
            }
            (Some(p_exp), None) => Some(p_exp),
            (None, Some(ttl)) => Some(now.saturating_add(ttl)),
            (None, None) => None,
        };

        // Clone needed parent fields so we can release borrow on self.capability_table
        let parent_owner = parent_record.owner_boundary.clone();
        let parent_name = parent_record.name.clone();
        let parent_delegable = parent_record.delegable;
        let parent_depth = parent_record.delegation_depth;

        let child_handle = self.generate_secure_handle();
        let child_owner = new_owner.map(|s| s.to_string()).or(parent_owner);

        let child_record = CapabilityRecord {
            handle: child_handle,
            name: parent_name,
            owner_boundary: child_owner,
            allowed_delegates: HashSet::new(),
            operations: new_ops,
            resource_prefix: restricted_prefix.to_string(),
            generation: self.current_generation,
            is_revoked: false,
            expires_at_micros: child_expires,
            delegable: parent_delegable,
            delegation_depth: parent_depth.saturating_sub(1),
            is_linear: false,
            is_consumed: false,
            parent: Some(parent_handle),
            children: Vec::new(),
        };

        self.capability_table.insert(child_handle, child_record);

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
        let now = self.current_time();
        let record = match self.capability_table.get(&handle) {
            Some(r) => r,
            None => {
                self.audit_engine.log(
                    now,
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
                now,
                caller,
                handle.0,
                operation,
                resource,
                AuditDecision::Denied("Capability has been revoked".to_string()),
            );
            return Err(KernelError::CapabilityRevoked { handle: handle.0 });
        }

        if record.is_consumed {
            self.audit_engine.log(
                now,
                caller,
                handle.0,
                operation,
                resource,
                AuditDecision::Trapped("Linear capability already consumed".to_string()),
            );
            return Err(KernelError::CapabilityConsumed { handle: handle.0 });
        }

        if let Some(expiry) = record.expires_at_micros {
            if now > expiry {
                self.audit_engine.log(
                    now,
                    caller,
                    handle.0,
                    operation,
                    resource,
                    AuditDecision::Denied(format!("Capability expired at {}µs", expiry)),
                );
                return Err(KernelError::CapabilityExpired {
                    handle: handle.0,
                    expired_at: expiry,
                    current_time: now,
                });
            }
        }

        if let Some(ref owner) = record.owner_boundary {
            if owner != caller && !record.allowed_delegates.contains(caller) {
                self.audit_engine.log(
                    now,
                    caller,
                    handle.0,
                    operation,
                    resource,
                    AuditDecision::Trapped(format!(
                        "Unauthorized caller `{}` attempting to use capability owned by `{}`",
                        caller, owner
                    )),
                );
                return Err(KernelError::UnauthorizedCaller {
                    caller: caller.to_string(),
                    owner: owner.clone(),
                    handle: handle.0,
                });
            }
        }

        if !record.operations.contains(operation) || !resource.starts_with(&record.resource_prefix)
        {
            self.audit_engine.log(
                now,
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
            now,
            caller,
            handle.0,
            operation,
            resource,
            AuditDecision::Granted,
        );

        if let Some(r) = self.capability_table.get_mut(&handle) {
            if r.is_linear {
                r.is_consumed = true;
            }
        }

        Ok(())
    }

    pub fn audit_records(&self) -> &[AuditRecord] {
        self.audit_engine.records()
    }

    pub fn audit_engine_mut(&mut self) -> &mut AuditEngine {
        &mut self.audit_engine
    }

    pub fn verify_audit_log(&self) -> Result<(), (usize, String)> {
        self.audit_engine.verify_integrity()
    }
}

#[allow(clippy::chunks_exact_to_as_chunks)]
pub fn compute_sha256(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut h_var = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h_var
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h_var = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(h_var);
    }

    h.iter().map(|word| format!("{:08x}", word)).collect()
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
        assert!(kernel.verify_audit_log().is_ok());
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
        assert!(kernel.verify_audit_log().is_ok());
    }

    #[test]
    fn test_monotonic_attenuation_and_cascading_revocation() {
        let mut kernel = SecurityKernel::new();
        let root_cap = kernel.issue_root_capability("Net", &["Connect", "Listen"], "10.0.");

        let child_cap = kernel.attenuate(root_cap, &["Connect"], "10.0.1.").unwrap();

        let invalid_attenuate = kernel.attenuate(child_cap, &["Connect", "Listen"], "10.0.1.");
        assert!(invalid_attenuate.is_err());

        assert!(kernel
            .authorize("ChildEntity", child_cap, "Connect", "10.0.1.5:8080")
            .is_ok());

        let revoked_count = kernel.revoke(root_cap).unwrap();
        assert_eq!(revoked_count, 2);

        let err = kernel
            .authorize("ChildEntity", child_cap, "Connect", "10.0.1.5:8080")
            .unwrap_err();
        assert!(matches!(err, KernelError::CapabilityRevoked { .. }));
        assert!(kernel.verify_audit_log().is_ok());
    }

    #[test]
    fn test_owner_bound_capability_prevents_theft() {
        let mut kernel = SecurityKernel::new();
        let owner_cap = kernel.issue_root_capability_for(
            "ServiceA",
            "VaultAccess",
            &["Read", "Write"],
            "vault/secrets",
        );

        // ServiceA can authorize
        assert!(kernel
            .authorize("ServiceA", owner_cap, "Read", "vault/secrets/key1")
            .is_ok());

        // ServiceB attempting to use ServiceA's handle is TRAPPED
        let theft_err = kernel
            .authorize("ServiceB", owner_cap, "Read", "vault/secrets/key1")
            .unwrap_err();
        assert!(matches!(theft_err, KernelError::UnauthorizedCaller { .. }));

        // Check audit log trapped
        let last_log = kernel.audit_records().last().unwrap();
        assert!(matches!(last_log.decision, AuditDecision::Trapped(_)));
    }

    #[test]
    fn test_time_limited_capability_expires() {
        let mut kernel = SecurityKernel::new();
        kernel.set_simulated_time(1_000_000); // 1.0s

        let ttl_cap = kernel.issue_time_limited_capability(
            "EngineerSession",
            "MaintenanceCap",
            &["Calibrate"],
            "refrig/unit1",
            500_000, // 0.5s TTL -> expires at 1_500_000
        );

        // Before expiry: succeeds
        assert!(kernel
            .authorize("EngineerSession", ttl_cap, "Calibrate", "refrig/unit1")
            .is_ok());

        // Advance past expiry
        kernel.advance_time(600_000); // now 1_600_000 > 1_500_000
        let exp_err = kernel
            .authorize("EngineerSession", ttl_cap, "Calibrate", "refrig/unit1")
            .unwrap_err();
        assert!(matches!(exp_err, KernelError::CapabilityExpired { .. }));
    }

    #[test]
    fn test_tamper_evident_audit_log_detects_modification() {
        let mut kernel = SecurityKernel::new();
        let cap = kernel.issue_root_capability("LogTest", &["Write"], "log/");
        kernel
            .authorize("TestEntity", cap, "Write", "log/1")
            .unwrap();
        kernel
            .authorize("TestEntity", cap, "Write", "log/2")
            .unwrap();

        assert!(kernel.verify_audit_log().is_ok());

        // Adversary tampers with the first record in memory
        kernel.audit_engine_mut().records_mut()[0].operation = "ForgedOp".to_string();

        // Verification fails!
        let verify_res = kernel.verify_audit_log();
        assert!(verify_res.is_err());
    }

    #[test]
    fn test_linear_capability_single_use_enforcement() {
        let mut kernel = SecurityKernel::new();
        let lin_cap = kernel.issue_linear_capability(
            Some("Vault"),
            "DisburseToken",
            &["Transfer"],
            "accounts/treasury",
        );

        // First use succeeds
        assert!(kernel
            .authorize("Vault", lin_cap, "Transfer", "accounts/treasury")
            .is_ok());

        // Second use fails immediately with CapabilityConsumed
        let second_use = kernel.authorize("Vault", lin_cap, "Transfer", "accounts/treasury");
        assert!(matches!(
            second_use,
            Err(KernelError::CapabilityConsumed { .. })
        ));

        // The double-use attempt is logged in tamper-evident audit log
        let records = kernel.audit_records();
        assert_eq!(records.len(), 2);
        assert!(matches!(records[1].decision, AuditDecision::Trapped(_)));
    }
}
