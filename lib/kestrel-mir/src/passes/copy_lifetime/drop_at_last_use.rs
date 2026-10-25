//! Drop each implicit copy right after its last read.
//!
//! For a copy `%y = copy_value %x` defined in block B, the destroy moves to just
//! after the last use of `%y` (or of any `@guaranteed` value derived from it:
//! borrows, field extracts, ref-returning calls) in B. After that point `%y` may
//! only be destroyed in B, or forwarded as a block argument into "dead-carry"
//! params: params whose only uses are their own destroy or further forwarding
//! into dead-carry params. Those params are deleted along with the arguments
//! feeding them, so the copy is destroyed exactly once, at its last use. A
//! param is only deleted when EVERY incoming argument is itself being removed
//! (a planned copy or another dead-carry param), so nothing leaks.
//!
//! Copies of observable types never qualify (see [`TypeFacts::observable`]), and a copy
//! that a raw pointer or view was derived from keeps its scope-end drop.
//!
//! **`let` slots.** A `let` lives at a stack slot (uniform binding lowering),
//! so `let b = a` is `y = copy_value ..; store_init slot, y` and `b`'s drop is
//! the slot's `destroy_addr` at scope exit. A slot that is initialized once
//! with such a copy and afterwards only read (borrowed in place, directly or
//! through field addresses) in its own block gets the same treatment: its
//! `destroy_addr` moves to just after the last read. See [`shorten_let_slots`].

use kestrel_debug::ktrace;
use rustc_hash::{FxHashMap, FxHashSet};

use super::TypeFacts;
use crate::ValueId;
use crate::body::OssaBody;
use crate::inst::{InstKind, Instruction};
use crate::mono::types::MonoModule;
use crate::value::Ownership;

pub(super) fn run(mono: &mut MonoModule, facts: &mut TypeFacts) -> usize {
    let mut total = 0usize;
    // Bodies are mutated while type tables are read: take bodies out first.
    let mut funcs = std::mem::take(&mut mono.functions);
    for func in funcs.iter_mut() {
        let Some(body) = &mut func.body else { continue };
        let n = shorten_in_function(mono, body, facts) + shorten_let_slots(mono, body, facts);
        if n > 0 {
            ktrace!(
                "copy-lifetime",
                "{}: {n} copies dropped at last use",
                func.name
            );
        }
        total += n;
    }
    mono.functions = funcs;
    total
}

struct CopyPlan {
    block: usize,
    value: ValueId,
    /// Index of the last instruction touching the copy's family.
    last_use: usize,
    /// In-block destroy to delete, if the copy is destroyed in its own block.
    destroy: Option<usize>,
    /// (target block, param index) slots the copy is forwarded into.
    forwards: Vec<(usize, usize)>,
}

/// One use of a block param outside plain destroys.
enum ParamUse {
    Destroy(usize, usize),
    Forward(usize, usize),
    Other,
}

fn shorten_in_function(mono: &MonoModule, body: &mut OssaBody, facts: &mut TypeFacts) -> usize {
    let entry = body.entry.index();

    // 1. Candidate copies, analysed within their defining block.
    let mut plans: FxHashMap<ValueId, CopyPlan> = FxHashMap::default();
    for bi in 0..body.blocks.len() {
        for (ci, inst) in body.blocks[bi].insts.iter().enumerate() {
            let InstKind::CopyValue { result: y, .. } = inst.kind else {
                continue;
            };
            if body.value(y).ownership != Ownership::Owned
                || facts.observable(mono, body.value(y).ty)
            {
                continue;
            }
            if let Some(plan) = scan_copy(mono, facts, body, bi, ci, y) {
                plans.insert(y, plan);
            }
        }
    }
    if plans.is_empty() {
        return 0;
    }

    // 2. Dead-carry params: owned, memory-only, used only by destroys/forwards.
    let mut param_uses: FxHashMap<ValueId, Vec<ParamUse>> = FxHashMap::default();
    let mut param_slot: FxHashMap<ValueId, (usize, usize)> = FxHashMap::default();
    for (bi, block) in body.blocks.iter().enumerate() {
        if bi == entry {
            continue;
        }
        for (k, p) in block.params.iter().enumerate() {
            if p.ownership == Ownership::Owned && !facts.observable(mono, p.ty) {
                param_uses.insert(p.value, Vec::new());
                param_slot.insert(p.value, (bi, k));
            }
        }
    }
    // Incoming argument values per (block, param index).
    let mut incoming: FxHashMap<(usize, usize), Vec<ValueId>> = FxHashMap::default();
    for (bi, block) in body.blocks.iter().enumerate() {
        for (ii, inst) in block.insts.iter().enumerate() {
            for op in inst.kind.operands() {
                let Some(uses) = param_uses.get_mut(&op) else {
                    continue;
                };
                let is_destroy =
                    matches!(inst.kind, InstKind::DestroyValue { operand } if operand == op);
                uses.push(if is_destroy {
                    ParamUse::Destroy(bi, ii)
                } else {
                    ParamUse::Other
                });
            }
        }
        let term = &block.terminator.kind;
        let arg_set: FxHashSet<ValueId> = term
            .successor_args()
            .iter()
            .flat_map(|(_, a)| a.iter().copied())
            .collect();
        // Non-argument terminator operands (return value, conditions) are real uses.
        for op in term.operands() {
            let is_arg = arg_set.contains(&op);
            let is_cond = super::terminator_value_use(term) == Some(op);
            if (is_cond || !is_arg)
                && let Some(uses) = param_uses.get_mut(&op)
            {
                uses.push(ParamUse::Other);
            }
        }
        for (target, args) in term.successor_args() {
            for (k, a) in args.iter().enumerate() {
                incoming.entry((target.index(), k)).or_default().push(*a);
                if let Some(uses) = param_uses.get_mut(a) {
                    uses.push(ParamUse::Forward(target.index(), k));
                }
            }
        }
    }
    let slot_value = |(b, k): (usize, usize)| body.blocks[b].params[k].value;
    let mut dead: FxHashSet<ValueId> = param_uses
        .iter()
        .filter(|(_, uses)| !uses.iter().any(|u| matches!(u, ParamUse::Other)))
        .map(|(v, _)| *v)
        .collect();

    // 3. Fixed point: a param stays dead only if all forwards go to dead params
    //    and all incoming args are removable; a copy stays planned only if all
    //    its forwards go to dead params.
    loop {
        let mut changed = false;
        let dead_now = dead.clone();
        for p in dead_now {
            let fwd_ok = param_uses[&p].iter().all(|u| match u {
                ParamUse::Forward(b, k) => dead.contains(&slot_value((*b, *k))),
                _ => true,
            });
            let in_ok = incoming
                .get(&param_slot[&p])
                .is_none_or(|vs| vs.iter().all(|v| dead.contains(v) || plans.contains_key(v)));
            if !fwd_ok || !in_ok {
                dead.remove(&p);
                changed = true;
            }
        }
        let bad: Vec<ValueId> = plans
            .iter()
            .filter(|(_, pl)| pl.forwards.iter().any(|s| !dead.contains(&slot_value(*s))))
            .map(|(v, _)| *v)
            .collect();
        for v in bad {
            plans.remove(&v);
            changed = true;
        }
        if !changed {
            break;
        }
    }
    // Skip no-op plans (already destroyed right after the last use).
    plans.retain(|_, pl| !(pl.forwards.is_empty() && pl.destroy == Some(pl.last_use + 1)));
    if plans.is_empty() {
        return 0;
    }
    apply(body, &plans, &dead, &param_uses, &param_slot)
}

fn apply(
    body: &mut OssaBody,
    plans: &FxHashMap<ValueId, CopyPlan>,
    dead: &FxHashSet<ValueId>,
    param_uses: &FxHashMap<ValueId, Vec<ParamUse>>,
    param_slot: &FxHashMap<ValueId, (usize, usize)>,
) -> usize {
    // Instructions to delete and destroys to insert, per block.
    let mut delete: FxHashMap<usize, FxHashSet<usize>> = FxHashMap::default();
    let mut insert_after: FxHashMap<usize, FxHashMap<usize, Vec<ValueId>>> = FxHashMap::default();
    for pl in plans.values() {
        if let Some(d) = pl.destroy {
            delete.entry(pl.block).or_default().insert(d);
        }
        insert_after
            .entry(pl.block)
            .or_default()
            .entry(pl.last_use)
            .or_default()
            .push(pl.value);
    }
    for p in dead {
        for u in &param_uses[p] {
            if let ParamUse::Destroy(b, i) = u {
                delete.entry(*b).or_default().insert(*i);
            }
        }
    }
    // Removed param indices per block.
    let mut removed: FxHashMap<usize, FxHashSet<usize>> = FxHashMap::default();
    for p in dead {
        let (b, k) = param_slot[p];
        removed.entry(b).or_default().insert(k);
    }

    for bi in 0..body.blocks.len() {
        let del = delete.remove(&bi).unwrap_or_default();
        let ins = insert_after.remove(&bi).unwrap_or_default();
        if !del.is_empty() || !ins.is_empty() {
            let old = std::mem::take(&mut body.blocks[bi].insts);
            let mut new_insts = Vec::with_capacity(old.len());
            for (i, inst) in old.into_iter().enumerate() {
                let span = inst.span.clone();
                if !del.contains(&i) {
                    new_insts.push(inst);
                }
                for &v in ins.get(&i).into_iter().flatten() {
                    new_insts.push(Instruction {
                        kind: InstKind::DestroyValue { operand: v },
                        span: span.clone(),
                    });
                }
            }
            body.blocks[bi].insts = new_insts;
        }
        // Drop terminator args feeding removed params.
        for (target, args) in super::successor_args_mut(&mut body.blocks[bi].terminator.kind) {
            if let Some(ks) = removed.get(&target) {
                let mut k = 0;
                args.retain(|_| {
                    let keep = !ks.contains(&k);
                    k += 1;
                    keep
                });
            }
        }
    }
    for (b, ks) in &removed {
        let mut k = 0;
        body.blocks[*b].params.retain(|_| {
            let keep = !ks.contains(&k);
            k += 1;
            keep
        });
    }
    plans.len()
}

/// Track `y` and its derived borrows through its defining block.
fn scan_copy(
    mono: &MonoModule,
    facts: &mut TypeFacts,
    body: &OssaBody,
    bi: usize,
    ci: usize,
    y: ValueId,
) -> Option<CopyPlan> {
    let block = &body.blocks[bi];
    let mut family: FxHashSet<ValueId> = FxHashSet::from_iter([y]);
    let mut destroy = None;
    let mut last_use = ci;

    for (j, inst) in block.insts.iter().enumerate().skip(ci + 1) {
        if !inst.kind.operands().iter().any(|o| family.contains(o)) {
            continue;
        }
        if let InstKind::DestroyValue { operand } = inst.kind
            && operand == y
        {
            if destroy.is_some() {
                return None;
            }
            destroy = Some(j);
            continue;
        }
        // Anything after the destroy that still touches the family is unsafe.
        // Other instructions may take derived borrows (call args), but not `y` by value.
        if destroy.is_some()
            || (!super::reads_only(&inst.kind) && inst.kind.operands().contains(&y))
        {
            return None;
        }
        last_use = j;
        for r in inst.kind.results() {
            let def = body.value(r);
            // A pointer (or view) derived from the copy may outlive it: bail.
            if facts.holds_raw_pointer(mono, def.ty) {
                return None;
            }
            if def.ownership == Ownership::Guaranteed {
                family.insert(r);
            }
        }
    }

    // Terminator: only plain forwarding of `y` itself is allowed.
    let term = &block.terminator.kind;
    let mut forwards = Vec::new();
    for (target, args) in term.successor_args() {
        for (k, a) in args.iter().enumerate() {
            if *a == y {
                forwards.push((target.index(), k));
            } else if family.contains(a) {
                return None;
            }
        }
    }
    // Used as a condition, discriminant or return value: a real use, not a forward.
    if super::terminator_value_use(term).is_some_and(|v| family.contains(&v)) {
        return None;
    }
    if destroy.is_some() != forwards.is_empty() {
        return None; // must leave the block exactly one way
    }
    Some(CopyPlan {
        block: bi,
        value: y,
        last_use,
        destroy,
        forwards,
    })
}

/// Move the `destroy_addr` of each `let` slot holding an implicit copy to just
/// after the slot's last read, within its block (see the module doc).
///
/// A slot qualifies when, in its defining block and nowhere else:
/// - it is initialized by exactly one `store_init` whose value is a
///   `copy_value` (an implicit copy; the original keeps its scope-end drop);
/// - every other use of it, or of a field address projected from it, is a
///   `begin_borrow_addr` (an in-place read), a further `field_addr`, a
///   trivial `destroy_value` of the address, or its single `destroy_addr`;
/// - no borrow derived from it yields a raw pointer or view (that could
///   outlive the move), and no borrow or address leaves the block;
/// - its type is not observable.
fn shorten_let_slots(mono: &MonoModule, body: &mut OssaBody, facts: &mut TypeFacts) -> usize {
    let mut moved = 0;
    for bi in 0..body.blocks.len() {
        let slots: Vec<(ValueId, usize)> = body.blocks[bi]
            .insts
            .iter()
            .enumerate()
            .filter_map(|(i, inst)| match inst.kind {
                InstKind::Uninit { result, .. } => Some((result, i)),
                _ => None,
            })
            .collect();
        for (slot, def_i) in slots {
            if let Some((destroy_i, last_use)) = scan_let_slot(mono, facts, body, bi, slot, def_i) {
                let insts = &mut body.blocks[bi].insts;
                let destroy = insts.remove(destroy_i);
                insts.insert(last_use + 1, destroy);
                moved += 1;
            }
        }
    }
    moved
}

/// The (destroy index, last-read index) of a qualifying slot, or `None`.
fn scan_let_slot(
    mono: &MonoModule,
    facts: &mut TypeFacts,
    body: &OssaBody,
    bi: usize,
    slot: ValueId,
    def_i: usize,
) -> Option<(usize, usize)> {
    // Every use of the slot's addresses must be in this block.
    let mut addrs: FxHashSet<ValueId> = FxHashSet::from_iter([slot]);
    for (b, block) in body.blocks.iter().enumerate() {
        let term_uses = block.terminator.kind.operands();
        if b != bi
            && block
                .insts
                .iter()
                .any(|i| i.kind.operands().contains(&slot))
        {
            return None;
        }
        if term_uses.contains(&slot) {
            return None;
        }
    }
    let block = &body.blocks[bi];
    let mut family: FxHashSet<ValueId> = FxHashSet::default(); // borrows and their projections
    let mut store = None;
    let mut destroy = None;
    let mut last_use = def_i;
    for (j, inst) in block.insts.iter().enumerate().skip(def_i + 1) {
        let ops = inst.kind.operands();
        let touches_addr = ops.iter().any(|o| addrs.contains(o));
        let touches_family = ops.iter().any(|o| family.contains(o));
        if !touches_addr && !touches_family {
            continue;
        }
        match inst.kind {
            InstKind::StoreInit { address, value } if address == slot && store.is_none() => {
                let is_copy = block.insts[..j].iter().any(
                    |d| matches!(d.kind, InstKind::CopyValue { result, .. } if result == value),
                );
                if !is_copy || facts.observable(mono, body.value(value).ty) {
                    return None;
                }
                store = Some(j);
                // An unread copy drops right after it is stored, never before.
                last_use = j;
                continue;
            },
            InstKind::DestroyAddr { address, .. } if address == slot && destroy.is_none() => {
                destroy = Some(j);
                continue;
            },
            // Dropping an address value is free and reads nothing.
            InstKind::DestroyValue { operand } if addrs.contains(&operand) => continue,
            InstKind::FieldAddr { result, base, .. } if addrs.contains(&base) => {
                addrs.insert(result);
                continue;
            },
            InstKind::BeginBorrowAddr {
                result, address, ..
            } if addrs.contains(&address) => {
                family.insert(result);
            },
            _ if touches_addr => return None, // a write, take, or escape of the address
            _ if !super::reads_only(&inst.kind)
                && !matches!(
                    inst.kind,
                    InstKind::Call { .. } | InstKind::EndBorrow { .. }
                ) =>
            {
                return None;
            },
            _ => {},
        }
        if store.is_none() || destroy.is_some() {
            return None; // read before init, or after the drop
        }
        last_use = j;
        for r in inst.kind.results() {
            let def = body.value(r);
            if facts.holds_raw_pointer(mono, def.ty) {
                return None;
            }
            if def.ownership == Ownership::Guaranteed {
                family.insert(r);
            }
        }
    }
    let destroy_i = destroy?;
    store?;
    // A borrow (or projection) carried out of the block outlives the move.
    let term = &block.terminator.kind;
    if term
        .operands()
        .iter()
        .any(|o| family.contains(o) || addrs.contains(o))
    {
        return None;
    }
    (last_use < destroy_i).then_some((destroy_i, last_use))
}
