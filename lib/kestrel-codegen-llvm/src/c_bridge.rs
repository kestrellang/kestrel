//! Bridges between Kestrel's ABI and the C ABI (`kestrel_codegen::c_abi`).
//!
//! Every MIR function, `@extern` ones included, is called with Kestrel's ABI.
//! An `@extern` gets an internal Kestrel-ABI *import thunk* that makes the C
//! call; an `@export` gets an external C-ABI *export wrapper* that calls the
//! Kestrel body. Both are always-inline, so the bridge folds into the caller.
//! Keeping the C ABI out of ordinary call sites means a function value, a
//! thunk or a witness that names an extern needs no special case. A call
//! through a C function value (`Callee::C`) goes through a *call bridge*: an
//! import thunk per C function type whose first Kestrel param is the address.

use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::types::{AnyType, BasicMetadataTypeEnum, BasicType, BasicTypeEnum, FunctionType};
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, FunctionValue, PointerValue};
use kestrel_codegen::c_abi::{CArg, CDecl, CPrim, CRet, CSig, Piece};
use kestrel_mir::TyId;

use crate::context::CodegenCtx;
use crate::mem;
use crate::ty::ScalarTy;

/// A C signature lowered to LLVM: the function type and the parameter and
/// return attributes every declaration and call site must carry.
pub struct CFnType<'ctx> {
    pub sig: CSig,
    pub fn_type: FunctionType<'ctx>,
    pub attrs: Vec<(AttributeLoc, Attribute)>,
}

impl<'ctx> CFnType<'ctx> {
    pub fn apply_to_fn(&self, f: FunctionValue<'ctx>) {
        for (loc, attr) in &self.attrs {
            f.add_attribute(*loc, *attr);
        }
    }

    fn apply_to_call(&self, call: inkwell::values::CallSiteValue<'ctx>) {
        for (loc, attr) in &self.attrs {
            call.add_attribute(*loc, *attr);
        }
    }
}

fn prim_type(cx: &Context, prim: CPrim) -> BasicTypeEnum<'_> {
    match prim {
        CPrim::I8 => cx.i8_type().into(),
        CPrim::I16 => cx.i16_type().into(),
        CPrim::I32 => cx.i32_type().into(),
        CPrim::I64 => cx.i64_type().into(),
        CPrim::F32 => cx.f32_type().into(),
        CPrim::F64 => cx.f64_type().into(),
        CPrim::F32x2 => cx.f32_type().vec_type(2).into(),
    }
}

/// The LLVM type of a split argument: one piece, or an array of equal pieces
/// when the target allocates them together.
fn split_arg_types<'ctx>(
    cx: &'ctx Context,
    pieces: &[Piece],
    array: bool,
) -> Vec<BasicTypeEnum<'ctx>> {
    if array && pieces.len() > 1 {
        return vec![
            prim_type(cx, pieces[0].prim)
                .array_type(pieces.len() as u32)
                .into(),
        ];
    }
    pieces.iter().map(|p| prim_type(cx, p.prim)).collect()
}

/// The LLVM type of a split return: one piece, or a struct of the pieces.
fn split_ret_type<'ctx>(cx: &'ctx Context, pieces: &[Piece]) -> BasicTypeEnum<'ctx> {
    if pieces.len() == 1 {
        return prim_type(cx, pieces[0].prim);
    }
    let fields: Vec<BasicTypeEnum> = pieces.iter().map(|p| prim_type(cx, p.prim)).collect();
    cx.struct_type(&fields, false).into()
}

fn enum_attr(cx: &Context, name: &str, value: u64) -> Attribute {
    cx.create_enum_attribute(Attribute::get_named_enum_kind_id(name), value)
}

fn type_attr<'ctx>(cx: &'ctx Context, name: &str, ty: impl AnyType<'ctx>) -> Attribute {
    cx.create_type_attribute(
        Attribute::get_named_enum_kind_id(name),
        ty.as_any_type_enum(),
    )
}

/// `zeroext` / `signext` for an 8- or 16-bit integer: C wants the caller to
/// widen it to 32 bits.
fn ext_attr(cx: &Context, scalar: ScalarTy, unsigned: bool) -> Option<Attribute> {
    matches!(scalar, ScalarTy::I8 | ScalarTy::I16)
        .then(|| enum_attr(cx, if unsigned { "zeroext" } else { "signext" }, 0))
}

/// Classify `decl` and lower it to an LLVM function type.
pub fn c_fn_type<'ctx>(ctx: &mut CodegenCtx<'ctx>, decl: &CDecl) -> CFnType<'ctx> {
    let cx = ctx.cx;
    let module = ctx.module;
    let arena = &module.ty_arena;
    let sig = decl.classify(ctx.c_abi, &mut ctx.tc, arena, module);
    let ptr: BasicTypeEnum = cx.ptr_type(inkwell::AddressSpace::default()).into();

    let mut types: Vec<BasicMetadataTypeEnum> = Vec::new();
    let mut attrs = Vec::new();
    if sig.ret == CRet::Sret {
        let size = ctx.tc.repr(decl.ret, arena, module).size();
        attrs.push((
            AttributeLoc::Param(0),
            type_attr(cx, "sret", cx.i8_type().array_type(size as u32)),
        ));
        types.push(ptr.into());
    }
    for (i, (carg, &(param_ty, _))) in sig.params.iter().zip(&decl.params).enumerate() {
        let index = types.len() as u32;
        match carg {
            CArg::Ignore => {},
            CArg::Pointer | CArg::Indirect => types.push(ptr.into()),
            CArg::Direct => {
                let scalar = ctx.tc.value_scalar(param_ty, arena, module);
                if let Some(a) = ext_attr(cx, scalar, decl.unsigned_param(i)) {
                    attrs.push((AttributeLoc::Param(index), a));
                }
                types.push(scalar.llvm(cx).into());
            },
            CArg::Split { pieces, array } => {
                types.extend(
                    split_arg_types(cx, pieces, *array)
                        .into_iter()
                        .map(BasicMetadataTypeEnum::from),
                );
            },
            CArg::Stack { size, align } => {
                let loc = AttributeLoc::Param(index);
                attrs.push((
                    loc,
                    type_attr(cx, "byval", cx.i8_type().array_type(*size as u32)),
                ));
                attrs.push((loc, enum_attr(cx, "align", (*align).max(8))));
                types.push(ptr.into());
            },
        }
    }
    let fn_type = match &sig.ret {
        CRet::Void | CRet::Sret => cx.void_type().fn_type(&types, false),
        CRet::Direct => {
            let scalar = ctx.tc.value_scalar(decl.ret, arena, module);
            if let Some(a) = ext_attr(cx, scalar, decl.unsigned_ret) {
                attrs.push((AttributeLoc::Return, a));
            }
            scalar.llvm(cx).fn_type(&types, false)
        },
        CRet::Split { pieces } => split_ret_type(cx, pieces).fn_type(&types, false),
    };
    CFnType {
        sig,
        fn_type,
        attrs,
    }
}

/// A stack slot of `size` bytes in the function `builder` is in.
fn slot<'ctx>(cx: &'ctx Context, builder: &Builder<'ctx>, size: u64) -> PointerValue<'ctx> {
    let ty = cx.i8_type().array_type(size.max(1) as u32);
    let slot = builder.build_alloca(ty, "cslot").unwrap();
    if let Some(inst) = slot.as_instruction() {
        let _ = inst.set_alignment(16);
    }
    slot
}

fn piece_ptr<'ctx>(
    cx: &'ctx Context,
    builder: &Builder<'ctx>,
    base: PointerValue<'ctx>,
    piece: &Piece,
) -> PointerValue<'ctx> {
    mem::raw_gep(
        cx,
        builder,
        base,
        cx.i64_type().const_int(piece.offset, false),
    )
}

/// Copy the `size`-byte value at `src` into a padded slot and load its
/// pieces as C arguments.
fn load_split<'ctx>(
    cx: &'ctx Context,
    builder: &Builder<'ctx>,
    ptr_size: u64,
    src: PointerValue<'ctx>,
    size: u64,
    pieces: &[Piece],
    array: bool,
) -> Vec<BasicMetadataValueEnum<'ctx>> {
    let tmp = slot(cx, builder, CSig::split_slot_size(pieces).max(size));
    mem::copy_aggregate(cx, builder, ptr_size, size, tmp, src);
    let vals: Vec<BasicValueEnum> = pieces
        .iter()
        .map(|p| {
            mem::load(
                builder,
                prim_type(cx, p.prim),
                piece_ptr(cx, builder, tmp, p),
                "piece",
            )
            .unwrap()
        })
        .collect();
    if !(array && pieces.len() > 1) {
        return vals.into_iter().map(Into::into).collect();
    }
    let arr_ty = prim_type(cx, pieces[0].prim).array_type(pieces.len() as u32);
    let mut agg = arr_ty.get_undef();
    for (i, v) in vals.into_iter().enumerate() {
        agg = builder
            .build_insert_value(agg, v, i as u32, "arr")
            .unwrap()
            .into_array_value();
    }
    vec![agg.into()]
}

/// Store C pieces (`vals`, in piece order) into `dest`.
fn store_split<'ctx>(
    cx: &'ctx Context,
    builder: &Builder<'ctx>,
    dest: PointerValue<'ctx>,
    pieces: &[Piece],
    vals: &[BasicValueEnum<'ctx>],
) {
    for (p, v) in pieces.iter().zip(vals) {
        mem::store(builder, piece_ptr(cx, builder, dest, p), *v).unwrap();
    }
}

/// Unpack a split C value (a lone piece, an array, or a struct) into pieces.
fn unpack<'ctx>(
    builder: &Builder<'ctx>,
    v: BasicValueEnum<'ctx>,
    n: usize,
) -> Vec<BasicValueEnum<'ctx>> {
    if n == 1 {
        return vec![v];
    }
    (0..n)
        .map(|i| match v {
            BasicValueEnum::ArrayValue(a) => builder.build_extract_value(a, i as u32, "p").unwrap(),
            BasicValueEnum::StructValue(s) => {
                builder.build_extract_value(s, i as u32, "p").unwrap()
            },
            other => other,
        })
        .collect()
}

/// What an import thunk calls: a declared C symbol, or (a call bridge) the
/// address passed as its first Kestrel param.
#[derive(Clone, Copy)]
pub enum CTarget<'ctx> {
    Symbol(FunctionValue<'ctx>),
    FirstParam,
}

/// Define `thunk` (Kestrel ABI) as a call to the C function `target`.
pub fn define_import_thunk<'ctx>(
    ctx: &mut CodegenCtx<'ctx>,
    decl: &CDecl,
    thunk: FunctionValue<'ctx>,
    target: CTarget<'ctx>,
    cty: &CFnType<'ctx>,
) {
    let cx = ctx.cx;
    let module = ctx.module;
    let ptr_size = ctx.ptr_size;
    let builder = cx.create_builder();
    builder.position_at_end(cx.append_basic_block(thunk, "entry"));

    let ret_repr = ctx.tc.repr(decl.ret, &module.ty_arena, module);
    let mut kparams = thunk.get_param_iter();
    let ksret = ret_repr
        .is_aggregate()
        .then(|| kparams.next().unwrap().into_pointer_value());
    let fn_ptr =
        matches!(target, CTarget::FirstParam).then(|| kparams.next().unwrap().into_pointer_value());

    let mut cargs: Vec<BasicMetadataValueEnum> = Vec::new();
    if cty.sig.ret == CRet::Sret {
        cargs.push(ksret.unwrap().into());
    }
    for (carg, &(param_ty, convention)) in cty.sig.params.iter().zip(&decl.params) {
        let repr = ctx.tc.repr(param_ty, &module.ty_arena, module);
        if repr.is_zst() && convention == kestrel_mir::ParamConvention::Consuming {
            continue;
        }
        let kval = kparams.next().unwrap();
        match carg {
            CArg::Ignore => {},
            CArg::Direct | CArg::Pointer | CArg::Stack { .. } => cargs.push(kval.into()),
            CArg::Split { pieces, array } => cargs.extend(load_split(
                cx,
                &builder,
                ptr_size,
                kval.into_pointer_value(),
                repr.size(),
                pieces,
                *array,
            )),
            CArg::Indirect => {
                let copy = slot(cx, &builder, repr.size());
                mem::copy_aggregate(
                    cx,
                    &builder,
                    ptr_size,
                    repr.size(),
                    copy,
                    kval.into_pointer_value(),
                );
                cargs.push(copy.into());
            },
        }
    }

    let call = match target {
        CTarget::Symbol(c_fn) => builder.build_call(c_fn, &cargs, "ccall"),
        CTarget::FirstParam => {
            builder.build_indirect_call(cty.fn_type, fn_ptr.unwrap(), &cargs, "ccall")
        },
    }
    .unwrap();
    cty.apply_to_call(call);
    match &cty.sig.ret {
        CRet::Void | CRet::Sret => {
            builder.build_return(None).unwrap();
        },
        CRet::Direct => {
            let v = call.try_as_basic_value().basic().unwrap();
            builder.build_return(Some(&v)).unwrap();
        },
        CRet::Split { pieces } => {
            let v = call.try_as_basic_value().basic().unwrap();
            let tmp = slot(cx, &builder, CSig::split_slot_size(pieces));
            store_split(
                cx,
                &builder,
                tmp,
                pieces,
                &unpack(&builder, v, pieces.len()),
            );
            mem::copy_aggregate(cx, &builder, ptr_size, ret_repr.size(), ksret.unwrap(), tmp);
            builder.build_return(None).unwrap();
        },
    }
}

/// Define `wrapper` (C ABI) as a call to the Kestrel function `body`.
pub fn define_export_wrapper<'ctx>(
    ctx: &mut CodegenCtx<'ctx>,
    decl: &CDecl,
    wrapper: FunctionValue<'ctx>,
    body: FunctionValue<'ctx>,
    cty: &CFnType<'ctx>,
) {
    let cx = ctx.cx;
    let module = ctx.module;
    let builder = cx.create_builder();
    builder.position_at_end(cx.append_basic_block(wrapper, "entry"));

    let ret_repr = ctx.tc.repr(decl.ret, &module.ty_arena, module);
    let mut cparams = wrapper.get_param_iter();
    let csret = (cty.sig.ret == CRet::Sret).then(|| cparams.next().unwrap().into_pointer_value());

    let mut kargs: Vec<BasicMetadataValueEnum> = Vec::new();
    // Kestrel returns an aggregate through its own sret slot: C's when C
    // returns indirectly too, else a padded local the pieces load from.
    let ksret = ret_repr.is_aggregate().then(|| {
        csret.unwrap_or_else(|| {
            let size = match &cty.sig.ret {
                CRet::Split { pieces } => CSig::split_slot_size(pieces).max(ret_repr.size()),
                _ => ret_repr.size(),
            };
            slot(cx, &builder, size)
        })
    });
    if let Some(p) = ksret {
        kargs.push(p.into());
    }
    for (carg, &(param_ty, _)) in cty.sig.params.iter().zip(&decl.params) {
        match carg {
            CArg::Ignore => {},
            CArg::Direct | CArg::Pointer | CArg::Stack { .. } | CArg::Indirect => {
                kargs.push(cparams.next().unwrap().into());
            },
            CArg::Split { pieces, array } => {
                let size = ctx.tc.repr(param_ty, &module.ty_arena, module).size();
                let vals: Vec<BasicValueEnum> = if *array && pieces.len() > 1 {
                    unpack(&builder, cparams.next().unwrap(), pieces.len())
                } else {
                    pieces.iter().map(|_| cparams.next().unwrap()).collect()
                };
                let tmp = slot(cx, &builder, CSig::split_slot_size(pieces).max(size));
                store_split(cx, &builder, tmp, pieces, &vals);
                kargs.push(tmp.into());
            },
        }
    }

    let call = builder.build_call(body, &kargs, "kcall").unwrap();
    match &cty.sig.ret {
        CRet::Void | CRet::Sret => {
            builder.build_return(None).unwrap();
        },
        CRet::Direct => {
            let v = call.try_as_basic_value().basic().unwrap();
            builder.build_return(Some(&v)).unwrap();
        },
        CRet::Split { pieces } => {
            let src = ksret.unwrap();
            let vals: Vec<BasicValueEnum> = pieces
                .iter()
                .map(|p| {
                    mem::load(
                        &builder,
                        prim_type(cx, p.prim),
                        piece_ptr(cx, &builder, src, p),
                        "piece",
                    )
                    .unwrap()
                })
                .collect();
            let v = if vals.len() == 1 {
                vals[0]
            } else {
                let sty = split_ret_type(cx, pieces).into_struct_type();
                let mut agg = sty.get_undef();
                for (i, v) in vals.into_iter().enumerate() {
                    agg = builder
                        .build_insert_value(agg, v, i as u32, "ret")
                        .unwrap()
                        .into_struct_value();
                }
                agg.into()
            };
            builder.build_return(Some(&v)).unwrap();
        },
    }
}

/// The call bridge for C function type `func_ty`: an always-inline
/// Kestrel-ABI function `(address, params…) -> ret` that makes the C call.
/// One per type, made on first use.
pub fn call_bridge<'ctx>(ctx: &mut CodegenCtx<'ctx>, func_ty: TyId) -> FunctionValue<'ctx> {
    if let Some(&f) = ctx.c_call_bridges.get(&func_ty) {
        return f;
    }
    let module = ctx.module;
    let decl = CDecl::of_func_c(&module.ty_arena, func_ty).expect("call bridge for a non-C type");
    let mut kparams = vec![(func_ty, kestrel_mir::ParamConvention::Consuming)];
    kparams.extend(decl.params.iter().copied());
    let sig = crate::abi::build_signature_parts(
        &kparams,
        decl.ret,
        &mut ctx.tc,
        &module.ty_arena,
        module,
        ctx.cx,
    );
    let name = format!("c_call_bridge.{}", ctx.c_call_bridges.len());
    let bridge = ctx
        .llmod
        .add_function(&name, sig, Some(inkwell::module::Linkage::Internal));
    let cty = c_fn_type(ctx, &decl);
    define_import_thunk(ctx, &decl, bridge, CTarget::FirstParam, &cty);
    ctx.add_inline_attribute(bridge, kestrel_mir::item::function::InlineMode::Always);
    ctx.c_call_bridges.insert(func_ty, bridge);
    bridge
}
