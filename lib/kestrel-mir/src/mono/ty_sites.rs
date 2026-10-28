//! The one list of every type a monomorphized function mentions.
//!
//! Two passes must agree on that list: layout seeding
//! (`resolve_types_and_layouts`, which builds a `MonoStruct`/`MonoEnum` for
//! each concrete `Named`) and `verify_mono` (which reports any `Named` that
//! did not get one). They used to be two hand-written walks, and every place
//! they disagreed was a type the verifier checked but seeding never saw —
//! extern signatures (no body to walk), `FuncThin`/`FuncThick` params, and
//! address-instruction types. Both now iterate [`for_each_fn_ty`] and
//! [`for_each_ty_node`], so a new type site added here is seeded AND verified.

use std::fmt;

use kestrel_span::Span;

use crate::body::OssaBody;
use crate::immediate::ImmediateKind;
use crate::inst::InstKind;
use crate::mono::types::MonoParam;
use crate::ty::{MirTy, TyArena};
use crate::{BlockId, TyId};

/// Where a type was mentioned. `Display` renders the verifier's context label.
#[derive(Debug, Clone, Copy)]
pub(crate) enum TySiteKind {
    Param(usize),
    Return,
    Value(usize),
    BlockParam { block: usize, param: usize },
    StructTy,
    EnumTy,
    ArrayElem,
    AddrTy,
    FieldAddrTy,
    UninitTy,
    OpTy,
    ImmediateTy,
}

impl fmt::Display for TySiteKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Param(i) => write!(f, "param {i}"),
            Self::Return => f.write_str("return type"),
            Self::Value(i) => write!(f, "value {i}"),
            Self::BlockParam { block, param } => write!(f, "block {block} param {param}"),
            Self::StructTy => f.write_str("Struct type"),
            Self::EnumTy => f.write_str("Enum type"),
            Self::ArrayElem => f.write_str("Array element type"),
            Self::AddrTy => f.write_str("address type"),
            Self::FieldAddrTy => f.write_str("FieldAddr type"),
            Self::UninitTy => f.write_str("Uninit type"),
            Self::OpTy => f.write_str("Op type"),
            Self::ImmediateTy => f.write_str("immediate type"),
        }
    }
}

/// A type mention plus the location the verifier reports it at.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TySite<'a> {
    pub kind: TySiteKind,
    pub block: Option<BlockId>,
    pub inst: Option<usize>,
    pub span: Option<&'a Span>,
}

/// Calls `f` for every top-level type a function mentions: its signature
/// (always — an extern has nothing else), then, when it has a body, every
/// value, block param, and type-carrying instruction operand.
pub(crate) fn for_each_fn_ty<'a>(
    params: &[MonoParam],
    ret: TyId,
    body: Option<&'a OssaBody>,
    mut f: impl FnMut(TyId, TySite<'a>),
) {
    let sig = |kind| TySite {
        kind,
        block: None,
        inst: None,
        span: None,
    };
    for (pi, param) in params.iter().enumerate() {
        f(param.ty, sig(TySiteKind::Param(pi)));
    }
    f(ret, sig(TySiteKind::Return));

    let Some(body) = body else { return };

    // Values inherit their defining span (interned types can't carry one).
    for (vi, value) in body.values.iter().enumerate() {
        f(
            value.ty,
            TySite {
                kind: TySiteKind::Value(vi),
                block: None,
                inst: None,
                span: value.span.as_ref(),
            },
        );
    }

    for (bi, block) in body.blocks.iter().enumerate() {
        let block_id = BlockId::new(bi);
        for (pi, param) in block.params.iter().enumerate() {
            let span = body
                .values
                .get(param.value.index())
                .and_then(|vd| vd.span.as_ref());
            f(
                param.ty,
                TySite {
                    kind: TySiteKind::BlockParam {
                        block: bi,
                        param: pi,
                    },
                    block: Some(block_id),
                    inst: None,
                    span,
                },
            );
        }
        for (ii, inst) in block.insts.iter().enumerate() {
            let Some((ty, kind)) = inst_ty(&inst.kind) else {
                continue;
            };
            f(
                ty,
                TySite {
                    kind,
                    block: Some(block_id),
                    inst: Some(ii),
                    span: inst.span.as_ref(),
                },
            );
        }
    }
}

/// The type an instruction carries as an operand (not its result — results
/// are values, walked separately), if any.
fn inst_ty(kind: &InstKind) -> Option<(TyId, TySiteKind)> {
    match kind {
        InstKind::Struct { ty, .. } => Some((*ty, TySiteKind::StructTy)),
        InstKind::Enum { enum_ty, .. } => Some((*enum_ty, TySiteKind::EnumTy)),
        InstKind::Array { element_ty, .. } => Some((*element_ty, TySiteKind::ArrayElem)),
        InstKind::CopyAddr { ty, .. }
        | InstKind::Take { ty, .. }
        | InstKind::BeginBorrowAddr { ty, .. }
        | InstKind::BeginMutBorrowAddr { ty, .. }
        | InstKind::DestroyAddr { ty, .. } => Some((*ty, TySiteKind::AddrTy)),
        InstKind::FieldAddr { ty, .. } => Some((*ty, TySiteKind::FieldAddrTy)),
        InstKind::Uninit { ty, .. } => Some((*ty, TySiteKind::UninitTy)),
        InstKind::Literal { value, .. } => match &value.kind {
            ImmediateKind::SizeOf(ty) | ImmediateKind::AlignOf(ty) | ImmediateKind::NullPtr(ty) => {
                Some((*ty, TySiteKind::ImmediateTy))
            },
            _ => None,
        },
        // Mirrors `substitute_op_type` via the one canonical variant list in
        // `op::op_ty_variants!`.
        InstKind::Op1 { op, .. } | InstKind::Op2 { op, .. } | InstKind::Op3 { op, .. } => {
            crate::op::op_type(op).map(|ty| (ty, TySiteKind::OpTy))
        },
        _ => None,
    }
}

/// Visits `ty` and every type nested inside it, children before parent.
/// `AssociatedProjection` is a leaf: post-mono it is itself the error.
pub(crate) fn for_each_ty_node(arena: &TyArena, ty: TyId, f: &mut impl FnMut(TyId, &MirTy)) {
    let node = arena.get(ty);
    match node {
        MirTy::Pointer(inner) => for_each_ty_node(arena, *inner, f),
        MirTy::Ref { pointee, .. } => for_each_ty_node(arena, *pointee, f),
        MirTy::Tuple(elems) => {
            for &elem in elems {
                for_each_ty_node(arena, elem, f);
            }
        },
        MirTy::Named { type_args, .. } => {
            for &arg in type_args {
                for_each_ty_node(arena, arg, f);
            }
        },
        MirTy::FuncThin { params, ret } | MirTy::FuncThick { params, ret, .. } => {
            for &(p, _) in params {
                for_each_ty_node(arena, p, f);
            }
            for_each_ty_node(arena, *ret, f);
        },
        _ => {},
    }
    f(ty, node);
}
