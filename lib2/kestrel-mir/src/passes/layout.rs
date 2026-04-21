//! Layout pass — compute struct sizes, field offsets, and alignment.
//!
//! Uses a simple layout algorithm:
//! - Primitives have known sizes (i8=1, i16=2, i32=4, i64=8, etc.)
//! - Struct fields are laid out sequentially with alignment padding
//! - Pointers and references are pointer-sized (8 bytes on 64-bit)
//! - Tuples are laid out like structs
//!
//! This is a best-effort pass — types that can't be resolved (generics,
//! recursive types) get no layout. Codegen can fall back to its own
//! layout computation for those cases.

use crate::MirModule;
use crate::item::StructLayout;
use crate::ty::MirTy;

const PTR_SIZE: u64 = 8;
const PTR_ALIGN: u64 = 8;

/// Compute layouts for all structs in the module.
///
/// Fills in `StructDef.layout` for structs whose field types have known sizes.
/// Structs with generic fields or unknown types are skipped.
pub fn run_layout_pass(module: &mut MirModule) {
    // Multi-pass fixed-point: keep iterating until no more layouts can be computed.
    // Each pass may resolve structs whose dependencies were laid out in a prior pass.
    // Cycles naturally terminate (a struct can't contain itself by value).
    loop {
        let mut progress = false;

        for i in 0..module.structs.len() {
            if module.structs[i].layout.is_some() {
                continue;
            }

            // Try to compute layout from field types
            let field_sizes: Vec<Option<(u64, u64)>> = module.structs[i]
                .fields
                .iter()
                .map(|f| size_and_align_of(&f.ty, module))
                .collect();

            // Skip if any field has unknown size
            if field_sizes.iter().any(|s| s.is_none()) {
                continue;
            }

            let field_sizes: Vec<(u64, u64)> = field_sizes.into_iter().flatten().collect();

            // Compute layout: sequential with alignment padding
            let mut offset: u64 = 0;
            let mut max_align: u64 = 1;
            let mut field_offsets = Vec::with_capacity(field_sizes.len());

            for (size, align) in &field_sizes {
                let padding = (align - (offset % align)) % align;
                offset += padding;
                field_offsets.push(offset);
                offset += size;
                max_align = max_align.max(*align);
            }

            // Final padding to align the struct size
            let total_padding = (max_align - (offset % max_align)) % max_align;
            let total_size = offset + total_padding;

            module.structs[i].layout = Some(StructLayout {
                size: total_size,
                align: max_align,
                field_offsets,
            });

            progress = true;
        }

        if !progress {
            break;
        }
    }
}

/// Get the size and alignment of a MIR type, if known.
/// Returns None for types that can't be sized (generics, named types without layouts).
fn size_and_align_of(ty: &MirTy, module: &MirModule) -> Option<(u64, u64)> {
    match ty {
        // Primitives
        MirTy::Bool => Some((1, 1)),
        MirTy::I8 => Some((1, 1)),
        MirTy::I16 => Some((2, 2)),
        MirTy::I32 => Some((4, 4)),
        MirTy::I64 => Some((8, 8)),
        MirTy::F16 => Some((2, 2)),
        MirTy::F32 => Some((4, 4)),
        MirTy::F64 => Some((8, 8)),
        MirTy::Unit => Some((0, 1)),
        MirTy::Never => Some((0, 1)),

        // Str is a fat pointer (ptr + len)
        MirTy::Str => Some((PTR_SIZE * 2, PTR_ALIGN)),

        // Pointers and references are pointer-sized
        MirTy::Pointer(_) | MirTy::Ref(_) | MirTy::RefMut(_) => Some((PTR_SIZE, PTR_ALIGN)),

        // Thin function pointer
        MirTy::FuncThin { .. } => Some((PTR_SIZE, PTR_ALIGN)),

        // Thick function = code ptr + env ptr
        MirTy::FuncThick { .. } => Some((PTR_SIZE * 2, PTR_ALIGN)),

        // Tuples: lay out elements sequentially
        MirTy::Tuple(elems) => {
            let mut offset: u64 = 0;
            let mut max_align: u64 = 1;
            for elem in elems {
                let (size, align) = size_and_align_of(elem, module)?;
                let padding = (align - (offset % align)) % align;
                offset += padding + size;
                max_align = max_align.max(align);
            }
            let total_padding = (max_align - (offset % max_align)) % max_align;
            Some((offset + total_padding, max_align))
        },

        // Named types: look up the struct's layout if it's been computed
        MirTy::Named { entity, type_args } => {
            // Only non-generic types can be sized here
            if !type_args.is_empty() {
                // Generic — can't compute layout without monomorphization
                return None;
            }
            // Find the struct with this entity
            let struct_def = module.structs.iter().find(|s| s.entity == *entity)?;
            let layout = struct_def.layout.as_ref()?;
            Some((layout.size, layout.align))
        },

        // Can't size these without more context
        MirTy::TypeParam(_)
        | MirTy::SelfType
        | MirTy::AssociatedProjection { .. }
        | MirTy::Error => None,
    }
}
