//! `flock.toml` discovery and source-set loading.
//!
//! Reads `[package]` (for the `source` directory) and `[dependencies]`. Path
//! deps are walked transitively. Registry deps are resolved via `flock.lock`
//! against the local flock cache (default `~/.kestrel/packages`, or the
//! `kestrel.flockCachePath` setting).
//!
//! Errors / unreadable manifests are skipped silently — diagnostics will
//! surface the missing imports anyway, and we don't want a malformed
//! upstream manifest to block opening the workspace. Cache misses for
//! registry deps are reported via [`CollectReport::missing_cache`] so the
//! LSP can hint at running `flock build`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use walkdir::WalkDir;

#[derive(Debug, Deserialize)]
struct ManifestFile {
    package: Option<Package>,
    #[serde(default)]
    dependencies: toml::value::Table,
}

#[derive(Debug, Deserialize)]
struct Package {
    #[serde(default = "default_source")]
    source: String,
}

fn default_source() -> String {
    ".".to_string()
}

#[derive(Debug, Deserialize)]
struct LockFile {
    #[serde(default)]
    package: Vec<LockEntry>,
}

#[derive(Debug, Deserialize)]
struct LockEntry {
    name: String,
    version: String,
    source: String,
    #[serde(default)]
    path: Option<String>,
}

/// Sources collected from a workspace, plus any registry deps whose cache
/// directory was missing. The LSP turns `missing_cache` into a log message
/// suggesting `flock build`.
#[derive(Default)]
pub struct CollectReport {
    pub sources: Vec<PathBuf>,
    pub missing_cache: Vec<String>,
}

/// Default cache root used when `kestrel.flockCachePath` is unset. Mirrors
/// flock's own default (`~/.kestrel/packages`).
pub fn default_cache_root() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".kestrel/packages"))
}

/// Walk up from `start` looking for a `flock.toml`. Returns the manifest
/// file path (caller derives the package root via `parent()`).
pub fn find_manifest(start: &Path) -> Option<PathBuf> {
    let mut cur: Option<&Path> = Some(start);
    while let Some(dir) = cur {
        let candidate = dir.join("flock.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        cur = dir.parent();
    }
    None
}

/// Resolve a Kestrel package rooted at `manifest_path` plus its transitive
/// path-dependency packages. Returns sources and any registry-dep cache
/// misses. `cache_root` overrides the default `~/.kestrel/packages` (used
/// to honor the `kestrel.flockCachePath` setting).
pub fn collect_sources(manifest_path: &Path, cache_root: Option<&Path>) -> CollectReport {
    let mut visited: HashSet<PathBuf> = HashSet::new();
    let mut report = CollectReport::default();
    let cache = cache_root.map(PathBuf::from).or_else(default_cache_root);
    collect_recursive(manifest_path, cache.as_deref(), &mut visited, &mut report);
    report
}

fn collect_recursive(
    manifest_path: &Path,
    cache_root: Option<&Path>,
    visited: &mut HashSet<PathBuf>,
    report: &mut CollectReport,
) {
    let canonical = match manifest_path.canonicalize() {
        Ok(p) => p,
        Err(_) => return,
    };
    if !visited.insert(canonical.clone()) {
        return;
    }
    let pkg_root = match canonical.parent() {
        Some(p) => p.to_path_buf(),
        None => return,
    };
    let raw = match std::fs::read_to_string(&canonical) {
        Ok(s) => s,
        Err(_) => return,
    };
    let manifest: ManifestFile = match toml::from_str(&raw) {
        Ok(m) => m,
        Err(_) => return,
    };

    let source_dir = pkg_root.join(
        manifest.package.as_ref().map(|p| p.source.as_str()).unwrap_or("."),
    );
    walk_kestrel_sources(&source_dir, &mut report.sources);

    // Path deps from [dependencies] table.
    for (_name, value) in manifest.dependencies.iter() {
        let Some(table) = value.as_table() else { continue };
        let Some(path) = table.get("path").and_then(|v| v.as_str()) else { continue };
        let dep_manifest = pkg_root.join(path).join("flock.toml");
        if dep_manifest.is_file() {
            collect_recursive(&dep_manifest, cache_root, visited, report);
        }
    }

    // Registry deps via flock.lock, resolved against the local flock cache.
    let lock_path = pkg_root.join("flock.lock");
    if lock_path.is_file() {
        if let Ok(raw) = std::fs::read_to_string(&lock_path) {
            if let Ok(lock) = toml::from_str::<LockFile>(&raw) {
                for entry in &lock.package {
                    if entry.source == "path" {
                        if let Some(p) = &entry.path {
                            let dep_manifest = PathBuf::from(p).join("flock.toml");
                            if dep_manifest.is_file() {
                                collect_recursive(&dep_manifest, cache_root, visited, report);
                            }
                        }
                        continue;
                    }
                    if entry.source != "registry" {
                        continue;
                    }
                    let Some(cache) = cache_root else {
                        report.missing_cache.push(format!(
                            "{}@{} (no cache root: HOME unset and kestrel.flockCachePath not configured)",
                            entry.name, entry.version
                        ));
                        continue;
                    };
                    // Names like "kestrel/swoop" already split into org/pkg
                    // segments; bare names (e.g. "swoop") cache directly
                    // under <cache>/<name>/<version>/.
                    let pkg_dir = cache.join(&entry.name).join(&entry.version);
                    let dep_manifest = pkg_dir.join("flock.toml");
                    if dep_manifest.is_file() {
                        collect_recursive(&dep_manifest, cache_root, visited, report);
                    } else {
                        report.missing_cache.push(format!(
                            "{}@{} (expected at {})",
                            entry.name,
                            entry.version,
                            pkg_dir.display()
                        ));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    fn write(path: &Path, body: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = fs::File::create(path).unwrap();
        f.write_all(body.as_bytes()).unwrap();
    }

    #[test]
    fn registry_dep_resolved_via_cache() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        // Workspace package
        let pkg = root.join("pkg");
        write(&pkg.join("flock.toml"), "[package]\nname = \"pkg\"\nsource = \".\"\n");
        write(&pkg.join("main.ks"), "module Main\n");
        write(
            &pkg.join("flock.lock"),
            r#"[[package]]
name = "swoop"
version = "1.0.0"
source = "registry"
checksum = "sha256:dummy"
"#,
        );

        // Cached package at <cache>/swoop/1.0.0/
        let cache = root.join("cache");
        let cached = cache.join("swoop").join("1.0.0");
        write(&cached.join("flock.toml"), "[package]\nname = \"swoop\"\nsource = \".\"\n");
        write(&cached.join("lib.ks"), "module Swoop\n");

        let report = collect_sources(&pkg.join("flock.toml"), Some(&cache));
        let names: Vec<String> = report
            .sources
            .iter()
            .filter_map(|p| p.file_name().and_then(|s| s.to_str()).map(String::from))
            .collect();
        assert!(names.contains(&"main.ks".into()), "got {names:?}");
        assert!(names.contains(&"lib.ks".into()), "got {names:?}");
        assert!(report.missing_cache.is_empty(), "{:?}", report.missing_cache);
    }

    #[test]
    fn registry_dep_missing_cache_reported() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let pkg = root.join("pkg");
        write(&pkg.join("flock.toml"), "[package]\nsource = \".\"\n");
        write(&pkg.join("main.ks"), "module Main\n");
        write(
            &pkg.join("flock.lock"),
            r#"[[package]]
name = "missing"
version = "0.1.0"
source = "registry"
"#,
        );
        let cache = root.join("empty_cache");
        fs::create_dir_all(&cache).unwrap();

        let report = collect_sources(&pkg.join("flock.toml"), Some(&cache));
        assert_eq!(report.missing_cache.len(), 1);
        assert!(report.missing_cache[0].contains("missing@0.1.0"));
    }
}

/// Recursively collect every `.ks` file under `dir`. Used by both
/// `collect_sources` (for package source dirs) and the LSP's stdlib loader
/// when `kestrel.stdlibPath` is configured.
pub fn walk_kestrel_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    if !dir.is_dir() {
        return;
    }
    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("ks") {
            out.push(path.to_path_buf());
        }
    }
}
