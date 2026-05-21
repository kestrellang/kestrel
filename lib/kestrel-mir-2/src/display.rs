use std::fmt;

use crate::immediate::ImmediateKind;
use crate::operand::{ArgMode, UseMode};
use crate::statement::{Callee, Rvalue, StatementKind};
use crate::terminator::{SwitchCase, TerminatorKind};
use crate::ty::MirTy;
use crate::{MirModule, TyArena, TyId};

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
                write!(f, " {}: {}", field.name, TyDisplay(field.ty, &m.ty_arena))?;
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
                write!(f, "{}{}: {}", conv, param.name, TyDisplay(param.ty, &m.ty_arena))?;
            }
            write!(f, ") -> {}", TyDisplay(func.ret, &m.ty_arena))?;

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
                        TyDisplay(local.ty, &m.ty_arena),
                        marker
                    )?;
                }
                for (i, block) in body.blocks.iter().enumerate() {
                    writeln!(f, "  bb{i}:")?;
                    for stmt in &block.stmts {
                        write!(f, "    ")?;
                        write_statement(f, &stmt.kind, &m.ty_arena)?;
                        writeln!(f)?;
                    }
                    write!(f, "    ")?;
                    write_terminator(f, &block.terminator.kind, &m.ty_arena)?;
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

fn write_statement(f: &mut fmt::Formatter<'_>, kind: &StatementKind, arena: &TyArena) -> fmt::Result {
    match kind {
        StatementKind::Assign { dest, rvalue } => {
            write!(f, "{} = ", PlaceDisplay(dest))?;
            write_rvalue(f, rvalue, arena)
        }
        StatementKind::Call { dest, callee, args } => {
            if let Some(d) = dest {
                write!(f, "{} = ", PlaceDisplay(d))?;
            }
            write!(f, "call ")?;
            write_callee(f, callee, arena)?;
            write!(f, "(")?;
            for (i, (op, _)) in args.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", OperandDisplay(op, arena))?;
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
        StatementKind::Drop { place } => write!(f, "drop {}", PlaceDisplay(place)),
        StatementKind::DropIf { place, flag } => {
            write!(f, "drop {} if %{}", PlaceDisplay(place), flag.index())
        }
        StatementKind::SetDropFlag { flag, value } => {
            write!(f, "%{} = {value}", flag.index())
        }
        StatementKind::ScopeLive(local) => write!(f, "scope_live %{}", local.index()),
    }
}

fn write_rvalue(f: &mut fmt::Formatter<'_>, rv: &Rvalue, arena: &TyArena) -> fmt::Result {
    match rv {
        Rvalue::Use(op, UseMode::Copy) => write!(f, "use copy {}", OperandDisplay(op, arena)),
        Rvalue::Use(op, UseMode::Move) => write!(f, "use move {}", OperandDisplay(op, arena)),
        Rvalue::Ref(place) => write!(f, "ref {}", PlaceDisplay(place)),
        Rvalue::RefMut(place) => write!(f, "ref_mut {}", PlaceDisplay(place)),
        Rvalue::Op1 { op, arg } => write!(f, "{op:?} {}", OperandDisplay(arg, arena)),
        Rvalue::Op2 { op, lhs, rhs } => {
            write!(
                f,
                "{op:?} {}, {}",
                OperandDisplay(lhs, arena),
                OperandDisplay(rhs, arena)
            )
        }
        Rvalue::Op3 { op, a, b, c } => {
            write!(
                f,
                "{op:?} {}, {}, {}",
                OperandDisplay(a, arena),
                OperandDisplay(b, arena),
                OperandDisplay(c, arena)
            )
        }
        Rvalue::Construct { ty, fields } => {
            write!(f, "construct {} {{", TyDisplay(*ty, arena))?;
            for (i, (idx, op, mode)) in fields.iter().enumerate() {
                if i > 0 {
                    write!(f, ",")?;
                }
                let m = match mode {
                    UseMode::Copy => "copy",
                    UseMode::Move => "move",
                };
                write!(f, " .{}: {m} {}", idx.index(), OperandDisplay(op, arena))?;
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
                write!(f, "{m} {}", OperandDisplay(op, arena))?;
            }
            write!(f, ")")
        }
        Rvalue::EnumVariant {
            enum_ty,
            variant,
            payload,
        } => {
            write!(f, "enum {}:{} (", TyDisplay(*enum_ty, arena), variant.index())?;
            for (i, (op, mode)) in payload.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                let m = match mode {
                    UseMode::Copy => "copy",
                    UseMode::Move => "move",
                };
                write!(f, "{m} {}", OperandDisplay(op, arena))?;
            }
            write!(f, ")")
        }
        Rvalue::ArrayLiteral { element_ty, values } => {
            write!(f, "array[{}] [", TyDisplay(*element_ty, arena))?;
            for (i, (op, mode)) in values.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                let m = match mode {
                    UseMode::Copy => "copy",
                    UseMode::Move => "move",
                };
                write!(f, "{m} {}", OperandDisplay(op, arena))?;
            }
            write!(f, "]")
        }
        Rvalue::ApplyPartial { func, captures } => {
            write!(f, "apply_partial e{} (", func.index())?;
            for (i, (op, mode)) in captures.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                let m = match mode {
                    UseMode::Copy => "copy",
                    UseMode::Move => "move",
                };
                write!(f, "{m} {}", OperandDisplay(op, arena))?;
            }
            write!(f, ")")
        }
    }
}

fn write_callee(f: &mut fmt::Formatter<'_>, callee: &Callee, arena: &TyArena) -> fmt::Result {
    match callee {
        Callee::Direct {
            func,
            type_args,
            ..
        } => {
            write!(f, "e{}", func.index())?;
            if !type_args.is_empty() {
                write!(f, "[")?;
                for (i, arg) in type_args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", TyDisplay(*arg, arena))?;
                }
                write!(f, "]")?;
            }
            Ok(())
        }
        Callee::Thin(place) => write!(f, "thin {}", PlaceDisplay(place)),
        Callee::Thick(place) => write!(f, "thick {}", PlaceDisplay(place)),
        Callee::Witness {
            protocol, method, ..
        } => write!(f, "witness e{}.{}", protocol.index(), method.name),
    }
}

fn write_terminator(
    f: &mut fmt::Formatter<'_>,
    kind: &TerminatorKind,
    arena: &TyArena,
) -> fmt::Result {
    match kind {
        TerminatorKind::Return(op) => write!(f, "return {}", OperandDisplay(op, arena)),
        TerminatorKind::Jump(target) => write!(f, "jump bb{}", target.index()),
        TerminatorKind::Branch {
            condition,
            then_block,
            else_block,
        } => write!(
            f,
            "branch {} -> bb{}, bb{}",
            OperandDisplay(condition, arena),
            then_block.index(),
            else_block.index()
        ),
        TerminatorKind::Switch {
            discriminant,
            cases,
        } => {
            write!(f, "switch {} -> [", PlaceDisplay(discriminant))?;
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

struct PlaceDisplay<'a>(&'a crate::Place);

impl fmt::Display for PlaceDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0.base {
            crate::PlaceBase::Local(id) => write!(f, "%{}", id.index())?,
            crate::PlaceBase::Global(entity) => write!(f, "@e{}", entity.index())?,
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

struct OperandDisplay<'a>(&'a crate::Operand, &'a TyArena);

impl fmt::Display for OperandDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            crate::Operand::Place(place) => write!(f, "{}", PlaceDisplay(place)),
            crate::Operand::Const(imm) => match &imm.kind {
                ImmediateKind::IntLiteral { bits, value } => write!(f, "{value}_{bits:?}"),
                ImmediateKind::FloatLiteral { bits, value } => write!(f, "{value}_{bits:?}"),
                ImmediateKind::BoolLiteral(b) => write!(f, "{b}"),
                ImmediateKind::StringLiteral(s) => write!(f, "\"{s}\""),
                ImmediateKind::StringPointer(s) => write!(f, "strptr(\"{s}\")"),
                ImmediateKind::Unit => write!(f, "()"),
                ImmediateKind::FunctionRef { func, .. } => write!(f, "e{}", func.index()),
                ImmediateKind::MonoFunctionRef(id) => write!(f, "mono_fn{}", id.index()),
                ImmediateKind::NullPtr(ty) => write!(f, "null({})", TyDisplay(*ty, self.1)),
                ImmediateKind::SizeOf(ty) => write!(f, "sizeof({})", TyDisplay(*ty, self.1)),
                ImmediateKind::AlignOf(ty) => write!(f, "alignof({})", TyDisplay(*ty, self.1)),
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

struct TyDisplay<'a>(TyId, &'a TyArena);

impl fmt::Display for TyDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.1.get(self.0) {
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
            MirTy::Pointer(inner) => write!(f, "p[{}]", TyDisplay(*inner, self.1)),
            MirTy::Tuple(elems) if elems.is_empty() => write!(f, "()"),
            MirTy::Tuple(elems) => {
                write!(f, "(")?;
                for (i, elem) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", TyDisplay(*elem, self.1))?;
                }
                write!(f, ")")
            }
            MirTy::Named { entity, type_args } => {
                write!(f, "e{}", entity.index())?;
                if !type_args.is_empty() {
                    write!(f, "[")?;
                    for (i, arg) in type_args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", TyDisplay(*arg, self.1))?;
                    }
                    write!(f, "]")?;
                }
                Ok(())
            }
            MirTy::TypeParam(e) => write!(f, "T{}", e.index()),
            MirTy::SelfType => write!(f, "Self"),
            MirTy::AssociatedProjection {
                base,
                protocol,
                assoc_type,
            } => write!(
                f,
                "({}.e{} for e{})",
                TyDisplay(*base, self.1),
                assoc_type.index(),
                protocol.index()
            ),
            MirTy::FuncThin { params, ret } => {
                write!(f, "func(")?;
                for (i, (ty, conv)) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write_param_conv(f, *conv)?;
                    write!(f, "{}", TyDisplay(*ty, self.1))?;
                }
                write!(f, ") -> {}", TyDisplay(*ret, self.1))
            }
            MirTy::FuncThick { params, ret } => {
                write!(f, "func escaping(")?;
                for (i, (ty, conv)) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write_param_conv(f, *conv)?;
                    write!(f, "{}", TyDisplay(*ty, self.1))?;
                }
                write!(f, ") -> {}", TyDisplay(*ret, self.1))
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
        let mut arena = crate::TyArena::new();
        let i64_ty = arena.i64();
        let d = TyDisplay(i64_ty, &arena);
        assert_eq!(d.to_string(), "i64");
    }

    #[test]
    fn display_type_pointer() {
        let mut arena = crate::TyArena::new();
        let i64_ty = arena.i64();
        let ptr = arena.pointer(i64_ty);
        let d = TyDisplay(ptr, &arena);
        assert_eq!(d.to_string(), "p[i64]");
    }

    #[test]
    fn display_type_unit() {
        let mut arena = crate::TyArena::new();
        let unit = arena.unit();
        let d = TyDisplay(unit, &arena);
        assert_eq!(d.to_string(), "()");
    }

    #[test]
    fn display_type_tuple() {
        let mut arena = crate::TyArena::new();
        let i64_ty = arena.i64();
        let bool_ty = arena.bool();
        let tup = arena.tuple(vec![i64_ty, bool_ty]);
        let d = TyDisplay(tup, &arena);
        assert_eq!(d.to_string(), "(i64, bool)");
    }
}
