//! Copy → borrow: read a `var` through a borrow instead of a copy.
//!
//! The lowering reads a `var` as `x = begin_borrow_addr slot; y = copy_value x;
//! end_borrow x` and then only reads `y` before destroying it — possibly after
//! threading it through later blocks as a block argument. If nothing writes
//! `slot` while `y` is alive, the copy is redundant: keep borrowing `slot`
//! instead, thread the borrow where `y` went, and end the borrow where `y` was
//! destroyed. That removes a retain/release pair (or a clone) per read and
//! changes no drop timing — the copy's destroy disappears, the original is
//! untouched.
//!
//! The copy's *family* is `y` plus every block param it is forwarded into.
//! Family members may only be read (borrowed, projected, copied), destroyed,
//! or forwarded to other members, and every incoming argument of a member
//! param must itself be a family member. A write to `slot` — under any of the
//! names it is threaded through blocks as — in any family block defeats the
//! plan. Writes through raw pointers created elsewhere are not tracked
//! (`unsafe` code's responsibility).

use rustc_hash::FxHashSet;

use super::TypeFacts;
use crate::ValueId;
use crate::body::OssaBody;
use crate::inst::InstKind;
use crate::mono::types::MonoModule;
use crate::value::Ownership;

/// Upper bound on rewrites per function, guarding against a non-converging scan.
const MAX_PER_FUNCTION: usize = 10_000;

pub(super) fn run(mono: &mut MonoModule, facts: &mut TypeFacts) -> usize {
    let mut total = 0;
    let mut funcs = std::mem::take(&mut mono.functions);
    for func in funcs.iter_mut() {
        let Some(body) = &mut func.body else { continue };
        // Each rewrite shifts indices and changes params, so rescan after each.
        let mut done = 0;
        while done < MAX_PER_FUNCTION
            && let Some(plan) = find_plan(mono, facts, body)
        {
            apply(body, &plan);
            done += 1;
        }
        total += done;
    }
    mono.functions = funcs;
    total
}

struct Plan {
    block: usize,
    copy_i: usize,
    end_i: usize,
    x: ValueId,
    y: ValueId,
    /// Block params the copy is threaded into; they become borrows.
    params: Vec<(usize, ValueId)>,
}

fn find_plan(mono: &MonoModule, facts: &mut TypeFacts, body: &OssaBody) -> Option<Plan> {
    for (bi, block) in body.blocks.iter().enumerate() {
        for (ci, inst) in block.insts.iter().enumerate() {
            let InstKind::CopyValue {
                result: y,
                operand: x,
            } = inst.kind
            else {
                continue;
            };
            if let Some(plan) = check(mono, facts, body, bi, ci, x, y) {
                return Some(plan);
            }
        }
    }
    None
}

fn check(
    mono: &MonoModule,
    facts: &mut TypeFacts,
    body: &OssaBody,
    bi: usize,
    ci: usize,
    x: ValueId,
    y: ValueId,
) -> Option<Plan> {
    let insts = &body.blocks[bi].insts;
    // `x` must be a slot borrow begun earlier in this block whose only other
    // use is its end_borrow after the copy.
    let slot = insts[..ci].iter().find_map(|i| match i.kind {
        InstKind::BeginBorrowAddr {
            result, address, ..
        } if result == x => Some(address),
        _ => None,
    })?;
    let mut end_i = None;
    for (j, inst) in insts.iter().enumerate() {
        if j == ci || !inst.kind.operands().contains(&x) {
            continue;
        }
        match inst.kind {
            InstKind::EndBorrow { operand } if operand == x && j > ci && end_i.is_none() => {
                end_i = Some(j)
            },
            InstKind::BeginBorrowAddr { result, .. } if result == x => {},
            _ => return None,
        }
    }
    if body.blocks[bi].terminator.kind.operands().contains(&x) {
        return None;
    }
    let end_i = end_i?;

    // Grow the family through forwarded block args.
    let mut members: Vec<(usize, ValueId)> = vec![(bi, y)];
    let mut seen: FxHashSet<ValueId> = FxHashSet::from_iter([y]);
    let mut k = 0;
    while k < members.len() {
        let (mb, v) = members[k];
        let start = if v == y { ci + 1 } else { 0 };
        for (target, param) in member_uses(mono, facts, body, mb, start, v)? {
            if target == body.entry.index() || body.value(param).ownership != Ownership::Owned {
                return None;
            }
            if seen.insert(param) {
                members.push((target, param));
            }
        }
        k += 1;
    }
    // Members may only be named in their own block, and every incoming arg of
    // a member param must be a member.
    for (b, block) in body.blocks.iter().enumerate() {
        let in_family = |v: &ValueId| seen.contains(v);
        let foreign_use = block.insts.iter().any(|i| {
            i.kind
                .operands()
                .iter()
                .any(|o| in_family(o) && !members.contains(&(b, *o)))
        });
        if foreign_use {
            return None;
        }
        for (target, args) in block.terminator.kind.successor_args() {
            let params = &body.blocks[target.index()].params;
            for (a, p) in args.iter().zip(params) {
                if seen.contains(&p.value) && !seen.contains(a) {
                    return None;
                }
            }
        }
    }
    // Nothing writes the slot (under any threaded name) while a member is
    // alive: from its definition (block start for params) to its destroy, or
    // to the block end when it is forwarded.
    let aliases = super::storage_names(body, slot);
    // A `&mutating` binding (or any mutable borrow) of the slot that is still
    // open can write it through its own name while the copy would be alive.
    if mut_borrow_open(body, bi, ci, &aliases) {
        return None;
    }
    for &(b, v) in &members {
        let insts = &body.blocks[b].insts;
        let start = if v == y { ci + 1 } else { 0 };
        let end = insts
            .iter()
            .position(|i| matches!(i.kind, InstKind::DestroyValue { operand } if operand == v))
            .unwrap_or(insts.len());
        let writes = insts[start.min(end)..end].iter().any(|i| {
            i.kind.operands().iter().any(|o| aliases.contains(o))
                && !matches!(
                    i.kind,
                    // Reads, and projecting a field address (its own uses are checked).
                    InstKind::BeginBorrowAddr { .. }
                        | InstKind::CopyAddr { .. }
                        | InstKind::Load { .. }
                        | InstKind::FieldAddr { .. }
                )
        });
        if writes {
            return None;
        }
    }
    let params = members.into_iter().filter(|(_, v)| *v != y).collect();
    Some(Plan {
        block: bi,
        copy_i: ci,
        end_i,
        x,
        y,
        params,
    })
}

/// Check how family member `v` is used in its block from `start` on. Returns
/// the (target block, param) slots it is forwarded into, or `None` if any use
/// is not a read, a single destroy, or a forward.
fn member_uses(
    mono: &MonoModule,
    facts: &mut TypeFacts,
    body: &OssaBody,
    b: usize,
    start: usize,
    v: ValueId,
) -> Option<Vec<(usize, ValueId)>> {
    let block = &body.blocks[b];
    let mut derived: FxHashSet<ValueId> = FxHashSet::from_iter([v]);
    let mut destroyed = false;
    for inst in block.insts.iter().skip(start) {
        let ops = inst.kind.operands();
        if !ops.iter().any(|o| derived.contains(o)) {
            continue;
        }
        if destroyed {
            return None; // touched after its destroy
        }
        match inst.kind {
            InstKind::DestroyValue { operand } if operand == v => destroyed = true,
            // Mutating the copy would now mutate the original.
            InstKind::BeginMutBorrow { .. } => return None,
            _ if super::reads_only(&inst.kind) => {},
            // Other instructions may read derived borrows (call args), not `v` itself.
            _ if ops.contains(&v) => return None,
            _ => {},
        }
        for r in inst.kind.results() {
            // A pointer (or view) read out of the value could outlive the borrow.
            if facts.holds_raw_pointer(mono, body.value(r).ty) {
                return None;
            }
            if body.value(r).ownership == Ownership::Guaranteed {
                derived.insert(r);
            }
        }
    }
    let term = &block.terminator.kind;
    if super::terminator_value_use(term).is_some_and(|t| derived.contains(&t)) {
        return None;
    }
    let mut forwards = Vec::new();
    for (target, args) in term.successor_args() {
        for (k, a) in args.iter().enumerate() {
            if *a == v {
                forwards.push((
                    target.index(),
                    body.blocks[target.index()].params.get(k)?.value,
                ));
            } else if derived.contains(a) {
                return None; // a derived borrow escapes the block
            }
        }
    }
    // Leaves the block exactly one way: destroyed here, or forwarded.
    if destroyed == !forwards.is_empty() {
        return None;
    }
    Some(forwards)
}

/// Some mutable borrow of the slot may be live at `ci` in block `bi`: one
/// begun earlier in the block and not yet ended, or one threaded in from
/// another block (a `@guaranteed` param borrowing any slot alias). Safe
/// `&mutating` references alias the slot, so the copy must stay a copy.
fn mut_borrow_open(body: &OssaBody, bi: usize, ci: usize, aliases: &FxHashSet<ValueId>) -> bool {
    let insts = &body.blocks[bi].insts[..ci];
    let open_here = insts.iter().any(|inst| match inst.kind {
        InstKind::BeginMutBorrowAddr { result, address, .. } if aliases.contains(&address) => {
            !insts.iter().any(|e| {
                matches!(e.kind, InstKind::EndBorrow { operand } | InstKind::EndMutBorrow { operand } if operand == result)
            })
        },
        _ => false,
    });
    let threaded_in = body.blocks.iter().flat_map(|b| &b.params).any(|p| {
        p.ownership == Ownership::Guaranteed
            && body
                .value(p.value)
                .borrow_source
                .is_some_and(|src| aliases.contains(&src))
    });
    open_here || threaded_in
}

fn apply(body: &mut OssaBody, plan: &Plan) {
    let Plan {
        block,
        copy_i,
        end_i,
        x,
        y,
        ref params,
    } = *plan;
    // In the copy's block: reads of `y` read `x`; its destroy ends `x`'s borrow;
    // forwarding `y` forwards `x`.
    for inst in body.blocks[block].insts.iter_mut() {
        match &mut inst.kind {
            InstKind::DestroyValue { operand } if *operand == y => {
                inst.kind = InstKind::EndBorrow { operand: x }
            },
            kind => replace_read_operand(kind, y, x),
        }
    }
    for (_, args) in super::successor_args_mut(&mut body.blocks[block].terminator.kind) {
        for a in args.iter_mut().filter(|a| **a == y) {
            *a = x;
        }
    }
    // Threaded params become borrows; their destroys end the borrow.
    for &(b, p) in params {
        for bp in body.blocks[b].params.iter_mut().filter(|bp| bp.value == p) {
            bp.ownership = Ownership::Guaranteed;
        }
        body.values[p.index()].ownership = Ownership::Guaranteed;
        for inst in body.blocks[b].insts.iter_mut() {
            if matches!(inst.kind, InstKind::DestroyValue { operand } if operand == p) {
                inst.kind = InstKind::EndBorrow { operand: p };
            }
        }
    }
    let insts = &mut body.blocks[block].insts;
    insts.remove(end_i);
    insts.remove(copy_i);
}

/// Point a read-only use of `from` at `to`.
fn replace_read_operand(kind: &mut InstKind, from: ValueId, to: ValueId) {
    match kind {
        InstKind::BeginBorrow { operand, .. }
        | InstKind::BeginMutBorrow { operand, .. }
        | InstKind::CopyValue { operand, .. }
        | InstKind::StructExtract { operand, .. }
        | InstKind::TupleExtract { operand, .. }
        | InstKind::EnumPayload { operand, .. }
        | InstKind::Discriminant { operand, .. }
            if *operand == from =>
        {
            *operand = to
        },
        _ => {},
    }
}
