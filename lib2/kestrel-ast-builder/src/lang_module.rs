//! Seeds the `lang` module with synthetic entities for built-in types.
//!
//! The `lang` module contains compiler-known primitive types (`i8`, `i64`,
//! `str`, `ptr[T]`, etc.) that are resolved through normal name resolution
//! rather than special-cased builtin handling. This gives consistent error
//! messages and prevents nonsensical paths like `lang[Int].i32[String]`.
//!
//! Intrinsic *functions* (e.g. `lang.i64_add`, `lang.cast_i64_i8`) are NOT
//! created as entities here — they're parsed by naming convention in the
//! binder/HIR lowering since there are hundreds of combinations.

use kestrel_hecs::{Entity, World};

use crate::components::*;

/// Create the `lang` module and its built-in type entities.
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

    // Integer types
    seed_scalar(world, lang, "i1");
    seed_scalar(world, lang, "i8");
    seed_scalar(world, lang, "i16");
    seed_scalar(world, lang, "i32");
    seed_scalar(world, lang, "i64");

    // Unsigned integer types
    seed_scalar(world, lang, "u8");
    seed_scalar(world, lang, "u16");
    seed_scalar(world, lang, "u32");
    seed_scalar(world, lang, "u64");

    // Float types
    seed_scalar(world, lang, "f16");
    seed_scalar(world, lang, "f32");
    seed_scalar(world, lang, "f64");

    // String primitive
    seed_scalar(world, lang, "str");

    // Pointer type: lang.ptr[T]
    seed_ptr(world, lang);

    lang
}

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

    // Type parameter T
    let t = world.spawn();
    world.set(t, NodeKind::TypeParameter);
    world.set(t, Name("T".into()));
    world.set_parent(t, ptr);

    world.set(ptr, TypeParams(vec![t]));
}
