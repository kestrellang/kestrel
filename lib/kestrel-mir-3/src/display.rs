// Pretty-printer for OSSA IR bodies.
//
// Produces textual output like:
//   bb0(%v0: @owned String, %v1: @none Int64):
//       %v2 = copy_value %v0
//       ...
//       return %v2

use std::fmt::Write;

use crate::body::OssaBody;
use crate::callee::Callee;
use crate::immediate::ImmediateKind;
use crate::inst::InstKind;
use crate::terminator::{SwitchCase, TerminatorKind};
use crate::ty::{MirTy, TyArena};
use crate::value::Ownership;
use crate::{BlockId, MirModule, TyId, ValueId};

/// Pretty-print an entire OSSA body to a string.
pub fn display_body(body: &OssaBody, module: &MirModule) -> String {
    let mut out = String::new();
    let arena = &module.ty_arena;

    for (i, block) in body.blocks.iter().enumerate() {
        let bid = BlockId::new(i);

        // Block header: bb0(%v0: @owned Type, ...):
        write!(out, "bb{}", bid.index()).unwrap();
        if !block.params.is_empty() {
            out.push('(');
            for (j, param) in block.params.iter().enumerate() {
                if j > 0 {
                    out.push_str(", ");
                }
                write!(
                    out,
                    "{}: {} {}",
                    fmt_value(param.value),
                    fmt_ownership(param.ownership),
                    fmt_ty(param.ty, arena, module),
                ).unwrap();
            }
            out.push(')');
        }
        out.push_str(":\n");

        // Instructions
        for inst in &block.insts {
            out.push_str("    ");
            fmt_inst(&mut out, &inst.kind, body, arena, module);
            out.push('\n');
        }

        // Terminator
        out.push_str("    ");
        fmt_terminator(&mut out, &block.terminator.kind, arena, module);
        out.push('\n');

        // Blank line between blocks (except after the last)
        if i + 1 < body.blocks.len() {
            out.push('\n');
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn fmt_value(v: ValueId) -> String {
    format!("%v{}", v.index())
}

fn fmt_block(b: BlockId) -> String {
    format!("bb{}", b.index())
}

fn fmt_ownership(o: Ownership) -> &'static str {
    match o {
        Ownership::Owned => "@owned",
        Ownership::Guaranteed => "@guaranteed",
        Ownership::None => "@none",
    }
}

fn fmt_ty(ty: TyId, arena: &TyArena, module: &MirModule) -> String {
    match arena.get(ty) {
        MirTy::I8 => "Int8".into(),
        MirTy::I16 => "Int16".into(),
        MirTy::I32 => "Int32".into(),
        MirTy::I64 => "Int64".into(),
        MirTy::F16 => "Float16".into(),
        MirTy::F32 => "Float32".into(),
        MirTy::F64 => "Float64".into(),
        MirTy::Bool => "Bool".into(),
        MirTy::Never => "Never".into(),
        MirTy::Str => "Str".into(),

        MirTy::Pointer(inner) => {
            format!("Pointer[{}]", fmt_ty(*inner, arena, module))
        }

        MirTy::Tuple(elems) => {
            if elems.is_empty() {
                "()".into()
            } else {
                let inner: Vec<_> = elems.iter().map(|t| fmt_ty(*t, arena, module)).collect();
                format!("({})", inner.join(", "))
            }
        }

        MirTy::Named { entity, type_args } => {
            let name = module.resolve_name(*entity);
            if type_args.is_empty() {
                name.to_string()
            } else {
                let args: Vec<_> = type_args.iter().map(|t| fmt_ty(*t, arena, module)).collect();
                format!("{}[{}]", name, args.join(", "))
            }
        }

        MirTy::TypeParam(entity) => module.resolve_name(*entity).to_string(),

        MirTy::AssociatedProjection { base, protocol, assoc_type } => {
            format!(
                "{}.{}::{}",
                fmt_ty(*base, arena, module),
                module.resolve_name(*protocol),
                module.resolve_name(*assoc_type),
            )
        }

        MirTy::FuncThin { params, ret } => {
            let ps: Vec<_> = params.iter().map(|(t, _)| fmt_ty(*t, arena, module)).collect();
            format!("@thin ({}) -> {}", ps.join(", "), fmt_ty(*ret, arena, module))
        }

        MirTy::FuncThick { params, ret } => {
            let ps: Vec<_> = params.iter().map(|(t, _)| fmt_ty(*t, arena, module)).collect();
            format!("@thick ({}) -> {}", ps.join(", "), fmt_ty(*ret, arena, module))
        }

        MirTy::Error => "<error>".into(),
    }
}

/// Format a type annotation comment for value-producing instructions.
fn fmt_type_comment(value: ValueId, body: &OssaBody, arena: &TyArena, module: &MirModule) -> String {
    let def = body.value(value);
    format!("  // {} {}", fmt_ownership(def.ownership), fmt_ty(def.ty, arena, module))
}

// ---------------------------------------------------------------------------
// Block argument list (for jumps/branches)
// ---------------------------------------------------------------------------

fn fmt_block_with_args(target: BlockId, args: &[ValueId]) -> String {
    if args.is_empty() {
        fmt_block(target)
    } else {
        let vals: Vec<_> = args.iter().map(|v| fmt_value(*v)).collect();
        format!("{}({})", fmt_block(target), vals.join(", "))
    }
}

// ---------------------------------------------------------------------------
// Instructions
// ---------------------------------------------------------------------------

fn fmt_inst(out: &mut String, kind: &InstKind, body: &OssaBody, arena: &TyArena, module: &MirModule) {
    match kind {
        // -- Value lifecycle --
        InstKind::CopyValue { result, operand } => {
            write!(out, "{} = copy_value {}", fmt_value(*result), fmt_value(*operand)).unwrap();
            out.push_str(&fmt_type_comment(*result, body, arena, module));
        }
        InstKind::MoveValue { result, operand } => {
            write!(out, "{} = move_value {}", fmt_value(*result), fmt_value(*operand)).unwrap();
            out.push_str(&fmt_type_comment(*result, body, arena, module));
        }
        InstKind::DestroyValue { operand } => {
            write!(out, "destroy_value {}", fmt_value(*operand)).unwrap();
        }

        // -- Borrowing --
        InstKind::BeginBorrow { result, operand } => {
            write!(out, "{} = begin_borrow {}", fmt_value(*result), fmt_value(*operand)).unwrap();
            out.push_str(&fmt_type_comment(*result, body, arena, module));
        }
        InstKind::EndBorrow { operand } => {
            write!(out, "end_borrow {}", fmt_value(*operand)).unwrap();
        }
        InstKind::BeginMutBorrow { result, operand } => {
            write!(out, "{} = begin_mut_borrow {}", fmt_value(*result), fmt_value(*operand)).unwrap();
            out.push_str(&fmt_type_comment(*result, body, arena, module));
        }
        InstKind::EndMutBorrow { operand } => {
            write!(out, "end_mut_borrow {}", fmt_value(*operand)).unwrap();
        }

        // -- Memory access --
        InstKind::Load { result, address } => {
            write!(out, "{} = load {}", fmt_value(*result), fmt_value(*address)).unwrap();
            out.push_str(&fmt_type_comment(*result, body, arena, module));
        }
        InstKind::CopyAddr { result, address, ty } => {
            write!(
                out, "{} = copy_addr {}, {}",
                fmt_value(*result), fmt_value(*address), fmt_ty(*ty, arena, module),
            ).unwrap();
            out.push_str(&fmt_type_comment(*result, body, arena, module));
        }
        InstKind::Take { result, address, ty } => {
            write!(
                out, "{} = take {}, {}",
                fmt_value(*result), fmt_value(*address), fmt_ty(*ty, arena, module),
            ).unwrap();
            out.push_str(&fmt_type_comment(*result, body, arena, module));
        }
        InstKind::BeginBorrowAddr { result, address, ty } => {
            write!(
                out, "{} = begin_borrow_addr {}, {}",
                fmt_value(*result), fmt_value(*address), fmt_ty(*ty, arena, module),
            ).unwrap();
            out.push_str(&fmt_type_comment(*result, body, arena, module));
        }
        InstKind::BeginMutBorrowAddr { result, address, ty } => {
            write!(
                out, "{} = begin_mut_borrow_addr {}, {}",
                fmt_value(*result), fmt_value(*address), fmt_ty(*ty, arena, module),
            ).unwrap();
            out.push_str(&fmt_type_comment(*result, body, arena, module));
        }
        InstKind::StoreInit { address, value } => {
            write!(out, "store_init {}, {}", fmt_value(*address), fmt_value(*value)).unwrap();
        }
        InstKind::StoreAssign { address, value } => {
            write!(out, "store_assign {}, {}", fmt_value(*address), fmt_value(*value)).unwrap();
        }
        InstKind::DestroyAddr { address, ty } => {
            write!(out, "destroy_addr {}, {}", fmt_value(*address), fmt_ty(*ty, arena, module)).unwrap();
        }

        // -- Discriminant --
        InstKind::Discriminant { result, operand } => {
            write!(out, "{} = discriminant {}", fmt_value(*result), fmt_value(*operand)).unwrap();
            out.push_str(&fmt_type_comment(*result, body, arena, module));
        }

        // -- Computation --
        InstKind::Op1 { result, op, arg } => {
            write!(out, "{} = op1 {:?} {}", fmt_value(*result), op, fmt_value(*arg)).unwrap();
            out.push_str(&fmt_type_comment(*result, body, arena, module));
        }
        InstKind::Op2 { result, op, lhs, rhs } => {
            write!(
                out, "{} = op2 {:?} {}, {}",
                fmt_value(*result), op, fmt_value(*lhs), fmt_value(*rhs),
            ).unwrap();
            out.push_str(&fmt_type_comment(*result, body, arena, module));
        }
        InstKind::Op3 { result, op, a, b, c } => {
            write!(
                out, "{} = op3 {:?} {}, {}, {}",
                fmt_value(*result), op, fmt_value(*a), fmt_value(*b), fmt_value(*c),
            ).unwrap();
            out.push_str(&fmt_type_comment(*result, body, arena, module));
        }

        // -- Constants --
        InstKind::Literal { result, value } => {
            write!(out, "{} = literal {}", fmt_value(*result), fmt_immediate(value, arena, module)).unwrap();
            out.push_str(&fmt_type_comment(*result, body, arena, module));
        }
        InstKind::GlobalRef { result, entity } => {
            write!(out, "{} = global_ref {}", fmt_value(*result), module.resolve_name(*entity)).unwrap();
            out.push_str(&fmt_type_comment(*result, body, arena, module));
        }

        // -- Aggregates: construction --
        InstKind::Struct { result, ty, fields } => {
            let fs: Vec<_> = fields.iter()
                .map(|(idx, v)| format!(".{}: {}", idx.index(), fmt_value(*v)))
                .collect();
            write!(
                out, "{} = struct {} {{ {} }}",
                fmt_value(*result), fmt_ty(*ty, arena, module), fs.join(", "),
            ).unwrap();
            out.push_str(&fmt_type_comment(*result, body, arena, module));
        }
        InstKind::Tuple { result, elements } => {
            let elems: Vec<_> = elements.iter().map(|v| fmt_value(*v)).collect();
            write!(out, "{} = tuple ({})", fmt_value(*result), elems.join(", ")).unwrap();
            out.push_str(&fmt_type_comment(*result, body, arena, module));
        }
        InstKind::Enum { result, enum_ty, variant, payload } => {
            let ty_str = fmt_ty(*enum_ty, arena, module);
            if payload.is_empty() {
                write!(out, "{} = enum {}.{}", fmt_value(*result), ty_str, variant.index()).unwrap();
            } else {
                let vals: Vec<_> = payload.iter().map(|v| fmt_value(*v)).collect();
                write!(
                    out, "{} = enum {}.{}({})",
                    fmt_value(*result), ty_str, variant.index(), vals.join(", "),
                ).unwrap();
            }
            out.push_str(&fmt_type_comment(*result, body, arena, module));
        }
        InstKind::Array { result, element_ty, elements } => {
            let elems: Vec<_> = elements.iter().map(|v| fmt_value(*v)).collect();
            write!(
                out, "{} = array {}[{}]",
                fmt_value(*result), fmt_ty(*element_ty, arena, module), elems.join(", "),
            ).unwrap();
            out.push_str(&fmt_type_comment(*result, body, arena, module));
        }

        // -- Aggregates: destructuring --
        InstKind::StructExtract { result, operand, field } => {
            write!(
                out, "{} = struct_extract {}, .{}",
                fmt_value(*result), fmt_value(*operand), field.index(),
            ).unwrap();
            out.push_str(&fmt_type_comment(*result, body, arena, module));
        }
        InstKind::TupleExtract { result, operand, index } => {
            write!(
                out, "{} = tuple_extract {}, .{}",
                fmt_value(*result), fmt_value(*operand), index,
            ).unwrap();
            out.push_str(&fmt_type_comment(*result, body, arena, module));
        }
        InstKind::EnumPayload { result, operand, variant, field } => {
            write!(
                out, "{} = enum_payload {}, variant {}, .{}",
                fmt_value(*result), fmt_value(*operand), variant.index(), field.index(),
            ).unwrap();
            out.push_str(&fmt_type_comment(*result, body, arena, module));
        }
        InstKind::DestructureStruct { results, operand } => {
            let rs: Vec<_> = results.iter().map(|r| fmt_value(*r)).collect();
            write!(out, "({}) = destructure_struct {}", rs.join(", "), fmt_value(*operand)).unwrap();
        }
        InstKind::DestructureTuple { results, operand } => {
            let rs: Vec<_> = results.iter().map(|r| fmt_value(*r)).collect();
            write!(out, "({}) = destructure_tuple {}", rs.join(", "), fmt_value(*operand)).unwrap();
        }
        InstKind::DestructureEnum { results, operand, variant } => {
            let rs: Vec<_> = results.iter().map(|r| fmt_value(*r)).collect();
            write!(
                out, "({}) = destructure_enum {}, variant {}",
                rs.join(", "), fmt_value(*operand), variant.index(),
            ).unwrap();
        }

        // -- Calls --
        InstKind::Call { result, callee, args } => {
            let callee_str = fmt_callee(callee, arena, module);
            let arg_strs: Vec<_> = args.iter().map(|a| fmt_value(a.value)).collect();
            match result {
                Some(r) => {
                    write!(
                        out, "{} = call {}({})",
                        fmt_value(*r), callee_str, arg_strs.join(", "),
                    ).unwrap();
                    out.push_str(&fmt_type_comment(*r, body, arena, module));
                }
                None => {
                    write!(out, "call {}({})", callee_str, arg_strs.join(", ")).unwrap();
                }
            }
        }
        InstKind::ApplyPartial { result, func, captures } => {
            let vals: Vec<_> = captures.iter().map(|v| fmt_value(*v)).collect();
            write!(
                out, "{} = apply_partial {}({})",
                fmt_value(*result), module.resolve_name(*func), vals.join(", "),
            ).unwrap();
            out.push_str(&fmt_type_comment(*result, body, arena, module));
        }

        // -- Address projection --
        InstKind::FieldAddr { result, base, ty, field } => {
            write!(
                out, "{} = field_addr {}, {}, .{}",
                fmt_value(*result), fmt_value(*base), fmt_ty(*ty, arena, module), field.index(),
            ).unwrap();
            out.push_str(&fmt_type_comment(*result, body, arena, module));
        }

        // -- Special --
        InstKind::Uninit { result, ty } => {
            write!(out, "{} = uninit {}", fmt_value(*result), fmt_ty(*ty, arena, module)).unwrap();
            out.push_str(&fmt_type_comment(*result, body, arena, module));
        }
    }
}

// ---------------------------------------------------------------------------
// Callee
// ---------------------------------------------------------------------------

fn fmt_callee(callee: &Callee, arena: &TyArena, module: &MirModule) -> String {
    match callee {
        Callee::Direct { func, type_args, self_type } => {
            let mut s = module.resolve_name(*func).to_string();
            if !type_args.is_empty() {
                let args: Vec<_> = type_args.iter().map(|t| fmt_ty(*t, arena, module)).collect();
                write!(s, "[{}]", args.join(", ")).unwrap();
            }
            if let Some(st) = self_type {
                write!(s, " for {}", fmt_ty(*st, arena, module)).unwrap();
            }
            s
        }
        Callee::Resolved(mono_id) => {
            format!("@mono({})", mono_id.index())
        }
        Callee::Thin(v) => {
            format!("@thin {}", fmt_value(*v))
        }
        Callee::Thick(v) => {
            format!("@thick {}", fmt_value(*v))
        }
        Callee::Witness { protocol, method, self_type, method_type_args } => {
            let mut s = format!(
                "@witness {}.{}",
                module.resolve_name(*protocol),
                method.name,
            );
            if !method.labels.is_empty() {
                let labels: Vec<_> = method.labels.iter()
                    .map(|l| l.as_deref().unwrap_or("_"))
                    .collect();
                write!(s, "({}:)", labels.join(":")).unwrap();
            }
            write!(s, " for {}", fmt_ty(*self_type, arena, module)).unwrap();
            if !method_type_args.is_empty() {
                let args: Vec<_> = method_type_args.iter().map(|t| fmt_ty(*t, arena, module)).collect();
                write!(s, "[{}]", args.join(", ")).unwrap();
            }
            s
        }
    }
}

// ---------------------------------------------------------------------------
// Immediate
// ---------------------------------------------------------------------------

fn fmt_immediate(imm: &crate::immediate::Immediate, arena: &TyArena, module: &MirModule) -> String {
    match &imm.kind {
        ImmediateKind::IntLiteral { bits, value } => {
            format!("{} as {:?}", value, bits)
        }
        ImmediateKind::FloatLiteral { bits, value } => {
            format!("{} as {:?}", value, bits)
        }
        ImmediateKind::BoolLiteral(b) => {
            format!("{}", b)
        }
        ImmediateKind::StringLiteral(s) => {
            format!("\"{}\"", s.escape_default())
        }
        ImmediateKind::StringPointer(s) => {
            format!("string_ptr \"{}\"", s.escape_default())
        }
        ImmediateKind::Unit => "()".into(),
        ImmediateKind::FunctionRef { func, type_args, self_type } => {
            let mut s = format!("func_ref {}", module.resolve_name(*func));
            if !type_args.is_empty() {
                let args: Vec<_> = type_args.iter().map(|t| fmt_ty(*t, arena, module)).collect();
                write!(s, "[{}]", args.join(", ")).unwrap();
            }
            if let Some(st) = self_type {
                write!(s, " for {}", fmt_ty(*st, arena, module)).unwrap();
            }
            s
        }
        ImmediateKind::MonoFunctionRef(id) => {
            format!("mono_func_ref @mono({})", id.index())
        }
        ImmediateKind::NullPtr(ty) => {
            format!("null_ptr {}", fmt_ty(*ty, arena, module))
        }
        ImmediateKind::SizeOf(ty) => {
            format!("size_of {}", fmt_ty(*ty, arena, module))
        }
        ImmediateKind::AlignOf(ty) => {
            format!("align_of {}", fmt_ty(*ty, arena, module))
        }
        ImmediateKind::FloatInfinity(bits) => {
            format!("infinity {:?}", bits)
        }
        ImmediateKind::FloatNan(bits) => {
            format!("nan {:?}", bits)
        }
        ImmediateKind::Error => "<error>".into(),
    }
}

// ---------------------------------------------------------------------------
// Terminators
// ---------------------------------------------------------------------------

fn fmt_terminator(out: &mut String, kind: &TerminatorKind, _arena: &TyArena, _module: &MirModule) {
    match kind {
        TerminatorKind::Return(v) => {
            write!(out, "return {}", fmt_value(*v)).unwrap();
        }
        TerminatorKind::Jump { target, args } => {
            write!(out, "jump {}", fmt_block_with_args(*target, args)).unwrap();
        }
        TerminatorKind::Branch { condition, then_block, then_args, else_block, else_args } => {
            write!(
                out, "branch {}, {}, {}",
                fmt_value(*condition),
                fmt_block_with_args(*then_block, then_args),
                fmt_block_with_args(*else_block, else_args),
            ).unwrap();
        }
        TerminatorKind::Switch { discriminant, cases } => {
            write!(out, "switch {} {{", fmt_value(*discriminant)).unwrap();
            for arm in cases {
                write!(out, "\n        {} => {}", fmt_switch_case(&arm.pattern), fmt_block_with_args(arm.target, &arm.args)).unwrap();
            }
            out.push_str("\n    }");
        }
        TerminatorKind::Panic(msg) => {
            write!(out, "panic \"{}\"", msg.escape_default()).unwrap();
        }
        TerminatorKind::Unreachable => {
            out.push_str("unreachable");
        }
    }
}

fn fmt_switch_case(case: &SwitchCase) -> String {
    match case {
        SwitchCase::Wildcard => "_".into(),
        SwitchCase::Variant(idx) => format!("variant {}", idx.index()),
        SwitchCase::Bool(b) => format!("{}", b),
        SwitchCase::IntLiteral(v) => format!("{}", v),
        SwitchCase::IntRange { start, end } => format!("{}..={}", start, end),
        SwitchCase::CharLiteral(c) => {
            if let Some(ch) = char::from_u32(*c) {
                format!("'{}'", ch.escape_default())
            } else {
                format!("\\u{{{:X}}}", c)
            }
        }
        SwitchCase::CharRange { start, end } => {
            let s = char::from_u32(*start).map_or_else(
                || format!("\\u{{{:X}}}", start),
                |ch| format!("'{}'", ch.escape_default()),
            );
            let e = char::from_u32(*end).map_or_else(
                || format!("\\u{{{:X}}}", end),
                |ch| format!("'{}'", ch.escape_default()),
            );
            format!("{}..={}", s, e)
        }
    }
}
