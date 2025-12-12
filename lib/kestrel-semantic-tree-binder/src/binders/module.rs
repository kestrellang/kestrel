use crate::declaration_binder::DeclarationBinder;

/// Binder for module declarations
///
/// Module declarations are handled specially during semantic tree building.
/// They define the namespace hierarchy but aren't created as symbols during
/// the normal tree walk. Instead, the module hierarchy is built before processing
/// other declarations. This binder returns None to skip module declarations
/// during the normal walk.
pub struct ModuleBinder;

impl DeclarationBinder for ModuleBinder {
    fn is_terminal(&self) -> bool {
        // Module declarations don't have children to walk
        true
    }
}
