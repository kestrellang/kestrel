use crate::declaration_binder::DeclarationBinder;

/// Terminal binder for wrapper nodes that don't produce symbols
/// Used for nodes like Visibility and Name that should stop tree traversal
pub struct TerminalBinder;

impl DeclarationBinder for TerminalBinder {
    fn is_terminal(&self) -> bool {
        true
    }
}
