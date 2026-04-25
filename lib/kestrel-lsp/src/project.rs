//! `flock.toml` discovery and source-set loading.
//!
//! M1 scope: parse `[package]` (we need `source`) and `[dependencies]` (path
//! deps only). Registry / cache resolution is deferred to M5 — for now we
//! quietly skip non-path deps and let the user open registry sources by hand
//! if they need them.
//!
//! The only fields we deserialize are the ones that affect file discovery.
//! Everything else is ignored, so partial / future-extended manifests parse.

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
/// path-dependency packages. Returns the de-duplicated list of `.ks` files.
///
/// Errors / unreadable manifests are skipped silently — diagnostics will
/// surface the missing imports anyway, and we don't want a malformed
/// upstream manifest to block opening the workspace.
pub fn collect_sources(manifest_path: &Path) -> Vec<PathBuf> {
    let mut visited: HashSet<PathBuf> = HashSet::new();
    let mut out = Vec::new();
    collect_recursive(manifest_path, &mut visited, &mut out);
    out
}

fn collect_recursive(
    manifest_path: &Path,
    visited: &mut HashSet<PathBuf>,
    out: &mut Vec<PathBuf>,
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
    walk_kestrel_sources(&source_dir, out);

    for (_name, value) in manifest.dependencies.iter() {
        let Some(table) = value.as_table() else { continue };
        let Some(path) = table.get("path").and_then(|v| v.as_str()) else { continue };
        let dep_manifest = pkg_root.join(path).join("flock.toml");
        if dep_manifest.is_file() {
            collect_recursive(&dep_manifest, visited, out);
        }
    }
}

fn walk_kestrel_sources(dir: &Path, out: &mut Vec<PathBuf>) {
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
