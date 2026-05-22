use indexmap::IndexMap;
use kestrel_hecs::Entity;

use crate::item::function::ReceiverConvention;
use crate::mono::types::MonoParam;
use crate::ty::{MirTy, ParamConvention, TyArena};
use crate::TyId;

/// Mangle a monomorphized function name using the v0 scheme.
///
/// Grammar:
///   mangled = "_K0" path receiver? signature? return? instantiation? self-disambig?
pub fn mangle_function(
    arena: &TyArena,
    entity_names: &IndexMap<Entity, String>,
    func_name: &str,
    type_args: &[TyId],
    self_type: Option<TyId>,
    params: &[MonoParam],
    ret: TyId,
    receiver: Option<ReceiverConvention>,
) -> String {
    let mut out = String::with_capacity(64);
    out.push_str("_K0");

    mangle_path(func_name, &mut out);

    if let Some(recv) = receiver {
        match recv {
            ReceiverConvention::Borrow => out.push('r'),
            ReceiverConvention::MutBorrow => out.push('m'),
            ReceiverConvention::Consuming => out.push('c'),
        }
    }

    if !params.is_empty() {
        mangle_signature(arena, entity_names, params, &mut out);
    }

    // Return type: "R" type — disambiguates overloads that differ only in return type
    out.push('R');
    mangle_type(arena, entity_names, ret, &mut out);

    if !type_args.is_empty() {
        out.push('I');
        for &ty in type_args {
            mangle_type(arena, entity_names, ty, &mut out);
        }
        out.push('E');
    }

    if let Some(st) = self_type {
        out.push_str("S_");
        mangle_type(arena, entity_names, st, &mut out);
    }

    out
}

// -- Signature: "Z" param* "E" --

fn mangle_signature(
    arena: &TyArena,
    entity_names: &IndexMap<Entity, String>,
    params: &[MonoParam],
    out: &mut String,
) {
    out.push('Z');
    for param in params {
        mangle_param(arena, entity_names, param, out);
    }
    out.push('E');
}

// -- Param: ("L" ident)? type --

fn mangle_param(
    arena: &TyArena,
    entity_names: &IndexMap<Entity, String>,
    param: &MonoParam,
    out: &mut String,
) {
    // Label prefix: "L" ident for labeled params
    if let Some(label) = &param.label {
        out.push('L');
        mangle_ident(label, out);
    }
    // Convention prefix for non-consuming params
    match param.convention {
        ParamConvention::Borrow => out.push('r'),
        ParamConvention::MutBorrow => out.push('m'),
        ParamConvention::Consuming => {}
    }
    mangle_type(arena, entity_names, param.ty, out);
}

// -- Path: ident | "N" ident+ "E" --

fn mangle_path(name: &str, out: &mut String) {
    let parts: Vec<&str> = name.split('.').collect();
    if parts.len() == 1 {
        mangle_ident(parts[0], out);
    } else {
        out.push('N');
        for part in &parts {
            mangle_ident(part, out);
        }
        out.push('E');
    }
}

// -- Ident: length "_" utf8-bytes --

fn mangle_ident(name: &str, out: &mut String) {
    out.push_str(&name.len().to_string());
    out.push('_');
    out.push_str(name);
}

// -- Type encoding --

fn mangle_type(
    arena: &TyArena,
    entity_names: &IndexMap<Entity, String>,
    ty: TyId,
    out: &mut String,
) {
    match arena.get(ty) {
        MirTy::I8 => out.push_str("i1"),
        MirTy::I16 => out.push_str("i2"),
        MirTy::I32 => out.push_str("i4"),
        MirTy::I64 => out.push_str("i8"),
        MirTy::F16 => out.push_str("f2"),
        MirTy::F32 => out.push_str("f4"),
        MirTy::F64 => out.push_str("f8"),
        MirTy::Bool => out.push('b'),
        MirTy::Str => out.push('s'),
        MirTy::Never => out.push('n'),
        MirTy::Error => out.push('X'),

        MirTy::Tuple(elems) => {
            let elems = elems.clone();
            out.push('T');
            if elems.is_empty() {
                // Unit = empty tuple
                out.push('v');
            }
            for elem in &elems {
                mangle_type(arena, entity_names, *elem, out);
            }
            out.push('E');
        }

        MirTy::Pointer(inner) => {
            let inner = *inner;
            out.push('P');
            mangle_type(arena, entity_names, inner, out);
        }

        MirTy::Named { entity, type_args } => {
            let entity = *entity;
            let type_args = type_args.clone();
            let name = entity_names
                .get(&entity)
                .map(|s| s.as_str())
                .unwrap_or("<unknown>");
            mangle_path(name, out);
            if !type_args.is_empty() {
                out.push('I');
                for &arg in &type_args {
                    mangle_type(arena, entity_names, arg, out);
                }
                out.push('E');
            }
        }

        MirTy::FuncThin { params, ret } => {
            let params = params.clone();
            let ret = *ret;
            out.push('F');
            out.push_str(&params.len().to_string());
            out.push('_');
            for (p, _conv) in &params {
                mangle_type(arena, entity_names, *p, out);
            }
            mangle_type(arena, entity_names, ret, out);
            out.push('E');
        }

        MirTy::FuncThick { params, ret } => {
            let params = params.clone();
            let ret = *ret;
            out.push('C');
            out.push_str(&params.len().to_string());
            out.push('_');
            for (p, _conv) in &params {
                mangle_type(arena, entity_names, *p, out);
            }
            mangle_type(arena, entity_names, ret, out);
            out.push('E');
        }

        MirTy::TypeParam(e) => {
            let name = entity_names.get(e).map(|s| s.as_str()).unwrap_or("?");
            panic!(
                "mangle_type: TypeParam({:?}, name={}) reached the mangler — monomorphization bug",
                e, name
            );
        }
        MirTy::AssociatedProjection { .. } => {
            panic!(
                "mangle_type: abstract type {:?} reached the mangler — monomorphization bug",
                arena.get(ty)
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_arena() -> TyArena {
        TyArena::new()
    }

    fn names() -> IndexMap<Entity, String> {
        IndexMap::new()
    }

    fn names_with(entries: &[(u32, &str)]) -> IndexMap<Entity, String> {
        let mut m = IndexMap::new();
        for &(id, name) in entries {
            m.insert(Entity::from_raw(id), name.to_string());
        }
        m
    }

    // -- Path mangling --

    #[test]
    fn mangle_simple_path() {
        let mut out = String::new();
        mangle_path("main", &mut out);
        assert_eq!(out, "4_main");
    }

    #[test]
    fn mangle_nested_path() {
        let mut out = String::new();
        mangle_path("std.Array.count", &mut out);
        assert_eq!(out, "N3_std5_Array5_countE");
    }

    #[test]
    fn mangle_two_part_path() {
        let mut out = String::new();
        mangle_path("std.main", &mut out);
        assert_eq!(out, "N3_std4_mainE");
    }

    // -- Type mangling --

    #[test]
    fn mangle_primitives() {
        let mut a = make_arena();
        let n = names();
        let cases = [
            (a.i8(), "i1"),
            (a.i16(), "i2"),
            (a.i32(), "i4"),
            (a.i64(), "i8"),
            (a.f16(), "f2"),
            (a.f32(), "f4"),
            (a.f64(), "f8"),
            (a.bool(), "b"),
            (a.str_ty(), "s"),
            (a.never(), "n"),
            (a.error(), "X"),
        ];
        for (ty, expected) in cases {
            let mut out = String::new();
            mangle_type(&a, &n, ty, &mut out);
            assert_eq!(out, expected, "failed for {:?}", a.get(ty));
        }
    }

    #[test]
    fn mangle_unit() {
        let mut a = make_arena();
        let n = names();
        let unit = a.unit();
        let mut out = String::new();
        mangle_type(&a, &n, unit, &mut out);
        assert_eq!(out, "TvE");
    }

    #[test]
    fn mangle_pointer() {
        let mut a = make_arena();
        let n = names();
        let inner = a.i64();
        let ptr = a.pointer(inner);
        let mut out = String::new();
        mangle_type(&a, &n, ptr, &mut out);
        assert_eq!(out, "Pi8");
    }

    #[test]
    fn mangle_tuple() {
        let mut a = make_arena();
        let n = names();
        let i64 = a.i64();
        let b = a.bool();
        let tup = a.tuple(vec![i64, b]);
        let mut out = String::new();
        mangle_type(&a, &n, tup, &mut out);
        assert_eq!(out, "Ti8bE");
    }

    #[test]
    fn mangle_named_no_args() {
        let mut a = make_arena();
        let e = Entity::from_raw(1);
        let n = names_with(&[(1, "MyStruct")]);
        let ty = a.named(e, vec![]);
        let mut out = String::new();
        mangle_type(&a, &n, ty, &mut out);
        assert_eq!(out, "8_MyStruct");
    }

    #[test]
    fn mangle_named_with_type_args() {
        let mut a = make_arena();
        let e = Entity::from_raw(1);
        let n = names_with(&[(1, "Array")]);
        let i64 = a.i64();
        let ty = a.named(e, vec![i64]);
        let mut out = String::new();
        mangle_type(&a, &n, ty, &mut out);
        assert_eq!(out, "5_ArrayIi8E");
    }

    #[test]
    fn mangle_named_nested_path() {
        let mut a = make_arena();
        let e = Entity::from_raw(1);
        let n = names_with(&[(1, "std.Array")]);
        let i64 = a.i64();
        let ty = a.named(e, vec![i64]);
        let mut out = String::new();
        mangle_type(&a, &n, ty, &mut out);
        assert_eq!(out, "N3_std5_ArrayEIi8E");
    }

    #[test]
    fn mangle_func_thin() {
        let mut a = make_arena();
        let n = names();
        let i32 = a.i32();
        let b = a.bool();
        let ft = a.intern(MirTy::FuncThin {
            params: vec![(i32, ParamConvention::Consuming), (i32, ParamConvention::Consuming)],
            ret: b,
        });
        let mut out = String::new();
        mangle_type(&a, &n, ft, &mut out);
        assert_eq!(out, "F2_i4i4bE");
    }

    #[test]
    fn mangle_func_thick() {
        let mut a = make_arena();
        let n = names();
        let i64 = a.i64();
        let unit = a.unit();
        let ft = a.intern(MirTy::FuncThick {
            params: vec![(i64, ParamConvention::Consuming)],
            ret: unit,
        });
        let mut out = String::new();
        mangle_type(&a, &n, ft, &mut out);
        assert_eq!(out, "C1_i8TvEE");
    }

    // -- Full function mangling --

    #[test]
    fn mangle_simple_main() {
        let mut a = make_arena();
        let n = names();
        let unit = a.unit();
        let result = mangle_function(&a, &n, "main", &[], None, &[], unit, None);
        assert_eq!(result, "_K04_mainRTvE");
    }

    #[test]
    fn mangle_method_with_receiver() {
        let mut a = make_arena();
        let n = names();
        let unit = a.unit();
        let result = mangle_function(
            &a,
            &n,
            "std.Array.count",
            &[],
            None,
            &[],
            unit,
            Some(ReceiverConvention::Borrow),
        );
        assert_eq!(result, "_K0N3_std5_Array5_countErRTvE");
    }

    #[test]
    fn mangle_generic_instantiation() {
        let mut a = make_arena();
        let n = names();
        let i64 = a.i64();
        let unit = a.unit();
        let result = mangle_function(
            &a,
            &n,
            "Array.append",
            &[i64],
            None,
            &[],
            unit,
            Some(ReceiverConvention::Borrow),
        );
        assert_eq!(result, "_K0N5_Array6_appendErRTvEIi8E");
    }

    #[test]
    fn mangle_with_self_type() {
        let mut a = make_arena();
        let e = Entity::from_raw(1);
        let n = names_with(&[(1, "ArrayIterator")]);
        let i64 = a.i64();
        let unit = a.unit();
        let self_ty = a.named(e, vec![i64]);
        let result = mangle_function(&a, &n, "Iterator.next", &[], Some(self_ty), &[], unit, None);
        assert_eq!(result, "_K0N8_Iterator4_nextERTvES_13_ArrayIteratorIi8E");
    }

    #[test]
    fn mangle_with_params() {
        let mut a = make_arena();
        let n = names();
        let i64 = a.i64();
        let unit = a.unit();
        let params = vec![
            MonoParam::new("x", i64, ParamConvention::Consuming),
            MonoParam::new("y", i64, ParamConvention::Borrow),
        ];
        let result = mangle_function(&a, &n, "add", &[], None, &params, unit, None);
        assert_eq!(result, "_K03_addZi8ri8ERTvE");
    }

    #[test]
    #[should_panic(expected = "monomorphization bug")]
    fn mangle_type_param_panics() {
        let mut a = make_arena();
        let n = names();
        let tp = a.intern(MirTy::TypeParam(Entity::from_raw(1)));
        let mut out = String::new();
        mangle_type(&a, &n, tp, &mut out);
    }

    #[test]
    fn mangle_nested_pointer_tuple() {
        let mut a = make_arena();
        let n = names();
        let i32 = a.i32();
        let b = a.bool();
        let tup = a.tuple(vec![i32, b]);
        let ptr = a.pointer(tup);
        let mut out = String::new();
        mangle_type(&a, &n, ptr, &mut out);
        assert_eq!(out, "PTi4bE");
    }
}
