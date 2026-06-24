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
// eliminate_cross_block_copies
// ============================================================================

/// Cross-block extension of [`eliminate_redundant_copies`]: collapse a
/// `CopyValue X → Y` whose operand `X` is then THREADED (via a `Jump`) into a
/// SINGLE-PREDECESSOR successor block where the matching block param is ONLY
/// destroyed. The block-local pass bails on such a copy because `X` is used in
/// the terminator (the jump arg); but when the value merely rides one edge to a
/// block that drops it, the copy + thread + drop IS the redundant copy+destroy
/// pair, just split across the edge.
///
/// This matters for a `consuming` param of a MONO-DEPENDENT type (a bare type
/// param): the lowering copies it (deferring cleanup to copy-prop), and at mono
/// a Cloneable instantiation expands the SURVIVING `CopyValue` into a real
/// CLONE while the threaded original is still dropped — double-freeing the
/// cloned-in resource for a non-Copyable element. #127: `[Res(..)]` →
/// `Array.init` → `CowBox`/`RcBox.init(consuming value)` cloned the consuming
/// `ArrayStorage` value into the heap and dropped the original (deferred to the
/// post-`if let` block), so each element deinited twice.
///
/// Safe because the successor `S` has exactly one predecessor (this block), so
/// removing its param breaks no other edge; `X`'s only use in this block is the
/// copy (a borrow would count as a use); `P`'s only use in `S` is the single
/// `DestroyValue`. Converting `CopyValue → MoveValue` consumes `X`; the jump
/// arg, the now-unused param, and the destroy are removed together, keeping
/// args and params aligned.
pub fn eliminate_cross_block_copies(mono: &mut MonoModule) {
    let debug = std::env::var("KESTREL_DEBUG_COPYPROP").is_ok();
    let mut total = 0usize;
    for func in mono.functions.iter_mut() {
        let Some(body) = &mut func.body else { continue };
        if body.blocks.len() < 2 {
            continue;
        }
        total += optimize_cross_block(body);
    }
    if debug && total > 0 {
        eprintln!("[copy_prop] cross-block: {total} copies converted to moves");
    }
}

struct CrossRewrite {
    /// Block holding the `CopyValue`.
    b: usize,
    /// Index of the `CopyValue` in `b`'s insts.
    copy_i: usize,
    /// Single-predecessor successor receiving the threaded operand.
    s: usize,
    /// Arg/param position of the threaded operand (jump arg index == param index).
    k: usize,
    /// Index of the sole `DestroyValue` of the param in `s`'s insts.
    destroy_j: usize,
}

fn optimize_cross_block(body: &mut OssaBody) -> usize {
    use crate::terminator::TerminatorKind;

    // Predecessor count per block (by successor index).
    let mut pred_count = vec![0u32; body.blocks.len()];
    for block in &body.blocks {
        for succ in block.terminator.kind.successors() {
            if succ.index() < pred_count.len() {
                pred_count[succ.index()] += 1;
            }
        }
    }

    let mut rewrites: Vec<CrossRewrite> = Vec::new();
    for bi in 0..body.blocks.len() {
        let block = &body.blocks[bi];
        // Only a plain `Jump` (single successor) — the operand rides one edge.
        let (target, args) = match &block.terminator.kind {
            TerminatorKind::Jump { target, args } => (target.index(), args.clone()),
            _ => continue,
        };
        // Skip self-loops, out-of-range, and any block with another predecessor.
        // The entry block is excluded too: its "params" are the function's ABI
        // parameters, so removing one (on a loop-back-to-entry edge that makes it
        // single-predecessor) would misalign every caller's arguments.
        if target == bi
            || target == body.entry.index()
            || target >= body.blocks.len()
            || pred_count[target] != 1
        {
            continue;
        }

        // A borrow of an @owned value can enter `b` as a @guaranteed block param
        // (the BeginBorrow lives in a predecessor; its EndBorrow here references
        // the BORROW result, not the source, so it would not show up in
        // `inst_use` below). Moving such a source out from under a live borrow is
        // a use-after-free. Collect the still-borrowed sources and refuse to move
        // them — the cross-block analogue of the block-local `frozen_at` guard.
        let borrowed_in: FxHashSet<ValueId> = block
            .params
            .iter()
            .filter(|p| p.ownership == Ownership::Guaranteed)
            .filter_map(|p| body.value(p.value).borrow_source)
            .collect();

        // How many times each value is used by an instruction operand in `b`.
        let mut inst_use: FxHashMap<ValueId, usize> = FxHashMap::default();
        for inst in &block.insts {
            for op in inst.kind.operands() {
                *inst_use.entry(op).or_default() += 1;
            }
        }

        let succ = &body.blocks[target];
        let succ_term_uses: FxHashSet<ValueId> =
            succ.terminator.kind.operands().into_iter().collect();

        for (ci, inst) in block.insts.iter().enumerate() {
            let InstKind::CopyValue { operand: x, .. } = &inst.kind else {
                continue;
            };
            let x = *x;
            if body.value(x).ownership != Ownership::Owned {
                continue;
            }
            // `X`'s ONLY instruction use in this block must be this copy (so no
            // in-block borrow or second consumer is in flight — both count as a
            // use), and it must not be borrowed via an incoming @guaranteed param.
            if inst_use.get(&x).copied().unwrap_or(0) != 1 || borrowed_in.contains(&x) {
                continue;
            }
            // `X` appears in the jump args exactly once → one threaded position.
            let mut positions = args.iter().enumerate().filter(|(_, a)| **a == x);
            let Some((k, _)) = positions.next() else {
                continue;
            };
            if positions.next().is_some() || k >= succ.params.len() {
                continue;
            }
            let p = succ.params[k].value;
            if succ_term_uses.contains(&p) {
                continue;
            }
            // `P`'s only use in `S` is a single `DestroyValue { operand: p }`.
            let mut destroy_j = None;
            let mut blocked = false;
            for (j, sinst) in succ.insts.iter().enumerate() {
                if !sinst.kind.operands().contains(&p) {
                    continue;
                }
                match &sinst.kind {
                    InstKind::DestroyValue { operand } if *operand == p && destroy_j.is_none() => {
                        destroy_j = Some(j);
                    },
                    _ => {
                        blocked = true;
                        break;
                    },
                }
            }
            if blocked {
                continue;
            }
            let Some(destroy_j) = destroy_j else {
                continue;
            };
            rewrites.push(CrossRewrite {
                b: bi,
                copy_i: ci,
                s: target,
                k,
                destroy_j,
            });
        }
    }

    if rewrites.is_empty() {
        return 0;
    }
    let count = rewrites.len();

    // Apply. Each `S` is single-predecessor, so all rewrites targeting a given
    // `S` come from the same `b`; remove args/params/destroys at descending
    // indices so lower positions stay valid.
    use std::collections::BTreeSet;
    let mut jump_rm: FxHashMap<usize, BTreeSet<usize>> = FxHashMap::default();
    let mut param_rm: FxHashMap<usize, BTreeSet<usize>> = FxHashMap::default();
    let mut destroy_rm: FxHashMap<usize, BTreeSet<usize>> = FxHashMap::default();
    for r in &rewrites {
        if let InstKind::CopyValue { result, operand } = body.blocks[r.b].insts[r.copy_i].kind {
            body.blocks[r.b].insts[r.copy_i].kind = InstKind::MoveValue { result, operand };
        }
        jump_rm.entry(r.b).or_default().insert(r.k);
        param_rm.entry(r.s).or_default().insert(r.k);
        destroy_rm.entry(r.s).or_default().insert(r.destroy_j);
    }
    for (b, ks) in &jump_rm {
        if let TerminatorKind::Jump { args, .. } = &mut body.blocks[*b].terminator.kind {
            for &k in ks.iter().rev() {
                args.remove(k);
            }
        }
    }
    for (s, ks) in &param_rm {
        for &k in ks.iter().rev() {
            body.blocks[*s].params.remove(k);
        }
    }
    for (s, js) in &destroy_rm {
        for &j in js.iter().rev() {
            body.blocks[*s].insts.remove(j);
        }
    }
    count
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
