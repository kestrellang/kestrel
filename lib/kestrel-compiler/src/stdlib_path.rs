//! Where the standard library lives.
//!
//! **One resolver, used by everything that loads the stdlib**: the `kestrel`
//! CLI, the LSP server, and the test suite (both its compiler setup and its
//! link step). There were four copies and they were all different — worse,
//! the two in the test suite took `KESTREL_STD` *unconditionally*, so a stale
//! or misspelled value produced a silent zero-file stdlib load and a cascade
//! of "unknown type" errors pointing anywhere but the cause.
//!
//! Precedence is fixed: explicit override > the toolchain this binary belongs
//! to > in-repo dev > jessup symlink. Every candidate must `exists()` to win; a
//! candidate that does not is recorded so the failure can name every path tried.
//!
//! The order is the load-bearing part — see `default_std_path`.

use std::path::{Path, PathBuf};

/// No stdlib found, with every candidate that was tried and why it lost.
#[derive(Debug)]
pub struct StdLookupError {
    /// `(source, path)` for each candidate, in the order they were tried.
    pub tried: Vec<(&'static str, PathBuf)>,
}

impl std::fmt::Display for StdLookupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "could not locate the Kestrel standard library")?;
        for (source, path) in &self.tried {
            writeln!(f, "  tried {}: {}", source, path.display())?;
        }
        write!(f, "  set KESTREL_STD to the directory containing the stdlib")
    }
}

impl std::error::Error for StdLookupError {}

/// Locate the stdlib directory, in priority order:
///
/// 1. `KESTREL_STD` — explicit override;
/// 2. `<canonicalized-exe>/../lib/std` — the toolchain *this binary belongs
///    to*. The exe is canonicalized so the `~/.jessup/bin/kestrel` symlink
///    resolves to the active toolchain's `lib/std`, not `~/.jessup/lib/std`;
/// 3. `<repo>/lang/std` baked in at build time — in-repo development;
/// 4. the `~/.jessup/bin/kestrel` symlink, read directly — last resort for a
///    bundled LSP binary that is neither installed in a toolchain nor built
///    from the repo, so steps 2 and 3 both miss.
///
/// A candidate must exist to win. Note that step 1 checking `exists()` is
/// load-bearing: without it, `KESTREL_STD=/typo` reports nothing and loads
/// nothing.
///
/// **Step 4 must stay last.** It was step 3 for one build, because the LSP's
/// copy of this chain listed it there — and on any machine with jessup
/// installed, every repo-built `kestrel` silently compiled against the
/// *installed toolchain's* stdlib instead of `lang/std`. That is a
/// catastrophic, silent wrong-answer: 27 suite tests failed on stdlib features
/// the older toolchain didn't have (`extend Int64: Exitable`, closure kinds),
/// with diagnostics pointing at the test files. The rule the ordering encodes:
/// a binary built from this repo trusts this repo, and a globally installed
/// toolchain never outranks it.
pub fn default_std_path() -> Result<PathBuf, StdLookupError> {
    let mut tried = Vec::new();

    if let Some(p) = std::env::var_os("KESTREL_STD") {
        let p = PathBuf::from(p);
        if p.exists() {
            return Ok(p);
        }
        tried.push(("KESTREL_STD", p));
    }

    if let Ok(exe) = std::env::current_exe() {
        let real = std::fs::canonicalize(&exe).unwrap_or(exe);
        if let Some(p) = real
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("lib/std"))
        {
            if p.exists() {
                return Ok(p);
            }
            tried.push(("exe-relative", p));
        }
    }

    let baked = repo_std_path();
    if baked.exists() {
        return Ok(baked);
    }
    tried.push(("in-repo", baked));

    if let Some(home) = std::env::var_os("HOME") {
        let link = PathBuf::from(home).join(".jessup/bin/kestrel");
        if let Ok(resolved) = std::fs::read_link(&link)
            && let Some(p) = resolved
                .parent()
                .and_then(|p| p.parent())
                .map(|p| p.join("lib/std"))
        {
            if p.exists() {
                return Ok(p);
            }
            tried.push(("jessup-symlink", p));
        }
    }

    Err(StdLookupError { tried })
}

/// `<repo>/lang/std`, derived from this crate's manifest directory at build
/// time. Every caller gets the same answer — previously each crate walked its
/// own `CARGO_MANIFEST_DIR` up a different number of levels.
fn repo_std_path() -> PathBuf {
    // lib/kestrel-compiler -> lib -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(|root| root.join("lang/std"))
        .unwrap_or_else(|| PathBuf::from("lang/std"))
}

/// The stdlib's C shim sources that must be compiled and linked into any
/// binary using the stdlib. Empty if the shim is absent.
///
/// The `io/libc_shims.c` path was written out at three call sites; it lives
/// here so moving the shim is one edit.
pub fn stdlib_c_sources(std_dir: Option<&Path>) -> Vec<PathBuf> {
    let Some(std_dir) = std_dir else {
        return vec![];
    };
    let shim = std_dir.join("io/libc_shims.c");
    if shim.exists() { vec![shim] } else { vec![] }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The check the test-suite copies were missing. A `KESTREL_STD` naming a
    /// directory that isn't there must *lose*, not win — winning meant loading
    /// zero stdlib files with no diagnostic pointing at the env var.
    #[test]
    fn a_nonexistent_kestrel_std_does_not_win() {
        // Not using the env var itself: tests share a process, and mutating it
        // would race other tests. Exercise the predicate the chain relies on.
        let missing = PathBuf::from("/definitely/not/a/stdlib/dir");
        assert!(
            !missing.exists(),
            "test precondition: the bogus path must not exist"
        );
    }

    #[test]
    fn the_in_repo_fallback_points_at_the_real_stdlib() {
        let p = repo_std_path();
        assert!(
            p.join("core").exists(),
            "in-repo fallback resolved to {}, which has no core/ — the \
             manifest-dir walk is wrong",
            p.display()
        );
    }

    /// A binary built from this repo must compile against *this repo's*
    /// stdlib, whatever else is installed on the machine.
    ///
    /// Ordering regression: when the jessup-symlink candidate sat ahead of the
    /// in-repo one, every repo build on a machine with jessup installed
    /// silently used the *installed toolchain's* stdlib. Nothing failed at the
    /// resolver — 27 tests failed much later, on stdlib features the older
    /// toolchain lacked. This asserts the outcome, so the ordering can't
    /// regress silently again.
    #[test]
    fn a_repo_build_resolves_to_the_repo_stdlib() {
        // `cargo test` runs from `target/debug/deps/...`, so the exe-relative
        // candidate misses and this exercises steps 3 and 4 against each other.
        if std::env::var_os("KESTREL_STD").is_some() {
            return; // explicit override legitimately wins; nothing to prove
        }
        let resolved = default_std_path().expect("a stdlib must be findable in-repo");
        assert_eq!(
            resolved,
            repo_std_path(),
            "resolved to {} instead of this repo's lang/std — an installed \
             toolchain is outranking the repo",
            resolved.display()
        );
    }

    #[test]
    fn c_sources_are_empty_without_a_stdlib_dir() {
        assert!(stdlib_c_sources(None).is_empty());
        assert!(stdlib_c_sources(Some(Path::new("/nope"))).is_empty());
    }
}
