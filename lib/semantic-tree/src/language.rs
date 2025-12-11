/// Trait for symbol kind enums that support transparency checking.
///
/// Symbol kinds can be "transparent" meaning they don't participate in name resolution
/// directly - instead their children are surfaced to the parent scope.
pub trait SymbolKind: Copy + Clone + PartialEq + Eq {
    /// Returns true if this symbol kind is transparent for name resolution.
    fn is_transparent(&self) -> bool;
}

pub trait Language: 'static {
    type BehaviorKind;
    type SymbolKind: SymbolKind;
}
