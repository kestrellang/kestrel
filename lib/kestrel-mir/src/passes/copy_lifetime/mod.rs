//! Copy lifetime optimization: stop implicit copies from keeping storage shared.
//!
//! The lowering copies every `@owned` read (`emit_value_use`) and drops each
//! copy at the end of its scope. For copy-on-write types (`String`, `Array`,
//! `Dictionary`, ...) a copy that outlives its last read keeps the refcount at
//! 2, so the next mutation of the original deep-copies the whole buffer, and
//! for deep-copy types (`indirect` enums) the copy itself is the cost. These
//! passes remove or shorten such copies. Explicit `.clone()` calls are never
//! touched, and values whose drop is observable keep their scope-end drops, so
//! RAII guards, files and locks behave exactly as written:
//!
//! 1. [`owned_params`] — copy → move. When a call borrows a value at its last
//!    use and the callee only copies that parameter, call a variant that takes
//!    it owned, so the value is moved through instead of copied.
//! 2. [`last_use_move`] — copy → move for locals. A by-value read of a
//!    `let`/`var` slot that nothing reads again before the slot is destroyed or
//!    overwritten takes the value out instead of cloning it.
//! 3. [`drop_at_last_use`] — drop each remaining copy right after its last
//!    read, including copies threaded through later blocks.
//! 4. [`borrow_copies`] — copy → borrow. A copy of a `var` that is only read,
//!    while nothing writes the `var`, is replaced by a longer borrow.
//!
//! (2) removes the original's drop and (3) moves a drop, so both check
//! [`TypeFacts::observable`]: values of a type marked `@observabledrop`, or
//! containing one, keep their copy and scope-end drop. (1) hands the value
//! over and (4) removes the copy entirely, so no drop moves. (3) and (4) never
//! touch a copy that a raw pointer (or a struct holding one, like a string
//! view) was derived from; (2) never touches a slot whose address escapes. Writes through raw pointers created
//! elsewhere are not tracked: that is `unsafe` territory by design.
//!
//! Runs on the monomorphized module after `copy_propagation`, before
//! `expand_destroy_copy` turns `CopyValue`/`DestroyValue` into clone/drop calls.

mod borrow_copies;
mod drop_at_last_use;
mod last_use_move;
mod observable_lint;
mod owned_params;
mod type_facts;

use kestrel_debug::ktrace;

use rustc_hash::FxHashSet;

use crate::ValueId;
use crate::body::OssaBody;
use crate::inst::InstKind;
use crate::mono::types::MonoModule;
use crate::terminator::TerminatorKind;

pub use observable_lint::{ObservableDropFinding, unmarked_observable_drops};
pub use type_facts::TypeFacts;

/// Run all four copy lifetime passes.
pub fn run(mono: &mut MonoModule) {
    let moved = owned_params::run(mono);
    let mut facts = TypeFacts::default();
    // After owned_params, so the `$own` variants it adds are covered too.
    let slot_moves = last_use_move::run(mono, &mut facts);
    let dropped = drop_at_last_use::run(mono, &mut facts);
    let borrowed = borrow_copies::run(mono, &mut facts);
    ktrace!(
        "copy-lifetime",
        "{moved} borrowed args moved, {slot_moves} slot reads moved, {dropped} copies dropped at last use, {borrowed} copies turned into borrows"
    );
}

/// Mutable view of each (successor block, args) edge of a terminator.
fn successor_args_mut(kind: &mut TerminatorKind) -> Vec<(usize, &mut Vec<ValueId>)> {
    match kind {
        TerminatorKind::Jump { target, args } => vec![(target.index(), args)],
        TerminatorKind::Branch {
            then_block,
            then_args,
            else_block,
            else_args,
            ..
        } => {
            vec![
                (then_block.index(), then_args),
                (else_block.index(), else_args),
            ]
        },
        TerminatorKind::Switch { cases, .. } => cases
            .iter_mut()
            .map(|arm| (arm.target.index(), &mut arm.args))
            .collect(),
        TerminatorKind::Return(_) | TerminatorKind::Panic(_) | TerminatorKind::Unreachable => {
            Vec::new()
        },
    }
}

/// The non-argument operand of a terminator (condition, discriminant or
/// returned value): a real use, unlike forwarding a value as a block argument.
fn terminator_value_use(kind: &TerminatorKind) -> Option<ValueId> {
    match kind {
        TerminatorKind::Branch { condition, .. } => Some(*condition),
        TerminatorKind::Switch { discriminant, .. } => Some(*discriminant),
        TerminatorKind::Return(v) => Some(*v),
        _ => None,
    }
}

/// Instructions that only read `v` (borrow it, project from it, copy it). Any
/// other instruction naming `v` is treated as taking it by value.
fn reads_only(kind: &InstKind) -> bool {
    matches!(
        kind,
        InstKind::BeginBorrow { .. }
            | InstKind::BeginMutBorrow { .. }
            | InstKind::EndBorrow { .. }
            | InstKind::EndMutBorrow { .. }
            | InstKind::CopyValue { .. }
            | InstKind::StructExtract { .. }
            | InstKind::TupleExtract { .. }
            | InstKind::EnumPayload { .. }
            | InstKind::Discriminant { .. }
    )
}

/// Every name of the storage object `slot` lives in: `slot` itself, the base
/// it was projected from (`field_addr`), every field address projected from
/// any of those, and every block param they are threaded through (both
/// directions). A write through *any* of these may change what `slot` holds —
/// e.g. `let start = self.byteIndex` reads one `field_addr self`, while
/// `self.byteIndex = ..` writes through a different `field_addr self`.
/// Deliberately conservative: writes to sibling fields count too.
fn storage_names(body: &OssaBody, slot: ValueId) -> FxHashSet<ValueId> {
    let mut aliases = FxHashSet::from_iter([slot]);
    loop {
        let before = aliases.len();
        for block in &body.blocks {
            for inst in &block.insts {
                if let InstKind::FieldAddr { result, base, .. } = inst.kind
                    && (aliases.contains(&result) || aliases.contains(&base))
                {
                    aliases.insert(result);
                    aliases.insert(base);
                }
            }
            for (target, args) in block.terminator.kind.successor_args() {
                for (a, p) in args.iter().zip(&body.blocks[target.index()].params) {
                    if aliases.contains(a) || aliases.contains(&p.value) {
                        aliases.insert(*a);
                        aliases.insert(p.value);
                    }
                }
            }
        }
        if aliases.len() == before {
            return aliases;
        }
    }
}
