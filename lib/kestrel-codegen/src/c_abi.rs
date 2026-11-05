//! How a value crosses the C boundary (`@extern` imports, `@export`
//! definitions, `CFunction` calls). Kestrel's own ABI passes every aggregate by
//! pointer; C splits a small struct into registers, copies a large one onto
//! the stack, or hands over a pointer to a copy, depending on the target.
//!
//! This module decides that once, from the MIR types, for both backends. A
//! backend then only bridges the two conventions: it loads a struct's
//! [`Piece`]s out of memory into C arguments (an import), or stores C
//! arguments back into memory for a Kestrel callee (an export).
//!
//! Supported: x86-64 System V and AArch64 AAPCS64 ([`CAbi`]). Any other
//! target keeps the old pointer-passing behaviour.

use kestrel_mir::mono::MonoModule;
use kestrel_mir::{Layout, MirTy, ParamConvention, StructLayout, TyArena, TyId};
pub use kestrel_target::CAbi;

use crate::layout::find_mono_struct;
use crate::repr::{ScalarModel, TypeCache, TypeRepr};

/// A register-sized piece of a struct passed in registers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CPrim {
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
    /// Two `float`s sharing one SSE register (x86-64 `<2 x float>`).
    F32x2,
}

impl CPrim {
    pub fn size(self) -> u64 {
        match self {
            CPrim::I8 => 1,
            CPrim::I16 => 2,
            CPrim::I32 | CPrim::F32 => 4,
            CPrim::I64 | CPrim::F64 | CPrim::F32x2 => 8,
        }
    }

    pub fn is_float(self) -> bool {
        matches!(self, CPrim::F32 | CPrim::F64 | CPrim::F32x2)
    }

    /// The integer piece covering the `rem` bytes left in an eightbyte.
    fn int_for(rem: u64) -> CPrim {
        match rem {
            0..=1 => CPrim::I8,
            2 => CPrim::I16,
            3..=4 => CPrim::I32,
            _ => CPrim::I64,
        }
    }
}

/// One piece of a split struct: `prim` loaded from `offset` bytes in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Piece {
    pub offset: u64,
    pub prim: CPrim,
}

/// How one parameter is passed to C.
#[derive(Clone, Debug, PartialEq)]
pub enum CArg {
    /// A zero-sized value: no argument.
    Ignore,
    /// A scalar, in the type's own scalar representation.
    Direct,
    /// A reference parameter (`x: &T`): the address, as a C pointer.
    Pointer,
    /// A struct split into registers. `array` asks the backend to keep the
    /// pieces together (AArch64 allocates them all-or-nothing).
    Split { pieces: Vec<Piece>, array: bool },
    /// A struct copied onto the stack by the call (x86-64 MEMORY class).
    Stack { size: u64, align: u64 },
    /// A pointer to a copy the caller makes (AArch64 structs over 16 bytes).
    Indirect,
}

/// How a C function returns its result.
#[derive(Clone, Debug, PartialEq)]
pub enum CRet {
    Void,
    /// A scalar, in the type's own scalar representation.
    Direct,
    /// A struct returned in registers.
    Split {
        pieces: Vec<Piece>,
    },
    /// A struct written through a hidden pointer the caller passes.
    Sret,
}

/// A whole C signature, parallel to the MIR params.
#[derive(Clone, Debug, PartialEq)]
pub struct CSig {
    pub ret: CRet,
    pub params: Vec<CArg>,
}

impl CSig {
    /// The byte size a split value's spill slot needs, rounded up so every
    /// piece can be loaded or stored whole without running past the slot.
    pub fn split_slot_size(pieces: &[Piece]) -> u64 {
        pieces
            .iter()
            .map(|p| p.offset + p.prim.size())
            .max()
            .unwrap_or(0)
            .next_multiple_of(8)
    }
}

/// Classify a C signature: return type `ret` and `(type, convention)` per param.
pub fn classify<M: ScalarModel>(
    abi: CAbi,
    tc: &mut TypeCache<M>,
    arena: &TyArena,
    module: &MonoModule,
    ret: TyId,
    params: &[(TyId, ParamConvention)],
) -> CSig {
    let mut cx = Classifier {
        abi,
        tc,
        arena,
        module,
        // x86-64: rdi..r9 and xmm0..xmm7.
        int_regs: 6,
        sse_regs: 8,
    };
    let ret = cx.ret(ret);
    if ret == CRet::Sret {
        cx.int_regs -= 1;
    }
    let params = params.iter().map(|&(ty, conv)| cx.arg(ty, conv)).collect();
    CSig { ret, params }
}

/// A primitive leaf of a type at a byte offset.
#[derive(Clone, Copy, Debug)]
struct Leaf {
    offset: u64,
    size: u64,
    float: bool,
}

struct Classifier<'a, M: ScalarModel> {
    abi: CAbi,
    tc: &'a mut TypeCache<M>,
    arena: &'a TyArena,
    module: &'a MonoModule,
    int_regs: u32,
    sse_regs: u32,
}

impl<M: ScalarModel> Classifier<'_, M> {
    fn ret(&mut self, ty: TyId) -> CRet {
        match self.tc.repr(ty, self.arena, self.module) {
            TypeRepr::Zst => CRet::Void,
            TypeRepr::Scalar(_) => CRet::Direct,
            TypeRepr::Aggregate { size, .. } => {
                let pieces = match self.abi {
                    CAbi::SysV64 => self.sysv_pieces(ty, size),
                    CAbi::Aapcs64 => self.aapcs_pieces(ty, size),
                    CAbi::Other => None,
                };
                pieces.map_or(CRet::Sret, |pieces| CRet::Split { pieces })
            },
        }
    }

    fn arg(&mut self, ty: TyId, conv: ParamConvention) -> CArg {
        if conv != ParamConvention::Consuming {
            self.take_regs(1, 0);
            return CArg::Pointer;
        }
        match self.tc.repr(ty, self.arena, self.module) {
            TypeRepr::Zst => CArg::Ignore,
            TypeRepr::Scalar(_) => {
                let float = self.leaves(ty).first().is_some_and(|l| l.float);
                if float {
                    self.take_regs(0, 1);
                } else {
                    self.take_regs(1, 0);
                }
                CArg::Direct
            },
            TypeRepr::Aggregate { size, align } => match self.abi {
                CAbi::SysV64 => self.sysv_arg(ty, size, align),
                CAbi::Aapcs64 => match self.aapcs_pieces(ty, size) {
                    Some(pieces) => CArg::Split {
                        pieces,
                        array: true,
                    },
                    None => CArg::Indirect,
                },
                CAbi::Other => CArg::Indirect,
            },
        }
    }

    /// x86-64: a struct goes in registers only when all its eightbytes fit in
    /// the registers left; otherwise the whole struct goes on the stack.
    fn sysv_arg(&mut self, ty: TyId, size: u64, align: u64) -> CArg {
        let Some(pieces) = self.sysv_pieces(ty, size) else {
            return CArg::Stack { size, align };
        };
        let sse = pieces.iter().filter(|p| p.prim.is_float()).count() as u32;
        let int = pieces.len() as u32 - sse;
        if int > self.int_regs || sse > self.sse_regs {
            return CArg::Stack { size, align };
        }
        self.take_regs(int, sse);
        CArg::Split {
            pieces,
            array: false,
        }
    }

    fn take_regs(&mut self, int: u32, sse: u32) {
        self.int_regs = self.int_regs.saturating_sub(int);
        self.sse_regs = self.sse_regs.saturating_sub(sse);
    }

    /// x86-64 System V: a struct of at most 16 bytes, all fields aligned,
    /// is one piece per eightbyte: INTEGER if any integer overlaps it, else SSE.
    fn sysv_pieces(&mut self, ty: TyId, size: u64) -> Option<Vec<Piece>> {
        if size > 16 {
            return None;
        }
        let leaves = self.leaves(ty);
        if leaves.iter().any(|l| l.size == 0 || l.offset % l.size != 0) {
            return None;
        }
        let pieces = (0..size.div_ceil(8))
            .map(|k| {
                let (start, end) = (k * 8, (k * 8 + 8).min(size));
                let inside: Vec<&Leaf> = leaves
                    .iter()
                    .filter(|l| l.offset < end && l.offset + l.size > start)
                    .collect();
                let prim = if inside.is_empty() || inside.iter().any(|l| !l.float) {
                    CPrim::int_for(end - start)
                } else if inside.iter().any(|l| l.size == 8) {
                    CPrim::F64
                } else if inside.iter().all(|l| l.offset < start + 4) {
                    CPrim::F32
                } else {
                    CPrim::F32x2
                };
                Piece {
                    offset: start,
                    prim,
                }
            })
            .collect();
        Some(pieces)
    }

    /// AArch64: a homogeneous float aggregate (1-4 equal floats) goes in float
    /// registers; any other struct of at most 16 bytes in one or two integer
    /// registers.
    fn aapcs_pieces(&mut self, ty: TyId, size: u64) -> Option<Vec<Piece>> {
        let leaves = self.leaves(ty);
        if let Some(first) = leaves.first()
            && leaves.len() <= 4
            && leaves.iter().all(|l| l.float && l.size == first.size)
            && leaves
                .iter()
                .enumerate()
                .all(|(i, l)| l.offset == i as u64 * first.size)
            && size == leaves.len() as u64 * first.size
        {
            let prim = if first.size == 8 {
                CPrim::F64
            } else {
                CPrim::F32
            };
            return Some(
                leaves
                    .iter()
                    .map(|l| Piece {
                        offset: l.offset,
                        prim,
                    })
                    .collect(),
            );
        }
        if size > 16 {
            return None;
        }
        Some(
            (0..size.div_ceil(8))
                .map(|k| Piece {
                    offset: k * 8,
                    prim: CPrim::I64,
                })
                .collect(),
        )
    }

    /// The primitive leaves of `ty`, in offset order.
    fn leaves(&mut self, ty: TyId) -> Vec<Leaf> {
        let mut out = Vec::new();
        self.collect_leaves(ty, 0, &mut out);
        out.sort_by_key(|l| l.offset);
        out
    }

    fn collect_leaves(&mut self, ty: TyId, base: u64, out: &mut Vec<Leaf>) {
        let (arena, module) = (self.arena, self.module);
        let int = |size| Leaf {
            offset: base,
            size,
            float: false,
        };
        let float = |size| Leaf {
            offset: base,
            size,
            float: true,
        };
        let repr = self.tc.repr(ty, arena, module);
        // Fat pointers, strings and closures are words of address and length.
        if module.is_fat(ty) {
            self.push_words(base, repr.size(), out);
            return;
        }
        match arena.get(ty) {
            MirTy::I8 | MirTy::Bool => out.push(int(1)),
            MirTy::I16 | MirTy::F16 => out.push(int(2)),
            MirTy::I32 => out.push(int(4)),
            MirTy::I64 => out.push(int(8)),
            MirTy::F32 => out.push(float(4)),
            MirTy::F64 => out.push(float(8)),
            MirTy::Pointer(_) | MirTy::FuncThin { .. } | MirTy::Ref { .. } => {
                out.push(int(self.tc.ptr_size))
            },
            MirTy::Tuple(elems) => {
                let mut layout = StructLayout::new();
                for &elem in elems {
                    let r = self.tc.repr(elem, arena, module);
                    layout.append_field(StructLayout::scalar(r.size(), r.align()));
                }
                for (i, &elem) in elems.iter().enumerate() {
                    self.collect_leaves(elem, base + layout.field_offsets[i], out);
                }
            },
            MirTy::Named { entity, type_args } => {
                if let Some(s) = find_mono_struct(entity, type_args, module)
                    && let Some(Layout::Struct(sl)) = &s.type_info.layout
                    && sl.field_offsets.len() == s.fields.len()
                {
                    for (field, &offset) in s.fields.iter().zip(&sl.field_offsets) {
                        self.collect_leaves(field.ty, base + offset, out);
                    }
                    return;
                }
                // An enum (its tag and payload words) or an unknown layout:
                // integer words, which is what C sees for a tagged union.
                match repr {
                    TypeRepr::Scalar(s) => out.push(int(M::size_bytes(s))),
                    _ => self.push_words(base, repr.size(), out),
                }
            },
            MirTy::Never | MirTy::Error => {},
            _ => self.push_words(base, repr.size(), out),
        }
    }

    /// Integer leaves covering `size` bytes from `base`, at most a word each.
    fn push_words(&self, base: u64, size: u64, out: &mut Vec<Leaf>) {
        let word = self.tc.ptr_size;
        let mut offset = 0;
        while offset < size {
            let len = (size - offset).min(word);
            out.push(Leaf {
                offset: base + offset,
                size: len,
                float: false,
            });
            offset += len;
        }
    }
}
