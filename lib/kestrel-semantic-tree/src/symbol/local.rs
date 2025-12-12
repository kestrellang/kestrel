use kestrel_span::Span;

use crate::ty::Ty;

/// A unique identifier for a local variable within a function
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(pub usize);

impl LocalId {
    /// Create a new LocalId
    pub fn new(id: usize) -> Self {
        LocalId(id)
    }

    /// Get the underlying index
    pub fn index(&self) -> usize {
        self.0
    }
}

/// Represents a local variable within a function.
///
/// Locals are created by:
/// - Function parameters
/// - let/var declarations
/// - Pattern bindings (in the future)
///
/// Each local has a unique ID within its function. When variables are shadowed,
/// a new Local is created with a different ID, even though they may share the
/// same source name.
///
/// For code generation, locals are given unique names like `name_0`, `name_1`, etc.
#[derive(Debug, Clone)]
pub struct Local {
    /// The unique ID of this local within its function
    id: LocalId,
    /// The source name of this local (without uniquification)
    name: String,
    /// The type of this local
    ty: Ty,
    /// Whether this local is mutable (var) or immutable (let)
    mutable: bool,
    /// The span where this local is defined
    span: Span,
}

impl Local {
    /// Create a new Local
    pub fn new(id: LocalId, name: String, ty: Ty, mutable: bool, span: Span) -> Self {
        Local {
            id,
            name,
            ty,
            mutable,
            span,
        }
    }

    /// Get the unique ID of this local
    pub fn id(&self) -> LocalId {
        self.id
    }

    /// Get the source name of this local
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the unique codegen name for this local (e.g., "x_0", "x_1")
    pub fn codegen_name(&self) -> String {
        format!("{}_{}", self.name, self.id.0)
    }

    /// Get the type of this local
    pub fn ty(&self) -> &Ty {
        &self.ty
    }

    /// Get a mutable reference to the type
    pub fn ty_mut(&mut self) -> &mut Ty {
        &mut self.ty
    }

    /// Check if this local is mutable
    pub fn is_mutable(&self) -> bool {
        self.mutable
    }

    /// Get the span where this local is defined
    pub fn span(&self) -> &Span {
        &self.span
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_span::Span;

    #[test]
    fn test_local_basic() {
        use crate::ty::IntBits;
        let ty = Ty::int(IntBits::I64, Span::from(0..3));
        let local = Local::new(
            LocalId::new(0),
            "x".to_string(),
            ty,
            false,
            Span::from(0..5),
        );

        assert_eq!(local.id().index(), 0);
        assert_eq!(local.name(), "x");
        assert_eq!(local.codegen_name(), "x_0");
        assert!(!local.is_mutable());
    }

    #[test]
    fn test_local_mutable() {
        use crate::ty::IntBits;
        let ty = Ty::int(IntBits::I64, Span::from(0..3));
        let local = Local::new(LocalId::new(1), "y".to_string(), ty, true, Span::from(0..5));

        assert_eq!(local.id().index(), 1);
        assert_eq!(local.name(), "y");
        assert_eq!(local.codegen_name(), "y_1");
        assert!(local.is_mutable());
    }

    #[test]
    fn test_local_shadowing_names() {
        use crate::ty::IntBits;
        let ty = Ty::int(IntBits::I64, Span::from(0..3));
        let local0 = Local::new(
            LocalId::new(0),
            "x".to_string(),
            ty.clone(),
            false,
            Span::from(0..5),
        );
        let local1 = Local::new(
            LocalId::new(1),
            "x".to_string(),
            ty.clone(),
            false,
            Span::from(10..15),
        );

        // Same source name, different codegen names
        assert_eq!(local0.name(), local1.name());
        assert_ne!(local0.codegen_name(), local1.codegen_name());
        assert_eq!(local0.codegen_name(), "x_0");
        assert_eq!(local1.codegen_name(), "x_1");
    }
}
