//! # Recursive Enum Analyzer
//!
//! Detects recursive enums that reference themselves in case parameters without
//! the `indirect` keyword. Non-indirect enums have value semantics and must have
//! a known size at compile time, which is impossible if the enum recursively
//! contains itself.
//!
//! ## Status: Shell
//!
//! This analyzer requires **resolved case payload types** to walk the type graph
//! and detect self-references. The available infrastructure provides:
//! - `IsIndirect` component on enum entities (to skip indirect enums early)
//! - `NodeKind::EnumCase` children with `TypeAnnotation(AstType)` for payload types
//! - `ResolveTypePath` to resolve simple `AstType::Named` to entities
//!
//! What's still missing:
//! - **Transitive type walk**: Detecting recursion requires following type references
//!   through structs, tuples, and nested enums. A simple `ResolveTypePath` check for
//!   direct self-reference (enum's own name in case payload) could be done now, but
//!   would miss indirect recursion (e.g., `enum A { case x(B) }; struct B { var a: A }`).
//! - **Generic type argument tracking**: `Optional[MyEnum]` adds heap indirection via
//!   Optional's enum representation, while `(MyEnum, Int)` does not. Need to know which
//!   generic wrappers provide indirection.
//!
//! Once resolved types with transitive reachability are available, the logic is:
//! 1. Skip if entity has `IsIndirect` component
//! 2. For each EnumCase child, get its payload type(s)
//! 3. Walk the resolved type graph checking for paths back to this enum entity
//! 4. Arrays/Optional/pointers provide heap indirection (not recursive)
//! 5. Tuples, structs, and bare enum references are inline (recursive)
//! 6. Emit E429 if any case has a recursive path without indirection
//!
//! ## Diagnostics
//!
//! ### E429 -- `recursive_enum` (Error, Correctness)
//!
//! **Message:** "enum '{enum_name}' is recursive without 'indirect'"
//!
//! **Labels:**
//! - Primary: the enum declaration
//!   - Span source: `util::entity_span` on the enum entity
//!   - Message: "recursive enum without 'indirect'"
//!
//! **Notes:**
//! - "add 'indirect' before the enum declaration to allow recursive cases"
//! - "indirect enums are heap-allocated, making recursive types representable"

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use kestrel_ast_builder::NodeKind;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E429",
    name: "recursive_enum",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct RecursiveEnumAnalyzer;

impl Describe for RecursiveEnumAnalyzer {
    fn id(&self) -> &'static str {
        "recursive_enum"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for RecursiveEnumAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Enum]
    }

    fn check(&self, _cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // Shell: blocked on resolved case payload types + transitive type walk.
        // See module doc for what's available and what's still needed.
        vec![]
    }
}
