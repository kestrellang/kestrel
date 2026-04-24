//! Category-based debug tracing for the Kestrel compiler.
//!
//! Enable tracing via the `KESTREL_DEBUG` env var with comma-separated categories:
//!   KESTREL_DEBUG=infer,hir-lower cargo test ...
//!
//! Use `KESTREL_DEBUG=all` to enable every category.
//!
//! Categories are free-form strings — each crate/module defines its own.
//! Common categories: `infer`, `hir-lower`, `name-res`, `solver`, `unify`.

use std::sync::OnceLock;

/// Parsed set of enabled debug categories.
static ENABLED: OnceLock<DebugConfig> = OnceLock::new();

struct DebugConfig {
    all: bool,
    categories: Vec<String>,
}

fn config() -> &'static DebugConfig {
    ENABLED.get_or_init(|| {
        let raw = std::env::var("KESTREL_DEBUG").unwrap_or_default();
        if raw.is_empty() {
            return DebugConfig {
                all: false,
                categories: Vec::new(),
            };
        }
        let cats: Vec<String> = raw.split(',').map(|s| s.trim().to_string()).collect();
        let all = cats.iter().any(|c| c == "all");
        DebugConfig {
            all,
            categories: cats,
        }
    })
}

/// Returns true if the given category is enabled for debug output.
#[inline]
pub fn is_enabled(category: &str) -> bool {
    let cfg = config();
    cfg.all || cfg.categories.iter().any(|c| c == category)
}

/// Debug trace macro with category filtering.
///
/// Usage:
/// ```ignore
/// ktrace!("infer", "unifying {a:?} with {b:?}");
/// ktrace!("solver", "round {round}: {n} constraints remaining");
/// ```
#[macro_export]
macro_rules! ktrace {
    ($cat:expr, $($arg:tt)*) => {
        if $crate::is_enabled($cat) {
            eprintln!(concat!("[", $cat, "] {}"), format_args!($($arg)*));
        }
    };
}
