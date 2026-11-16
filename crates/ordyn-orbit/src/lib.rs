use ordyn_guard::{CapHandle, SecurityKernel};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum OrbitError {
    #[error("Route not found: '{0}'")]
    NotFound(String),

    #[error("Boundary capability violation: handler in boundary '{boundary}' failed capability check: {reason}")]
    CapabilityDenied { boundary: String, reason: String },

    #[error("Rate limit exceeded for client: '{0}'")]
    RateLimited(String),

    #[error("Internal service error: {0}")]
    InternalError(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrbitRequest {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub client_ip: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrbitResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl OrbitResponse {
    pub fn ok(body: impl Into<Vec<u8>>) -> Self {
        Self {
            status: 200,
            headers: HashMap::new(),
            body: body.into(),
        }
    }

    pub fn error(status: u16, message: &str) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: message.as_bytes().to_vec(),
        }
    }
}

pub type HandlerFn = fn(
    &OrbitRequest,
    &mut SecurityKernel,
    &HashMap<String, CapHandle>,
) -> Result<OrbitResponse, OrbitError>;

pub struct OrbitRoute {
    pub method: String,
    pub path: String,
    pub boundary_name: String,
    pub required_capability: Option<String>,
    pub handler: HandlerFn,
}

pub struct RateLimiter {
    max_requests_per_window: usize,
    history: HashMap<String, usize>,
}

impl RateLimiter {
    pub fn new(max_requests_per_window: usize) -> Self {
        Self {
            max_requests_per_window,
            history: HashMap::new(),
        }
    }

    pub fn check(&mut self, client: &str) -> bool {
        let count = self.history.entry(client.to_string()).or_insert(0);
        if *count >= self.max_requests_per_window {
            false
        } else {
            *count += 1;
            true
        }
    }
}

/// The Orbit Service Engine: routes incoming HTTP requests to capability-confined handlers.
pub struct OrbitService {
    routes: Vec<OrbitRoute>,
    rate_limiter: RateLimiter,
    boundary_capabilities: HashMap<String, HashMap<String, CapHandle>>,
}

impl Default for OrbitService {
    fn default() -> Self {
        Self::new(100)
    }
}

impl OrbitService {
    pub fn new(rate_limit_per_client: usize) -> Self {
        Self {
            routes: Vec::new(),
            rate_limiter: RateLimiter::new(rate_limit_per_client),
            boundary_capabilities: HashMap::new(),
        }
    }

    /// Register a route bound to an architectural boundary.
    pub fn route(
        &mut self,
        method: &str,
        path: &str,
        boundary: &str,
        required_capability: Option<&str>,
        handler: HandlerFn,
    ) {
        self.routes.push(OrbitRoute {
            method: method.to_uppercase(),
            path: path.to_string(),
            boundary_name: boundary.to_string(),
            required_capability: required_capability.map(|s| s.to_string()),
            handler,
        });
    }

    /// Bind a root capability handle to a service boundary.
    pub fn bind_capability(&mut self, boundary: &str, capability_name: &str, handle: CapHandle) {
        self.boundary_capabilities
            .entry(boundary.to_string())
            .or_default()
            .insert(capability_name.to_string(), handle);
    }

    /// Dispatch a request through rate limiting and capability verification.
    pub fn dispatch(
        &mut self,
        kernel: &mut SecurityKernel,
        request: &OrbitRequest,
    ) -> OrbitResponse {
        // 1. Rate limiter check
        if !self.rate_limiter.check(&request.client_ip) {
            return OrbitResponse::error(429, "Rate limit exceeded");
        }

        // 2. Route matching
        let route = self
            .routes
            .iter()
            .find(|r| r.method == request.method.to_uppercase() && r.path == request.path);

        let route = match route {
            Some(r) => r,
            None => return OrbitResponse::error(404, "Endpoint not found"),
        };

        // 3. Verify boundary capability if required
        let empty_map = HashMap::new();
        let caps = self
            .boundary_capabilities
            .get(&route.boundary_name)
            .unwrap_or(&empty_map);

        if let Some(req_cap) = &route.required_capability {
            match caps.get(req_cap) {
                Some(handle) => {
                    if let Err(e) =
                        kernel.authorize(&route.boundary_name, *handle, "Invoke", &route.path)
                    {
                        return OrbitResponse::error(
                            403,
                            &format!("Forbidden: capability authorization failed ({})", e),
                        );
                    }
                }
                None => {
                    return OrbitResponse::error(
                        403,
                        &format!(
                            "Forbidden: boundary '{}' lacks required capability '{}'",
                            route.boundary_name, req_cap
                        ),
                    );
                }
            }
        }

        // 4. Execute handler
        match (route.handler)(request, kernel, caps) {
            Ok(resp) => resp,
            Err(OrbitError::CapabilityDenied { boundary, reason }) => OrbitResponse::error(
                403,
                &format!("Capability violation in {}: {}", boundary, reason),
            ),
            Err(OrbitError::RateLimited(client)) => {
                OrbitResponse::error(429, &format!("Rate limit exceeded for {}", client))
            }
            Err(OrbitError::NotFound(msg)) => OrbitResponse::error(404, &msg),
            Err(OrbitError::InternalError(msg)) => OrbitResponse::error(500, &msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn health_check(
        _req: &OrbitRequest,
        _kernel: &mut SecurityKernel,
        _caps: &HashMap<String, CapHandle>,
    ) -> Result<OrbitResponse, OrbitError> {
        Ok(OrbitResponse::ok(b"{\"status\":\"UP\"}".to_vec()))
    }

    fn secure_payment_handler(
        req: &OrbitRequest,
        kernel: &mut SecurityKernel,
        caps: &HashMap<String, CapHandle>,
    ) -> Result<OrbitResponse, OrbitError> {
        let storage_cap = caps
            .get("StorageWrite")
            .ok_or_else(|| OrbitError::CapabilityDenied {
                boundary: "PaymentBoundary".to_string(),
                reason: "StorageWrite capability missing".to_string(),
            })?;

        // Authorize storage write
        kernel
            .authorize("PaymentBoundary", *storage_cap, "Write", "ledger/tx")
            .map_err(|e| OrbitError::CapabilityDenied {
                boundary: "PaymentBoundary".to_string(),
                reason: e.to_string(),
            })?;

        let body_str = String::from_utf8_lossy(&req.body);
        Ok(OrbitResponse::ok(
            format!("Processed payment: {}", body_str).into_bytes(),
        ))
    }

    #[test]
    fn test_orbit_route_dispatch_success() {
        let mut kernel = SecurityKernel::new();
        let mut service = OrbitService::new(10);

        service.route("GET", "/health", "PublicBoundary", None, health_check);

        let req = OrbitRequest {
            method: "GET".to_string(),
            path: "/health".to_string(),
            headers: HashMap::new(),
            body: Vec::new(),
            client_ip: "127.0.0.1".to_string(),
        };

        let resp = service.dispatch(&mut kernel, &req);
        assert_eq!(resp.status, 200);
        assert_eq!(String::from_utf8_lossy(&resp.body), "{\"status\":\"UP\"}");
    }

    #[test]
    fn test_orbit_capability_isolated_endpoint() {
        let mut kernel = SecurityKernel::new();
        let mut service = OrbitService::new(10);

        // Grant RouteAccess capability for the HTTP endpoint
        let route_cap = kernel.issue_root_capability("RouteAccess", &["Invoke"], "/api/pay");
        service.bind_capability("PaymentBoundary", "RouteAccess", route_cap);

        // Grant StorageWrite capability to PaymentBoundary for the ledger
        let write_cap = kernel.issue_root_capability("StorageWrite", &["Write"], "ledger/tx");
        service.bind_capability("PaymentBoundary", "StorageWrite", write_cap);

        service.route(
            "POST",
            "/api/pay",
            "PaymentBoundary",
            Some("RouteAccess"),
            secure_payment_handler,
        );

        let req = OrbitRequest {
            method: "POST".to_string(),
            path: "/api/pay".to_string(),
            headers: HashMap::new(),
            body: b"amount=100".to_vec(),
            client_ip: "10.0.0.1".to_string(),
        };

        let resp = service.dispatch(&mut kernel, &req);
        assert_eq!(resp.status, 200);
        assert!(String::from_utf8_lossy(&resp.body).contains("Processed payment: amount=100"));
    }

    #[test]
    fn test_orbit_rate_limiting_enforcement() {
        let mut kernel = SecurityKernel::new();
        let mut service = OrbitService::new(2); // Max 2 requests

        service.route("GET", "/health", "PublicBoundary", None, health_check);

        let req = OrbitRequest {
            method: "GET".to_string(),
            path: "/health".to_string(),
            headers: HashMap::new(),
            body: Vec::new(),
            client_ip: "192.168.1.50".to_string(),
        };

        assert_eq!(service.dispatch(&mut kernel, &req).status, 200);
        assert_eq!(service.dispatch(&mut kernel, &req).status, 200);
        // Third request exceeds limit
        let resp3 = service.dispatch(&mut kernel, &req);
        assert_eq!(resp3.status, 429);
    }
}
