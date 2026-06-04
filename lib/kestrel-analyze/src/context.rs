//! Analysis context types — one per granularity level.
//!
//! Each context provides the data an analyzer needs for its check.

use kestrel_ast_builder::NodeKind;
use kestrel_hecs::{Entity, QueryContext};
use kestrel_hir::body::HirBody;
use kestrel_type_infer::result::TypedBody;

/// Context for body-level analysis (functions, inits, getters).
pub struct BodyContext<'a> {
    /// ECS query access (read components, call sub-queries).
    pub query: &'a QueryContext<'a>,
    /// The function/init entity being analyzed.
    pub entity: Entity,
    /// Root module entity.
    pub root: Entity,
    /// Desugared HIR body.
    pub hir: &'a HirBody,
    /// Type inference results (resolved types for all exprs/locals).
    pub typed: &'a TypedBody,
}

/// Context for declaration-level analysis (structs, enums, protocols).
pub struct DeclContext<'a> {
    /// ECS query access.
    pub query: &'a QueryContext<'a>,
    /// The declaration entity being analyzed.
    pub entity: Entity,
    /// Root module entity.
    pub root: Entity,
    /// What kind of declaration this is.
    pub kind: NodeKind,
}

/// Context for whole-compilation analysis.
pub struct CompilationContext<'a> {
    /// ECS query access.
    pub query: &'a QueryContext<'a>,
    /// Root module entity.
    pub root: Entity,
    /// True when the current compilation is producing an executable binary
    /// (`kestrel build`, execution tests). Library / `kestrel check` / LSP /
    /// diagnostics-test runs pass `false`. Gates the "missing `@main`" check
    /// (E618), which must not fire on non-executable compilations.
    pub is_executable: bool,
}
