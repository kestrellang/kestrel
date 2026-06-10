//! Read-only post-mono audit: flag silent bitwise duplications of non-trivial
//! (heap/Clone/droppable) values whose source stays live — the "missing-clone /
//! uncounted alias" footgun behind use-after-free and double-free miscompiles.
//!
//! In a correct OSSA program every duplication of a non-trivial value goes
//! through a clone `Call` (post-`expand`, `CopyValue` on a Clone type *becomes*
//! such a call). When a *forwarding* instruction instead hands out an `@owned`
//! non-trivial value that bitwise-aliases a still-live source, the alias is
//! uncounted: both holders later drop it → premature free / double-free. This
//! class of bug is invisible to refcount tracing (a *missing* clone leaves no
//! event), so we catch it structurally on the monomorphized IR.
//!
//! The pass is purely diagnostic and **entirely env-gated** — [`run_audit`]
//! emits nothing unless `KESTREL_AUDIT_DUP` is set, so it is inert in normal
//! builds and the test suite. `KESTREL_AUDIT_FILTER=<substr>` restricts output
//! to functions whose mangled name contains `<substr>`.
//!
//! Copyability/drop is read from the post-mono `type_info` (the per-instantiation
//! authority resolved during monomorphization), NOT recomputed from the generic
//! `copy_behavior` — the `MonoModule` has no generic `MirModule` to query.

use std::collections::{HashMap, HashSet};

use crate::body::OssaBody;
use crate::inst::InstKind;
use crate::mono::types::{MonoFunction, MonoModule};
use crate::terminator::TerminatorKind;
use crate::ty::{MirTy, ParamConvention};
use crate::value::Ownership;
use crate::{DropBehavior, TyId, ValueId};

/// Which duplication signature a finding matches. Ordered by confidence:
/// the first three are structurally unambiguous; `AddrRead` is heuristic
/// (we cannot prove the pointee stays live from the IR alone).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DupKind {
    /// A forwarding extraction/move (`StructExtract`/`TupleExtract`/`EnumPayload`/
    /// `MoveValue`) produced an `@owned` non-trivial value from a `@guaranteed`
    /// (borrowed) operand — ownership laundered without a clone. The borrow keeps
    /// the owned root alive, so the source is provably live: a counted alias was
    /// owed and not emitted.
    OwnedFromBorrow,
    /// A `CopyValue` of a non-trivial type survived `expand` (which turns
    /// `CopyValue` on Clone types into a clone `Call` and drops it on Bitwise
    /// types). A survivor is a bitwise alias of a heap value with no shim.
    SurvivingCopy,
    /// A `MoveValue` of a non-trivial value whose operand is still used elsewhere
    /// in the body — the "move" left a live bitwise duplicate behind.
    UseAfterMove,
    /// A `Load`/`CopyAddr` produced an `@owned` non-trivial value from an address
    /// with no clone `Call` — the `Pointer.read` (`lang.ptr_read`) aliasing
    /// footgun. Heuristic: flagged regardless of whether the pointee is proven live.
    AddrRead,
    /// A `@guaranteed` (borrowed) non-trivial value is *consumed* (moved into an
    /// aggregate/store/call-arg/return ≥1 time). Consuming a borrow stores an
    /// uncounted alias into an owning slot: the dual of `OwnedFromBorrow`.
    ConsumeBorrow,
    /// An `@owned` non-trivial value is consumed **zero** times — never dropped,
    /// moved, stored, or returned. A leak (the OSSA consume-exactly-once invariant
    /// says every `@owned` value is consumed once). Computed post-`expand`, where
    /// drops are real `__drop` calls, so a genuine 0 means a real leak.
    Leak,
    /// An `@owned` non-trivial value is consumed **two or more** times — a
    /// double-free / over-release. With per-terminator branch-arg dedup (mutually
    /// exclusive successors count once), ≥2 is a true second consume on some path.
    DoubleConsume,
}

impl DupKind {
    pub fn tag(self) -> &'static str {
        match self {
            DupKind::OwnedFromBorrow => "OWNED-FROM-BORROW",
            DupKind::SurvivingCopy => "SURVIVING-COPY",
            DupKind::UseAfterMove => "USE-AFTER-MOVE",
            DupKind::AddrRead => "ADDR-READ",
            DupKind::ConsumeBorrow => "CONSUME-BORROW",
            DupKind::Leak => "LEAK",
            DupKind::DoubleConsume => "DOUBLE-CONSUME",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DupFinding {
    pub kind: DupKind,
    pub func_idx: usize,
    pub func_name: String,
    pub block: usize,
    pub inst: usize,
    /// Human-readable instruction + value/type/ownership detail.
    pub detail: String,
}

/// Env-gated entry: run the audit and print findings to stderr. No-op unless
/// `KESTREL_AUDIT_DUP` is set. Hooked after `expand_destroy_copy` in the driver.
pub fn run_audit(module: &MonoModule) {
    if std::env::var("KESTREL_AUDIT_DUP").is_err() {
        return;
    }
    let filter = std::env::var("KESTREL_AUDIT_FILTER").unwrap_or_default();
    // KESTREL_AUDIT_KINDS=<comma tags>: restrict printed findings to these kinds
    // (e.g. "DOUBLE-CONSUME,OWNED-FROM-BORROW"). Summary counts still cover all.
    let kinds: Vec<String> = std::env::var("KESTREL_AUDIT_KINDS")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // KESTREL_AUDIT_DUMP=<comma-substrs>: print the full post-expand body of every
    // function whose mangled name contains one of the substrings. Lets the same
    // env-gated build hook serve as a targeted `dump mir -s expand` for tracing.
    if let Ok(dump) = std::env::var("KESTREL_AUDIT_DUMP") {
        let needles: Vec<&str> = dump
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        for func in &module.functions {
            let Some(body) = &func.body else { continue };
            if needles.iter().any(|n| func.name.contains(n)) {
                eprintln!("[audit-dump] ; function: {}", func.name);
                eprint!("{}", crate::display::display_body(body, module));
                eprintln!();
            }
        }
    }

    let findings = audit_duplications(module);
    let mut counts: HashMap<&'static str, usize> = HashMap::new();
    let mut shown = 0usize;

    eprintln!("[audit-dup] === post-mono bitwise-duplication audit ===");
    for f in &findings {
        *counts.entry(f.kind.tag()).or_default() += 1;
        if !filter.is_empty() && !f.func_name.contains(&filter) {
            continue;
        }
        if !kinds.is_empty() && !kinds.iter().any(|k| k == f.kind.tag()) {
            continue;
        }
        shown += 1;
        eprintln!(
            "[audit-dup] [{}] {} @ bb{}:{}\n             {}",
            f.kind.tag(),
            f.func_name,
            f.block,
            f.inst,
            f.detail,
        );
    }
    eprintln!(
        "[audit-dup] --- {} finding(s) total ({} shown){} ---",
        findings.len(),
        shown,
        if filter.is_empty() {
            String::new()
        } else {
            format!(", filter=\"{filter}\"")
        },
    );
    let mut tags: Vec<_> = counts.into_iter().collect();
    tags.sort();
    for (tag, n) in tags {
        eprintln!("[audit-dup]   {tag}: {n}");
    }
}

/// Scan every monomorphized body and collect duplication findings. Pure (no
/// I/O), so it is also unit-testable and reusable by a future driver/CLI hook.
pub fn audit_duplications(module: &MonoModule) -> Vec<DupFinding> {
    let mut findings = Vec::new();
    for (fi, func) in module.functions.iter().enumerate() {
        let Some(body) = &func.body else { continue };
        if body.values.is_empty() || body.blocks.is_empty() {
            continue;
        }
        audit_body(module, fi, func, body, &mut findings);
    }
    findings
}

fn audit_body(
    module: &MonoModule,
    fi: usize,
    func: &MonoFunction,
    body: &OssaBody,
    findings: &mut Vec<DupFinding>,
) {
    // Whole-body operand use counts: a non-trivial value moved while still used
    // elsewhere is a live duplicate. Counts every instruction + terminator use.
    let uses = use_counts(body);

    for (bi, block) in body.blocks.iter().enumerate() {
        for (ii, inst) in block.insts.iter().enumerate() {
            let Some(kind) = classify(module, body, &inst.kind, &uses) else {
                continue;
            };
            findings.push(DupFinding {
                kind,
                func_idx: fi,
                func_name: func.name.clone(),
                block: bi,
                inst: ii,
                detail: describe(module, body, &inst.kind),
            });
        }
    }

    audit_consume_balance(module, fi, func, body, findings);
}

/// Consume-balance check: in OSSA every `@owned` value is consumed exactly once.
/// Post-`expand` (drops are real `__drop` calls) we count consume sites per value
/// and flag deviations — a leak (0), a double-free (≥2), or a consumed borrow.
fn audit_consume_balance(
    module: &MonoModule,
    fi: usize,
    func: &MonoFunction,
    body: &OssaBody,
    findings: &mut Vec<DupFinding>,
) {
    let sites = consume_sites(body);
    let defs = def_locations(body);

    for (vi, def) in body.values.iter().enumerate() {
        if !is_non_trivial(module, def.ty) {
            continue;
        }
        let v = ValueId::new(vi);
        let where_consumed = sites.get(&v).map(Vec::as_slice).unwrap_or(&[]);
        let n = where_consumed.len();
        let kind = match def.ownership {
            Ownership::Owned if n == 0 => DupKind::Leak,
            Ownership::Owned if n >= 2 => DupKind::DoubleConsume,
            Ownership::Guaranteed if n >= 1 => DupKind::ConsumeBorrow,
            _ => continue,
        };
        let (block, inst) = defs.get(&v).copied().unwrap_or((0, 0));
        findings.push(DupFinding {
            kind,
            func_idx: fi,
            func_name: func.name.clone(),
            block,
            inst,
            detail: format!(
                "%v{vi} {} {} consumed {n}x [{}]",
                match def.ownership {
                    Ownership::Owned => "@owned",
                    Ownership::Guaranteed => "@guaranteed",
                },
                ty_name(module, def.ty),
                where_consumed.join(", "),
            ),
        });
    }
}

/// Classify a single instruction against the four duplication signatures.
/// Returns `None` for the (overwhelming) majority that are benign.
fn classify(
    module: &MonoModule,
    body: &OssaBody,
    kind: &InstKind,
    uses: &HashMap<ValueId, usize>,
) -> Option<DupKind> {
    let owned = |v: ValueId| body.value(v).ownership == Ownership::Owned;
    let guaranteed = |v: ValueId| body.value(v).ownership == Ownership::Guaranteed;
    let heap = |v: ValueId| is_non_trivial(module, body.value(v).ty);

    match kind {
        // Forwarding value extraction / move: @owned non-trivial result laundered
        // out of a @guaranteed (borrowed) operand — the missing-clone signature.
        InstKind::StructExtract {
            result, operand, ..
        }
        | InstKind::TupleExtract {
            result, operand, ..
        }
        | InstKind::EnumPayload {
            result, operand, ..
        }
        | InstKind::MoveValue { result, operand } => {
            if owned(*result) && heap(*result) && guaranteed(*operand) {
                return Some(DupKind::OwnedFromBorrow);
            }
            // A move whose source is still referenced elsewhere = live duplicate.
            if let InstKind::MoveValue { operand, .. } = kind
                && heap(*operand)
                && uses.get(operand).copied().unwrap_or(0) > 1
            {
                return Some(DupKind::UseAfterMove);
            }
            None
        },

        // A CopyValue on a non-trivial type should have been lowered to a clone
        // Call by `expand`; a survivor is a raw bitwise alias. Only a concern when
        // the SOURCE stays live (used more than once — i.e. besides this copy):
        // otherwise the copy is effectively a move and the bitwise alias is benign.
        InstKind::CopyValue { result, operand } => ((heap(*result) || heap(*operand))
            && uses.get(operand).copied().unwrap_or(0) > 1)
            .then_some(DupKind::SurvivingCopy),

        // Reading an @owned non-trivial value out of memory with no clone Call —
        // the Pointer.read aliasing footgun (heuristic, pointee liveness unknown).
        InstKind::Load { result, .. } => {
            (owned(*result) && heap(*result)).then_some(DupKind::AddrRead)
        },
        InstKind::CopyAddr { result, ty, .. } => {
            (owned(*result) && is_non_trivial(module, *ty)).then_some(DupKind::AddrRead)
        },

        _ => None,
    }
}

/// Count how many times each value is used as an operand across the whole body
/// (instruction operands + terminator operands/successor args).
fn use_counts(body: &OssaBody) -> HashMap<ValueId, usize> {
    let mut uses: HashMap<ValueId, usize> = HashMap::new();
    for block in &body.blocks {
        for inst in &block.insts {
            for op in inst.kind.operands() {
                *uses.entry(op).or_default() += 1;
            }
        }
        for op in block.terminator.kind.operands() {
            *uses.entry(op).or_default() += 1;
        }
    }
    uses
}

/// Record, per value, the locations where it is *consumed* (ownership transferred
/// away): moved, dropped, stored into memory, placed in an aggregate, passed as a
/// `Consuming` call arg, destructured, returned, or passed as a successor block
/// arg. Reads/borrows (CopyValue, BeginBorrow*, Load, extraction, `Borrow`/
/// `MutBorrow` call args, conditions/discriminants) are NOT consumes.
///
/// Branch/Switch successor args are deduplicated **per terminator**: a value
/// passed to multiple mutually-exclusive successors is consumed once at runtime,
/// so counting it once avoids a spurious double-consume.
fn consume_sites(body: &OssaBody) -> HashMap<ValueId, Vec<String>> {
    let mut sites: HashMap<ValueId, Vec<String>> = HashMap::new();
    for (bi, block) in body.blocks.iter().enumerate() {
        for (ii, inst) in block.insts.iter().enumerate() {
            let loc = format!("bb{bi}:{ii}");
            match &inst.kind {
                InstKind::MoveValue { operand, .. } => push(&mut sites, *operand, &loc, "move"),
                InstKind::DestroyValue { operand } => push(&mut sites, *operand, &loc, "destroy"),
                InstKind::StoreInit { value, .. } | InstKind::StoreAssign { value, .. } => {
                    push(&mut sites, *value, &loc, "store")
                },
                InstKind::Struct { fields, .. } => {
                    for (_, v) in fields {
                        push(&mut sites, *v, &loc, "struct-field");
                    }
                },
                InstKind::Tuple { elements, .. } | InstKind::Array { elements, .. } => {
                    for v in elements {
                        push(&mut sites, *v, &loc, "aggregate");
                    }
                },
                InstKind::Enum { payload, .. } => {
                    for v in payload {
                        push(&mut sites, *v, &loc, "enum-payload");
                    }
                },
                InstKind::ApplyPartial { captures, .. } => {
                    for v in captures {
                        push(&mut sites, *v, &loc, "capture");
                    }
                },
                InstKind::DestructureStruct { operand, .. }
                | InstKind::DestructureTuple { operand, .. }
                | InstKind::DestructureEnum { operand, .. } => {
                    push(&mut sites, *operand, &loc, "destructure")
                },
                InstKind::Call { args, .. } => {
                    for a in args {
                        if a.convention == ParamConvention::Consuming {
                            push(&mut sites, a.value, &loc, "call-arg");
                        }
                    }
                },
                // Everything else borrows/reads its operands — no consume.
                _ => {},
            }
        }

        let tloc = format!("bb{bi}:{}", block.insts.len());
        match &block.terminator.kind {
            TerminatorKind::Return(v) => push(&mut sites, *v, &tloc, "return"),
            TerminatorKind::Jump { args, .. } => {
                dedup_succ_args(&mut sites, &tloc, args.iter().copied())
            },
            TerminatorKind::Branch {
                then_args,
                else_args,
                ..
            } => dedup_succ_args(
                &mut sites,
                &tloc,
                then_args.iter().chain(else_args).copied(),
            ),
            TerminatorKind::Switch { cases, .. } => dedup_succ_args(
                &mut sites,
                &tloc,
                cases.iter().flat_map(|a| a.args.iter().copied()),
            ),
            TerminatorKind::Panic(_) | TerminatorKind::Unreachable => {},
        }
    }
    sites
}

fn push(sites: &mut HashMap<ValueId, Vec<String>>, v: ValueId, loc: &str, what: &str) {
    sites.entry(v).or_default().push(format!("{loc} {what}"));
}

/// Successor args consume their value once per terminator (mutually exclusive
/// successors run once), so dedup the value set before recording.
fn dedup_succ_args(
    sites: &mut HashMap<ValueId, Vec<String>>,
    loc: &str,
    args: impl Iterator<Item = ValueId>,
) {
    let mut seen = HashSet::new();
    for v in args {
        if seen.insert(v) {
            push(sites, v, loc, "succ-arg");
        }
    }
}

/// Map each value to its definition site (block, inst). Block params are defined
/// at their block entry (`inst = 0`); instruction results at their instruction.
fn def_locations(body: &OssaBody) -> HashMap<ValueId, (usize, usize)> {
    let mut defs = HashMap::new();
    for (bi, block) in body.blocks.iter().enumerate() {
        for param in &block.params {
            defs.insert(param.value, (bi, 0));
        }
        for (ii, inst) in block.insts.iter().enumerate() {
            for r in inst.kind.results() {
                defs.insert(r, (bi, ii));
            }
        }
    }
    defs
}

// -- Post-mono copy/drop classification (reads resolved `type_info`) ----------

/// True when `ty` has a real destructor — the only types with a lifetime
/// obligation (an `RcBox` release / nested drop) in the MIR. A bitwise duplicate
/// of such a value is an uncounted alias → double-free; an un-consumed one leaks.
///
/// Gating on `needs_drop` (NOT `copy != Bitwise`) is deliberate: a nominally
/// `Clone` type with no heap payload (e.g. `IoErrorKind`, `Optional[RawPointer]`)
/// is bit-copyable with no cleanup, so the lowering emits no explicit
/// consume/drop for it — counting consumes there yields only false leaks.
fn is_non_trivial(module: &MonoModule, ty: TyId) -> bool {
    mono_needs_drop(module, ty)
}

fn mono_needs_drop(module: &MonoModule, ty: TyId) -> bool {
    match module.ty_arena.get(ty) {
        MirTy::Tuple(elems) => {
            let elems = elems.clone();
            elems.iter().any(|&e| mono_needs_drop(module, e))
        },
        MirTy::Named { entity, type_args } => {
            let key = (*entity, type_args.clone());
            module
                .structs
                .get(&key)
                .map(|s| s.type_info.drop != DropBehavior::None)
                .or_else(|| {
                    module
                        .enums
                        .get(&key)
                        .map(|e| e.type_info.drop != DropBehavior::None)
                })
                .unwrap_or(false)
        },
        _ => false,
    }
}

// -- Diagnostic rendering -----------------------------------------------------

fn describe(module: &MonoModule, body: &OssaBody, kind: &InstKind) -> String {
    let v = |id: ValueId| -> String {
        let d = body.value(id);
        let own = match d.ownership {
            Ownership::Owned => "@owned",
            Ownership::Guaranteed => "@guaranteed",
        };
        format!("%v{} {} {}", id.index(), own, ty_name(module, d.ty))
    };
    match kind {
        InstKind::StructExtract {
            result,
            operand,
            field,
        } => format!(
            "struct_extract {} <- {} .field{}",
            v(*result),
            v(*operand),
            field.index()
        ),
        InstKind::TupleExtract {
            result,
            operand,
            index,
        } => {
            format!("tuple_extract {} <- {} .{}", v(*result), v(*operand), index)
        },
        InstKind::EnumPayload {
            result,
            operand,
            variant,
            field,
        } => format!(
            "enum_payload {} <- {} variant{}.field{}",
            v(*result),
            v(*operand),
            variant.index(),
            field.index()
        ),
        InstKind::MoveValue { result, operand } => {
            format!("move_value {} <- {}", v(*result), v(*operand))
        },
        InstKind::CopyValue { result, operand } => {
            format!("copy_value {} <- {}", v(*result), v(*operand))
        },
        InstKind::Load { result, address } => {
            format!("load {} <- *{}", v(*result), v(*address))
        },
        InstKind::CopyAddr {
            result,
            address,
            ty,
        } => format!(
            "copy_addr {} <- *{} : {}",
            v(*result),
            v(*address),
            ty_name(module, *ty)
        ),
        other => format!("{other:?}"),
    }
}

/// Compact type name for diagnostics (mirrors `mono::verify::describe_mono_ty`).
fn ty_name(module: &MonoModule, ty: TyId) -> String {
    let name = |e: &kestrel_hecs::Entity| module.resolve_name(*e).to_string();
    match module.ty_arena.get(ty) {
        MirTy::Named { entity, type_args } if type_args.is_empty() => name(entity),
        MirTy::Named { entity, type_args } => {
            let args: Vec<String> = type_args.iter().map(|&a| ty_name(module, a)).collect();
            format!("{}[{}]", name(entity), args.join(", "))
        },
        MirTy::Pointer(inner) => format!("Pointer[{}]", ty_name(module, *inner)),
        MirTy::Ref { pointee, mutating } => format!(
            "{}{}",
            if *mutating { "&mutating " } else { "&" },
            ty_name(module, *pointee)
        ),
        MirTy::Tuple(elems) if elems.is_empty() => "()".into(),
        MirTy::Tuple(elems) => {
            let parts: Vec<String> = elems.iter().map(|&e| ty_name(module, e)).collect();
            format!("({})", parts.join(", "))
        },
        MirTy::Str => "Str".into(),
        other => format!("{other:?}"),
    }
}
