//! Fat-pointer legalization (existentials plan Phase 9).
//!
//! A pointer (or reference) to an unsized type, and a borrowed raw
//! existential, is a two-word (data, table) value: `MonoModule::is_fat`.
//! Once every type is concrete, this pass rewrites the pointer operations
//! that meet one, so codegen only ever sees thin pointers plus the three fat
//! ops (`FatWord`, `WordInt`, `FatPtr`):
//!
//! - a cast to a thin pointer, `PtrToAddress`, `PtrIsNull` read the data word;
//! - a cast from a thin pointer pairs it with a null table;
//! - `PtrWithMeta` pairs one pointer's data word with another's table;
//! - `SizeOfVal` / `AlignOfVal` fold to the static answer for a sized pointee
//!   and read the table (plus a struct's sized prefix) for an unsized one;
//! - `destroy_addr` of an unsized pointee calls the table's drop thunk.
//!
//! A cast between two fat pointers, `PtrRead`, and `begin_borrow_addr` of an
//! unsized pointee need nothing: a fat value and a fat pointer share one
//! representation (the address of the two words).

use crate::callee::Callee;
use crate::immediate::{Immediate, ImmediateKind};
use crate::inst::{CallArg, InstKind, Instruction};
use crate::item::Layout;
use crate::item::vtable::{VTABLE_ALIGN, VTABLE_DROP, VTABLE_SIZE};
use crate::mono::MonoModule;
use crate::ty::MirTy;
use crate::value::ValueDef;
use crate::{IntBits, Op, ParamConvention, Signedness, TyId, ValueId};

pub fn legalize(module: &mut MonoModule) {
    for i in 0..module.functions.len() {
        let Some(mut body) = module.functions[i].body.take() else {
            continue;
        };
        let mut cx = Legalize { module, body: &mut body };
        cx.run();
        module.functions[i].body = Some(body);
    }
}

struct Legalize<'a> {
    module: &'a mut MonoModule,
    body: &'a mut crate::body::OssaBody,
}

impl Legalize<'_> {
    fn run(&mut self) {
        for b in 0..self.body.blocks.len() {
            let insts = std::mem::take(&mut self.body.blocks[b].insts);
            let mut out = Vec::with_capacity(insts.len());
            for inst in insts {
                self.rewrite(inst, &mut out);
            }
            self.body.blocks[b].insts = out;
        }
    }

    fn ty_of(&self, v: ValueId) -> TyId {
        self.body.values[v.index()].ty
    }

    fn fat(&self, ty: TyId) -> bool {
        self.module.is_fat(ty)
    }

    fn raw_ptr(&mut self) -> TyId {
        let i8 = self.module.ty_arena.i8();
        self.module.ty_arena.pointer(i8)
    }

    fn value(&mut self, ty: TyId) -> ValueId {
        self.body.alloc_value(ValueDef::owned(ty))
    }

    fn op1(&mut self, out: &mut Vec<Instruction>, op: Op, arg: ValueId, ty: TyId) -> ValueId {
        let result = self.value(ty);
        out.push(Instruction::new(InstKind::Op1 { result, op, arg }));
        result
    }

    fn op2(&mut self, out: &mut Vec<Instruction>, op: Op, lhs: ValueId, rhs: ValueId) -> ValueId {
        let ty = self.module.ty_arena.i64();
        let result = self.value(ty);
        out.push(Instruction::new(InstKind::Op2 {
            result,
            op,
            lhs,
            rhs,
        }));
        result
    }

    fn int(&mut self, out: &mut Vec<Instruction>, value: i128) -> ValueId {
        let ty = self.module.ty_arena.i64();
        let result = self.value(ty);
        out.push(Instruction::new(InstKind::Literal {
            result,
            value: Immediate::i64(value),
        }));
        result
    }

    /// The data word of `v` when it is fat, else `v` itself.
    fn data(&mut self, out: &mut Vec<Instruction>, v: ValueId) -> ValueId {
        if !self.fat(self.ty_of(v)) {
            return v;
        }
        let raw = self.raw_ptr();
        self.op1(out, Op::FatWord(0), v, raw)
    }

    /// `result = (data, table)` viewed as the fat type `ty`.
    fn make_fat(
        &mut self,
        out: &mut Vec<Instruction>,
        result: ValueId,
        data: ValueId,
        table: ValueId,
        ty: TyId,
    ) {
        let raw = self.raw_ptr();
        let pair_ty = self.module.ty_arena.tuple(vec![raw, raw]);
        let pair = self.value(pair_ty);
        out.push(Instruction::new(InstKind::Tuple {
            result: pair,
            elements: vec![data, table],
        }));
        out.push(Instruction::new(InstKind::Op1 {
            result,
            op: Op::FatPtr(ty),
            arg: pair,
        }));
    }

    fn rewrite(&mut self, inst: Instruction, out: &mut Vec<Instruction>) {
        match inst.kind {
            InstKind::Op1 {
                result,
                op: op @ (Op::PtrCast(to) | Op::PtrBitcast(to)),
                arg,
            } => {
                let (from_fat, to_fat) = (self.fat(self.ty_of(arg)), self.fat(to));
                if from_fat && !to_fat {
                    let data = self.data(out, arg);
                    out.push(Instruction::new(InstKind::Op1 { result, op, arg: data }));
                } else if !from_fat && to_fat {
                    let raw = self.raw_ptr();
                    let null = self.value(raw);
                    out.push(Instruction::new(InstKind::Literal {
                        result: null,
                        value: Immediate::new(ImmediateKind::NullPtr(raw)),
                    }));
                    let data = self.op1(out, Op::PtrCast(raw), arg, raw);
                    self.make_fat(out, result, data, null, to);
                } else {
                    out.push(Instruction::new(InstKind::Op1 { result, op, arg }));
                }
            },
            InstKind::Op1 {
                result,
                op: op @ (Op::PtrToAddress | Op::PtrIsNull),
                arg,
            } => {
                let data = self.data(out, arg);
                out.push(Instruction::new(InstKind::Op1 { result, op, arg: data }));
            },
            InstKind::Op1 {
                result,
                op: op @ (Op::SizeOfVal(ty) | Op::AlignOfVal(ty)),
                arg,
            } => {
                let size = matches!(op, Op::SizeOfVal(_));
                self.size_or_align(out, result, ty, arg, size);
            },
            InstKind::Op2 {
                result,
                op: Op::PtrWithMeta(to),
                lhs,
                rhs,
            } => {
                let data = self.data(out, lhs);
                if self.fat(to) && self.fat(self.ty_of(rhs)) {
                    let raw = self.raw_ptr();
                    let table = self.op1(out, Op::FatWord(1), rhs, raw);
                    let data = self.op1(out, Op::PtrCast(raw), data, raw);
                    self.make_fat(out, result, data, table, to);
                } else {
                    out.push(Instruction::new(InstKind::Op1 {
                        result,
                        op: Op::PtrCast(to),
                        arg: data,
                    }));
                }
            },
            InstKind::DestroyAddr { address, ty } if self.fat(self.ty_of(address)) => {
                // Drop the payload through its table.
                let raw = self.raw_ptr();
                let data = self.op1(out, Op::FatWord(0), address, raw);
                let table = self.op1(out, Op::FatWord(1), address, raw);
                let unit = self.module.ty_arena.unit();
                let fn_ty = self.module.ty_arena.intern(MirTy::FuncThin {
                    params: vec![(raw, ParamConvention::Consuming)],
                    ret: unit,
                });
                let drop = self.op1(out, Op::FatWord(VTABLE_DROP), table, fn_ty);
                kestrel_debug::ktrace!("vtable", "dynamic drop of {:?}", ty);
                out.push(Instruction::new(InstKind::Call {
                    result: None,
                    callee: Callee::Thin(drop),
                    args: vec![CallArg {
                        value: data,
                        convention: ParamConvention::Consuming,
                    }],
                }));
            },
            other => out.push(Instruction {
                kind: other,
                span: inst.span,
            }),
        }
    }

    /// `SizeOfVal` / `AlignOfVal` of `ty` at `ptr`, into `result`.
    fn size_or_align(
        &mut self,
        out: &mut Vec<Instruction>,
        result: ValueId,
        ty: TyId,
        ptr: ValueId,
        size: bool,
    ) {
        if !self.fat(self.ty_of(ptr)) {
            let op = if size { Op::SizeOf(ty) } else { Op::AlignOf(ty) };
            out.push(Instruction::new(InstKind::Op1 { result, op, arg: ptr }));
            return;
        }
        let raw = self.raw_ptr();
        let table = self.op1(out, Op::FatWord(1), ptr, raw);
        let word = if size { VTABLE_SIZE } else { VTABLE_ALIGN };
        // A struct with an unsized tail: its sized prefix, then the payload at
        // the payload's alignment. Otherwise the payload itself.
        let Some((psize, palign)) = self.prefix_layout(ty) else {
            out.push(Instruction::new(InstKind::Op1 {
                result,
                op: Op::WordInt(word),
                arg: table,
            }));
            return;
        };
        let i64_ty = self.module.ty_arena.i64();
        let talign = self.op1(out, Op::WordInt(VTABLE_ALIGN), table, i64_ty);
        // Alignments are powers of two: max(a, b) = ((a-1) | (b-1)) + 1.
        let one = self.int(out, 1);
        let pa_m1 = self.int(out, palign as i128 - 1);
        let ta_m1 = self.op2(out, Op::Sub(IntBits::I64, Signedness::Signed), talign, one);
        let mask = self.op2(out, Op::Or(IntBits::I64), pa_m1, ta_m1);
        if !size {
            out.push(Instruction::new(InstKind::Op2 {
                result,
                op: Op::Add(IntBits::I64, Signedness::Signed),
                lhs: mask,
                rhs: one,
            }));
            return;
        }
        // offset = round_up(prefix, payload align); end = offset + payload size;
        // size = round_up(end, max align).
        let tsize = self.op1(out, Op::WordInt(VTABLE_SIZE), table, i64_ty);
        let p = self.int(out, psize as i128);
        let offset = self.round_up(out, p, ta_m1);
        let end = self.op2(out, Op::Add(IntBits::I64, Signedness::Signed), offset, tsize);
        let sum = self.op2(out, Op::Add(IntBits::I64, Signedness::Signed), end, mask);
        let not = self.op1(out, Op::Not(IntBits::I64), mask, i64_ty);
        out.push(Instruction::new(InstKind::Op2 {
            result,
            op: Op::And(IntBits::I64),
            lhs: sum,
            rhs: not,
        }));
    }

    /// The sized prefix (size, align) of an unsized-tail struct.
    fn prefix_layout(&self, ty: TyId) -> Option<(u64, u64)> {
        let MirTy::Named { entity, type_args } = self.module.ty_arena.get(ty).clone() else {
            return None;
        };
        match &self.module.structs.get(&(entity, type_args))?.type_info.layout {
            Some(Layout::Struct(l)) if l.unsized_tail => Some((l.size, l.align)),
            _ => None,
        }
    }

    /// `round_up(x, a) = (x + (a-1)) & !(a-1)`, given `a - 1`.
    fn round_up(&mut self, out: &mut Vec<Instruction>, x: ValueId, a_m1: ValueId) -> ValueId {
        let i64_ty = self.module.ty_arena.i64();
        let sum = self.op2(out, Op::Add(IntBits::I64, Signedness::Signed), x, a_m1);
        let not = self.op1(out, Op::Not(IntBits::I64), a_m1, i64_ty);
        self.op2(out, Op::And(IntBits::I64), sum, not)
    }
}
