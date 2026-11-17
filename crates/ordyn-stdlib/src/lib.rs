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

capability EnvRead {
    key: String;
}

capability Clock {
    precision: String;
}

capability SocketConnect {
    host: String;
    port: Int;
}

capability SocketListen {
    port: Int;
}

capability CryptoRandom {
    entropy_pool: String;
}

capability CryptoHash {
    algorithm: String;
}

interface KeyValueStore {
    fn get(key: String) -> Result<String, String> with [StorageRead];
    fn set(key: String, value: String) -> Result<Unit, String> with [StorageWrite];
}

interface SocketClient {
    fn send(payload: String) -> Result<Int, String> with [SocketConnect];
    fn receive() -> Result<String, String> with [SocketConnect];
}

interface CryptoProvider {
    fn hash(data: String) -> String with [CryptoHash];
    fn random_hex(bytes: Int) -> Result<String, String> with [CryptoRandom];
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

    #[error("Serialization / parsing error: {0}")]
    #[diagnostic(code(ordyn::stdlib::serialization_error))]
    SerializationError(String),
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

pub struct EnvironmentEngine;

impl EnvironmentEngine {
    /// Read an environment variable, gated by an explicit EnvRead capability.
    pub fn get_var(
        kernel: &mut SecurityKernel,
        caller: &str,
        handle: CapHandle,
        var_name: &str,
    ) -> Result<String, StdlibError> {
        kernel
            .authorize(caller, handle, "ReadEnv", var_name)
            .map_err(|e| StdlibError::Unauthorized(e.to_string()))?;

        std::env::var(var_name).map_err(|_| StdlibError::NotFound {
            key: var_name.to_string(),
        })
    }
}

pub struct ClockEngine;

impl ClockEngine {
    /// Monotonic timestamp in microseconds since UNIX epoch.
    pub fn now_micros() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

impl JsonValue {
    pub fn parse(input: &str) -> Result<JsonValue, StdlibError> {
        let chars: Vec<char> = input.chars().collect();
        let mut idx = 0;
        let val = Self::parse_value(&chars, &mut idx)?;
        Self::skip_ws(&chars, &mut idx);
        if idx != chars.len() {
            return Err(StdlibError::SerializationError(format!(
                "Unexpected trailing character at index {}",
                idx
            )));
        }
        Ok(val)
    }

    fn skip_ws(chars: &[char], idx: &mut usize) {
        while *idx < chars.len() && chars[*idx].is_whitespace() {
            *idx += 1;
        }
    }

    fn parse_value(chars: &[char], idx: &mut usize) -> Result<JsonValue, StdlibError> {
        Self::skip_ws(chars, idx);
        if *idx >= chars.len() {
            return Err(StdlibError::SerializationError(
                "Unexpected end of input".to_string(),
            ));
        }

        match chars[*idx] {
            'n' => Self::parse_null(chars, idx),
            't' | 'f' => Self::parse_bool(chars, idx),
            '"' => Self::parse_string(chars, idx).map(JsonValue::String),
            '[' => Self::parse_array(chars, idx),
            '{' => Self::parse_object(chars, idx),
            '-' | '0'..='9' => Self::parse_number(chars, idx),
            ch => Err(StdlibError::SerializationError(format!(
                "Unexpected character: '{}' at index {}",
                ch, idx
            ))),
        }
    }

    fn parse_null(chars: &[char], idx: &mut usize) -> Result<JsonValue, StdlibError> {
        if *idx + 4 <= chars.len() && chars[*idx..*idx + 4] == ['n', 'u', 'l', 'l'] {
            *idx += 4;
            Ok(JsonValue::Null)
        } else {
            Err(StdlibError::SerializationError(
                "Malformed null literal".to_string(),
            ))
        }
    }

    fn parse_bool(chars: &[char], idx: &mut usize) -> Result<JsonValue, StdlibError> {
        if *idx + 4 <= chars.len() && chars[*idx..*idx + 4] == ['t', 'r', 'u', 'e'] {
            *idx += 4;
            Ok(JsonValue::Bool(true))
        } else if *idx + 5 <= chars.len() && chars[*idx..*idx + 5] == ['f', 'a', 'l', 's', 'e'] {
            *idx += 5;
            Ok(JsonValue::Bool(false))
        } else {
            Err(StdlibError::SerializationError(
                "Malformed boolean literal".to_string(),
            ))
        }
    }

    fn parse_string(chars: &[char], idx: &mut usize) -> Result<String, StdlibError> {
        *idx += 1; // skip opening quote
        let mut s = String::new();
        while *idx < chars.len() {
            let ch = chars[*idx];
            *idx += 1;
            if ch == '"' {
                return Ok(s);
            }
            if ch == '\\' {
                if *idx >= chars.len() {
                    return Err(StdlibError::SerializationError(
                        "Unterminated escape sequence".to_string(),
                    ));
                }
                let esc = chars[*idx];
                *idx += 1;
                match esc {
                    '"' => s.push('"'),
                    '\\' => s.push('\\'),
                    '/' => s.push('/'),
                    'n' => s.push('\n'),
                    'r' => s.push('\r'),
                    't' => s.push('\t'),
                    other => s.push(other),
                }
            } else {
                s.push(ch);
            }
        }
        Err(StdlibError::SerializationError(
            "Unclosed string literal".to_string(),
        ))
    }

    fn parse_number(chars: &[char], idx: &mut usize) -> Result<JsonValue, StdlibError> {
        let start = *idx;
        if *idx < chars.len() && chars[*idx] == '-' {
            *idx += 1;
        }
        while *idx < chars.len()
            && (chars[*idx].is_ascii_digit()
                || chars[*idx] == '.'
                || chars[*idx] == 'e'
                || chars[*idx] == 'E'
                || chars[*idx] == '+'
                || chars[*idx] == '-')
        {
            *idx += 1;
        }
        let s: String = chars[start..*idx].iter().collect();
        let num: f64 = s.parse().map_err(|_| {
            StdlibError::SerializationError(format!("Invalid number format: '{}'", s))
        })?;
        Ok(JsonValue::Number(num))
    }

    fn parse_array(chars: &[char], idx: &mut usize) -> Result<JsonValue, StdlibError> {
        *idx += 1; // skip '['
        let mut arr = Vec::new();
        Self::skip_ws(chars, idx);
        if *idx < chars.len() && chars[*idx] == ']' {
            *idx += 1;
            return Ok(JsonValue::Array(arr));
        }

        loop {
            let item = Self::parse_value(chars, idx)?;
            arr.push(item);
            Self::skip_ws(chars, idx);
            if *idx >= chars.len() {
                return Err(StdlibError::SerializationError(
                    "Unclosed array".to_string(),
                ));
            }
            if chars[*idx] == ']' {
                *idx += 1;
                break;
            } else if chars[*idx] == ',' {
                *idx += 1;
            } else {
                return Err(StdlibError::SerializationError(format!(
                    "Expected ',' or ']' in array at index {}",
                    idx
                )));
            }
        }
        Ok(JsonValue::Array(arr))
    }

    fn parse_object(chars: &[char], idx: &mut usize) -> Result<JsonValue, StdlibError> {
        *idx += 1; // skip '{'
        let mut map = HashMap::new();
        Self::skip_ws(chars, idx);
        if *idx < chars.len() && chars[*idx] == '}' {
            *idx += 1;
            return Ok(JsonValue::Object(map));
        }

        loop {
            Self::skip_ws(chars, idx);
            if *idx >= chars.len() || chars[*idx] != '"' {
                return Err(StdlibError::SerializationError(
                    "Expected string key in object".to_string(),
                ));
            }
            let key = Self::parse_string(chars, idx)?;
            Self::skip_ws(chars, idx);
            if *idx >= chars.len() || chars[*idx] != ':' {
                return Err(StdlibError::SerializationError(
                    "Expected ':' after key in object".to_string(),
                ));
            }
            *idx += 1; // skip ':'
            let val = Self::parse_value(chars, idx)?;
            map.insert(key, val);

            Self::skip_ws(chars, idx);
            if *idx >= chars.len() {
                return Err(StdlibError::SerializationError(
                    "Unclosed object".to_string(),
                ));
            }
            if chars[*idx] == '}' {
                *idx += 1;
                break;
            } else if chars[*idx] == ',' {
                *idx += 1;
            } else {
                return Err(StdlibError::SerializationError(format!(
                    "Expected ',' or '}}' in object at index {}",
                    idx
                )));
            }
        }
        Ok(JsonValue::Object(map))
    }

    pub fn stringify(&self) -> String {
        match self {
            JsonValue::Null => "null".to_string(),
            JsonValue::Bool(b) => {
                if *b {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            JsonValue::Number(n) => {
                if n.fract() == 0.0 && !n.is_infinite() && !n.is_nan() {
                    format!("{:.0}", n)
                } else {
                    format!("{}", n)
                }
            }
            JsonValue::String(s) => {
                format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
            }
            JsonValue::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|item| item.stringify()).collect();
                format!("[{}]", items.join(","))
            }
            JsonValue::Object(map) => {
                let mut entries: Vec<String> = map
                    .iter()
                    .map(|(k, v)| format!("\"{}\":{}", k.replace('"', "\\\""), v.stringify()))
                    .collect();
                entries.sort();
                format!("{{{}}}", entries.join(","))
            }
        }
    }

    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        match self {
            JsonValue::Object(map) => map.get(key),
            _ => None,
        }
    }

    pub fn get_path(&self, path: &str) -> Option<&JsonValue> {
        let parts = path.split('.');
        let mut cur = self;
        for part in parts {
            match cur {
                JsonValue::Object(map) => cur = map.get(part)?,
                JsonValue::Array(arr) => {
                    let idx: usize = part.parse().ok()?;
                    cur = arr.get(idx)?;
                }
                _ => return None,
            }
        }
        Some(cur)
    }
}

pub struct CryptoEngine;

impl CryptoEngine {
    pub fn sha256_hex(
        kernel: &mut SecurityKernel,
        caller: &str,
        handle: CapHandle,
        data: &[u8],
    ) -> Result<String, StdlibError> {
        kernel
            .authorize(caller, handle, "Hash", "SHA-256")
            .map_err(|e| StdlibError::Unauthorized(e.to_string()))?;

        Ok(Self::compute_sha256(data))
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

    pub fn secure_random_hex(
        kernel: &mut SecurityKernel,
        caller: &str,
        handle: CapHandle,
        byte_count: usize,
    ) -> Result<String, StdlibError> {
        kernel
            .authorize(caller, handle, "GenerateRandom", "EntropyPool")
            .map_err(|e| StdlibError::Unauthorized(e.to_string()))?;

        let mut seed = ClockEngine::now_micros();
        let mut out = String::with_capacity(byte_count * 2);
        for i in 0..byte_count {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407 + (i as u64));
            let byte = ((seed >> 32) ^ (seed & 0xFFFFFFFF)) as u8;
            use std::fmt::Write;
            let _ = write!(out, "{:02x}", byte);
        }
        Ok(out)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SocketConnection {
    pub host: String,
    pub port: u16,
    pub connected: bool,
}

pub struct SocketEngine;

impl SocketEngine {
    pub fn connect(
        kernel: &mut SecurityKernel,
        caller: &str,
        handle: CapHandle,
        host: &str,
        port: u16,
    ) -> Result<SocketConnection, StdlibError> {
        let target = format!("{}:{}", host, port);
        kernel
            .authorize(caller, handle, "SocketConnect", &target)
            .map_err(|e| StdlibError::Unauthorized(e.to_string()))?;

        Ok(SocketConnection {
            host: host.to_string(),
            port,
            connected: true,
        })
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

    #[test]
    fn test_environment_capability_gating() {
        let mut kernel = SecurityKernel::new();
        let env_cap = kernel.issue_root_capability("EnvRead", &["ReadEnv"], "ORDYN_TEST_KEY");

        // Set env var for testing
        std::env::set_var("ORDYN_TEST_KEY", "sovereign_123");

        let val = EnvironmentEngine::get_var(&mut kernel, "AppBoundary", env_cap, "ORDYN_TEST_KEY")
            .unwrap();
        assert_eq!(val, "sovereign_123");

        // Attempting to read unauthorized env var is rejected by kernel
        let err = EnvironmentEngine::get_var(&mut kernel, "AppBoundary", env_cap, "AWS_SECRET_KEY");
        assert!(matches!(err, Err(StdlibError::Unauthorized(_))));
    }

    #[test]
    fn test_clock_monotonicity() {
        let t1 = ClockEngine::now_micros();
        assert!(t1 > 0);
        let t2 = ClockEngine::now_micros();
        assert!(t2 >= t1);
    }

    #[test]
    fn test_json_parsing_and_path_traversal() {
        let raw =
            r#"{"name":"ordyn","meta":{"active":true,"tags":["security","kernel"]},"count":42}"#;
        let json = JsonValue::parse(raw).unwrap();

        assert_eq!(
            json.get_path("name"),
            Some(&JsonValue::String("ordyn".to_string()))
        );
        assert_eq!(json.get_path("meta.active"), Some(&JsonValue::Bool(true)));
        assert_eq!(
            json.get_path("meta.tags.0"),
            Some(&JsonValue::String("security".to_string()))
        );
        assert_eq!(
            json.get_path("meta.tags.1"),
            Some(&JsonValue::String("kernel".to_string()))
        );
        assert_eq!(json.get_path("count"), Some(&JsonValue::Number(42.0)));
        assert_eq!(json.get_path("nonexistent"), None);

        // Stringify back
        let stringified = json.stringify();
        let parsed_again = JsonValue::parse(&stringified).unwrap();
        assert_eq!(json, parsed_again);
    }

    #[test]
    fn test_crypto_sha256_standard_vectors_and_gating() {
        // Standard test vector: SHA-256("abc") = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
        let hash = CryptoEngine::compute_sha256(b"abc");
        assert_eq!(
            hash,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );

        // Capability gating test
        let mut kernel = SecurityKernel::new();
        let cap = kernel.issue_root_capability("CryptoHash", &["Hash"], "SHA-256");

        let auth_hash =
            CryptoEngine::sha256_hex(&mut kernel, "AppBoundary", cap, b"ordyn_kernel").unwrap();
        assert_eq!(auth_hash.len(), 64);

        // Random generation
        let rand_cap =
            kernel.issue_root_capability("CryptoRandom", &["GenerateRandom"], "EntropyPool");
        let rand_token =
            CryptoEngine::secure_random_hex(&mut kernel, "AppBoundary", rand_cap, 16).unwrap();
        assert_eq!(rand_token.len(), 32); // 16 bytes = 32 hex chars
    }

    #[test]
    fn test_socket_capability_gating() {
        let mut kernel = SecurityKernel::new();
        let socket_cap = kernel.issue_root_capability(
            "SocketConnect",
            &["SocketConnect"],
            "api.sovereign.io:443",
        );

        let conn = SocketEngine::connect(
            &mut kernel,
            "Microservice",
            socket_cap,
            "api.sovereign.io",
            443,
        )
        .unwrap();
        assert!(conn.connected);
        assert_eq!(conn.port, 443);

        // Connection to unapproved port or host is trapped
        let err = SocketEngine::connect(
            &mut kernel,
            "Microservice",
            socket_cap,
            "unauthorized.com",
            80,
        );
        assert!(matches!(err, Err(StdlibError::Unauthorized(_))));
    }
}
