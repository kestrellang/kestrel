// Registry configuration and naming utilities

module flock.registry

import quill.toml.parser.(parseToml)
import flock.error.(FlockError)
import flock.source.(joinPath)

// ============================================================================
// REGISTRY CONFIG
// ============================================================================

/// Configuration for connecting to a package registry.
public struct RegistryConfig: Cloneable {
    public var url: String

    public init(url url: String) {
        self.url = url;
    }

    public func clone() -> RegistryConfig {
        RegistryConfig(url: self.url.clone())
    }
}

// ============================================================================
// URL RESOLUTION
// ============================================================================

/// Resolves the registry URL using three-tier lookup:
/// 1. Project-level override (from flock.toml [registry] section)
/// 2. Global config (~/.kestrel/config.toml)
/// 3. Hardcoded default
public func resolveRegistryUrl(projectUrl projectUrl: Optional[String]) -> String {
    // 1. Project-level override
    match projectUrl {
        .Some(url) => return url,
        .None => {}
    }

    // 2. Global config
    match getenv("HOME") {
        .Some(home) => {
            let configPath = joinPath(base: home, rel: ".kestrel/config.toml");
            if fileExists(configPath) {
                match readFileString(configPath) {
                    .Ok(source) => {
                        match parseToml(source) {
                            .Ok(root) => {
                                match root.value(forKey: "registry") {
                                    .Some(regVal) => {
                                        match regVal.value(forKey: "url") {
                                            .Some(urlVal) => {
                                                match urlVal.asString() {
                                                    .Some(url) => return url,
                                                    .None => {}
                                                }
                                            },
                                            .None => {}
                                        }
                                    },
                                    .None => {}
                                }
                            },
                            .Err(_) => {}
                        }
                    },
                    .Err(_) => {}
                }
            }
        },
        .None => {}
    }

    // 3. Hardcoded default
    "https://registry.kestrel-lang.com"
}

// ============================================================================
// PACKAGE NAMING
// ============================================================================

/// Splits "org/pkg" into (org, pkg). Returns None if no slash found.
public func splitPackageName(name name: String) -> Optional[(String, String)] {
    var i: Int64 = 0;
    while i < name.byteCount {
        if name.byteAtUnchecked(i) == UInt8(intLiteral: 47) {
            let org = name.substringBytes(from: 0, to: i);
            let pkg = name.substringBytes(from: i + 1, to: name.byteCount);
            return .Some((org, pkg))
        }
        i = i + 1
    }
    .None
}

/// Returns true if the name contains a slash (i.e., is an org/pkg name).
public func isRegistryName(name name: String) -> Bool {
    var i: Int64 = 0;
    while i < name.byteCount {
        if name.byteAtUnchecked(i) == UInt8(intLiteral: 47) {
            return true
        }
        i = i + 1
    }
    false
}
