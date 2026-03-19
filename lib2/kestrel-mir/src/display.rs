//! Display implementations for MIR types.
//!
//! Types that need entity name resolution use the pattern:
//!   `value.display(module) -> impl Display`
//!
//! Types that are self-contained (Op, IntBits, etc.) implement Display directly
//! in their own files.

use crate::MirModule;
use crate::body::BasicBlock;
use crate::immediate::{Immediate, ImmediateKind};
use crate::item::*;
use crate::place::Place;
use crate::statement::{Callee, CallArg, Rvalue, Statement, StatementKind};
use crate::terminator::{Terminator, TerminatorKind};
use crate::ty::MirTy;
use crate::value::Value;
use std::fmt;

// === Display helpers ===

/// Write a comma-separated list of displayable items.
fn write_comma_sep(
    f: &mut fmt::Formatter<'_>,
    items: &[impl DisplayWithModule],
    module: &MirModule,
) -> fmt::Result {
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            write!(f, ", ")?;
        }
        item.fmt_with(f, module)?;
    }
    Ok(())
}

/// Write type parameters: `[T, U]` or nothing if empty.
fn write_type_params(
    f: &mut fmt::Formatter<'_>,
    params: &[TypeParamDef],
) -> fmt::Result {
    if !params.is_empty() {
        write!(f, "[")?;
        for (i, tp) in params.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", tp.name)?;
        }
        write!(f, "]")?;
    }
    Ok(())
}

/// Trait for types that can be displayed with a module reference.
trait DisplayWithModule {
    fn fmt_with(&self, f: &mut fmt::Formatter<'_>, module: &MirModule) -> fmt::Result;
}

// === MirTy ===

impl MirTy {
    pub fn display<'a>(&'a self, module: &'a MirModule) -> impl fmt::Display + 'a {
        MirTyDisplay { ty: self, module }
    }
}

impl DisplayWithModule for MirTy {
    fn fmt_with(&self, f: &mut fmt::Formatter<'_>, module: &MirModule) -> fmt::Result {
        write!(f, "{}", self.display(module))
    }
}

struct MirTyDisplay<'a> {
    ty: &'a MirTy,
    module: &'a MirModule,
}

impl fmt::Display for MirTyDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.ty {
            MirTy::I8 => write!(f, "i8"),
            MirTy::I16 => write!(f, "i16"),
            MirTy::I32 => write!(f, "i32"),
            MirTy::I64 => write!(f, "i64"),
            MirTy::F16 => write!(f, "f16"),
            MirTy::F32 => write!(f, "f32"),
            MirTy::F64 => write!(f, "f64"),
            MirTy::Bool => write!(f, "bool"),
            MirTy::Unit => write!(f, "()"),
            MirTy::Never => write!(f, "!"),
            MirTy::Str => write!(f, "str"),
            MirTy::Pointer(inner) => write!(f, "p[{}]", inner.display(self.module)),
            MirTy::Ref(inner) => write!(f, "&{}", inner.display(self.module)),
            MirTy::RefMut(inner) => write!(f, "&var {}", inner.display(self.module)),
            MirTy::Tuple(elems) => {
                write!(f, "(")?;
                write_comma_sep(f, elems, self.module)?;
                write!(f, ")")
            },
            MirTy::Named { entity, type_args } => {
                write!(f, "{}", self.module.resolve_name(*entity))?;
                if !type_args.is_empty() {
                    write!(f, "[")?;
                    write_comma_sep(f, type_args, self.module)?;
                    write!(f, "]")?;
                }
                Ok(())
            },
            MirTy::TypeParam(entity) => {
                write!(f, "{}", self.module.resolve_name(*entity))
            },
            MirTy::SelfType => write!(f, "Self"),
            MirTy::AssociatedProjection {
                base,
                protocol,
                name,
            } => {
                write!(
                    f,
                    "({}.{} for {})",
                    self.module.resolve_name(*protocol),
                    name,
                    base.display(self.module),
                )
            },
            MirTy::FuncThin { params, ret } => {
                write!(f, "func(")?;
                write_comma_sep(f, params, self.module)?;
                write!(f, ") -> {}", ret.display(self.module))
            },
            MirTy::FuncThick { params, ret } => {
                write!(f, "func escaping(")?;
                write_comma_sep(f, params, self.module)?;
                write!(f, ") -> {}", ret.display(self.module))
            },
            MirTy::Error => write!(f, "<error>"),
        }
    }
}

// === Place ===

impl Place {
    pub fn display<'a>(&'a self, module: &'a MirModule) -> impl fmt::Display + 'a {
        PlaceDisplay {
            place: self,
            module,
        }
    }
}

struct PlaceDisplay<'a> {
    place: &'a Place,
    module: &'a MirModule,
}

impl fmt::Display for PlaceDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.place {
            Place::Local(id) => {
                write!(f, "%{}", self.module.resolve_local_name(*id))
            },
            Place::Global(entity) => {
                write!(f, "@{}", self.module.resolve_name(*entity))
            },
            Place::Field { parent, name } => {
                write!(f, "{}.{}", parent.display(self.module), name)
            },
            Place::Index { parent, index } => {
                write!(f, "{}.{}", parent.display(self.module), index)
            },
            Place::Downcast { parent, variant } => {
                write!(f, "{}.{}", parent.display(self.module), variant)
            },
            Place::Deref(inner) => {
                write!(f, "(deref {})", inner.display(self.module))
            },
        }
    }
}

// === Value ===

impl Value {
    pub fn display<'a>(&'a self, module: &'a MirModule) -> impl fmt::Display + 'a {
        ValueDisplay {
            value: self,
            module,
        }
    }
}

struct ValueDisplay<'a> {
    value: &'a Value,
    module: &'a MirModule,
}

impl fmt::Display for ValueDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.value {
            Value::Place(p) => write!(f, "{}", p.display(self.module)),
            Value::Immediate(i) => write!(f, "{}", i.display(self.module)),
        }
    }
}

// === Immediate ===

impl Immediate {
    pub fn display<'a>(&'a self, module: &'a MirModule) -> impl fmt::Display + 'a {
        ImmediateDisplay { imm: self, module }
    }
}

struct ImmediateDisplay<'a> {
    imm: &'a Immediate,
    module: &'a MirModule,
}

impl fmt::Display for ImmediateDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.imm.kind {
            ImmediateKind::IntLiteral { bits, value } => {
                write!(f, "{}.literal {}", bits, value)
            },
            ImmediateKind::FloatLiteral { bits, value } => {
                write!(f, "{}.literal {}", bits, value)
            },
            ImmediateKind::BoolLiteral(b) => write!(f, "{}", b),
            ImmediateKind::StringLiteral(s) => write!(f, "str.literal {:?}", s),
            ImmediateKind::StringPointer(s) => write!(f, "str.ptr {:?}", s),
            ImmediateKind::Unit => write!(f, "()"),
            ImmediateKind::FunctionRef { func, type_args } => {
                write!(f, "{}", self.module.resolve_name(*func))?;
                if !type_args.is_empty() {
                    write!(f, "[")?;
                    write_comma_sep(f, type_args, self.module)?;
                    write!(f, "]")?;
                }
                Ok(())
            },
            ImmediateKind::WitnessMethod {
                protocol,
                method,
                for_type,
            } => {
                write!(
                    f,
                    "witness_method {}.{} for {}",
                    self.module.resolve_name(*protocol),
                    method,
                    for_type.display(self.module),
                )
            },
            ImmediateKind::NullPtr(ty) => {
                write!(f, "ptr.null[{}]", ty.display(self.module))
            },
            ImmediateKind::Error => write!(f, "<error>"),
        }
    }
}

// === Statement ===

impl Statement {
    pub fn display<'a>(&'a self, module: &'a MirModule) -> impl fmt::Display + 'a {
        StatementDisplay {
            stmt: self,
            module,
        }
    }
}

struct StatementDisplay<'a> {
    stmt: &'a Statement,
    module: &'a MirModule,
}

impl fmt::Display for StatementDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.stmt.kind {
            StatementKind::Assign { dest, rvalue } => {
                write!(
                    f,
                    "{} = {}",
                    dest.display(self.module),
                    rvalue.display(self.module),
                )
            },
            StatementKind::Call { dest, callee, args } => {
                if let Some(d) = dest {
                    write!(f, "{} = ", d.display(self.module))?;
                }
                write!(f, "call {}", callee.display(self.module))?;
                write!(f, "(")?;
                write_call_args(f, args, self.module)?;
                write!(f, ")")
            },
            StatementKind::Deinit { place } => {
                write!(f, "deinit {}", place.display(self.module))
            },
            StatementKind::DeinitIf { place, flag } => {
                write!(
                    f,
                    "deinit {} if %{}",
                    place.display(self.module),
                    self.module.resolve_local_name(*flag),
                )
            },
            StatementKind::SetDeinitFlag { flag, value } => {
                write!(
                    f,
                    "%{} = {}",
                    self.module.resolve_local_name(*flag),
                    value,
                )
            },
        }
    }
}

// === Rvalue ===

impl Rvalue {
    pub fn display<'a>(&'a self, module: &'a MirModule) -> impl fmt::Display + 'a {
        RvalueDisplay {
            rvalue: self,
            module,
        }
    }
}

struct RvalueDisplay<'a> {
    rvalue: &'a Rvalue,
    module: &'a MirModule,
}

impl fmt::Display for RvalueDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.rvalue {
            Rvalue::Move(p) => write!(f, "move {}", p.display(self.module)),
            Rvalue::Copy(p) => write!(f, "copy {}", p.display(self.module)),
            Rvalue::Ref(p) => write!(f, "ref {}", p.display(self.module)),
            Rvalue::RefMut(p) => write!(f, "ref var {}", p.display(self.module)),
            Rvalue::Const(imm) => write!(f, "{}", imm.display(self.module)),
            Rvalue::Op1 { op, arg } => {
                write!(f, "{} {}", op, arg.display(self.module))
            },
            Rvalue::Op2 { op, lhs, rhs } => {
                write!(
                    f,
                    "{} {}, {}",
                    op,
                    lhs.display(self.module),
                    rhs.display(self.module),
                )
            },
            Rvalue::Op3 { op, a, b, c } => {
                write!(
                    f,
                    "{} {}, {}, {}",
                    op,
                    a.display(self.module),
                    b.display(self.module),
                    c.display(self.module),
                )
            },
            Rvalue::Construct { ty, fields } => {
                write!(f, "construct {} {{ ", ty.display(self.module))?;
                for (i, (name, value)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", name, value.display(self.module))?;
                }
                write!(f, " }}")
            },
            Rvalue::Tuple(elements) => {
                write!(f, "tuple (")?;
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", elem.display(self.module))?;
                }
                write!(f, ")")
            },
            Rvalue::ApplyPartial { func, captures } => {
                write!(f, "apply partial {}", self.module.resolve_name(*func))?;
                write!(f, "(")?;
                for (i, cap) in captures.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", cap.display(self.module))?;
                }
                write!(f, ")")
            },
            Rvalue::EnumVariant {
                enum_ty,
                variant,
                payload,
            } => {
                write!(f, "enum {}.{}", enum_ty.display(self.module), variant)?;
                if !payload.is_empty() {
                    write!(f, "(")?;
                    for (i, val) in payload.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", val.display(self.module))?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            },
        }
    }
}

// === Callee ===

impl Callee {
    pub fn display<'a>(&'a self, module: &'a MirModule) -> impl fmt::Display + 'a {
        CalleeDisplay {
            callee: self,
            module,
        }
    }
}

struct CalleeDisplay<'a> {
    callee: &'a Callee,
    module: &'a MirModule,
}

impl fmt::Display for CalleeDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.callee {
            Callee::Direct {
                func, type_args, ..
            } => {
                write!(f, "{}", self.module.resolve_name(*func))?;
                if !type_args.is_empty() {
                    write!(f, "[")?;
                    write_comma_sep(f, type_args, self.module)?;
                    write!(f, "]")?;
                }
                Ok(())
            },
            Callee::Thin(p) => write!(f, "{}", p.display(self.module)),
            Callee::Thick(p) => write!(f, "escaping {}", p.display(self.module)),
            Callee::Witness {
                protocol,
                method,
                self_type,
                method_type_args,
            } => {
                write!(
                    f,
                    "witness_method {}.{}",
                    self.module.resolve_name(*protocol),
                    method,
                )?;
                if !method_type_args.is_empty() {
                    write!(f, "[")?;
                    write_comma_sep(f, method_type_args, self.module)?;
                    write!(f, "]")?;
                }
                write!(f, " for {}", self_type.display(self.module))
            },
        }
    }
}

/// Write call arguments with passing modes.
fn write_call_args(
    f: &mut fmt::Formatter<'_>,
    args: &[CallArg],
    module: &MirModule,
) -> fmt::Result {
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            write!(f, ", ")?;
        }
        write!(f, "{} {}", arg.mode, arg.value.display(module))?;
    }
    Ok(())
}

// === Terminator ===

impl Terminator {
    pub fn display<'a>(
        &'a self,
        module: &'a MirModule,
    ) -> impl fmt::Display + 'a {
        TerminatorDisplay {
            term: self,
            module,
        }
    }
}

struct TerminatorDisplay<'a> {
    term: &'a Terminator,
    module: &'a MirModule,
}

impl fmt::Display for TerminatorDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.term.kind {
            TerminatorKind::Return(v) => {
                write!(f, "return {}", v.display(self.module))
            },
            TerminatorKind::Jump(target) => {
                write!(f, "jump bb{}", target.index())
            },
            TerminatorKind::Branch {
                condition,
                then_block,
                else_block,
            } => {
                write!(
                    f,
                    "branch if {}, bb{} else bb{}",
                    condition.display(self.module),
                    then_block.index(),
                    else_block.index(),
                )
            },
            TerminatorKind::Switch {
                discriminant,
                cases,
            } => {
                writeln!(f, "switch {} {{", discriminant.display(self.module))?;
                for (case_name, target) in cases {
                    writeln!(f, "    {} => bb{}", case_name, target.index())?;
                }
                write!(f, "}}")
            },
            TerminatorKind::Panic(msg) => write!(f, "panic {:?}", msg),
            TerminatorKind::Unreachable => write!(f, "unreachable"),
        }
    }
}

// === BasicBlock ===

impl BasicBlock {
    pub fn display<'a>(
        &'a self,
        module: &'a MirModule,
        indent: &'a str,
    ) -> impl fmt::Display + 'a {
        BasicBlockDisplay {
            block: self,
            module,
            indent,
        }
    }
}

struct BasicBlockDisplay<'a> {
    block: &'a BasicBlock,
    module: &'a MirModule,
    indent: &'a str,
}

impl fmt::Display for BasicBlockDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for stmt in &self.block.stmts {
            writeln!(f, "{}{}", self.indent, stmt.display(self.module))?;
        }
        writeln!(f, "{}{}", self.indent, self.block.terminator.display(self.module))
    }
}

// === FunctionDef ===

impl FunctionDef {
    pub fn display<'a>(&'a self, module: &'a MirModule) -> impl fmt::Display + 'a {
        FunctionDefDisplay { def: self, module }
    }
}

struct FunctionDefDisplay<'a> {
    def: &'a FunctionDef,
    module: &'a MirModule,
}

impl fmt::Display for FunctionDefDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "func {}", self.def.name)?;
        write_type_params(f, &self.def.type_params)?;

        // Parameters
        write!(f, "(")?;
        for (i, param) in self.def.params.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}: {}", param.name, param.ty.display(self.module))?;
        }
        write!(f, ") -> {}", self.def.ret.display(self.module))?;

        // Where clause
        if let Some(wc) = &self.def.where_clause {
            write!(f, "\nwhere ")?;
            for (i, constraint) in wc.constraints.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                match constraint {
                    WhereConstraint::Implements {
                        type_param,
                        protocol,
                    } => {
                        write!(
                            f,
                            "{}: {}",
                            self.module.resolve_name(*type_param),
                            self.module.resolve_name(*protocol),
                        )?;
                    },
                    WhereConstraint::TypeEquals {
                        base,
                        associated,
                        equals,
                    } => {
                        write!(
                            f,
                            "{}.{} = {}",
                            self.module.resolve_name(*base),
                            associated,
                            equals.display(self.module),
                        )?;
                    },
                }
            }
        }

        // Body
        if let Some(body) = &self.def.body {
            writeln!(f, "\n{{")?;

            // Non-parameter locals
            let non_param_locals: Vec<_> = body
                .locals
                .iter()
                .enumerate()
                .skip(body.param_count)
                .collect();

            if !non_param_locals.is_empty() {
                writeln!(f, "    locals:")?;
                for (_, local) in &non_param_locals {
                    writeln!(
                        f,
                        "        %{}: {}",
                        local.name,
                        local.ty.display(self.module),
                    )?;
                }
                writeln!(f)?;
            }

            // Blocks
            for (i, block) in body.blocks.iter().enumerate() {
                writeln!(f, "    bb{}:", i)?;
                write!(f, "{}", block.display(self.module, "        "))?;
                if i < body.blocks.len() - 1 {
                    writeln!(f)?;
                }
            }

            write!(f, "}}")?;
        }

        Ok(())
    }
}

// === StructDef ===

impl StructDef {
    pub fn display<'a>(&'a self, module: &'a MirModule) -> impl fmt::Display + 'a {
        StructDefDisplay { def: self, module }
    }
}

struct StructDefDisplay<'a> {
    def: &'a StructDef,
    module: &'a MirModule,
}

impl fmt::Display for StructDefDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "struct {}", self.def.name)?;
        write_type_params(f, &self.def.type_params)?;
        writeln!(f, " {{")?;
        for field in &self.def.fields {
            writeln!(f, "    {}: {}", field.name, field.ty.display(self.module))?;
        }
        write!(f, "}}")
    }
}

// === EnumDef ===

impl EnumDef {
    pub fn display<'a>(&'a self, module: &'a MirModule) -> impl fmt::Display + 'a {
        EnumDefDisplay { def: self, module }
    }
}

struct EnumDefDisplay<'a> {
    def: &'a EnumDef,
    module: &'a MirModule,
}

impl fmt::Display for EnumDefDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "enum {}", self.def.name)?;
        write_type_params(f, &self.def.type_params)?;
        writeln!(f, " {{")?;
        for case in &self.def.cases {
            let payload_struct = &self.module.structs[case.payload_struct.index()];
            writeln!(f, "    {}: {}", case.name, payload_struct.name)?;
        }
        write!(f, "}}")
    }
}

// === ProtocolDef ===

impl ProtocolDef {
    pub fn display<'a>(&'a self, module: &'a MirModule) -> impl fmt::Display + 'a {
        ProtocolDefDisplay { def: self, module }
    }
}

struct ProtocolDefDisplay<'a> {
    def: &'a ProtocolDef,
    module: &'a MirModule,
}

impl fmt::Display for ProtocolDefDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "protocol {}", self.def.name)?;
        write_type_params(f, &self.def.type_params)?;
        if !self.def.parent_protocols.is_empty() {
            write!(f, ": ")?;
            for (i, parent) in self.def.parent_protocols.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", self.module.resolve_name(*parent))?;
            }
        }
        writeln!(f, " {{")?;
        for assoc in &self.def.associated_types {
            writeln!(f, "    type {}", assoc.name)?;
        }
        for method in &self.def.methods {
            write!(f, "    func {}(", method.name)?;
            for (i, (pname, pty)) in method.params.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}: {}", pname, pty.display(self.module))?;
            }
            writeln!(f, ") -> {}", method.ret.display(self.module))?;
        }
        write!(f, "}}")
    }
}

// === WitnessDef ===

impl WitnessDef {
    pub fn display<'a>(&'a self, module: &'a MirModule) -> impl fmt::Display + 'a {
        WitnessDefDisplay { def: self, module }
    }
}

struct WitnessDefDisplay<'a> {
    def: &'a WitnessDef,
    module: &'a MirModule,
}

impl fmt::Display for WitnessDefDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "witness {}: {}",
            self.def.implementing_type.display(self.module),
            self.module.resolve_name(self.def.protocol),
        )?;
        if !self.def.protocol_type_args.is_empty() {
            write!(f, "[")?;
            for (i, (name, ty)) in self.def.protocol_type_args.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{} = {}", name, ty.display(self.module))?;
            }
            write!(f, "]")?;
        }
        writeln!(f, " {{")?;
        for (name, ty) in &self.def.type_bindings {
            writeln!(f, "    type {} = {}", name, ty.display(self.module))?;
        }
        for (name, binding) in &self.def.method_bindings {
            write!(
                f,
                "    func {} = {}",
                name,
                self.module.resolve_name(binding.implementation),
            )?;
            if !binding.type_args.is_empty() {
                write!(f, "[")?;
                write_comma_sep(f, &binding.type_args, self.module)?;
                write!(f, "]")?;
            }
            writeln!(f)?;
        }
        write!(f, "}}")
    }
}

// === StaticDef ===

impl StaticDef {
    pub fn display<'a>(&'a self, module: &'a MirModule) -> impl fmt::Display + 'a {
        StaticDefDisplay { def: self, module }
    }
}

struct StaticDefDisplay<'a> {
    def: &'a StaticDef,
    module: &'a MirModule,
}

impl fmt::Display for StaticDefDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "static ")?;
        if self.def.is_mutable {
            write!(f, "var ")?;
        }
        write!(
            f,
            "{}: {}",
            self.def.name,
            self.def.ty.display(self.module),
        )?;
        if let Some(init) = &self.def.initializer {
            write!(f, " = {}", init.display(self.module))?;
        }
        Ok(())
    }
}

// === MirModule ===

impl MirModule {
    pub fn display(&self) -> impl fmt::Display + '_ {
        MirModuleDisplay { module: self }
    }
}

struct MirModuleDisplay<'a> {
    module: &'a MirModule,
}

impl fmt::Display for MirModuleDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for def in &self.module.structs {
            writeln!(f, "{}\n", def.display(self.module))?;
        }
        for def in &self.module.enums {
            writeln!(f, "{}\n", def.display(self.module))?;
        }
        for def in &self.module.protocols {
            writeln!(f, "{}\n", def.display(self.module))?;
        }
        for def in &self.module.witnesses {
            writeln!(f, "{}\n", def.display(self.module))?;
        }
        for def in &self.module.statics {
            writeln!(f, "{}\n", def.display(self.module))?;
        }
        for def in &self.module.functions {
            writeln!(f, "{}\n", def.display(self.module))?;
        }
        Ok(())
    }
}

// === Op display ===

impl fmt::Display for crate::op::Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use crate::op::Op;
        match self {
            Op::Add(b, s) => write!(f, "{}.add.{}", b, s),
            Op::Sub(b, s) => write!(f, "{}.sub.{}", b, s),
            Op::Mul(b, s) => write!(f, "{}.mul.{}", b, s),
            Op::Div(b, s) => write!(f, "{}.div.{}", b, s),
            Op::Rem(b, s) => write!(f, "{}.rem.{}", b, s),
            Op::Neg(b) => write!(f, "{}.neg", b),
            Op::FAdd(b) => write!(f, "{}.add", b),
            Op::FSub(b) => write!(f, "{}.sub", b),
            Op::FMul(b) => write!(f, "{}.mul", b),
            Op::FDiv(b) => write!(f, "{}.div", b),
            Op::FNeg(b) => write!(f, "{}.neg", b),
            Op::And(b) => write!(f, "{}.and", b),
            Op::Or(b) => write!(f, "{}.or", b),
            Op::Xor(b) => write!(f, "{}.xor", b),
            Op::Shl(b) => write!(f, "{}.shl", b),
            Op::Shr(b, s) => write!(f, "{}.shr.{}", b, s),
            Op::Not(b) => write!(f, "{}.not", b),
            Op::Popcount(b) => write!(f, "{}.popcount", b),
            Op::Clz(b) => write!(f, "{}.clz", b),
            Op::Ctz(b) => write!(f, "{}.ctz", b),
            Op::Bswap(b) => write!(f, "{}.bswap", b),
            Op::Eq(b) => write!(f, "{}.eq", b),
            Op::Ne(b) => write!(f, "{}.ne", b),
            Op::Lt(b, s) => write!(f, "{}.lt.{}", b, s),
            Op::Le(b, s) => write!(f, "{}.le.{}", b, s),
            Op::Gt(b, s) => write!(f, "{}.gt.{}", b, s),
            Op::Ge(b, s) => write!(f, "{}.ge.{}", b, s),
            Op::FEq(b) => write!(f, "{}.eq", b),
            Op::FNe(b) => write!(f, "{}.ne", b),
            Op::FLt(b) => write!(f, "{}.lt", b),
            Op::FLe(b) => write!(f, "{}.le", b),
            Op::FGt(b) => write!(f, "{}.gt", b),
            Op::FGe(b) => write!(f, "{}.ge", b),
            Op::BoolAnd => write!(f, "bool.and"),
            Op::BoolOr => write!(f, "bool.or"),
            Op::BoolNot => write!(f, "bool.not"),
            Op::BoolEq => write!(f, "bool.eq"),
            Op::IntToFloat(from, to) => write!(f, "{}.to.{}", from, to),
            Op::FloatToInt(from, to) => write!(f, "{}.to.{}", from, to),
            Op::IntWiden(from, to) => write!(f, "{}.widen.{}", from, to),
            Op::IntTruncate(from, to) => write!(f, "{}.truncate.{}", from, to),
            Op::FloatWiden(from, to) => write!(f, "{}.widen.{}", from, to),
            Op::FloatTruncate(from, to) => write!(f, "{}.truncate.{}", from, to),
            Op::RefToImmut => write!(f, "ref.to.immut"),
            Op::PtrOffset => write!(f, "ptr.offset"),
            Op::PtrNull(_) => write!(f, "ptr.null"),
            Op::PtrFromAddress(_) => write!(f, "ptr.from_address"),
            Op::PtrToAddress => write!(f, "ptr.to_address"),
            Op::PtrRead(_) => write!(f, "ptr.read"),
            Op::PtrWrite => write!(f, "ptr.write"),
            Op::PtrIsNull => write!(f, "ptr.is_null"),
            Op::PtrCast(_) => write!(f, "ptr.cast"),
            Op::PtrBitcast(_) => write!(f, "ptr.bitcast"),
            Op::RefToPtr => write!(f, "ref.to.ptr"),
            Op::SizeOf(_) => write!(f, "sizeof"),
            Op::AlignOf(_) => write!(f, "alignof"),
            Op::StackAlloc(_) => write!(f, "stack_alloc"),
            Op::StrPtr => write!(f, "str.ptr"),
            Op::StrLen => write!(f, "str.len"),
            Op::StrEq => write!(f, "str.eq"),
            Op::IntToString => write!(f, "int.to_string"),
            Op::AtomicAdd => write!(f, "atomic.add"),
            Op::AtomicSub => write!(f, "atomic.sub"),
            Op::FloatConst(b, k) => {
                let k_str = match k {
                    crate::op::FloatConstantKind::Infinity => "infinity",
                    crate::op::FloatConstantKind::Nan => "nan",
                };
                write!(f, "{}.{}", b, k_str)
            },
            Op::FloatPred(b, p) => {
                let p_str = match p {
                    crate::op::FloatPredicateKind::IsNan => "is_nan",
                    crate::op::FloatPredicateKind::IsInfinite => "is_infinite",
                };
                write!(f, "{}.{}", b, p_str)
            },
            Op::FloatMath(b, op) => {
                let op_str = match op {
                    crate::op::FloatMathKind::Floor => "floor",
                    crate::op::FloatMathKind::Ceil => "ceil",
                    crate::op::FloatMathKind::Round => "round",
                    crate::op::FloatMathKind::Trunc => "trunc",
                    crate::op::FloatMathKind::Sqrt => "sqrt",
                };
                write!(f, "{}.{}", b, op_str)
            },
            Op::FloatFma(b) => write!(f, "{}.fma", b),
            Op::FloatCopysign(b) => write!(f, "{}.copysign", b),
        }
    }
}
