use std::sync::{Arc, RwLock};

use kestrel_span::Span;
use semantic_tree::symbol::{Symbol, SymbolMetadata, SymbolMetadataBuilder};

use crate::{
    language::KestrelLanguage,
    symbol::kind::KestrelSymbolKind,
    symbol::local::{Local, LocalContainer, LocalId},
    ty::Ty,
};

/// Represents a deinitializer declaration in the semantic tree.
///
/// Deinit blocks are called automatically when a value goes out of scope.
/// They provide RAII-style resource cleanup similar to Rust's `Drop` trait
/// or C++ destructors.
///
/// # Key Properties
///
/// - **No parameters**: Deinit takes no arguments (implicit `self` only)
/// - **No return value**: Deinit cannot return anything
/// - **At most one per struct**: Each struct can have at most one deinit
/// - **Runs before field drops**: The deinit body runs while `self` is still valid
/// - **Cannot be called directly**: Only invoked by the runtime when values are dropped
///
/// # Drop Order
///
/// When a value is dropped:
/// 1. The deinit body executes (if present)
/// 2. Fields are dropped in reverse declaration order
///
/// # Example
///
/// ```kestrel
/// struct FileHandle: not Copyable {
///     var fd: Int
///     
///     deinit {
///         close(self.fd)
///     }
/// }
/// ```
#[derive(Debug)]
pub struct DeinitSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
    /// Local variables within this deinit (populated during body resolution)
    /// This includes any let/var declarations in the deinit body.
    locals: RwLock<Vec<Local>>,
}

impl Symbol<KestrelLanguage> for DeinitSymbol {
    fn metadata(&self) -> &SymbolMetadata<KestrelLanguage> {
        &self.metadata
    }
}

impl LocalContainer for DeinitSymbol {
    fn add_local(&self, name: String, ty: Ty, mutable: bool, span: Span) -> LocalId {
        let mut locals = self.locals.write().unwrap();
        let id = LocalId::new(locals.len());
        locals.push(Local::new(id, name, ty, mutable, span));
        id
    }

    fn get_local(&self, id: LocalId) -> Option<Local> {
        self.read_locals().get(id.index()).cloned()
    }

    fn locals(&self) -> Vec<Local> {
        self.read_locals().clone()
    }

    fn local_count(&self) -> usize {
        self.read_locals().len()
    }

    fn update_local_type(&self, id: LocalId, ty: Ty) {
        let mut locals = self.locals.write().unwrap();
        if let Some(local) = locals.get_mut(id.index()) {
            *local.ty_mut() = ty;
        }
    }
}

impl DeinitSymbol {
    fn read_locals(&self) -> std::sync::RwLockReadGuard<'_, Vec<Local>> {
        self.locals.read().unwrap()
    }

    /// Create a new DeinitSymbol
    ///
    /// Deinit has no visibility (always private to the type) and no parameters.
    pub fn new(
        span: Span,
        declaration_span: Span,
        parent: Option<Arc<dyn Symbol<KestrelLanguage>>>,
    ) -> Self {
        // Deinit uses "deinit" as its name for display purposes
        let name = kestrel_span::Name::new("deinit".to_string(), declaration_span.clone());

        let mut builder = SymbolMetadataBuilder::new(KestrelSymbolKind::Deinit)
            .with_name(name)
            .with_declaration_span(declaration_span)
            .with_span(span);

        if let Some(p) = parent {
            builder = builder.with_parent(Arc::downgrade(&p));
        }

        DeinitSymbol {
            metadata: builder.build(),
            locals: RwLock::new(Vec::new()),
        }
    }
}
