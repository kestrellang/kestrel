//! # Visibility Consistency Analyzer
//!
//! Ensures that public APIs don't expose less-visible types:
//! - Public functions can't have private/internal parameter types
//! - Public functions can't have private/internal return types
//! - Public type aliases can't alias less-visible types
//! - Public fields can't have less-visible types
//!
//! ## Status: Shell
//!
//! This analyzer requires **resolved types with visibility information**. The
//! available infrastructure provides:
//! - `ResolveTypePath` to resolve `AstType::Named` to entities
//! - `Visibility` component on entities (to check entity's own visibility)
//! - `TypeAnnotation(AstType)` on callables, fields, type aliases
//!
//! What's still missing:
//! - **Deep type visibility walk**: A function returning `Array[InternalType]` needs
//!   to check visibility of `InternalType` inside the generic argument. This requires
//!   recursively walking `AstType` and resolving each `Named` segment, including type
//!   arguments. Simple single-segment `Named` types can be checked now, but compound
//!   types (tuples, functions, generics) need recursive resolution.
//! - **Protocol method context**: Methods declared in a public protocol are implicitly
//!   public, even if the method itself has no explicit visibility modifier. Need to
//!   detect protocol context to avoid false positives.
//! - **Callable parameter/return type access**: `Callable` component stores `AstType`
//!   for params and return type, which can be resolved via `ResolveTypePath`, but
//!   the recursive walk for generic args is still needed.
//!
//! Once recursive type resolution + visibility queries are available:
//! 1. Get entity's `Visibility` — skip if not `Public`
//! 2. For functions: check each param type and return type
//! 3. For type aliases: check the aliased type
//! 4. For fields: check the field type
//! 5. Recursively walk type arguments for generic types
//! 6. Emit E430-433 for each less-visible referenced type
//!
//! ## Diagnostics
//!
//! ### E430 -- `return_type_less_visible` (Error, Correctness)
//!
//! **Message:** "public function '{name}' has a {type_visibility} return type"
//!
//! **Labels:**
//! - Primary: the function declaration
//!   - Span source: `util::entity_span` on the function entity
//!   - Message: "less-visible return type"
//!
//! **Notes:** (none)
//!
//! ### E431 -- `parameter_type_less_visible` (Error, Correctness)
//!
//! **Message:** "public function '{name}' has a {type_visibility} parameter type"
//!
//! **Labels:**
//! - Primary: the function declaration
//!   - Span source: `util::entity_span` on the function entity
//!   - Message: "less-visible parameter type"
//!
//! **Notes:** (none)
//!
//! ### E432 -- `aliased_type_less_visible` (Error, Correctness)
//!
//! **Message:** "public type alias '{name}' aliases a {type_visibility} type"
//!
//! **Labels:**
//! - Primary: the type alias declaration
//!   - Span source: `util::entity_span` on the type alias entity
//!   - Message: "less-visible aliased type"
//!
//! **Notes:** (none)
//!
//! ### E433 -- `field_type_less_visible` (Error, Correctness)
//!
//! **Message:** "public field '{name}' has a {type_visibility} type"
//!
//! **Labels:**
//! - Primary: the field declaration
//!   - Span source: `util::entity_span` on the field entity
//!   - Message: "less-visible field type"
//!
//! **Notes:** (none)

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use kestrel_ast_builder::NodeKind;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E430",
        name: "return_type_less_visible",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E431",
        name: "parameter_type_less_visible",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E432",
        name: "aliased_type_less_visible",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E433",
        name: "field_type_less_visible",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct VisibilityAnalyzer;

impl Describe for VisibilityAnalyzer {
    fn id(&self) -> &'static str {
        "visibility"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for VisibilityAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Function, NodeKind::TypeAlias, NodeKind::Field]
    }

    fn check(&self, _cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // Shell: blocked on recursive type resolution + visibility walk.
        // See module doc for what's available and what's still needed.
        vec![]
    }
}
