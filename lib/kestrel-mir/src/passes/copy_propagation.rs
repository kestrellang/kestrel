use crate::ValueId;
#[allow(unused_imports)]
use crate::body::OssaBody;
use crate::inst::InstKind;
use crate::mono::types::MonoModule;
use crate::op::Op;
use crate::value::Ownership;
use rustc_hash::{FxHashMap, FxHashSet};

/// Eliminate redundant CopyValue+DestroyValue pairs on monomorphized bodies,
/// before mono expand turns them into clone/drop calls. When the operand's
/// only remaining use after the copy is its destruction, both are deleted
/// and the copy result is remapped to the original.
pub fn eliminate_redundant_copies(mono: &mut MonoModule) {
    let debug = std::env::var("KESTREL_DEBUG_COPYPROP").is_ok();
    let limit: usize = std::env::var("KESTREL_COPYPROP_LIMIT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(usize::MAX);
    let mut total = 0usize;

    for func in mono.functions.iter_mut() {
        if total >= limit {
            break;
        }
        let Some(body) = &mut func.body else { continue };
        if body.blocks.is_empty() {
            continue;
        }

        let mut func_total = 0usize;
        for block_idx in 0..body.blocks.len() {
            if total + func_total >= limit {
                break;
            }
            func_total += optimize_block(body, block_idx);
        }
        if debug && func_total > 0 {
            eprintln!(
                "[copy_prop] {}: {func_total} copy+destroy pairs eliminated",
                func.name
            );
        }
        total += func_total;
    }
    if debug {
        eprintln!("[copy_prop] total: {total} copy+destroy pairs eliminated (limit: {limit})");
    }
}

fn optimize_block(body: &mut OssaBody, block_idx: usize) -> usize {
    let block = &body.blocks[block_idx];
    let insts = &block.insts;
    if insts.is_empty() {
        return 0;
    }

    // Build use map: for each ValueId, instruction indices where it's an operand.
    let mut uses: FxHashMap<ValueId, Vec<usize>> = FxHashMap::default();
    for (i, inst) in insts.iter().enumerate() {
        for op in inst.kind.operands() {
            uses.entry(op).or_default().push(i);
        }
    }
    let terminator_uses: FxHashSet<ValueId> = block.terminator.kind.operands().into_iter().collect();

    // Forward scan: track active borrows at each instruction index.
    let mut frozen: FxHashMap<ValueId, u32> = FxHashMap::default();
    let mut borrow_source_map: FxHashMap<ValueId, ValueId> = FxHashMap::default();
    let mut frozen_at: Vec<FxHashSet<ValueId>> = Vec::with_capacity(insts.len());

    for inst in insts {
        frozen_at.push(frozen.keys().filter(|k| frozen[k] > 0).copied().collect());
        match &inst.kind {
            InstKind::BeginBorrow { result, operand }
            | InstKind::BeginMutBorrow { result, operand } => {
                let src = body.value(*result).borrow_source.unwrap_or(*operand);
                *frozen.entry(src).or_default() += 1;
                borrow_source_map.insert(*result, src);
            },
            InstKind::EndBorrow { operand } | InstKind::EndMutBorrow { operand } => {
                if let Some(&src) = borrow_source_map.get(operand)
                    && let Some(count) = frozen.get_mut(&src)
                {
                    *count = count.saturating_sub(1);
                }
            },
            _ => {},
        }
    }

    // Find CopyValue+DestroyValue pairs to eliminate.
    let mut replace_with_move: FxHashSet<usize> = FxHashSet::default();
    let mut delete_indices: FxHashSet<usize> = FxHashSet::default();
    let mut claimed: FxHashSet<ValueId> = FxHashSet::default();

    for (i, inst) in insts.iter().enumerate() {
        let InstKind::CopyValue { result, operand } = &inst.kind else {
            continue;
        };
        let x = *operand;
        let _y = *result;

        if body.value(x).ownership != Ownership::Owned {
            continue;
        }
        if terminator_uses.contains(&x) {
            continue;
        }
        if claimed.contains(&x) {
            continue;
        }
        if frozen_at.get(i).is_some_and(|f| f.contains(&x)) {
            continue;
        }

        // Only remaining use of x after the CopyValue must be a single DestroyValue.
        let remaining: Vec<usize> = uses
            .get(&x)
            .map(|u| u.iter().copied().filter(|&idx| idx > i).collect())
            .unwrap_or_default();

        if remaining.len() == 1 {
            let j = remaining[0];
            if matches!(insts[j].kind, InstKind::DestroyValue { operand } if operand == x) {
                // Convert CopyValue → MoveValue and delete DestroyValue.
                // MoveValue consumes %x and produces %y — mono expand
                // won't expand it to a clone call (only CopyValue is expanded).
                replace_with_move.insert(i);
                delete_indices.insert(j);
                claimed.insert(x);
            }
        }
    }

    if replace_with_move.is_empty() {
        return 0;
    }
    let eliminated = replace_with_move.len();

    // Rebuild: convert CopyValue→MoveValue, delete DestroyValue.
    let old_insts = std::mem::take(&mut body.blocks[block_idx].insts);
    let mut new_insts = Vec::with_capacity(old_insts.len());
    for (idx, inst) in old_insts.into_iter().enumerate() {
        if delete_indices.contains(&idx) {
            continue;
        }
        if replace_with_move.contains(&idx)
            && let InstKind::CopyValue { result, operand } = &inst.kind
        {
            new_insts.push(crate::inst::Instruction {
                kind: InstKind::MoveValue {
                    result: *result,
                    operand: *operand,
                },
                span: inst.span,
            });
            continue;
        }
        new_insts.push(inst);
    }
    body.blocks[block_idx].insts = new_insts;

    eliminated
}

// ============================================================================
// mark_independent_takes
// ============================================================================

/// One step of address provenance toward the underlying slot a pointer denotes.
enum Prov {
    /// `result` denotes the same slot as `inner` (a `PtrTo`/`FieldAddr` view).
    Forward(ValueId),
    /// `result` IS a freshly-allocated slot (`StackAlloc`/`Uninit`) — provably
    /// distinct storage we can reason about.
    Slot,
}

/// Flip aggregate `Take`s from independent (memcpy) to aliasing (zero-copy)
/// where provably safe. A `Take` is a destructive move-out; codegen memcpys an
/// aggregate into a fresh slot so the moved value is independent of its source
/// (the #141 fix). That copy is only needed when the source slot is
/// re-initialized while the moved value is still live — otherwise the move can
/// alias the source storage for free.
///
/// Block-local and deliberately conservative — anything it cannot prove safe
/// stays `independent: true` (copy). A missed alias is a perf nit; a wrong
/// alias re-introduces the #141 double-free. We alias a `Take` only when:
///   1. its source address roots to a known local slot (`StackAlloc`/`Uninit`);
///      an inout/param-rooted source (e.g. `mutating self` in
///      `Optional.take()`/`replace()`) is `None` here → kept as copy;
///   2. the result does NOT escape the block (not in the terminator's
///      operands) — a value carried to a successor or returned must be
///      independent of any local stack slot;
///   3. no later `StoreInit`/`StoreAssign` to the same slot in this block
///      happens while the result is still live (the move-out-then-reinit
///      hazard, e.g. `var x = agg; let y = x; x = new`).
///
/// Runs at the CopyProp stage (post-mono, pre-expand), so the expand-generated
/// drop-capture `Take`s are not visited and keep their safe default.
pub fn mark_independent_takes(mono: &mut MonoModule) {
    let debug = std::env::var("KESTREL_DEBUG_TAKEALIAS").is_ok();
    let mut total = 0usize;

    for func in mono.functions.iter_mut() {
        let Some(body) = &mut func.body else { continue };
        if body.blocks.is_empty() {
            continue;
        }

        // Whole-body provenance map (SSA: each value defined once). Only
        // slot-rooting forms are recorded; everything else (Load/Call results,
        // params, block params) is absent → `root` returns `None`.
        let mut prov: FxHashMap<ValueId, Prov> = FxHashMap::default();
        for block in &body.blocks {
            for inst in &block.insts {
                match &inst.kind {
                    InstKind::Op1 {
                        result,
                        op: Op::StackAlloc(_),
                        ..
                    }
                    | InstKind::Uninit { result, .. } => {
                        prov.insert(*result, Prov::Slot);
                    },
                    InstKind::Op1 {
                        result,
                        op: Op::PtrTo(_),
                        arg,
                    } => {
                        prov.insert(*result, Prov::Forward(*arg));
                    },
                    InstKind::FieldAddr { result, base, .. } => {
                        prov.insert(*result, Prov::Forward(*base));
                    },
                    _ => {},
                }
            }
        }

        let mut func_total = 0usize;
        for block_idx in 0..body.blocks.len() {
            func_total += mark_block_takes(body, block_idx, &prov);
        }
        if debug && func_total > 0 {
            eprintln!("[take_alias] {}: {func_total} Take(s) marked aliasable", func.name);
        }
        total += func_total;
    }
    if debug {
        eprintln!("[take_alias] total: {total} Take(s) marked aliasable");
    }
}

/// Is `kind` an alias-SAFE consumer of an @owned aggregate value — one that
/// settles (copies/consumes) the value's bytes at that instruction, rather than
/// a zero-copy forwarder that propagates the value's storage alias to a
/// longer-lived SSA value?
///
/// This is a default-DENY allowlist. The hazard the alias optimization must
/// avoid is a `Take` result whose *storage alias* outlives a re-initialization
/// of its source slot. `mark_block_takes` checks liveness of the take result
/// `V` directly, but a forwarder like `MoveValue` (a pure rename — codegen
/// emits no copy for aggregates), a field/payload extract (a sub-address into
/// `V`), or a `BeginBorrow` (a @guaranteed view of `V`) hands `V`'s alias to a
/// new value `W` that can be read *after* a same-slot reinit — clobbering it.
/// So we only alias a `Take` when every use of its result is one of these
/// settle points; anything else (forwarders, or ops we haven't proven safe)
/// keeps the `Take` independent (copy).
fn is_alias_safe_consumer(kind: &InstKind) -> bool {
    matches!(
        kind,
        // Consumed by a call (borrowed/moved into the callee for the duration
        // of the call — `V`'s last in-function use).
        InstKind::Call { .. }
        // Copied byte-for-byte into another slot / a new aggregate.
        | InstKind::StoreInit { .. }
        | InstKind::StoreAssign { .. }
        | InstKind::Struct { .. }
        | InstKind::Tuple { .. }
        | InstKind::Enum { .. }
        | InstKind::Array { .. }
        // Cloned (a real copy-point; expand turns it into clone()).
        | InstKind::CopyValue { .. }
        // Dropped (reads the bytes to run deinit, then frees — terminal).
        | InstKind::DestroyValue { .. }
    )
}

/// Follow `PtrTo`/`FieldAddr` views to the underlying slot. Returns `Some(slot)`
/// only when the address bottoms out at a `StackAlloc`/`Uninit` (a local slot we
/// can reason about); `None` for inout/param-rooted or opaque pointers, which we
/// must treat conservatively (cannot prove non-aliasing).
fn root(addr: ValueId, prov: &FxHashMap<ValueId, Prov>) -> Option<ValueId> {
    let mut cur = addr;
    for _ in 0..64 {
        match prov.get(&cur) {
            Some(Prov::Slot) => return Some(cur),
            Some(Prov::Forward(next)) => cur = *next,
            None => return None,
        }
    }
    None
}

fn mark_block_takes(
    body: &mut OssaBody,
    block_idx: usize,
    prov: &FxHashMap<ValueId, Prov>,
) -> usize {
    let block = &body.blocks[block_idx];
    if block.insts.is_empty() {
        return 0;
    }

    // In-block uses of each value (operand positions).
    let mut uses: FxHashMap<ValueId, Vec<usize>> = FxHashMap::default();
    for (i, inst) in block.insts.iter().enumerate() {
        for op in inst.kind.operands() {
            uses.entry(op).or_default().push(i);
        }
    }
    let term_uses: FxHashSet<ValueId> =
        block.terminator.kind.operands().into_iter().collect();

    // Reinitialization sites in this block: (index, slot the store targets).
    let mut stores: Vec<(usize, Option<ValueId>)> = Vec::new();
    for (j, inst) in block.insts.iter().enumerate() {
        match &inst.kind {
            InstKind::StoreInit { address, .. } | InstKind::StoreAssign { address, .. } => {
                stores.push((j, root(*address, prov)));
            },
            _ => {},
        }
    }

    // Decide (immutable phase), then flip (mutable phase).
    let mut to_alias: Vec<usize> = Vec::new();
    for (i, inst) in block.insts.iter().enumerate() {
        let InstKind::Take {
            result,
            address,
            independent,
            ..
        } = &inst.kind
        else {
            continue;
        };
        if !*independent {
            continue;
        }
        let v = *result;
        // (2) result must not escape the block.
        if term_uses.contains(&v) {
            continue;
        }
        // (1) source must root to a known local slot.
        let Some(r) = root(*address, prov) else {
            continue;
        };
        // `None` here means `v` has no in-block use and (by the check above) does
        // not escape via the terminator — a dead move result. Aliasing it is
        // harmless (nothing reads it); a genuine leak would be caught by
        // `verify_mono` independently.
        let v_uses = uses.get(&v);
        // (2b) every use of `v` must be an alias-safe consumer — no zero-copy
        // forwarder (MoveValue / extract / borrow) that would carry `v`'s
        // storage alias past a same-slot reinit. Default-deny.
        if v_uses.is_some_and(|us| {
            us.iter()
                .any(|&k| !is_alias_safe_consumer(&block.insts[k].kind))
        }) {
            continue;
        }
        // (3) no reinit of slot `r` while `v` is still live in this block.
        let v_last_use = v_uses.and_then(|u| u.iter().copied().filter(|&k| k > i).max());
        let conflict = stores.iter().any(|&(j, sr)| {
            j > i && sr == Some(r) && v_last_use.is_some_and(|k| k >= j)
        });
        if !conflict {
            to_alias.push(i);
        }
    }

    for &i in &to_alias {
        if let InstKind::Take { independent, .. } = &mut body.blocks[block_idx].insts[i].kind {
            *independent = false;
        }
    }
    to_alias.len()
}
