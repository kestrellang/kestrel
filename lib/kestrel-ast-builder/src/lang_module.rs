//! Seeds the `lang` module with synthetic entities for built-in types
//! and intrinsic functions.
//!
//! The `lang` module contains compiler-known primitive types (`i8`, `i64`,
//! `str`, `ptr[T]`, etc.) and intrinsic functions (`i64_add`, `cast_i64_i32`,
//! `ptr_read[T]`, etc.) that are resolved through normal name resolution.
//!
//! Types are seeded as `NodeKind::Struct` with `Intrinsic` marker.
//! Functions are seeded as `NodeKind::Function` with `Intrinsic` marker,
//! `Callable` (parameter types), and `TypeAnnotation` (return type).

use kestrel_ast::{AstType, PathSegment};
use kestrel_hecs::{Entity, World};
use kestrel_span::Span;

use crate::components::*;

/// Create the `lang` module and its built-in type and function entities.
///
/// Must be called once during world initialization, before any source files
/// are processed. The `lang` module becomes a child of `root`.
///
/// Returns the `lang` module entity.
pub fn seed_lang_module(world: &mut World, root: Entity) -> Entity {
    let lang = world.spawn();
    world.set(lang, NodeKind::Module);
    world.set(lang, Name("lang".into()));
    world.set_parent(lang, root);

    // Primitive types
    seed_scalar(world, lang, "i1");
    seed_scalar(world, lang, "i8");
    seed_scalar(world, lang, "i16");
    seed_scalar(world, lang, "i32");
    seed_scalar(world, lang, "i64");
    // No uN types — signedness is determined by the operation, not the type.
    // Both signed and unsigned integers use iN representation.
    seed_scalar(world, lang, "f16");
    seed_scalar(world, lang, "f32");
    seed_scalar(world, lang, "f64");
    seed_scalar(world, lang, "str");
    seed_ptr(world, lang);

    // Intrinsic functions
    seed_integer_ops(world, lang);
    seed_float_ops(world, lang);
    seed_cast_ops(world, lang);
    seed_pointer_ops(world, lang);
    seed_string_ops(world, lang);
    seed_misc_ops(world, lang);

    lang
}

// ===== Type seeding =====

/// Create a scalar type entity (no type parameters).
fn seed_scalar(world: &mut World, lang: Entity, name: &str) {
    let e = world.spawn();
    world.set(e, NodeKind::Struct);
    world.set(e, Name(name.into()));
    world.set(e, Vis::Public);
    world.set(e, Typed);
    world.set(e, Intrinsic);
    world.set_parent(e, lang);
}

/// Create `lang.ptr[T]` — a generic intrinsic pointer type.
fn seed_ptr(world: &mut World, lang: Entity) {
    let ptr = world.spawn();
    world.set(ptr, NodeKind::Struct);
    world.set(ptr, Name("ptr".into()));
    world.set(ptr, Vis::Public);
    world.set(ptr, Typed);
    world.set(ptr, Intrinsic);
    world.set_parent(ptr, lang);

    let t = world.spawn();
    world.set(t, NodeKind::TypeParameter);
    world.set(t, Name("T".into()));
    world.set_parent(t, ptr);

    world.set(ptr, TypeParams(vec![t]));
}

// ===== AstType helpers =====

/// `lang.<name>` type reference (e.g., `lang.i64`).
fn lang_ty(name: &str) -> AstType {
    AstType::Named {
        segments: vec![
            PathSegment {
                name: "lang".into(),
                type_args: vec![],
                span: Span::synthetic(0),
            },
            PathSegment {
                name: name.into(),
                type_args: vec![],
                span: Span::synthetic(0),
            },
        ],
        span: Span::synthetic(0),
    }
}

/// Bare type parameter reference (e.g., `T`).
fn param_ty(name: &str) -> AstType {
    AstType::Named {
        segments: vec![PathSegment {
            name: name.into(),
            type_args: vec![],
            span: Span::synthetic(0),
        }],
        span: Span::synthetic(0),
    }
}

/// `lang.ptr[T]` type reference.
fn ptr_of(inner: AstType) -> AstType {
    AstType::Named {
        segments: vec![
            PathSegment {
                name: "lang".into(),
                type_args: vec![],
                span: Span::synthetic(0),
            },
            PathSegment {
                name: "ptr".into(),
                type_args: vec![inner],
                span: Span::synthetic(0),
            },
        ],
        span: Span::synthetic(0),
    }
}

/// Unit type `()`.
fn unit_ty() -> AstType {
    AstType::Tuple(vec![], Span::synthetic(0))
}

// ===== Function seeding helpers =====

/// Seed a non-generic intrinsic function: `NodeKind::Function` + `Intrinsic`.
fn seed_fn(world: &mut World, lang: Entity, name: &str, params: &[(&str, AstType)], ret: AstType) {
    let e = world.spawn();
    world.set(e, NodeKind::Function);
    world.set(e, Name(name.into()));
    world.set(e, Vis::Public);
    world.set(e, Intrinsic);
    world.set(
        e,
        Callable {
            params: params
                .iter()
                .map(|(n, ty)| AstParam {
                    label: None,
                    name: n.to_string(),
                    ty: Some(ty.clone()),
                    default_entity: None,
                    pattern: None,
                    is_mut: false,
                    is_consuming: false,
                })
                .collect(),
            receiver: None,
        },
    );
    world.set(e, TypeAnnotation(ret));
    world.set_parent(e, lang);
}

/// Seed a generic intrinsic function with one type parameter `T`.
fn seed_generic_fn(
    world: &mut World,
    lang: Entity,
    name: &str,
    params: &[(&str, AstType)],
    ret: AstType,
) {
    seed_generic_fn_multi(world, lang, name, &["T"], params, ret);
}

/// Seed a generic intrinsic function with an explicit list of type parameters.
/// Each name becomes a `TypeParameter` entity in declaration order; parameter
/// and return types may refer to them with `param_ty("Name")`.
fn seed_generic_fn_multi(
    world: &mut World,
    lang: Entity,
    name: &str,
    type_param_names: &[&str],
    params: &[(&str, AstType)],
    ret: AstType,
) {
    let e = world.spawn();
    world.set(e, NodeKind::Function);
    world.set(e, Name(name.into()));
    world.set(e, Vis::Public);
    world.set(e, Intrinsic);

    let type_params: Vec<Entity> = type_param_names
        .iter()
        .map(|tp_name| {
            let t = world.spawn();
            world.set(t, NodeKind::TypeParameter);
            world.set(t, Name((*tp_name).into()));
            world.set_parent(t, e);
            t
        })
        .collect();
    world.set(e, TypeParams(type_params));

    world.set(
        e,
        Callable {
            params: params
                .iter()
                .map(|(n, ty)| AstParam {
                    label: None,
                    name: n.to_string(),
                    ty: Some(ty.clone()),
                    default_entity: None,
                    pattern: None,
                    is_mut: false,
                    is_consuming: false,
                })
                .collect(),
            receiver: None,
        },
    );
    world.set(e, TypeAnnotation(ret));
    world.set_parent(e, lang);
}

// ===== Intrinsic function categories =====

/// Integer arithmetic, comparison, bitwise, and unary ops for i1/i8/i16/i32/i64.
fn seed_integer_ops(world: &mut World, lang: Entity) {
    let int_types = ["i1", "i8", "i16", "i32", "i64"];
    let i1 = lang_ty("i1");

    for ty_name in int_types {
        let ty = lang_ty(ty_name);

        // Binary ops returning same type
        for op in ["add", "sub", "mul", "and", "or", "xor", "shl"] {
            seed_fn(
                world,
                lang,
                &format!("{ty_name}_{op}"),
                &[("a", ty.clone()), ("b", ty.clone())],
                ty.clone(),
            );
        }

        // Binary comparison ops returning i1
        for op in ["eq", "ne"] {
            seed_fn(
                world,
                lang,
                &format!("{ty_name}_{op}"),
                &[("a", ty.clone()), ("b", ty.clone())],
                i1.clone(),
            );
        }

        // Signed binary ops: arithmetic returns same type, comparison returns i1
        for op in ["div", "rem", "shr"] {
            seed_fn(
                world,
                lang,
                &format!("{ty_name}_signed_{op}"),
                &[("a", ty.clone()), ("b", ty.clone())],
                ty.clone(),
            );
        }
        for op in ["lt", "le", "gt", "ge"] {
            seed_fn(
                world,
                lang,
                &format!("{ty_name}_signed_{op}"),
                &[("a", ty.clone()), ("b", ty.clone())],
                i1.clone(),
            );
        }

        // Unsigned binary ops: same pattern as signed
        for op in ["div", "rem", "shr"] {
            seed_fn(
                world,
                lang,
                &format!("{ty_name}_unsigned_{op}"),
                &[("a", ty.clone()), ("b", ty.clone())],
                ty.clone(),
            );
        }
        for op in ["lt", "le", "gt", "ge"] {
            seed_fn(
                world,
                lang,
                &format!("{ty_name}_unsigned_{op}"),
                &[("a", ty.clone()), ("b", ty.clone())],
                i1.clone(),
            );
        }

        // Unary ops returning same type
        for op in ["neg", "not", "popcount", "clz", "ctz", "bswap"] {
            seed_fn(
                world,
                lang,
                &format!("{ty_name}_{op}"),
                &[("a", ty.clone())],
                ty.clone(),
            );
        }
    }
}

/// Float arithmetic, comparison, math, constants, and predicates for f32/f64.
fn seed_float_ops(world: &mut World, lang: Entity) {
    let float_types = ["f32", "f64"];
    let i1 = lang_ty("i1");

    for ty_name in float_types {
        let ty = lang_ty(ty_name);

        // Binary arithmetic → same type
        for op in ["add", "sub", "mul", "div"] {
            seed_fn(
                world,
                lang,
                &format!("{ty_name}_{op}"),
                &[("a", ty.clone()), ("b", ty.clone())],
                ty.clone(),
            );
        }

        // Binary comparison → i1
        for op in ["eq", "ne", "lt", "le", "gt", "ge"] {
            seed_fn(
                world,
                lang,
                &format!("{ty_name}_{op}"),
                &[("a", ty.clone()), ("b", ty.clone())],
                i1.clone(),
            );
        }

        // Unary
        seed_fn(
            world,
            lang,
            &format!("{ty_name}_neg"),
            &[("a", ty.clone())],
            ty.clone(),
        );

        // Constants (0-ary)
        for op in ["infinity", "nan"] {
            seed_fn(world, lang, &format!("{ty_name}_{op}"), &[], ty.clone());
        }

        // Predicates → i1
        for op in ["is_nan", "is_infinite"] {
            seed_fn(
                world,
                lang,
                &format!("{ty_name}_{op}"),
                &[("a", ty.clone())],
                i1.clone(),
            );
        }

        // Math functions → same type
        for op in ["floor", "ceil", "round", "trunc", "sqrt"] {
            seed_fn(
                world,
                lang,
                &format!("{ty_name}_{op}"),
                &[("a", ty.clone())],
                ty.clone(),
            );
        }

        // Ternary: fma(a, b, c) → same type
        seed_fn(
            world,
            lang,
            &format!("{ty_name}_fma"),
            &[("a", ty.clone()), ("b", ty.clone()), ("c", ty.clone())],
            ty.clone(),
        );

        // copysign(a, b) → same type
        seed_fn(
            world,
            lang,
            &format!("{ty_name}_copysign"),
            &[("a", ty.clone()), ("b", ty.clone())],
            ty.clone(),
        );
    }
}

/// Map unsigned type names to their intrinsic iN representation.
/// uN types don't exist as separate intrinsics; signedness is per-operation.
fn intrinsic_ty(name: &str) -> &str {
    match name {
        "u8" => "i8",
        "u16" => "i16",
        "u32" => "i32",
        "u64" => "i64",
        other => other,
    }
}

/// Cast intrinsics: `cast_<from>_<to>(value) -> to_type` for all primitive pairs.
/// Includes uN names for stdlib compatibility (e.g. `cast_u8_i32` accepts lang.i8).
fn seed_cast_ops(world: &mut World, lang: Entity) {
    let all_types = [
        "i1", "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f32", "f64",
    ];

    for from in &all_types {
        for to in &all_types {
            // Skip identity (same name) and also skip where both map to same intrinsic
            // (e.g. cast_u8_i8 is a no-op since both are i8)
            if from == to || intrinsic_ty(from) == intrinsic_ty(to) {
                continue;
            }
            seed_fn(
                world,
                lang,
                &format!("cast_{from}_{to}"),
                &[("value", lang_ty(intrinsic_ty(from)))],
                lang_ty(intrinsic_ty(to)),
            );
        }
    }
}

/// Pointer intrinsics — generic over pointee type T.
fn seed_pointer_ops(world: &mut World, lang: Entity) {
    let i64_ty = lang_ty("i64");
    let i1_ty = lang_ty("i1");
    let t = param_ty("T");
    let ptr_t = ptr_of(t.clone());

    // Generic pointer ops with type parameter T
    seed_generic_fn(world, lang, "ptr_null", &[], ptr_t.clone());
    seed_generic_fn(
        world,
        lang,
        "ptr_from_address",
        &[("addr", i64_ty.clone())],
        ptr_t.clone(),
    );
    seed_generic_fn(
        world,
        lang,
        "ptr_to",
        &[("value", t.clone())],
        ptr_t.clone(),
    );
    seed_generic_fn(
        world,
        lang,
        "ptr_read",
        &[("ptr", ptr_t.clone())],
        t.clone(),
    );
    // Mutable by-reference view of the pointee (used by `Pointer.withMut`).
    // Lowers to `BeginMutBorrowAddr`; the result is passed as a `mutating`
    // closure arg so the pointee is mutated in place.
    seed_generic_fn(
        world,
        lang,
        "ptr_mut_borrow",
        &[("ptr", ptr_t.clone())],
        t.clone(),
    );
    seed_generic_fn(
        world,
        lang,
        "ptr_write",
        &[("ptr", ptr_t.clone()), ("value", t.clone())],
        unit_ty(),
    );
    seed_generic_fn(
        world,
        lang,
        "drop_in_place",
        &[("ptr", ptr_t.clone())],
        unit_ty(),
    );
    seed_generic_fn(
        world,
        lang,
        "ptr_offset",
        &[("ptr", ptr_t.clone()), ("offset", i64_ty.clone())],
        ptr_t.clone(),
    );
    // cast_ptr reinterprets a ptr[From] as ptr[To]. Callers supply `From`
    // as `_` to let inference fill it from the argument: the canonical form
    // is `lang.cast_ptr[_, To](p)`.
    seed_generic_fn_multi(
        world,
        lang,
        "cast_ptr",
        &["From", "To"],
        &[("ptr", ptr_of(param_ty("From")))],
        ptr_of(param_ty("To")),
    );
    seed_generic_fn(world, lang, "sizeof", &[], i64_ty.clone());
    seed_generic_fn(world, lang, "alignof", &[], i64_ty.clone());

    // Pointer queries (generic — work on any ptr[T])
    seed_generic_fn(world, lang, "ptr_is_null", &[("ptr", ptr_t.clone())], i1_ty);
    seed_generic_fn(
        world,
        lang,
        "ptr_to_address",
        &[("ptr", ptr_t.clone())],
        i64_ty,
    );
}

/// String intrinsics — raw pointer and length queries on `lang.str`.
/// MIR lowering recognizes these by name (see body_lower.rs).
fn seed_string_ops(world: &mut World, lang: Entity) {
    let str_ty = lang_ty("str");
    let i64_ty = lang_ty("i64");
    let ptr_i8 = ptr_of(lang_ty("i8"));

    seed_fn(world, lang, "str_ptr", &[("s", str_ty.clone())], ptr_i8);
    seed_fn(world, lang, "str_len", &[("s", str_ty)], i64_ty);
}

/// Misc intrinsics: panic, atomics.
fn seed_misc_ops(world: &mut World, lang: Entity) {
    let str_ty = lang_ty("str");

    // panic(message) → Never (diverging function)
    let never = AstType::Never(Span::synthetic(0));
    seed_fn(
        world,
        lang,
        "panic",
        &[("message", str_ty.clone())],
        never.clone(),
    );
    seed_fn(world, lang, "panic_unwind", &[("message", str_ty)], never);

    // Atomic ops — generic over value type
    let t = param_ty("T");
    let ptr_t = ptr_of(t.clone());
    seed_generic_fn(
        world,
        lang,
        "atomic_add",
        &[("ptr", ptr_t.clone()), ("value", t.clone())],
        t.clone(),
    );
    seed_generic_fn(
        world,
        lang,
        "atomic_sub",
        &[("ptr", ptr_t), ("value", t.clone())],
        t,
    );
}
