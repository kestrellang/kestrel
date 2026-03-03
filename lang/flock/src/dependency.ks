// Dependency specification types

module flock.dependency

import quill.value.(Value)
import flock.error.(FlockError)
import flock.version.(VersionConstraint, parseConstraint)

// ============================================================================
// DEPENDENCY SPEC
// ============================================================================

/// How a dependency is specified in flock.toml.
public enum DependencySpec: Cloneable {
    /// Local path dependency: { path = "../quill" }
    case Path(String)
    /// Registry dependency with version constraint: "^1.0.0" (future)
    case Registry(VersionConstraint)

    public func clone() -> DependencySpec {
        match self {
            .Path(s) => .Path(s.clone()),
            .Registry(c) => .Registry(c.clone())
        }
    }
}

// ============================================================================
// DEPENDENCY
// ============================================================================

/// A single dependency declaration from [dependencies] in flock.toml.
public struct Dependency: Cloneable {
    public var name: String
    public var spec: DependencySpec

    public init(name name: String, spec spec: DependencySpec) {
        self.name = name;
        self.spec = spec;
    }

    public func clone() -> Dependency {
        Dependency(name: self.name.clone(), spec: self.spec.clone())
    }
}

// ============================================================================
// PARSING
// ============================================================================

/// Parses the [dependencies] table from a quill Value.
/// Each entry is either a string version or an object with a "path" key.
public func parseDependencies(depsValue depsValue: Value) -> Result[Array[Dependency], FlockError] {
    match depsValue.asObject() {
        .None => .Ok(Array[Dependency]()),
        .Some(obj) => {
            var result = Array[Dependency]();

            for (key, val) in obj.iter() {
                match parseSingleDep(name: key, value: val) {
                    .Ok(dep) => result.append(dep),
                    .Err(e) => return .Err(e)
                }
            }

            .Ok(result)
        }
    }
}

/// Parses a single dependency value.
func parseSingleDep(name name: String, value value: Value) -> Result[Dependency, FlockError] {
    // String value: version constraint (e.g. "^1.0.0")
    match value.asString() {
        .Some(versionStr) => {
            match parseConstraint(s: versionStr) {
                .Ok(constraint) => return .Ok(Dependency(
                    name: name,
                    spec: DependencySpec.Registry(constraint)
                )),
                .Err(e) => return .Err(e)
            }
        },
        .None => {}
    }

    // Object value: look for "path" key
    match value.value(forKey: "path") {
        .Some(pathVal) => {
            match pathVal.asString() {
                .Some(pathStr) => return .Ok(Dependency(
                    name: name,
                    spec: DependencySpec.Path(pathStr)
                )),
                .None => return .Err(FlockError.ManifestParse(
                    "dependency '" + name + "' path must be a string"
                ))
            }
        },
        .None => {}
    }

    .Err(FlockError.ManifestParse(
        "dependency '" + name + "' must be a version string or object with 'path'"
    ))
}
