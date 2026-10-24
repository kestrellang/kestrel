//! Copy lifetime optimization: stop implicit copies from keeping storage shared.
//!
//! The lowering copies every `@owned` read (`emit_value_use`) and drops each
//! copy at the end of its scope. For copy-on-write types (`String`, `Array`,
//! `Dictionary`, ...) a copy that outlives its last read keeps the refcount at
//! 2, so the next mutation of the original deep-copies the whole buffer. These
//! passes remove or shorten such copies without changing when any *original*
//! is dropped (originals and explicit `.clone()` calls keep scope-end drops, so
//! RAII guards, files and locks behave exactly as written):
//!
//! 1. [`owned_params`] — copy → move. When a call borrows a value at its last
//!    use and the callee only copies that parameter, call a variant that takes
//!    it owned, so the value is moved through instead of copied.
//! 2. [`drop_at_last_use`] — drop each remaining copy right after its last
//!    read, including copies threaded through later blocks.
//! 3. [`borrow_copies`] — copy → borrow. A copy of a `var` that is only read,
//!    while nothing writes the `var`, is replaced by a longer borrow.
//!
//! Only (2) moves a drop, so only it checks [`TypeFacts::observable`]: copies
//! of a type marked `@observabledrop`, or containing one, keep their scope-end
//! drop. (1) hands the value over and (3) removes the copy entirely, so no
//! drop moves. (2) and (3) never touch a copy that a raw pointer (or a struct
//! holding one, like a string view) was derived from. Writes through raw pointers created
//! elsewhere are not tracked: that is `unsafe` territory by design.
//!
//! Runs on the monomorphized module after `copy_propagation`, before
//! `expand_destroy_copy` turns `CopyValue`/`DestroyValue` into clone/drop calls.

mod borrow_copies;
mod drop_at_last_use;
mod owned_params;
mod type_facts;

use kestrel_debug::ktrace;

use crate::ValueId;
use crate::inst::InstKind;
use crate::mono::types::MonoModule;
use crate::terminator::TerminatorKind;

pub use type_facts::TypeFacts;

/// Run all three copy lifetime passes.
pub fn run(mono: &mut MonoModule) {
    let moved = owned_params::run(mono);
    let mut facts = TypeFacts::default();
    let dropped = drop_at_last_use::run(mono, &mut facts);
    let borrowed = borrow_copies::run(mono, &mut facts);
    ktrace!(
        "copy-lifetime",
        "{moved} borrowed args moved, {dropped} copies dropped at last use, {borrowed} copies turned into borrows"
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
