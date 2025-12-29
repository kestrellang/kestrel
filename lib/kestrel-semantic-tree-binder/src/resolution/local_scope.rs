//! Local scope management for function body resolution
//!
//! This module provides a scope stack for tracking local variable bindings
//! within function bodies. It supports shadowing - when a new variable with
//! the same name is declared, it creates a new Local and shadows the old one.

use std::collections::HashMap;
use std::sync::Arc;

use kestrel_semantic_tree::symbol::local::{LocalContainer, LocalId};
use kestrel_semantic_tree::ty::Ty;
use kestrel_span::Span;

/// A scope level in the local scope stack
#[derive(Debug, Clone)]
struct ScopeLevel {
    /// Bindings introduced at this scope level: name -> LocalId
    bindings: HashMap<String, LocalId>,
}

impl ScopeLevel {
    fn new() -> Self {
        ScopeLevel {
            bindings: HashMap::new(),
        }
    }
}

/// Manages local variable scopes within a function body
///
/// The scope stack supports:
/// - Nested scopes (blocks, if/else, loops, etc.)
/// - Variable shadowing (same name can be rebound)
/// - O(1) lookup of current binding for a name
///
/// # Example
///
/// ```ignore
/// fn example(x: Int) {
///     let y = x + 1;     // y binds to Local(1)
///     {
///         let y = y * 2;  // shadows y, binds to Local(2), RHS uses Local(1)
///         print(y);       // uses Local(2)
///     }
///     print(y);           // uses Local(1) again
/// }
/// ```
#[derive(Debug)]
pub struct LocalScope {
    /// The container we're building locals for
    container: Arc<dyn LocalContainer>,
    /// Stack of scope levels (innermost at the end)
    scopes: Vec<ScopeLevel>,
    /// Cached lookup: name -> current LocalId (for O(1) access)
    current_bindings: HashMap<String, LocalId>,
    /// History stack for restoring bindings when exiting scopes
    shadow_stack: Vec<Vec<(String, Option<LocalId>)>>,
    /// Tracks which scope depth each local was created at (for capture analysis)
    local_depths: HashMap<LocalId, usize>,
}

impl LocalScope {
    /// Create a new LocalScope for the given container
    pub fn new(container: Arc<dyn LocalContainer>) -> Self {
        let mut scope = LocalScope {
            container,
            scopes: Vec::new(),
            current_bindings: HashMap::new(),
            shadow_stack: Vec::new(),
            local_depths: HashMap::new(),
        };
        // Start with the function's parameter scope
        scope.push_scope();
        scope
    }

    /// Push a new scope level (e.g., entering a block)
    pub fn push_scope(&mut self) {
        self.scopes.push(ScopeLevel::new());
        self.shadow_stack.push(Vec::new());
    }

    /// Pop a scope level, restoring previous bindings
    pub fn pop_scope(&mut self) {
        self.scopes.pop();

        // Restore previous bindings
        if let Some(shadows) = self.shadow_stack.pop() {
            for (name, prev_id) in shadows.into_iter().rev() {
                match prev_id {
                    Some(id) => {
                        self.current_bindings.insert(name, id);
                    }
                    None => {
                        self.current_bindings.remove(&name);
                    }
                }
            }
        }
    }

    /// Bind a new local variable in the current scope.
    /// Returns the LocalId for the new binding.
    pub fn bind(&mut self, name: String, ty: Ty, mutable: bool, span: Span) -> LocalId {
        // Record the previous binding (if any) for restoration
        let prev = self.current_bindings.get(&name).copied();
        if let Some(shadows) = self.shadow_stack.last_mut() {
            shadows.push((name.clone(), prev));
        }

        // Create a new local in the container
        let local_id = self.container.add_local(name.clone(), ty, mutable, span);

        // Record the scope depth this local was created at (for capture analysis)
        self.local_depths.insert(local_id, self.scopes.len());

        // Update current bindings
        self.current_bindings.insert(name.clone(), local_id);

        // Record in current scope level
        if let Some(scope) = self.scopes.last_mut() {
            scope.bindings.insert(name, local_id);
        }

        local_id
    }

    /// Look up a name in the current scope.
    /// Returns the LocalId if found, None otherwise.
    pub fn lookup(&self, name: &str) -> Option<LocalId> {
        self.current_bindings.get(name).copied()
    }

    /// Get the container this scope is for
    pub fn container(&self) -> &Arc<dyn LocalContainer> {
        &self.container
    }

    /// Get a local by ID from the container
    pub fn get_local(&self, id: LocalId) -> Option<kestrel_semantic_tree::symbol::local::Local> {
        self.container.get_local(id)
    }

    /// Get the current scope depth (for debugging)
    pub fn depth(&self) -> usize {
        self.scopes.len()
    }

    /// Get the scope depth at which a local was created.
    /// Used for capture analysis to determine if a variable is from an outer scope.
    pub fn scope_depth_of(&self, id: LocalId) -> Option<usize> {
        self.local_depths.get(&id).copied()
    }

    /// Snapshot the current name->LocalId bindings.
    /// Used for or-pattern resolution to restore bindings after each alternative.
    pub fn snapshot_bindings(&self) -> HashMap<String, LocalId> {
        self.current_bindings.clone()
    }

    /// Restore bindings from a snapshot.
    /// Used for or-pattern resolution to ensure the arm body sees the first alternative's bindings.
    pub fn restore_bindings(&mut self, bindings: HashMap<String, LocalId>) {
        self.current_bindings = bindings;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_semantic_tree::behavior::visibility::{Visibility, VisibilityBehavior};
    use kestrel_semantic_tree::language::KestrelLanguage;
    use kestrel_semantic_tree::symbol::function::FunctionSymbol;
    use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
    use kestrel_span::Name;
    use kestrel_span::Span;
    use semantic_tree::symbol::{Symbol, SymbolMetadataBuilder};

    // Helper to create a test root symbol for visibility scope
    fn create_test_root() -> Arc<dyn Symbol<KestrelLanguage>> {
        let root_name = Name::new("TestRoot".to_string(), Span::from(0..8));
        let metadata = SymbolMetadataBuilder::new(KestrelSymbolKind::Module)
            .with_name(root_name)
            .with_declaration_span(Span::from(0..8))
            .with_span(Span::from(0..100))
            .build();

        #[derive(Debug)]
        struct TestRootSymbol {
            metadata: semantic_tree::symbol::SymbolMetadata<KestrelLanguage>,
        }

        impl Symbol<KestrelLanguage> for TestRootSymbol {
            fn metadata(&self) -> &semantic_tree::symbol::SymbolMetadata<KestrelLanguage> {
                &self.metadata
            }
        }

        Arc::new(TestRootSymbol { metadata })
    }

    fn create_test_function() -> Arc<FunctionSymbol> {
        let root = create_test_root();
        let name = Name::new("test".to_string(), Span::from(0..4));
        let visibility =
            VisibilityBehavior::new(Some(Visibility::Internal), Span::from(0..0), root);
        let return_type = Ty::unit(Span::from(0..2));

        Arc::new(FunctionSymbol::new(
            name,
            Span::from(0..50),
            visibility,
            true, // is_static
            true, // has_body
            None, // no parent
        ))
    }

    #[test]
    fn test_simple_binding() {
        use kestrel_semantic_tree::ty::IntBits;
        let func = create_test_function();
        let mut scope = LocalScope::new(func.clone());

        let ty = Ty::int(IntBits::I64, Span::from(0..3));
        let id = scope.bind("x".to_string(), ty, false, Span::from(0..5));

        assert_eq!(scope.lookup("x"), Some(id));
        assert_eq!(func.local_count(), 1);
    }

    #[test]
    fn test_shadowing() {
        use kestrel_semantic_tree::ty::IntBits;
        let func = create_test_function();
        let mut scope = LocalScope::new(func.clone());

        let ty = Ty::int(IntBits::I64, Span::from(0..3));

        // First binding
        let id1 = scope.bind("x".to_string(), ty.clone(), false, Span::from(0..5));
        assert_eq!(scope.lookup("x"), Some(id1));

        // Push new scope and shadow
        scope.push_scope();
        let id2 = scope.bind("x".to_string(), ty.clone(), false, Span::from(10..15));
        assert_eq!(scope.lookup("x"), Some(id2));
        assert_ne!(id1, id2);

        // Pop scope - should restore old binding
        scope.pop_scope();
        assert_eq!(scope.lookup("x"), Some(id1));

        // Function should have 2 locals
        assert_eq!(func.local_count(), 2);
    }

    #[test]
    fn test_nested_scopes() {
        use kestrel_semantic_tree::ty::IntBits;
        let func = create_test_function();
        let mut scope = LocalScope::new(func.clone());

        let ty = Ty::int(IntBits::I64, Span::from(0..3));

        let id_a = scope.bind("a".to_string(), ty.clone(), false, Span::from(0..1));

        scope.push_scope();
        let id_b = scope.bind("b".to_string(), ty.clone(), false, Span::from(5..6));

        scope.push_scope();
        let id_c = scope.bind("c".to_string(), ty.clone(), false, Span::from(10..11));

        // All visible at innermost scope
        assert_eq!(scope.lookup("a"), Some(id_a));
        assert_eq!(scope.lookup("b"), Some(id_b));
        assert_eq!(scope.lookup("c"), Some(id_c));

        scope.pop_scope();
        assert_eq!(scope.lookup("a"), Some(id_a));
        assert_eq!(scope.lookup("b"), Some(id_b));
        assert_eq!(scope.lookup("c"), None);

        scope.pop_scope();
        assert_eq!(scope.lookup("a"), Some(id_a));
        assert_eq!(scope.lookup("b"), None);
        assert_eq!(scope.lookup("c"), None);
    }
}
