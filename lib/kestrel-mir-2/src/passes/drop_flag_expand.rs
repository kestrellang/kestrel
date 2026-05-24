use crate::body::BasicBlock;
use crate::immediate::Immediate;
use crate::operand::{Operand, UseMode};
use crate::place::Place;
use crate::statement::{Rvalue, Statement, StatementKind};
use crate::terminator::{Terminator, TerminatorKind};
use crate::{BlockId, MirModule};

/// Expand `DropIf` and `SetDropFlag` into primitive CFG operations.
///
/// After this pass, no `DropIf` or `SetDropFlag` statements remain.
/// Plain `Drop` statements survive for post-mono shim expansion.
pub fn run_drop_flag_expansion(module: &mut MirModule) {
    for fi in 0..module.functions.len() {
        if module.functions[fi].body.is_none() {
            continue;
        }
        expand_function(&mut module.functions[fi]);
    }
}

fn expand_function(func: &mut crate::item::function::FunctionDef) {
    let body = func.body.as_mut().unwrap();

    // Pass 1: Replace SetDropFlag → Assign (in-place, no index shifts)
    for block in &mut body.blocks {
        for stmt in &mut block.stmts {
            if let StatementKind::SetDropFlag { flag, value } = &stmt.kind {
                let flag = *flag;
                let value = *value;
                stmt.kind = StatementKind::Assign {
                    dest: Place::local(flag),
                    rvalue: Rvalue::Use(
                        Operand::Const(Immediate::bool(value)),
                        UseMode::Copy,
                    ),
                };
            }
        }
    }

    // Pass 2: Collect DropIf locations
    let mut drop_ifs: Vec<(usize, usize)> = Vec::new();
    for (bi, block) in body.blocks.iter().enumerate() {
        for (si, stmt) in block.stmts.iter().enumerate() {
            if matches!(stmt.kind, StatementKind::DropIf { .. }) {
                drop_ifs.push((bi, si));
            }
        }
    }

    // Process in reverse order so block/stmt indices stay valid
    for &(bi, si) in drop_ifs.iter().rev() {
        let stmt = &body.blocks[bi].stmts[si];
        let StatementKind::DropIf { place, flag } = &stmt.kind else {
            continue;
        };
        let place = place.clone();
        let flag = *flag;
        let span = body.blocks[bi].stmts[si].span.clone();

        let continue_block = BlockId::new(body.blocks.len());
        let drop_block = BlockId::new(body.blocks.len() + 1);
        let skip_block = BlockId::new(body.blocks.len() + 2);

        // Split: stmts after the DropIf + old terminator → continue_block
        let remaining_stmts = body.blocks[bi].stmts.split_off(si + 1);
        let old_terminator = std::mem::replace(
            &mut body.blocks[bi].terminator,
            Terminator {
                kind: TerminatorKind::Branch {
                    condition: Operand::Place(Place::local(flag)),
                    then_block: drop_block,
                    else_block: skip_block,
                },
                span: span.clone(),
            },
        );
        // Remove the DropIf statement itself
        body.blocks[bi].stmts.pop();

        // continue_block: remaining stmts + old terminator
        body.blocks.push(BasicBlock {
            stmts: remaining_stmts,
            terminator: old_terminator,
        });

        // drop_block: Drop { place } → jump continue
        body.blocks.push(BasicBlock {
            stmts: vec![Statement {
                kind: StatementKind::Drop { place },
                span,
            }],
            terminator: Terminator {
                kind: TerminatorKind::Jump(continue_block),
                span: None,
            },
        });

        // skip_block: empty → jump continue
        body.blocks.push(BasicBlock {
            stmts: vec![],
            terminator: Terminator {
                kind: TerminatorKind::Jump(continue_block),
                span: None,
            },
        });
    }
}
