//! Copy → move: pass a value owned when the caller is done with it.
//!
//! Parameters are borrowed by default. A callee that immediately copies a
//! borrowed parameter (`var c = xs`) forces the caller's value and the copy to
//! share storage, so mutating the copy deep-copies. When the caller's argument
//! is at its last use, the caller can hand the value over instead:
//!
//! - **Callee side (inference):** a `Borrow` param whose only use is a single
//!   `CopyValue` in the entry block is "consumable". A variant of the function
//!   is created that takes it `Consuming` and turns that copy into a move.
//! - **Caller side:** a call that borrows a value at its last use is rewritten
//!   to move the value into the variant. Three last-use shapes are recognized:
//!   - `slot = f(&slot)`: the borrowed slot is overwritten by the result;
//!   - `f(&v)` where owned `v` is only destroyed afterwards;
//!   - `f(&slot)` where `slot` is only destroyed afterwards.
//!
//! No original is dropped at a different time: it is consumed by the callee
//! instead of being destroyed by the caller, and the callee's copy simply
//! becomes the value itself. Variants are created lazily, one per
//! (function, param) pair.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::body::OssaBody;
use crate::callee::Callee;
use crate::inst::InstKind;
use crate::mono::types::MonoModule;
use crate::ty::ParamConvention;
use crate::value::Ownership;
use crate::{MonoFuncId, ValueId};

pub(super) fn run(mono: &mut MonoModule) -> usize {
    let consumable = consumable_params(mono);
    if consumable.is_empty() {
        return 0;
    }
    let mut variants = Variants::default();
    let mut total = 0;
    // Variants appended below are not rescanned: their callers were already
    // rewritten, and their own call sites match the original's.
    let original_count = mono.functions.len();
    for fi in 0..original_count {
        // One site at a time: a rewrite deletes instructions, so sites found
        // in the same scan could overlap and hold stale indices.
        while let Some(site) = mono.functions[fi]
            .body
            .as_ref()
            .and_then(|b| find_site(b, &consumable, &variants))
        {
            let vid = variants.get_or_make(mono, site.origin, &site.owned);
            let body = mono.functions[fi].body.as_mut().expect("site has a body");
            rewrite_site(body, &site, vid);
            total += 1;
        }
    }
    total
}

/// Owned-parameter variants, keyed by original function and the sorted set of
/// params it takes owned. A call to a variant can be specialized further, so
/// `f(a, b)` with both args at their last use ends up calling one variant
/// owning both.
#[derive(Default)]
struct Variants {
    /// Variant function index -> (original function index, owned params).
    origin_of: FxHashMap<usize, (usize, Vec<usize>)>,
    by_key: FxHashMap<(usize, Vec<usize>), MonoFuncId>,
}

impl Variants {
    /// The original function and already-owned params behind `fi`.
    fn resolve(&self, fi: usize) -> (usize, &[usize]) {
        match self.origin_of.get(&fi) {
            Some((origin, owned)) => (*origin, owned),
            None => (fi, &[]),
        }
    }

    fn get_or_make(&mut self, mono: &mut MonoModule, origin: usize, owned: &[usize]) -> MonoFuncId {
        let key = (origin, owned.to_vec());
        if let Some(id) = self.by_key.get(&key) {
            return *id;
        }
        let id = make_owned_variant(mono, origin, owned);
        self.origin_of.insert(id.index(), key.clone());
        self.by_key.insert(key, id);
        id
    }
}

/// Borrowed params whose only use is a single `CopyValue` in the entry block.
fn consumable_params(mono: &MonoModule) -> FxHashSet<(usize, usize)> {
    let mut out = FxHashSet::default();
    for (fi, func) in mono.functions.iter().enumerate() {
        let Some(body) = &func.body else { continue };
        let entry = body.entry.index();
        // A jump back into the entry block would run the copy more than once.
        let reenters = body.blocks.iter().any(|b| {
            b.terminator
                .kind
                .successors()
                .iter()
                .any(|s| s.index() == entry)
        });
        if reenters {
            continue;
        }
        for (k, param) in func.params.iter().enumerate() {
            // Function params are the first `param_count` values.
            if param.convention != ParamConvention::Borrow || k >= body.param_count {
                continue;
            }
            let only_copy = matches!(
                single_use(body, ValueId::new(k)),
                Some((b, InstKind::CopyValue { .. })) if b == entry
            );
            if only_copy {
                out.insert((fi, k));
            }
        }
    }
    out
}

/// The only use of `v` in the body, if it has exactly one (instruction) use.
fn single_use(body: &OssaBody, v: ValueId) -> Option<(usize, InstKind)> {
    let mut found = None;
    for (bi, block) in body.blocks.iter().enumerate() {
        if block.terminator.kind.operands().contains(&v) {
            return None;
        }
        for inst in &block.insts {
            if inst.kind.operands().contains(&v) {
                if found.is_some() {
                    return None;
                }
                found = Some((bi, inst.kind.clone()));
            }
        }
    }
    found
}

/// What the caller does with the borrowed value after the call.
enum LastUse {
    /// `store_assign slot, result` at this index: the slot is overwritten.
    Overwritten { store_i: usize },
    /// `destroy_addr slot` at this index: the slot's value is dropped.
    SlotDropped { destroy_i: usize },
    /// `destroy_value v` at this index, `v` being the borrowed owned value.
    ValueDropped { destroy_i: usize },
}

struct Site {
    block: usize,
    borrow_i: usize,
    call_i: usize,
    end_i: usize,
    arg: usize,
    last: LastUse,
    /// Original callee and the params the new variant owns (sorted).
    origin: usize,
    owned: Vec<usize>,
}

/// The first rewritable call site in `body`. A rewritten call targets a
/// `Consuming` variant, so it never matches again and the scan terminates.
fn find_site(
    body: &OssaBody,
    consumable: &FxHashSet<(usize, usize)>,
    variants: &Variants,
) -> Option<Site> {
    for (bi, block) in body.blocks.iter().enumerate() {
        for (ci, inst) in block.insts.iter().enumerate() {
            let InstKind::Call {
                result,
                callee: Callee::Resolved(fid),
                args,
            } = &inst.kind
            else {
                continue;
            };
            for (k, a) in args.iter().enumerate() {
                let (origin, owned) = variants.resolve(fid.index());
                if a.convention != ParamConvention::Borrow || !consumable.contains(&(origin, k)) {
                    continue;
                }
                let mut owned = owned.to_vec();
                owned.push(k);
                owned.sort_unstable();
                if let Some(site) = match_site(body, bi, ci, *result, a.value, k, origin, owned) {
                    return Some(site);
                }
            }
        }
    }
    None
}

/// Match `b = begin_borrow(_addr) src; r = call(.., b, ..); end_borrow b;`
/// followed by one of the [`LastUse`] shapes, with nothing else touching `src`
/// or `b` in between.
#[allow(clippy::too_many_arguments)]
fn match_site(
    body: &OssaBody,
    bi: usize,
    ci: usize,
    result: Option<ValueId>,
    b: ValueId,
    arg: usize,
    origin: usize,
    owned: Vec<usize>,
) -> Option<Site> {
    let block = &body.blocks[bi];
    let insts = &block.insts;
    let borrow_i = (0..ci)
        .rev()
        .find(|&i| insts[i].kind.results().contains(&b))?;
    let (src, is_slot) = match insts[borrow_i].kind {
        InstKind::BeginBorrowAddr { address, .. } => (address, true),
        InstKind::BeginBorrow { operand, .. }
            if body.value(operand).ownership == Ownership::Owned =>
        {
            (operand, false)
        },
        _ => return None,
    };
    // An owned value forwarded onward is used later. A slot *pointer* may be
    // forwarded: the slot is re-initialized (or left dropped) before the jump.
    let term_ops = block.terminator.kind.operands();
    let forwarded = term_ops.contains(&b) || (!is_slot && term_ops.contains(&src));
    if forwarded || has_open_borrow(insts, borrow_i, src) {
        return None;
    }
    // A borrow of `src` threaded into another block would outlive the move.
    if body.blocks.iter().any(|blk| {
        blk.params
            .iter()
            .any(|p| body.value(p.value).borrow_source == Some(src))
    }) {
        return None;
    }

    let mut end_i = None;
    for (i, inst) in insts.iter().enumerate().skip(borrow_i + 1) {
        let ops = inst.kind.operands();
        if !ops.contains(&src) && !ops.contains(&b) {
            continue;
        }
        if i == ci {
            // The borrow is passed exactly once, and `src` itself not at all.
            if ops.iter().filter(|o| **o == b).count() != 1 || ops.contains(&src) {
                return None;
            }
            continue;
        }
        let last = match inst.kind {
            InstKind::EndBorrow { operand } if operand == b && i > ci && end_i.is_none() => {
                end_i = Some(i);
                continue;
            },
            _ if end_i.is_none() => return None,
            InstKind::StoreAssign { address, value }
                if is_slot && address == src && Some(value) == result =>
            {
                LastUse::Overwritten { store_i: i }
            },
            InstKind::DestroyAddr { address, .. } if is_slot && address == src => {
                LastUse::SlotDropped { destroy_i: i }
            },
            InstKind::DestroyValue { operand } if !is_slot && operand == src => {
                LastUse::ValueDropped { destroy_i: i }
            },
            _ => return None,
        };
        return Some(Site {
            block: bi,
            borrow_i,
            call_i: ci,
            end_i: end_i?,
            arg,
            last,
            origin,
            owned,
        });
    }
    None
}

/// Some borrow of `src` begun before `before` has not ended by then.
fn has_open_borrow(insts: &[crate::inst::Instruction], before: usize, src: ValueId) -> bool {
    insts[..before].iter().any(|inst| match &inst.kind {
        InstKind::BeginBorrowAddr {
            result, address, ..
        }
        | InstKind::BeginMutBorrowAddr {
            result, address, ..
        } if *address == src => !ended(&insts[..before], *result),
        InstKind::BeginBorrow { result, operand }
        | InstKind::BeginMutBorrow { result, operand }
            if *operand == src =>
        {
            !ended(&insts[..before], *result)
        },
        _ => false,
    })
}

fn ended(insts: &[crate::inst::Instruction], borrow: ValueId) -> bool {
    insts.iter().any(|e| {
        matches!(e.kind, InstKind::EndBorrow { operand } | InstKind::EndMutBorrow { operand } if operand == borrow)
    })
}

/// Clone function `fi` with every param in `owned` taken owned and its copy
/// turned into a move.
fn make_owned_variant(mono: &mut MonoModule, fi: usize, owned: &[usize]) -> MonoFuncId {
    let mut f = mono.functions[fi].clone();
    let suffix: Vec<String> = owned.iter().map(usize::to_string).collect();
    f.name = format!("{}$own{}", f.name, suffix.join("_"));
    f.is_main = false;
    let body = f.body.as_mut().expect("consumable param has a body");
    let entry = body.entry.index();
    for &k in owned {
        f.params[k].convention = ParamConvention::Consuming;
        let p = ValueId::new(k);
        body.values[p.index()].ownership = Ownership::Owned;
        body.values[p.index()].borrow_source = None;
        for inst in &mut body.blocks[entry].insts {
            if let InstKind::CopyValue { result, operand } = inst.kind
                && operand == p
            {
                inst.kind = InstKind::MoveValue { result, operand };
            }
        }
    }
    mono.functions.push(f);
    MonoFuncId::new(mono.functions.len() - 1)
}

fn rewrite_site(body: &mut OssaBody, site: &Site, vid: MonoFuncId) {
    let insts = &mut body.blocks[site.block].insts;
    // The value handed to the callee: moved out of the slot, or the owned value itself.
    let moved = match insts[site.borrow_i].kind {
        InstKind::BeginBorrowAddr {
            result,
            address,
            ty,
        } => {
            insts[site.borrow_i].kind = InstKind::Take {
                result,
                address,
                ty,
                independent: true,
            };
            body.values[result.index()].ownership = Ownership::Owned;
            body.values[result.index()].borrow_source = None;
            result
        },
        InstKind::BeginBorrow { operand, .. } => operand,
        _ => unreachable!("matched site"),
    };
    let insts = &mut body.blocks[site.block].insts;
    if let InstKind::Call { callee, args, .. } = &mut insts[site.call_i].kind {
        *callee = Callee::Resolved(vid);
        args[site.arg].value = moved;
        args[site.arg].convention = ParamConvention::Consuming;
    }
    // Delete highest index first so earlier indices stay valid.
    let mut delete = vec![site.end_i];
    match site.last {
        // The slot is uninitialized after the take, so the result is an init.
        LastUse::Overwritten { store_i } => {
            if let InstKind::StoreAssign { address, value } = insts[store_i].kind {
                insts[store_i].kind = InstKind::StoreInit { address, value };
            }
        },
        // The callee now owns the value: the caller's drop goes away.
        LastUse::SlotDropped { destroy_i } | LastUse::ValueDropped { destroy_i } => {
            delete.push(destroy_i)
        },
    }
    if matches!(insts[site.borrow_i].kind, InstKind::BeginBorrow { .. }) {
        delete.push(site.borrow_i);
    }
    delete.sort_unstable();
    for i in delete.into_iter().rev() {
        insts.remove(i);
    }
}
