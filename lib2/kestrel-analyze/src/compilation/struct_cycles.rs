//! # Struct Cycle Analyzer
//!
//! Detects circular struct containment that would produce infinite-size types.
//! A struct that (transitively) contains itself by value cannot be represented
//! in memory because its layout would be infinitely recursive.
//!
//! The walk follows struct field types through other structs and tuples, but
//! stops at heap-indirected types (Array, pointers) and function types.
//! Computed properties are skipped since they don't store values.
//!
//! TODO: This check requires resolved field types. Currently the ECS stores
//! `TypeAnnotation(AstType)` on field entities, which is unresolved. Cycle
//! detection needs to walk resolved field types and check for struct
//! self-references.
//!
//! ## Diagnostics
//!
//! ### KS449 -- `self_containing_struct` (Error, Correctness)
//!
//! **Message:** "struct '{name}' contains itself through field '{field_name}'"
//!
//! **Labels:**
//! - Primary: the struct declaration
//!   - Span source: `util::entity_span` on the struct entity
//!   - Message: "struct contains itself"
//! - Secondary: the field causing the self-reference
//!   - Span source: `util::entity_span` on the field entity
//!   - Message: "self-referencing field"
//!
//! **Notes:**
//! - "use an Array or Optional to break the cycle with heap indirection"
//!
//! ### KS450 -- `circular_struct_containment` (Error, Correctness)
//!
//! **Message:** "circular struct containment: '{origin}' -> {cycle} -> '{origin}'"
//!
//! **Labels:**
//! - Primary: the origin struct
//!   - Span source: `util::entity_span` on the origin struct entity
//!   - Message: "cycle starts here"
//! - Secondary: each struct in the cycle
//!   - Span source: `util::entity_span` on each participating struct entity
//!   - Message: "part of the cycle"
//!
//! **Notes:**
//! - "use an Array or Optional to break the cycle with heap indirection"

use crate::context::CompilationContext;
use crate::diagnostic::*;
use crate::traits::{CompilationCheck, Describe};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "KS449",
        name: "self_containing_struct",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "KS450",
        name: "circular_struct_containment",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct StructCycleAnalyzer;

impl Describe for StructCycleAnalyzer {
    fn id(&self) -> &'static str {
        "struct_cycles"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl CompilationCheck for StructCycleAnalyzer {
    fn check(&self, cx: &CompilationContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // TODO: Struct cycle detection requires:
        // 1. Walk all Struct entities in the world
        // 2. For each struct, walk its stored fields (skip computed properties)
        // 3. Resolve each field's type and recursively check for struct references
        // 4. Use a visited set / cycle detector to find back-edges
        // 5. Stop at heap-indirected types (Array, Optional via enum, pointers)
        //
        // Currently the ECS stores AstType (unresolved) on field entities.
        // Cycle detection needs resolved field types. Shell returns empty.
        let _ = cx;
        vec![]
    }
}
