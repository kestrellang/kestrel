//! Metadata and provenance tracking for MIR nodes.

use crate::id::Id;
use kestrel_span::Span;
use std::sync::Arc;

/// Metadata attached to any MIR node.
#[derive(Debug, Clone, Default)]
pub struct Metadata {
    /// Source span (preserved through lowering).
    pub span: Option<Span>,

    /// Where this item originated from (for cross-type provenance).
    pub origin: Option<Origin>,

    /// Debug comments (printed as `// comment`).
    pub comments: Vec<String>,
}

impl Metadata {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_span(span: Span) -> Self {
        Self {
            span: Some(span),
            ..Self::default()
        }
    }

    pub fn set_span(&mut self, span: Span) -> &mut Self {
        self.span = Some(span);
        self
    }

    pub fn set_origin(&mut self, origin: Origin) -> &mut Self {
        self.origin = Some(origin);
        self
    }

    pub fn add_comment(&mut self, comment: impl Into<String>) -> &mut Self {
        self.comments.push(comment.into());
        self
    }
}

/// Records same-type transformation history.
///
/// When a pass transforms a node, the new node can store the original
/// via `Prior<T>`, enabling debugging and analysis of transformations.
#[derive(Debug, Clone)]
pub struct Prior<T> {
    /// Which pass made this transformation.
    pub pass_name: String,

    /// Description of what the transformation did.
    pub description: Option<String>,

    /// The original node before transformation.
    pub original: Arc<T>,
}

impl<T> Prior<T> {
    pub fn new(pass_name: impl Into<String>, original: T) -> Self {
        Self {
            pass_name: pass_name.into(),
            description: None,
            original: Arc::new(original),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Cross-type provenance for items that originated from other constructs.
#[derive(Debug, Clone)]
pub enum Origin {
    /// Lowered directly from semantic tree.
    Source { span: Span },

    /// Generated as a closure's environment struct.
    ClosureEnv {
        containing_function: Id<crate::id::QualifiedName>,
        closure_span: Span,
    },

    /// Generated as a closure's call method.
    ClosureCall {
        env_struct: Id<crate::id::Struct>,
        closure_span: Span,
    },

    /// Generated as a thunk for a function reference used as a thick callable.
    /// Thunks bridge the calling convention gap between regular functions and
    /// thick function pointers by accepting an env_ptr parameter and ignoring it.
    FunctionThunk {
        /// The original function being wrapped.
        original_function: Id<crate::id::QualifiedName>,
    },

    /// Synthesized by a pass.
    Synthesized { pass_name: String, reason: String },
}
