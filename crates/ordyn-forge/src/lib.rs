use miette::Diagnostic;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

#[derive(Error, Diagnostic, Debug, Clone, PartialEq)]
pub enum ForgeError {
    #[error("Package verification failed: dependency `{package}` demands capability `{capability}`, but root project grants only `{allowed:?}`")]
    #[diagnostic(code(ordyn::forge::ungranted_capability))]
    UngrantedCapabilityDependency {
        package: String,
        capability: String,
        allowed: Vec<String>,
    },

    #[error("Corrupt package or integrity checksum mismatch for `{package}`")]
    #[diagnostic(code(ordyn::forge::integrity_mismatch))]
    IntegrityMismatch { package: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct DependencySpec {
    pub version: String,
    pub allowed_capabilities: HashSet<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PackageManifest {
    pub name: String,
    pub version: String,
    pub declared_capabilities: HashSet<String>,
    pub dependencies: HashMap<String, DependencySpec>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SealedPackage {
    pub manifest: PackageManifest,
    pub source_content: String,
    pub integrity_hash: String,
}

pub struct ForgeVerifier;

impl ForgeVerifier {
    /// Validates that dependencies do not possess or request ambient authority
    pub fn verify_dependency_capabilities(
        root_manifest: &PackageManifest,
        dependency_name: &str,
        dep_manifest: &PackageManifest,
    ) -> Result<(), ForgeError> {
        let allowed = match root_manifest.dependencies.get(dependency_name) {
            Some(spec) => &spec.allowed_capabilities,
            None => {
                return Err(ForgeError::UngrantedCapabilityDependency {
                    package: dependency_name.to_string(),
                    capability: "*".to_string(),
                    allowed: vec![],
                });
            }
        };

        for req_cap in &dep_manifest.declared_capabilities {
            if !allowed.contains(req_cap) {
                return Err(ForgeError::UngrantedCapabilityDependency {
                    package: dependency_name.to_string(),
                    capability: req_cap.clone(),
                    allowed: allowed.iter().cloned().collect(),
                });
            }
        }

        Ok(())
    }
}

pub struct PackagePackager;

impl PackagePackager {
    pub fn package_project(manifest: PackageManifest, source: &str) -> SealedPackage {
        // Deterministic pseudo-hash of content
        let mut hash_val: u64 = 0xcbf29ce484222325;
        for b in source.bytes() {
            hash_val ^= b as u64;
            hash_val = hash_val.wrapping_mul(0x100000001b3);
        }
        let integrity_hash = format!("{:016x}", hash_val);

        SealedPackage {
            manifest,
            source_content: source.to_string(),
            integrity_hash,
        }
    }
}

pub struct PackageInspector;

impl PackageInspector {
    pub fn inspect(pkg: &SealedPackage) -> String {
        let mut report = String::new();
        report.push_str(&format!("Package: {} (v{})\n", pkg.manifest.name, pkg.manifest.version));
        report.push_str(&format!("Integrity Hash: {}\n", pkg.integrity_hash));
        report.push_str("Declared Capabilities:\n");
        if pkg.manifest.declared_capabilities.is_empty() {
            report.push_str("  - (None: Pure Computation Only)\n");
        } else {
            for cap in &pkg.manifest.declared_capabilities {
                report.push_str(&format!("  - {}\n", cap));
            }
        }
        report.push_str("Dependencies:\n");
        if pkg.manifest.dependencies.is_empty() {
            report.push_str("  - (Zero external dependencies)\n");
        } else {
            for (dep, spec) in &pkg.manifest.dependencies {
                let caps: Vec<String> = spec.allowed_capabilities.iter().cloned().collect();
                report.push_str(&format!("  - {} ({}) -> Allowed Caps: [{}]\n", dep, spec.version, caps.join(", ")));
            }
        }
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forge_enforces_zero_ambient_authority_on_dependencies() {
        let mut root_caps = HashSet::new();
        root_caps.insert("NetConnect".to_string());

        let mut root_deps = HashMap::new();
        // Root strictly grants only StorageRead to json_parser
        let mut json_allowed = HashSet::new();
        json_allowed.insert("StorageRead".to_string());
        root_deps.insert("json_parser".to_string(), DependencySpec {
            version: "1.0.0".to_string(),
            allowed_capabilities: json_allowed,
        });

        let root_manifest = PackageManifest {
            name: "BankingApp".to_string(),
            version: "0.1.0".to_string(),
            declared_capabilities: root_caps,
            dependencies: root_deps,
        };

        // Dependency requesting only StorageRead -> Approved
        let mut dep_caps_safe = HashSet::new();
        dep_caps_safe.insert("StorageRead".to_string());
        let dep_manifest_safe = PackageManifest {
            name: "json_parser".to_string(),
            version: "1.0.0".to_string(),
            declared_capabilities: dep_caps_safe,
            dependencies: HashMap::new(),
        };
        assert!(ForgeVerifier::verify_dependency_capabilities(&root_manifest, "json_parser", &dep_manifest_safe).is_ok());

        // Malicious dependency demanding ungranted NetConnect -> Rejected
        let mut dep_caps_malicious = HashSet::new();
        dep_caps_malicious.insert("NetConnect".to_string());
        let dep_manifest_malicious = PackageManifest {
            name: "json_parser".to_string(),
            version: "1.0.0".to_string(),
            declared_capabilities: dep_caps_malicious,
            dependencies: HashMap::new(),
        };
        let err = ForgeVerifier::verify_dependency_capabilities(&root_manifest, "json_parser", &dep_manifest_malicious).unwrap_err();
        assert!(matches!(err, ForgeError::UngrantedCapabilityDependency { .. }));
    }

    #[test]
    fn test_package_builder_and_inspector() {
        let manifest = PackageManifest {
            name: "VaultCore".to_string(),
            version: "0.1.0".to_string(),
            declared_capabilities: HashSet::from(["StorageWrite".to_string()]),
            dependencies: HashMap::new(),
        };
        let source = "entity Vault { mut balance: Int; }";
        let pkg = PackagePackager::package_project(manifest, source);

        let report = PackageInspector::inspect(&pkg);
        assert!(report.contains("Package: VaultCore"));
        assert!(report.contains("StorageWrite"));
    }
}
