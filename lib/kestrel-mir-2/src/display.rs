use std::fmt;

use crate::body::MirBody;
use crate::immediate::ImmediateKind;
use crate::operand::{ArgMode, UseMode};
use crate::statement::{Callee, Rvalue, StatementKind};
use crate::terminator::{SwitchCase, TerminatorKind};
use crate::ty::MirTy;
use crate::{LocalId, MirModule, TyId};

pub struct ModuleDisplay<'a> {
    module: &'a MirModule,
}

impl MirModule {
    pub fn display(&self) -> ModuleDisplay<'_> {
        ModuleDisplay { module: self }
    }
}

impl fmt::Display for ModuleDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let m = self.module;
        writeln!(f, "module \"{}\"", m.name)?;
        writeln!(f)?;

        for def in &m.structs {
            write!(f, "struct {}", def.name)?;
            if !def.type_params.is_empty() {
                write!(f, "[")?;
                for (i, tp) in def.type_params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", tp.name)?;
                }
                write!(f, "]")?;
            }
            write!(f, " {{")?;
            for (i, field) in def.fields.iter().enumerate() {
                if i > 0 {
                    write!(f, ",")?;
                }
                write!(f, " {}: {}", field.name, TyDisplay(field.ty, m))?;
            }
            writeln!(f, " }}")?;
        }

        for def in &m.enums {
            write!(f, "enum {}", def.name)?;
            if !def.type_params.is_empty() {
                write!(f, "[")?;
                for (i, tp) in def.type_params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", tp.name)?;
                }
                write!(f, "]")?;
            }
            write!(f, " {{")?;
            for (i, case) in def.cases.iter().enumerate() {
                if i > 0 {
                    write!(f, ",")?;
                }
                write!(f, " {}({})", case.name, case.discriminant)?;
            }
            writeln!(f, " }}")?;
        }

        if !m.structs.is_empty() || !m.enums.is_empty() {
            writeln!(f)?;
        }

        for func in &m.functions {
            write!(f, "fn {}", func.name)?;
            if !func.type_params.is_empty() {
                write!(f, "[")?;
                for (i, tp) in func.type_params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", tp.name)?;
                }
                write!(f, "]")?;
            }
            write!(f, "(")?;
            for (i, param) in func.params.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                let conv = match param.convention {
                    crate::ParamConvention::Borrow => "&",
                    crate::ParamConvention::MutBorrow => "&var ",
                    crate::ParamConvention::Consuming => "",
                };
                write!(f, "{}{}: {}", conv, param.name, TyDisplay(param.ty, m))?;
            }
            write!(f, ") -> {}", TyDisplay(func.ret, m))?;

            if let Some(body) = &func.body {
                writeln!(f, " {{")?;
                writeln!(f, "  locals:")?;
                for (i, local) in body.locals.iter().enumerate() {
                    let marker = if i < body.param_count {
                        format!(" — param {i}")
                    } else {
                        String::new()
                    };
                    writeln!(
                        f,
                        "    %{}: {}{}",
                        local.name,
                        TyDisplay(local.ty, m),
                        marker
                    )?;
                }
                for (i, block) in body.blocks.iter().enumerate() {
                    writeln!(f, "  bb{i}:")?;
                    for stmt in &block.stmts {
                        write!(f, "    ")?;
                        write_statement(f, &stmt.kind, body, m)?;
                        writeln!(f)?;
                    }
                    write!(f, "    ")?;
                    write_terminator(f, &block.terminator.kind, body, m)?;
                    writeln!(f)?;
                }
                writeln!(f, "}}")?;
            } else if let Some(ext) = &func.extern_info {
                writeln!(f, " [extern, symbol=\"{}\"]", ext.symbol_name)?;
            } else {
                writeln!(f)?;
            }
        }

        Ok(())
    }
}

fn local_name(body: &MirBody, id: LocalId) -> &str {
    body.locals
        .get(id.index())
        .map(|l| l.name.as_str())
        .unwrap_or("?")
}

fn write_statement(
    f: &mut fmt::Formatter<'_>,
    kind: &StatementKind,
    body: &MirBody,
    module: &MirModule,
) -> fmt::Result {
    match kind {
        StatementKind::Assign { dest, rvalue } => {
            write!(f, "{} = ", PlaceDisplay(dest, body, module))?;
            write_rvalue(f, rvalue, body, module)
        }
        StatementKind::Call { dest, callee, args } => {
            if let Some(d) = dest {
                write!(f, "{} = ", PlaceDisplay(d, body, module))?;
            }
            write!(f, "call ")?;
            write_callee(f, callee, body, module)?;
            write!(f, "(")?;
            for (i, (op, _)) in args.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", OperandDisplay(op, body, module))?;
            }
            write!(f, ") [")?;
            for (i, (_, mode)) in args.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                let s = match mode {
                    ArgMode::Copy => "copy",
                    ArgMode::Move => "move",
                    ArgMode::Ref => "ref",
                    ArgMode::RefMut => "ref_mut",
                };
                write!(f, "{s}")?;
            }
            write!(f, "]")
        }
        StatementKind::Uninit { dest } => {
            write!(f, "{} = uninit", PlaceDisplay(dest, body, module))
        }
        StatementKind::Drop { place } => write!(f, "drop {}", PlaceDisplay(place, body, module)),
        StatementKind::DropIf { place, flag } => {
            write!(
                f,
                "drop {} if %{}",
                PlaceDisplay(place, body, module),
                local_name(body, *flag)
            )
        }
        StatementKind::SetDropFlag { flag, value } => {
            write!(f, "%{} = {value}", local_name(body, *flag))
        }
        StatementKind::ScopeLive(local) => write!(f, "scope_live %{}", local_name(body, *local)),
    }
}

fn write_rvalue(
    f: &mut fmt::Formatter<'_>,
    rv: &Rvalue,
    body: &MirBody,
    module: &MirModule,
) -> fmt::Result {
    match rv {
        Rvalue::Use(op, UseMode::Copy) => {
            write!(f, "use copy {}", OperandDisplay(op, body, module))
        }
        Rvalue::Use(op, UseMode::Move) => {
            write!(f, "use move {}", OperandDisplay(op, body, module))
        }
        Rvalue::Ref(place) => write!(f, "ref {}", PlaceDisplay(place, body, module)),
        Rvalue::RefMut(place) => write!(f, "ref_mut {}", PlaceDisplay(place, body, module)),
        Rvalue::Op1 { op, arg } => write!(f, "{op:?} {}", OperandDisplay(arg, body, module)),
        Rvalue::Op2 { op, lhs, rhs } => {
            write!(
                f,
                "{op:?} {}, {}",
                OperandDisplay(lhs, body, module),
                OperandDisplay(rhs, body, module)
            )
        }
        Rvalue::Op3 { op, a, b, c } => {
            write!(
                f,
                "{op:?} {}, {}, {}",
                OperandDisplay(a, body, module),
                OperandDisplay(b, body, module),
                OperandDisplay(c, body, module)
            )
        }
        Rvalue::Construct { ty, fields } => {
            write!(f, "construct {} {{", TyDisplay(*ty, module))?;
            for (i, (idx, op, mode)) in fields.iter().enumerate() {
                if i > 0 {
                    write!(f, ",")?;
                }
                let m = match mode {
                    UseMode::Copy => "copy",
                    UseMode::Move => "move",
                };
                write!(
                    f,
                    " .{}: {m} {}",
                    idx.index(),
                    OperandDisplay(op, body, module)
                )?;
            }
            write!(f, " }}")
        }
        Rvalue::Tuple(elems) => {
            write!(f, "tuple (")?;
            for (i, (op, mode)) in elems.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                let m = match mode {
                    UseMode::Copy => "copy",
                    UseMode::Move => "move",
                };
                write!(f, "{m} {}", OperandDisplay(op, body, module))?;
            }
            write!(f, ")")
        }
        Rvalue::EnumVariant {
            enum_ty,
            variant,
            payload,
        } => {
            write!(
                f,
                "enum {}:{} (",
                TyDisplay(*enum_ty, module),
                variant.index()
            )?;
            for (i, (op, mode)) in payload.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                let m = match mode {
                    UseMode::Copy => "copy",
                    UseMode::Move => "move",
                };
                write!(f, "{m} {}", OperandDisplay(op, body, module))?;
            }
            write!(f, ")")
        }
        Rvalue::ArrayLiteral { element_ty, values } => {
            write!(f, "array[{}] [", TyDisplay(*element_ty, module))?;
            for (i, (op, mode)) in values.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                let m = match mode {
                    UseMode::Copy => "copy",
                    UseMode::Move => "move",
                };
                write!(f, "{m} {}", OperandDisplay(op, body, module))?;
            }
            write!(f, "]")
        }
        Rvalue::ApplyPartial { func, captures } => {
            write!(f, "apply_partial {} (", module.resolve_name(*func))?;
            for (i, (op, mode)) in captures.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                let m = match mode {
                    UseMode::Copy => "copy",
                    UseMode::Move => "move",
                };
                write!(f, "{m} {}", OperandDisplay(op, body, module))?;
            }
            write!(f, ")")
        }
    }
}

fn write_callee(
    f: &mut fmt::Formatter<'_>,
    callee: &Callee,
    body: &MirBody,
    module: &MirModule,
) -> fmt::Result {
    match callee {
        Callee::Direct {
            func, type_args, ..
        } => {
            write!(f, "{}", module.resolve_name(*func))?;
            if !type_args.is_empty() {
                write!(f, "[")?;
                for (i, arg) in type_args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", TyDisplay(*arg, module))?;
                }
                write!(f, "]")?;
            }
            Ok(())
        }
        Callee::Resolved(id) => write!(f, "mono_fn{}", id.index()),
        Callee::Thin(place) => {
            write!(f, "thin {}", PlaceDisplay(place, body, module))
        }
        Callee::Thick(place) => {
            write!(f, "thick {}", PlaceDisplay(place, body, module))
        }
        Callee::Witness {
            protocol, method, self_type, ..
        } => write!(
            f,
            "witness {}.{}<self={}>",
            module.resolve_name(*protocol),
            method.name,
            TyDisplay(*self_type, module),
        ),
    }
}

fn write_terminator(
    f: &mut fmt::Formatter<'_>,
    kind: &TerminatorKind,
    body: &MirBody,
    module: &MirModule,
) -> fmt::Result {
    match kind {
        TerminatorKind::Return(op) => {
            write!(f, "return {}", OperandDisplay(op, body, module))
        }
        TerminatorKind::Jump(target) => write!(f, "jump bb{}", target.index()),
        TerminatorKind::Branch {
            condition,
            then_block,
            else_block,
        } => write!(
            f,
            "branch {} -> bb{}, bb{}",
            OperandDisplay(condition, body, module),
            then_block.index(),
            else_block.index()
        ),
        TerminatorKind::Switch {
            discriminant,
            cases,
        } => {
            write!(f, "switch {} -> [", PlaceDisplay(discriminant, body, module))?;
            for (i, (case, block)) in cases.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                match case {
                    SwitchCase::Wildcard => write!(f, "_: bb{}", block.index())?,
                    SwitchCase::Variant(v) => {
                        write!(f, "{}: bb{}", v.index(), block.index())?;
                    }
                    SwitchCase::Bool(b) => write!(f, "{b}: bb{}", block.index())?,
                    SwitchCase::IntLiteral(v) => write!(f, "{v}: bb{}", block.index())?,
                    SwitchCase::IntRange { start, end } => {
                        write!(f, "{start}..={end}: bb{}", block.index())?;
                    }
                    SwitchCase::CharLiteral(c) => write!(f, "'{c}': bb{}", block.index())?,
                    SwitchCase::CharRange { start, end } => {
                        write!(f, "'{start}'..='{end}': bb{}", block.index())?;
                    }
                }
            }
            write!(f, "]")
        }
        TerminatorKind::Panic(msg) => write!(f, "panic \"{msg}\""),
        TerminatorKind::Unreachable => write!(f, "unreachable"),
    }
}

struct PlaceDisplay<'a>(&'a crate::Place, &'a MirBody, &'a MirModule);

impl fmt::Display for PlaceDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0.base {
            crate::PlaceBase::Local(id) => write!(f, "%{}", local_name(self.1, *id))?,
            crate::PlaceBase::Global(entity) => {
                write!(f, "@{}", self.2.resolve_name(*entity))?;
            }
        }
        for elem in &self.0.projections {
            match elem {
                crate::PlaceElem::Field(idx) => write!(f, ".{}", idx.index())?,
                crate::PlaceElem::TupleIndex(i) => write!(f, ".{i}")?,
                crate::PlaceElem::Downcast(v) => write!(f, ":{}", v.index())?,
                crate::PlaceElem::Deref => write!(f, ".*")?,
            }
        }
        Ok(())
    }
}

struct OperandDisplay<'a>(&'a crate::Operand, &'a MirBody, &'a MirModule);

impl fmt::Display for OperandDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            crate::Operand::Place(place) => {
                write!(f, "{}", PlaceDisplay(place, self.1, self.2))
            }
            crate::Operand::Const(imm) => match &imm.kind {
                ImmediateKind::IntLiteral { bits, value } => write!(f, "{value}_{bits:?}"),
                ImmediateKind::FloatLiteral { bits, value } => write!(f, "{value}_{bits:?}"),
                ImmediateKind::BoolLiteral(b) => write!(f, "{b}"),
                ImmediateKind::StringLiteral(s) => write!(f, "\"{s}\""),
                ImmediateKind::StringPointer(s) => write!(f, "strptr(\"{s}\")"),
                ImmediateKind::Unit => write!(f, "()"),
                ImmediateKind::FunctionRef { func, .. } => {
                    write!(f, "{}", self.2.resolve_name(*func))
                }
                ImmediateKind::MonoFunctionRef(id) => write!(f, "mono_fn{}", id.index()),
                ImmediateKind::NullPtr(ty) => write!(f, "null({})", TyDisplay(*ty, self.2)),
                ImmediateKind::SizeOf(ty) => write!(f, "sizeof({})", TyDisplay(*ty, self.2)),
                ImmediateKind::AlignOf(ty) => write!(f, "alignof({})", TyDisplay(*ty, self.2)),
                ImmediateKind::FloatInfinity(bits) => write!(f, "inf_{bits:?}"),
                ImmediateKind::FloatNan(bits) => write!(f, "nan_{bits:?}"),
                ImmediateKind::Error => write!(f, "<error>"),
            },
        }
    }
}

fn write_param_conv(f: &mut fmt::Formatter<'_>, conv: crate::ParamConvention) -> fmt::Result {
    match conv {
        crate::ParamConvention::Borrow => write!(f, "&"),
        crate::ParamConvention::MutBorrow => write!(f, "&var "),
        crate::ParamConvention::Consuming => Ok(()),
    }
}

struct TyDisplay<'a>(TyId, &'a MirModule);

impl fmt::Display for TyDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let module = self.1;
        let arena = &module.ty_arena;
        match arena.get(self.0) {
            MirTy::I8 => write!(f, "i8"),
            MirTy::I16 => write!(f, "i16"),
            MirTy::I32 => write!(f, "i32"),
            MirTy::I64 => write!(f, "i64"),
            MirTy::F16 => write!(f, "f16"),
            MirTy::F32 => write!(f, "f32"),
            MirTy::F64 => write!(f, "f64"),
            MirTy::Bool => write!(f, "bool"),
            MirTy::Never => write!(f, "!"),
            MirTy::Str => write!(f, "str"),
            MirTy::Error => write!(f, "<error>"),
            MirTy::Pointer(inner) => write!(f, "p[{}]", TyDisplay(*inner, module)),
            MirTy::Tuple(elems) if elems.is_empty() => write!(f, "()"),
            MirTy::Tuple(elems) => {
                write!(f, "(")?;
                for (i, elem) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", TyDisplay(*elem, module))?;
                }
                write!(f, ")")
            }
            MirTy::Named { entity, type_args } => {
                write!(f, "{}", module.resolve_name(*entity))?;
                if !type_args.is_empty() {
                    write!(f, "[")?;
                    for (i, arg) in type_args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", TyDisplay(*arg, module))?;
                    }
                    write!(f, "]")?;
                }
                Ok(())
            }
            MirTy::TypeParam(e) => {
                let name = module.resolve_name(*e);
                if name == "<unknown>" {
                    write!(f, "T{}", e.index())
                } else {
                    write!(f, "{name}")
                }
            }
            MirTy::SelfType => write!(f, "Self"),
            MirTy::AssociatedProjection {
                base,
                protocol,
                assoc_type,
            } => {
                let proto_name = module.resolve_name(*protocol);
                let assoc_name = module.resolve_name(*assoc_type);
                write!(
                    f,
                    "({}.{} for {})",
                    TyDisplay(*base, module),
                    assoc_name,
                    proto_name
                )
            }
            MirTy::FuncThin { params, ret } => {
                write!(f, "func(")?;
                for (i, (ty, conv)) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write_param_conv(f, *conv)?;
                    write!(f, "{}", TyDisplay(*ty, module))?;
                }
                write!(f, ") -> {}", TyDisplay(*ret, module))
            }
            MirTy::FuncThick { params, ret } => {
                write!(f, "func escaping(")?;
                for (i, (ty, conv)) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write_param_conv(f, *conv)?;
                    write!(f, "{}", TyDisplay(*ty, module))?;
                }
                write!(f, ") -> {}", TyDisplay(*ret, module))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::struct_def::{FieldDef, StructDef};
    use kestrel_hecs::Entity;

    #[test]
    fn display_module_header() {
        let module = MirModule::new("test");
        let output = module.display().to_string();
        assert!(output.contains("module \"test\""));
    }

    #[test]
    fn display_struct() {
        let mut module = MirModule::new("test");
        let i64_ty = module.ty_arena.i64();
        let mut def = StructDef::new(Entity::from_raw(1), "Point");
        def.add_field(FieldDef::new("x", i64_ty));
        def.add_field(FieldDef::new("y", i64_ty));
        module.add_struct(def);

        let output = module.display().to_string();
        assert!(output.contains("struct Point"));
        assert!(output.contains("x: i64"));
        assert!(output.contains("y: i64"));
    }

    #[test]
    fn display_type_primitives() {
        let mut module = MirModule::new("test");
        let i64_ty = module.ty_arena.i64();
        let d = TyDisplay(i64_ty, &module);
        assert_eq!(d.to_string(), "i64");
    }

    #[test]
    fn display_type_pointer() {
        let mut module = MirModule::new("test");
        let i64_ty = module.ty_arena.i64();
        let ptr = module.ty_arena.pointer(i64_ty);
        let d = TyDisplay(ptr, &module);
        assert_eq!(d.to_string(), "p[i64]");
    }

    #[test]
    fn display_type_unit() {
        let mut module = MirModule::new("test");
        let unit = module.ty_arena.unit();
        let d = TyDisplay(unit, &module);
        assert_eq!(d.to_string(), "()");
    }

    #[test]
    fn display_type_tuple() {
        let mut module = MirModule::new("test");
        let i64_ty = module.ty_arena.i64();
        let bool_ty = module.ty_arena.bool();
        let tup = module.ty_arena.tuple(vec![i64_ty, bool_ty]);
        let d = TyDisplay(tup, &module);
        assert_eq!(d.to_string(), "(i64, bool)");
    }

    #[test]
    fn display_named_type_with_registered_name() {
        let mut module = MirModule::new("test");
        let entity = Entity::from_raw(42);
        module.register_name(entity, "std.numeric.Int64");
        let ty = module.ty_arena.intern(MirTy::Named {
            entity,
            type_args: vec![],
        });
        let d = TyDisplay(ty, &module);
        assert_eq!(d.to_string(), "std.numeric.Int64");
    }

    #[test]
    fn display_named_type_with_type_args() {
        let mut module = MirModule::new("test");
        let entity = Entity::from_raw(42);
        module.register_name(entity, "std.Array");
        let i64_ty = module.ty_arena.i64();
        let ty = module.ty_arena.intern(MirTy::Named {
            entity,
            type_args: vec![i64_ty],
        });
        let d = TyDisplay(ty, &module);
        assert_eq!(d.to_string(), "std.Array[i64]");
    }

    #[test]
    fn display_type_param_with_name() {
        let mut module = MirModule::new("test");
        let entity = Entity::from_raw(10);
        module.register_name(entity, "T");
        let ty = module.ty_arena.intern(MirTy::TypeParam(entity));
        let d = TyDisplay(ty, &module);
        assert_eq!(d.to_string(), "T");
    }

    #[test]
    fn display_type_param_without_name() {
        let mut module = MirModule::new("test");
        let entity = Entity::from_raw(999);
        let ty = module.ty_arena.intern(MirTy::TypeParam(entity));
        let d = TyDisplay(ty, &module);
        assert_eq!(d.to_string(), "T999");
    }
}
