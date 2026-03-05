use semantic_tree::behavior::Behavior;

use crate::behavior::KestrelBehaviorKind;
use crate::language::KestrelLanguage;

/// Marker behavior indicating that a field is a computed property.
///
/// Presence of this behavior means the field has getter/setter accessors
/// rather than stored storage. Replaces concrete type downcasts for
/// `is_computed()` checks.
#[derive(Debug, Clone)]
pub struct ComputedPropertyMarker;

impl Behavior<KestrelLanguage> for ComputedPropertyMarker {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::ComputedProperty
    }
}
