//! Kestrel MIR — Mid-level Intermediate Representation
//!
//! A flat, explicit representation of Kestrel programs suitable for analysis,
//! optimization, and codegen. Sits between the typed AST (ECS) and Cranelift IR.
//!
//! # Design Principles
//!
//! 1. **Separate, self-describing artifact** — can be printed without the ECS
//! 2. **Flat namespace** — all items fully qualified
//! 3. **Explicit** — self types, generics, calling conventions all visible
//! 4. **Place/Value distinction** — places are memory locations, values are computed
//! 5. **No SSA** — places can be reassigned (like Rust MIR)
//! 6. **Generic** — monomorphization happens at codegen time
//! 7. **Deterministic** — ordered maps for reproducible output

pub mod body;
pub mod builder;
pub mod display;
pub mod id;
pub mod immediate;
pub mod item;
pub mod op;
pub mod passes;
pub mod place;
pub mod statement;
pub mod terminator;
pub mod ty;
pub mod value;
pub mod witness_lookup;

// Re-export core types
pub use body::{BasicBlock, LocalDef, MirBody, ScopeId};
pub use builder::{BlockBuilder, FunctionBuilder};
pub use id::*;
pub use immediate::{Immediate, ImmediateKind};
pub use item::{
    AssociatedTypeDef, CallingConvention, CaptureInfo, CaptureMode, ClosureInfo, CopyBehavior,
    DeinitBehavior, EnumCaseDef, EnumDef, ExternInfo, FieldDef, FileConstantData, FunctionDef,
    FunctionKind, MethodBinding, MethodSource, ParamDef, ProtocolDef, ProtocolMethodDef,
    ReceiverConvention, StaticDef, StructDef, StructLayout, TypeParamDef, WhereClause,
    WhereConstraint, WitnessDef, WitnessMethodKey,
};
pub use op::{
    FloatBits, FloatConstantKind, FloatMathKind, FloatPredicateKind, IntBits, Op, Signedness,
};
pub use place::Place;
pub use statement::{Callee, Rvalue, Statement, StatementKind};
pub use terminator::{SwitchCase, Terminator, TerminatorKind};
pub use ty::MirTy;
pub use value::Value;

use indexmap::IndexMap;
use kestrel_hecs::Entity;

/// The top-level MIR container. A complete, self-describing snapshot of the
/// compiled program.
#[derive(Debug, Clone)]
pub struct MirModule {
    /// Module name.
    pub name: String,

    // === Items ===
    pub structs: Vec<StructDef>,
    pub enums: Vec<EnumDef>,
    pub protocols: Vec<ProtocolDef>,
    pub witnesses: Vec<WitnessDef>,
    pub functions: Vec<FunctionDef>,
    pub statics: Vec<StaticDef>,
    pub closures: Vec<ClosureInfo>,

    // === Module-level metadata ===
    /// The program entry point (main function).
    pub entry_point: Option<FunctionId>,
    /// Module initialization function (runs static initializers).
    pub module_init: Option<FunctionId>,

    // === Name resolution for display ===
    /// Maps entity references to their qualified names so display works
    /// without the ECS.
    pub entity_names: IndexMap<Entity, String>,

    /// Number of `MirTy::Error` locations detected by the post-lowering
    /// validator. Nonzero means some upstream phase (bind/inference/type
    /// lowering) silently fell back to `MirTy::Error`; codegen must not run
    /// because Cranelift's IR type-checker would panic.
    pub lowering_error_count: usize,
}

impl MirModule {
    /// Create a new empty module.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            structs: Vec::new(),
            enums: Vec::new(),
            protocols: Vec::new(),
            witnesses: Vec::new(),
            functions: Vec::new(),
            statics: Vec::new(),
            closures: Vec::new(),
            entry_point: None,
            module_init: None,
            entity_names: IndexMap::new(),
            lowering_error_count: 0,
        }
    }

    // === Name registration ===

    /// Register a name for an entity (for display).
    pub fn register_name(&mut self, entity: Entity, name: impl Into<String>) {
        self.entity_names.insert(entity, name.into());
    }

    /// Resolve an entity to its registered name, or return a placeholder.
    pub fn resolve_name(&self, entity: Entity) -> &str {
        self.entity_names
            .get(&entity)
            .map(|s| s.as_str())
            .unwrap_or("<unknown>")
    }

    /// Access the full entity name map (for associated type resolution).
    pub fn name_map(&self) -> &IndexMap<Entity, String> {
        &self.entity_names
    }

    /// Resolve a local ID to its name. Searches all function bodies.
    ///
    /// This is used by display code for `DropIf` which references locals by
    /// ID outside of a function context. For efficiency, callers displaying a
    /// whole function should use the body's locals directly.
    pub fn resolve_local_name(&self, id: LocalId) -> &str {
        // Search through all functions for a body containing this local
        for func in &self.functions {
            if let Some(body) = &func.body
                && id.index() < body.locals.len()
            {
                return &body.locals[id.index()].name;
            }
        }
        "<unknown_local>"
    }

    // === Item addition ===

    /// Add a struct and return its ID.
    pub fn add_struct(&mut self, def: StructDef) -> StructId {
        let id = StructId::new(self.structs.len());
        self.structs.push(def);
        id
    }

    /// Add an enum and return its ID.
    pub fn add_enum(&mut self, def: EnumDef) -> EnumId {
        let id = EnumId::new(self.enums.len());
        self.enums.push(def);
        id
    }

    /// Add a protocol and return its ID.
    pub fn add_protocol(&mut self, def: ProtocolDef) -> ProtocolId {
        let id = ProtocolId::new(self.protocols.len());
        self.protocols.push(def);
        id
    }

    /// Add a witness and return its ID.
    pub fn add_witness(&mut self, def: WitnessDef) -> WitnessId {
        let id = WitnessId::new(self.witnesses.len());
        self.witnesses.push(def);
        id
    }

    /// Add a function and return its ID.
    pub fn add_function(&mut self, def: FunctionDef) -> FunctionId {
        let id = FunctionId::new(self.functions.len());
        self.functions.push(def);
        id
    }

    /// Add a static and return its ID.
    pub fn add_static(&mut self, def: StaticDef) -> StaticId {
        let id = StaticId::new(self.statics.len());
        self.statics.push(def);
        id
    }

    /// Add closure info and return its ID.
    pub fn add_closure(&mut self, info: ClosureInfo) -> ClosureId {
        let id = ClosureId::new(self.closures.len());
        self.closures.push(info);
        id
    }

    // === Builders ===

    /// Get a builder for a function.
    pub fn function_builder(&mut self, id: FunctionId) -> FunctionBuilder<'_> {
        FunctionBuilder::new(self, id)
    }

    // === Passes (chainable) ===

    /// Run the layout pass: compute struct sizes, field offsets, alignment.
    pub fn with_layouts(mut self) -> Self {
        passes::run_layout_pass(&mut self);
        self
    }

    /// Run the thunk pass: generate/deduplicate thunk wrappers for ApplyPartial.
    pub fn with_thunks(mut self) -> Self {
        passes::run_thunk_pass(&mut self);
        self
    }

    /// Unified drop elaboration: dataflow-based destructor insertion + expansion.
    pub fn with_drop_elaboration(mut self) -> Self {
        passes::run_drop_elaboration(&mut self);
        self
    }

    /// Run MIR verification and print diagnostics. Does not abort — reports all issues.
    pub fn verify(&self) -> passes::VerifyResult {
        passes::verify(self)
    }

    /// Run all post-lowering passes in the recommended order.
    pub fn with_all_passes(self) -> Self {
        self.with_drop_elaboration().with_thunks().with_layouts()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a dummy entity for testing.
    fn dummy_entity(id: u32) -> Entity {
        Entity::from_raw(id)
    }

    #[test]
    fn test_basic_function() {
        let mut module = MirModule::new("test");

        // Register entity names
        let func_entity = dummy_entity(1);
        module.register_name(func_entity, "example.add");

        // Add function
        let func_def = FunctionDef::new(func_entity, "example.add", MirTy::I64);
        let func_id = module.add_function(func_def);

        // Build function body
        {
            let mut func = module.function_builder(func_id);
            let x = func.param("x", MirTy::I64);
            let y = func.param("y", MirTy::I64);
            let result = func.local("result", MirTy::I64);

            let mut bb0 = func.add_block();
            bb0.assign_op2(
                Place::local(result),
                Op::Add(IntBits::I64, Signedness::Signed),
                Place::local(x),
                Place::local(y),
            );
            bb0.ret(Place::local(result));
        }

        // Verify
        let func_def = &module.functions[func_id.index()];
        assert_eq!(func_def.params.len(), 2);
        let body = func_def.body.as_ref().unwrap();
        assert_eq!(body.locals.len(), 3); // 2 params + 1 local
        assert_eq!(body.blocks.len(), 1);
        assert_eq!(body.param_count, 2);

        // Print and verify
        let output = module.display().to_string();
        assert!(output.contains("func example.add"));
        assert!(output.contains("x: i64"));
        assert!(output.contains("y: i64"));
        assert!(output.contains("i64.add.signed"));
    }

    #[test]
    fn test_struct_definition() {
        let mut module = MirModule::new("test");

        let struct_entity = dummy_entity(1);
        let mut def = StructDef::new(struct_entity, "example.Point");
        def.add_field(FieldDef::new("x", MirTy::I64));
        def.add_field(FieldDef::new("y", MirTy::I64));
        module.add_struct(def);

        let output = module.display().to_string();
        assert!(output.contains("struct example.Point"));
        assert!(output.contains("x: i64"));
        assert!(output.contains("y: i64"));
    }

    #[test]
    fn test_control_flow() {
        let mut module = MirModule::new("test");

        let func_entity = dummy_entity(1);
        module.register_name(func_entity, "example.abs");
        let func_def = FunctionDef::new(func_entity, "example.abs", MirTy::I64);
        let func_id = module.add_function(func_def);

        {
            let mut func = module.function_builder(func_id);
            let x = func.param("x", MirTy::I64);
            let result = func.local("result", MirTy::I64);
            let is_neg = func.local("is_neg", MirTy::Bool);

            // Create all blocks first to get their IDs
            let bb0 = func.add_block().id();
            let bb_neg = func.add_block().id();
            let bb_pos = func.add_block().id();
            let bb_ret = func.add_block().id();

            // bb0: check if negative
            {
                let mut b = func.block(bb0);
                b.assign_op2(
                    Place::local(is_neg),
                    Op::Lt(IntBits::I64, Signedness::Signed),
                    Place::local(x),
                    Immediate::i64(0),
                );
                b.branch(Place::local(is_neg), bb_neg, bb_pos);
            }

            // bb_neg: negate
            {
                let mut b = func.block(bb_neg);
                b.assign_op1(Place::local(result), Op::Neg(IntBits::I64), Place::local(x));
                b.jump(bb_ret);
            }

            // bb_pos: copy
            {
                let mut b = func.block(bb_pos);
                b.assign_copy(Place::local(result), Place::local(x));
                b.jump(bb_ret);
            }

            // bb_ret: return
            {
                let mut b = func.block(bb_ret);
                b.ret(Place::local(result));
            }
        }

        let func_def = &module.functions[func_id.index()];
        assert_eq!(func_def.body.as_ref().unwrap().blocks.len(), 4);

        let output = module.display().to_string();
        assert!(output.contains("branch if"));
        assert!(output.contains("jump bb"));
    }

    #[test]
    fn test_enum_and_witness() {
        let mut module = MirModule::new("test");

        // Create payload structs for enum cases
        let none_entity = dummy_entity(10);
        let some_entity = dummy_entity(11);
        let none_struct = module.add_struct(StructDef::new(none_entity, "Optional.cases.None"));
        let mut some_def = StructDef::new(some_entity, "Optional.cases.Some");
        some_def.add_field(FieldDef::new("0", MirTy::I64));
        let some_struct = module.add_struct(some_def);

        // Create enum
        let enum_entity = dummy_entity(1);
        let mut enum_def = EnumDef::new(enum_entity, "Optional");
        enum_def.add_case(EnumCaseDef::new("None", 0, none_struct));
        enum_def.add_case(EnumCaseDef::new("Some", 1, some_struct));
        module.add_enum(enum_def);

        // Create witness
        let protocol_entity = dummy_entity(2);
        let impl_entity = dummy_entity(3);
        module.register_name(protocol_entity, "Equatable");
        module.register_name(impl_entity, "Optional.eq");

        let optional_ty = MirTy::Named {
            entity: enum_entity,
            type_args: vec![],
        };
        let mut witness = WitnessDef::new(optional_ty, protocol_entity);
        witness.bind_method("eq", MethodBinding::direct(impl_entity, vec![]));
        module.add_witness(witness);

        let output = module.display().to_string();
        assert!(output.contains("enum Optional"));
        assert!(output.contains("None"));
        assert!(output.contains("Some"));
        assert!(output.contains("witness"));
        assert!(output.contains("Equatable"));
    }
}
