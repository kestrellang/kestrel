use crate::ValueId;
#[allow(unused_imports)]
use crate::body::OssaBody;
use crate::inst::InstKind;
use crate::mono::types::MonoModule;
use crate::value::Ownership;
use std::collections::{HashMap, HashSet};

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
            if func.name.contains("RcBox")
                && func.name.contains("release")
                && func.name.contains("StringStorage")
            {
                // Dump the optimized body
                for (bi, block) in func.body.as_ref().unwrap().blocks.iter().enumerate() {
                    eprintln!("[copy_prop]   bb{bi}:");
                    for inst in &block.insts {
                        eprintln!("[copy_prop]     {:?}", inst.kind);
                    }
                    eprintln!("[copy_prop]     TERM: {:?}", block.terminator.kind);
                }
            }
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
    let mut uses: HashMap<ValueId, Vec<usize>> = HashMap::new();
    for (i, inst) in insts.iter().enumerate() {
        for op in inst.kind.operands() {
            uses.entry(op).or_default().push(i);
        }
    }
    let terminator_uses: HashSet<ValueId> = block.terminator.kind.operands().into_iter().collect();

    // Forward scan: track active borrows at each instruction index.
    let mut frozen: HashMap<ValueId, u32> = HashMap::new();
    let mut borrow_source_map: HashMap<ValueId, ValueId> = HashMap::new();
    let mut frozen_at: Vec<HashSet<ValueId>> = Vec::with_capacity(insts.len());

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
                if let Some(&src) = borrow_source_map.get(operand) {
                    if let Some(count) = frozen.get_mut(&src) {
                        *count = count.saturating_sub(1);
                    }
                }
            },
            _ => {},
        }
    }

    // Find CopyValue+DestroyValue pairs to eliminate.
    let mut replace_with_move: HashSet<usize> = HashSet::new();
    let mut delete_indices: HashSet<usize> = HashSet::new();
    let mut claimed: HashSet<ValueId> = HashSet::new();

    for (i, inst) in insts.iter().enumerate() {
        let InstKind::CopyValue { result, operand } = &inst.kind else {
            continue;
        };
        let x = *operand;
        let y = *result;

        if body.value(x).ownership != Ownership::Owned {
            continue;
        }
        if terminator_uses.contains(&x) {
            continue;
        }
        if claimed.contains(&x) {
            continue;
        }
        if frozen_at.get(i).map_or(false, |f| f.contains(&x)) {
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
        if replace_with_move.contains(&idx) {
            if let InstKind::CopyValue { result, operand } = &inst.kind {
                new_insts.push(crate::inst::Instruction {
                    kind: InstKind::MoveValue {
                        result: *result,
                        operand: *operand,
                    },
                    span: inst.span,
                });
                continue;
            }
        }
        new_insts.push(inst);
    }
    body.blocks[block_idx].insts = new_insts;

    eliminated
}

fn resolve(v: ValueId, remap: &HashMap<ValueId, ValueId>) -> ValueId {
    let mut current = v;
    while let Some(&target) = remap.get(&current) {
        current = target;
    }
    current
}

fn remap_operands(kind: &mut InstKind, remap: &HashMap<ValueId, ValueId>) {
    if remap.is_empty() {
        return;
    }
    match kind {
        InstKind::CopyValue { operand, .. }
        | InstKind::MoveValue { operand, .. }
        | InstKind::DestroyValue { operand }
        | InstKind::BeginBorrow { operand, .. }
        | InstKind::EndBorrow { operand }
        | InstKind::BeginMutBorrow { operand, .. }
        | InstKind::EndMutBorrow { operand }
        | InstKind::Discriminant { operand, .. }
        | InstKind::StructExtract { operand, .. }
        | InstKind::TupleExtract { operand, .. }
        | InstKind::EnumPayload { operand, .. }
        | InstKind::DestructureStruct { operand, .. }
        | InstKind::DestructureTuple { operand, .. }
        | InstKind::DestructureEnum { operand, .. } => {
            *operand = resolve(*operand, remap);
        },
        InstKind::Load { address, .. } => {
            *address = resolve(*address, remap);
        },
        InstKind::CopyAddr { address, .. }
        | InstKind::Take { address, .. }
        | InstKind::BeginBorrowAddr { address, .. }
        | InstKind::BeginMutBorrowAddr { address, .. }
        | InstKind::DestroyAddr { address, .. } => {
            *address = resolve(*address, remap);
        },
        InstKind::StoreInit { address, value } | InstKind::StoreAssign { address, value } => {
            *address = resolve(*address, remap);
            *value = resolve(*value, remap);
        },
        InstKind::Op1 { arg, .. } => {
            *arg = resolve(*arg, remap);
        },
        InstKind::Op2 { lhs, rhs, .. } => {
            *lhs = resolve(*lhs, remap);
            *rhs = resolve(*rhs, remap);
        },
        InstKind::Op3 { a, b, c, .. } => {
            *a = resolve(*a, remap);
            *b = resolve(*b, remap);
            *c = resolve(*c, remap);
        },
        InstKind::Struct { fields, .. } => {
            for (_, v) in fields {
                *v = resolve(*v, remap);
            }
        },
        InstKind::Tuple { elements, .. } | InstKind::Array { elements, .. } => {
            for v in elements {
                *v = resolve(*v, remap);
            }
        },
        InstKind::Enum { payload, .. } => {
            for v in payload {
                *v = resolve(*v, remap);
            }
        },
        InstKind::Call { args, .. } => {
            for arg in args {
                arg.value = resolve(arg.value, remap);
            }
        },
        InstKind::ApplyPartial { captures, .. } => {
            for v in captures {
                *v = resolve(*v, remap);
            }
        },
        InstKind::FieldAddr { base, .. } => {
            *base = resolve(*base, remap);
        },
        InstKind::Literal { .. } | InstKind::GlobalRef { .. } | InstKind::Uninit { .. } => {},
    }
}

fn remap_terminator(
    kind: &mut crate::terminator::TerminatorKind,
    remap: &HashMap<ValueId, ValueId>,
) {
    if remap.is_empty() {
        return;
    }
    use crate::terminator::TerminatorKind;
    match kind {
        TerminatorKind::Return(v) => {
            *v = resolve(*v, remap);
        },
        TerminatorKind::Jump { args, .. } => {
            for v in args {
                *v = resolve(*v, remap);
            }
        },
        TerminatorKind::Branch {
            condition,
            then_args,
            else_args,
            ..
        } => {
            *condition = resolve(*condition, remap);
            for v in then_args {
                *v = resolve(*v, remap);
            }
            for v in else_args {
                *v = resolve(*v, remap);
            }
        },
        TerminatorKind::Switch {
            discriminant,
            cases,
        } => {
            *discriminant = resolve(*discriminant, remap);
            for arm in cases {
                for v in &mut arm.args {
                    *v = resolve(*v, remap);
                }
            }
        },
        TerminatorKind::Panic(_) | TerminatorKind::Unreachable => {},
    }
}
