//! Kestrel Execution Graph - Mid-level Intermediate Representation (MIR)
//!
//! The Execution Graph is a flat, explicit representation of Kestrel programs
//! suitable for analysis and optimization. It sits between the semantic tree
//! (typed AST) and LLVM IR.
//!
//! # Design Principles
//!
//! 1. **Flat namespace** - All items are fully qualified, no nesting
//! 2. **Explicit** - Self types, generics, calling conventions all visible
//! 3. **Place/Value distinction** - Places are memory locations, values are computed results
//! 4. **No SSA** - Places can be reassigned (like Rust MIR)
//! 5. **Generic** - Monomorphization happens at LLVM lowering
//!
//! # Example
//!
//! ```rust
//! use kestrel_execution_graph::*;
//!
//! let mut ctx = MirContext::new();
//!
//! // Intern types
//! let i64_ty = ctx.intern_type(MirTy::I64);
//! let unit_ty = ctx.intern_type(MirTy::Unit);
//!
//! // Intern name
//! let name = ctx.intern_name(QualifiedNameData::from_parts(&["example", "add"]));
//!
//! // Build function
//! let mut func = ctx.add_function(name, i64_ty);
//! let x = func.param("x", i64_ty);
//! let y = func.param("y", i64_ty);
//! let result = func.local("result", i64_ty);
//!
//! let mut bb0 = func.add_block();
//! bb0.assign_binop(
//!     Place::local(result),
//!     BinOp::AddSigned,
//!     Place::local(x),
//!     Place::local(y),
//! );
//! bb0.ret(Place::local(result));
//!
//! // Print the MIR
//! println!("{}", ctx.display());
//! ```

pub mod id;
pub mod metadata;
pub mod qualified_name;
pub mod ty;
pub mod item;
pub mod function;
pub mod builder;
pub mod pass;

// Re-export main types at crate root
pub use id::{
    Arena, AssociatedType, Block, Enum, EnumCase, Field, Function, Id, Local, Param, Protocol,
    ProtocolMethod, QualifiedName, Static, Statement, Struct, Ty, TypeParam, Witness,
};
pub use metadata::{Metadata, Origin, Prior};
pub use qualified_name::QualifiedNameData;
pub use ty::MirTy;
pub use item::{
    AssociatedTypeDef, EnumCaseDef, EnumDef, FieldDef, FunctionDef, ParamDef, ProtocolDef,
    ProtocolMethodDef, StaticDef, StructDef, WhereClause, WhereConstraint, WitnessDef,
};
pub use function::{
    BasicBlock, BinOp, Callee, CallArg, CastKind, FloatBits, Immediate, ImmediateKind, IntBits,
    LocalDef, PassingMode, Place, PlaceKind, Rvalue, Statement as StatementData, StatementKind,
    Terminator, TerminatorKind, TypeParamDef, TypeParamOwner, UnOp, Value,
};
pub use builder::{BlockBuilder, FunctionBuilder};
pub use pass::{FunctionPass, FunctionPassAdapter, MirPass, PassManager, PassResult};

use std::collections::HashMap;
use std::fmt;

/// The central context holding all MIR data.
///
/// All MIR items are stored in arenas within this context. Items reference
/// each other via `Id<T>` handles.
#[derive(Debug, Clone, Default)]
pub struct MirContext {
    // === Top-level items ===
    pub structs: Arena<Struct, StructDef>,
    pub enums: Arena<Enum, EnumDef>,
    pub protocols: Arena<Protocol, ProtocolDef>,
    pub witnesses: Arena<Witness, WitnessDef>,
    pub functions: Arena<Function, FunctionDef>,
    pub statics: Arena<Static, StaticDef>,

    // === Struct/enum children ===
    pub fields: Arena<Field, FieldDef>,
    pub enum_cases: Arena<EnumCase, EnumCaseDef>,

    // === Protocol children ===
    pub associated_types: Arena<AssociatedType, AssociatedTypeDef>,
    pub protocol_methods: Arena<ProtocolMethod, ProtocolMethodDef>,

    // === Function children ===
    pub blocks: Arena<Block, BasicBlock>,
    pub statements: Arena<Statement, StatementData>,
    pub locals: Arena<Local, LocalDef>,
    pub params: Arena<Param, ParamDef>,

    // === Type system ===
    pub type_params: Arena<TypeParam, TypeParamDef>,
    types: Arena<Ty, MirTy>,
    type_lookup: HashMap<MirTy, Id<Ty>>,

    // === Names ===
    names: Arena<QualifiedName, QualifiedNameData>,
    name_lookup: HashMap<QualifiedNameData, Id<QualifiedName>>,
}

impl MirContext {
    /// Create a new empty context.
    pub fn new() -> Self {
        Self::default()
    }

    // === Type interning ===

    /// Intern a type, returning its ID.
    ///
    /// If the type already exists, returns the existing ID.
    pub fn intern_type(&mut self, ty: MirTy) -> Id<Ty> {
        if let Some(&id) = self.type_lookup.get(&ty) {
            return id;
        }
        let id = self.types.alloc(ty.clone());
        self.type_lookup.insert(ty, id);
        id
    }

    /// Get a type by its ID.
    pub fn ty(&self, id: Id<Ty>) -> &MirTy {
        &self.types[id]
    }

    // === Primitive type helpers ===

    /// Intern the i8 type.
    pub fn ty_i8(&mut self) -> Id<Ty> {
        self.intern_type(MirTy::I8)
    }

    /// Intern the i16 type.
    pub fn ty_i16(&mut self) -> Id<Ty> {
        self.intern_type(MirTy::I16)
    }

    /// Intern the i32 type.
    pub fn ty_i32(&mut self) -> Id<Ty> {
        self.intern_type(MirTy::I32)
    }

    /// Intern the i64 type.
    pub fn ty_i64(&mut self) -> Id<Ty> {
        self.intern_type(MirTy::I64)
    }

    /// Intern the f32 type.
    pub fn ty_f32(&mut self) -> Id<Ty> {
        self.intern_type(MirTy::F32)
    }

    /// Intern the f64 type.
    pub fn ty_f64(&mut self) -> Id<Ty> {
        self.intern_type(MirTy::F64)
    }

    /// Intern the bool type.
    pub fn ty_bool(&mut self) -> Id<Ty> {
        self.intern_type(MirTy::Bool)
    }

    /// Intern the unit type.
    pub fn ty_unit(&mut self) -> Id<Ty> {
        self.intern_type(MirTy::Unit)
    }

    /// Intern the never type.
    pub fn ty_never(&mut self) -> Id<Ty> {
        self.intern_type(MirTy::Never)
    }

    /// Intern the str type.
    pub fn ty_str(&mut self) -> Id<Ty> {
        self.intern_type(MirTy::Str)
    }

    /// Intern a pointer type.
    pub fn ty_ptr(&mut self, inner: Id<Ty>) -> Id<Ty> {
        self.intern_type(MirTy::Pointer(inner))
    }

    /// Intern an immutable reference type.
    pub fn ty_ref(&mut self, inner: Id<Ty>) -> Id<Ty> {
        self.intern_type(MirTy::Ref(inner))
    }

    /// Intern a mutable reference type.
    pub fn ty_ref_mut(&mut self, inner: Id<Ty>) -> Id<Ty> {
        self.intern_type(MirTy::RefMut(inner))
    }

    /// Intern a tuple type.
    pub fn ty_tuple(&mut self, elems: Vec<Id<Ty>>) -> Id<Ty> {
        self.intern_type(MirTy::Tuple(elems))
    }

    /// Intern an array type.
    pub fn ty_array(&mut self, elem: Id<Ty>) -> Id<Ty> {
        self.intern_type(MirTy::Array(elem))
    }

    /// Intern a named type.
    pub fn ty_named(&mut self, name: Id<QualifiedName>, type_args: Vec<Id<Ty>>) -> Id<Ty> {
        self.intern_type(MirTy::Named { name, type_args })
    }

    /// Intern the Self type (for protocol method signatures).
    pub fn ty_self(&mut self) -> Id<Ty> {
        self.intern_type(MirTy::SelfType)
    }

    /// Intern an associated type projection.
    ///
    /// This represents `T.Element` where `T: Protocol` and `Element` is an
    /// associated type of `Protocol`. During monomorphization, this is resolved
    /// to the concrete type from the witness table.
    pub fn ty_assoc_projection(
        &mut self,
        base: Id<Ty>,
        protocol: Id<QualifiedName>,
        associated: impl Into<String>,
    ) -> Id<Ty> {
        self.intern_type(MirTy::AssociatedTypeProjection {
            base,
            protocol,
            associated: associated.into(),
        })
    }

    /// Intern the error type.
    /// Used when lowering fails and a placeholder type is needed.
    pub fn ty_error(&mut self) -> Id<Ty> {
        self.intern_type(MirTy::Error)
    }

    // === Name interning ===

    /// Intern a qualified name, returning its ID.
    ///
    /// If the name already exists, returns the existing ID.
    pub fn intern_name(&mut self, name: QualifiedNameData) -> Id<QualifiedName> {
        if let Some(&id) = self.name_lookup.get(&name) {
            return id;
        }
        let id = self.names.alloc(name.clone());
        self.name_lookup.insert(name, id);
        id
    }

    /// Intern a name from string parts.
    pub fn intern_name_parts(&mut self, parts: &[&str]) -> Id<QualifiedName> {
        self.intern_name(QualifiedNameData::from_parts(parts))
    }

    /// Get a name by its ID.
    pub fn name(&self, id: Id<QualifiedName>) -> &QualifiedNameData {
        &self.names[id]
    }

    // === Convenience accessors ===

    /// Get a function by its ID.
    pub fn function(&self, id: Id<Function>) -> &FunctionDef {
        &self.functions[id]
    }

    /// Get a mutable function by its ID.
    pub fn function_mut(&mut self, id: Id<Function>) -> &mut FunctionDef {
        &mut self.functions[id]
    }

    /// Get a block by its ID.
    pub fn block(&self, id: Id<Block>) -> &BasicBlock {
        &self.blocks[id]
    }

    /// Get a mutable block by its ID.
    pub fn block_mut(&mut self, id: Id<Block>) -> &mut BasicBlock {
        &mut self.blocks[id]
    }

    /// Get a statement by its ID.
    pub fn statement(&self, id: Id<Statement>) -> &StatementData {
        &self.statements[id]
    }

    /// Get a mutable statement by its ID.
    pub fn statement_mut(&mut self, id: Id<Statement>) -> &mut StatementData {
        &mut self.statements[id]
    }

    /// Get a local by its ID.
    pub fn local(&self, id: Id<Local>) -> &LocalDef {
        &self.locals[id]
    }

    /// Get a type parameter by its ID.
    pub fn type_param(&self, id: Id<TypeParam>) -> &TypeParamDef {
        &self.type_params[id]
    }

    /// Get a struct by its ID.
    pub fn struct_def(&self, id: Id<Struct>) -> &StructDef {
        &self.structs[id]
    }

    /// Get an enum by its ID.
    pub fn enum_def(&self, id: Id<Enum>) -> &EnumDef {
        &self.enums[id]
    }

    // === Builders ===

    /// Add a new function and return a builder for it.
    pub fn add_function(&mut self, name: Id<QualifiedName>, ret: Id<Ty>) -> FunctionBuilder<'_> {
        let def = FunctionDef::new(name, ret);
        let id = self.functions.alloc(def);
        FunctionBuilder { ctx: self, id }
    }

    /// Get a builder for an existing function.
    pub fn function_builder(&mut self, id: Id<Function>) -> FunctionBuilder<'_> {
        FunctionBuilder { ctx: self, id }
    }

    /// Add a new struct.
    pub fn add_struct(&mut self, name: Id<QualifiedName>) -> Id<Struct> {
        let def = StructDef::new(name);
        self.structs.alloc(def)
    }

    /// Add a field to a struct.
    pub fn add_field(
        &mut self,
        struct_id: Id<Struct>,
        name: impl Into<String>,
        ty: Id<Ty>,
    ) -> Id<Field> {
        let name = name.into();
        let field = FieldDef::new(name.clone(), ty);
        let field_id = self.fields.alloc(field);
        self.structs[struct_id].add_field(name, field_id);
        field_id
    }

    /// Add a new enum.
    pub fn add_enum(&mut self, name: Id<QualifiedName>) -> Id<Enum> {
        let def = EnumDef::new(name);
        self.enums.alloc(def)
    }

    /// Add a case to an enum.
    pub fn add_enum_case(
        &mut self,
        enum_id: Id<Enum>,
        name: impl Into<String>,
        struct_name: Id<QualifiedName>,
    ) -> Id<EnumCase> {
        let name = name.into();
        let discriminant = self.enums[enum_id].cases.len() as u32;
        let case = EnumCaseDef::new(name.clone(), discriminant, struct_name);
        let case_id = self.enum_cases.alloc(case);
        self.enums[enum_id].add_case(name, case_id);
        case_id
    }

    /// Add a new protocol.
    pub fn add_protocol(&mut self, name: Id<QualifiedName>) -> Id<Protocol> {
        let def = ProtocolDef::new(name);
        self.protocols.alloc(def)
    }

    /// Add a new witness.
    pub fn add_witness(
        &mut self,
        implementing_type: Id<Ty>,
        protocol: Id<QualifiedName>,
    ) -> Id<Witness> {
        let def = WitnessDef::new(implementing_type, protocol);
        self.witnesses.alloc(def)
    }

    /// Add a new static.
    pub fn add_static(&mut self, name: Id<QualifiedName>, ty: Id<Ty>) -> Id<Static> {
        let def = StaticDef::new(name, ty);
        self.statics.alloc(def)
    }

    // === Display ===

    /// Create a display wrapper for printing the entire context.
    pub fn display(&self) -> impl fmt::Display + '_ {
        MirContextDisplay { ctx: self }
    }
}

struct MirContextDisplay<'a> {
    ctx: &'a MirContext,
}

impl fmt::Display for MirContextDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Print structs
        for (_, def) in self.ctx.structs.iter() {
            writeln!(f, "{}\n", def.display(self.ctx))?;
        }

        // Print enums
        for (_, def) in self.ctx.enums.iter() {
            writeln!(f, "{}\n", def.display(self.ctx))?;
        }

        // Print protocols
        for (_, def) in self.ctx.protocols.iter() {
            writeln!(f, "{}\n", def.display(self.ctx))?;
        }

        // Print witnesses
        for (_, def) in self.ctx.witnesses.iter() {
            writeln!(f, "{}\n", def.display(self.ctx))?;
        }

        // Print statics
        for (_, def) in self.ctx.statics.iter() {
            writeln!(f, "{}\n", def.display(self.ctx))?;
        }

        // Print functions
        for (_, def) in self.ctx.functions.iter() {
            writeln!(f, "{}\n", def.display(self.ctx))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_function() {
        let mut ctx = MirContext::new();

        // Intern types
        let i64_ty = ctx.ty_i64();

        // Intern name
        let name = ctx.intern_name_parts(&["example", "add"]);

        // Build function and get its ID
        let func_id = {
            let mut func = ctx.add_function(name, i64_ty);
            let x = func.param("x", i64_ty);
            let y = func.param("y", i64_ty);
            let result = func.local("result", i64_ty);

            let mut bb0 = func.add_block();
            bb0.assign_binop(
                Place::local(result),
                BinOp::AddSigned,
                Place::local(x),
                Place::local(y),
            );
            bb0.ret(Place::local(result));
            func.id()
        };

        // Verify
        let func_def = ctx.function(func_id);
        assert_eq!(func_def.params.len(), 2);
        assert_eq!(func_def.locals.len(), 3); // 2 params + 1 local
        assert_eq!(func_def.blocks.len(), 1);

        // Print and verify output contains expected elements
        let output = ctx.display().to_string();
        assert!(output.contains("func example.add"));
        assert!(output.contains("x: i64"));
        assert!(output.contains("y: i64"));
        assert!(output.contains("i64.add.signed"));
    }

    #[test]
    fn test_struct_definition() {
        let mut ctx = MirContext::new();

        let i64_ty = ctx.ty_i64();
        let name = ctx.intern_name_parts(&["example", "Point"]);

        let struct_id = ctx.add_struct(name);
        ctx.add_field(struct_id, "x", i64_ty);
        ctx.add_field(struct_id, "y", i64_ty);

        let output = ctx.display().to_string();
        assert!(output.contains("struct example.Point"));
        assert!(output.contains("x: i64"));
        assert!(output.contains("y: i64"));
    }

    #[test]
    fn test_control_flow() {
        let mut ctx = MirContext::new();

        let i64_ty = ctx.ty_i64();
        let bool_ty = ctx.ty_bool();
        let name = ctx.intern_name_parts(&["example", "abs"]);

        let func_id = {
            let mut func = ctx.add_function(name, i64_ty);
            let x = func.param("x", i64_ty);
            let result = func.local("result", i64_ty);
            let is_neg = func.local("is_neg", bool_ty);

            // bb0: check if negative
            let bb0 = func.add_block().id();
            let bb_neg = func.add_block().id();
            let bb_pos = func.add_block().id();
            let bb_ret = func.add_block().id();

            {
                let mut b = func.block(bb0);
                b.assign_binop(
                    Place::local(is_neg),
                    BinOp::LtSigned,
                    Place::local(x),
                    Immediate::i64(0),
                );
                b.branch(Place::local(is_neg), bb_neg, bb_pos);
            }

            {
                let mut b = func.block(bb_neg);
                b.assign_unop(Place::local(result), UnOp::Neg, Place::local(x));
                b.jump(bb_ret);
            }

            {
                let mut b = func.block(bb_pos);
                b.assign_copy(Place::local(result), Place::local(x));
                b.jump(bb_ret);
            }

            {
                let mut b = func.block(bb_ret);
                b.ret(Place::local(result));
            }

            func.id()
        };

        let func_def = ctx.function(func_id);
        assert_eq!(func_def.blocks.len(), 4);

        let output = ctx.display().to_string();
        assert!(output.contains("branch if"));
        assert!(output.contains("jump bb"));
    }
}
