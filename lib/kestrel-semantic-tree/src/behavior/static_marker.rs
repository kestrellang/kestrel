use semantic_tree::behavior::Behavior;

use crate::behavior::KestrelBehaviorKind;
use crate::language::KestrelLanguage;

/// Marker behavior indicating that a symbol is static.
///
/// Presence of this behavior means the symbol is static (e.g., a static field,
/// static method, or static subscript). Replaces concrete type downcasts for
/// `is_static()` checks.
#[derive(Debug, Clone)]
pub struct StaticBehavior;

impl Behavior<KestrelLanguage> for StaticBehavior {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::Static
    }
}
