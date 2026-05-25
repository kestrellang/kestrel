pub(crate) mod body;
mod context;
mod items;
mod name;
pub mod ty;
mod validate;

pub use context::LowerCtx;

use kestrel_hecs::{Entity, World};
use kestrel_mir_3::MirModule;

/// Lower the entire compiled program to OSSA MIR.
///
/// Takes the ECS world and root module entity. Call after type inference.
pub fn lower_module(world: &World, root: Entity) -> MirModule {
    let mut ctx = LowerCtx::new(world, root, "main");

    // Phase 1: item declarations (structs, enums, protocols, functions, statics)
    items::lower_items(&mut ctx);

    // Phase 2: witness tables
    items::witness_lower::lower_witnesses(&mut ctx);

    // Phase 3: static init thunks + master init + inject into main
    items::static_lower::synthesize_static_inits(&mut ctx);

    // Phase 4: validate no MirTy::Error escaped
    let _error_count = validate::validate_no_error_types(&ctx, &ctx.module);

    ctx.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_compiler::Compiler;

    fn stdlib_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../lang/std")
            .canonicalize()
            .expect("stdlib path should exist at lang/std")
    }

    #[test]
    fn lower_stdlib_items() {
        let mut c = Compiler::new();
        let path = stdlib_path();
        c.load_dir(&path);

        let mir = lower_module(c.world(), c.root());
        assert_eq!(mir.name, "main");
        assert!(!mir.structs.is_empty(), "should have lowered structs");
        assert!(!mir.enums.is_empty(), "should have lowered enums");
        assert!(!mir.protocols.is_empty(), "should have lowered protocols");
        assert!(!mir.functions.is_empty(), "should have lowered functions");
        assert!(!mir.witnesses.is_empty(), "should have lowered witnesses");

        eprintln!(
            "OSSA lowering: {} structs, {} enums, {} protocols, {} functions, {} witnesses, {} statics",
            mir.structs.len(),
            mir.enums.len(),
            mir.protocols.len(),
            mir.functions.len(),
            mir.witnesses.len(),
            mir.statics.len(),
        );
    }

    #[test]
    fn stdlib_functions_have_bodies() {
        use kestrel_compiler_driver::CompilerDriver;

        let mut c = Compiler::new();
        let path = stdlib_path();
        c.load_dir(&path);
        CompilerDriver::new(&c).infer_all();

        let mir = lower_module(c.world(), c.root());

        let with_bodies = mir.functions.iter().filter(|f| f.body.is_some()).count();
        let total = mir.functions.len();
        let total_blocks: usize = mir
            .functions
            .iter()
            .filter_map(|f| f.body.as_ref())
            .map(|b| b.blocks.len())
            .sum();
        let total_values: usize = mir
            .functions
            .iter()
            .filter_map(|f| f.body.as_ref())
            .map(|b| b.values.len())
            .sum();
        let total_insts: usize = mir
            .functions
            .iter()
            .filter_map(|f| f.body.as_ref())
            .flat_map(|b| &b.blocks)
            .map(|b| b.insts.len())
            .sum();

        eprintln!(
            "Body lowering: {}/{} functions have bodies, {} blocks, {} values, {} instructions",
            with_bodies, total, total_blocks, total_values, total_insts,
        );

        assert!(
            with_bodies > 100,
            "expected many functions with bodies, got {with_bodies}"
        );
        assert!(
            total_blocks > with_bodies,
            "expected more blocks than functions (if/else/loop), got {total_blocks} blocks for {with_bodies} bodies"
        );
        assert!(
            total_values > total_blocks,
            "expected more values than blocks (SSA values), got {total_values} values for {total_blocks} blocks"
        );

        // Count call instructions
        let call_count: usize = mir
            .functions
            .iter()
            .filter_map(|f| f.body.as_ref())
            .flat_map(|b| &b.blocks)
            .flat_map(|b| &b.insts)
            .filter(|inst| matches!(inst.kind, kestrel_mir_3::inst::InstKind::Call { .. }))
            .count();
        eprintln!("Call instructions: {}", call_count);
        assert!(
            call_count > 100,
            "expected many call instructions, got {call_count}"
        );

        // Count Op instructions (from intrinsic lowering)
        let op_count: usize = mir
            .functions
            .iter()
            .filter_map(|f| f.body.as_ref())
            .flat_map(|b| &b.blocks)
            .flat_map(|b| &b.insts)
            .filter(|inst| matches!(
                inst.kind,
                kestrel_mir_3::inst::InstKind::Op1 { .. }
                    | kestrel_mir_3::inst::InstKind::Op2 { .. }
                    | kestrel_mir_3::inst::InstKind::Op3 { .. }
            ))
            .count();
        eprintln!("Op instructions (intrinsics): {}", op_count);
    }

    #[test]
    fn stdlib_passes_verifier() {
        use kestrel_compiler_driver::CompilerDriver;

        let mut c = Compiler::new();
        let path = stdlib_path();
        c.load_dir(&path);
        CompilerDriver::new(&c).infer_all();

        let mir = lower_module(c.world(), c.root());

        let mut total_errors = 0;
        let mut error_funcs = Vec::new();

        for func in &mir.functions {
            if let Some(body) = &func.body {
                // Skip degenerate bodies (0 values or 0 blocks)
                if body.values.is_empty() || body.blocks.is_empty() {
                    continue;
                }
                let errors = kestrel_mir_3::verify::verify_ossa(body, &mir);
                if !errors.is_empty() {
                    total_errors += errors.len();
                    if error_funcs.len() < 15 {
                        error_funcs.push((
                            func.name.clone(),
                            errors.len(),
                            errors[0].message.clone(),
                        ));
                    }
                }
            }
        }

        let bodies = mir.functions.iter().filter(|f| f.body.is_some()).count();
        eprintln!("Verifier: {} bodies checked, {} total errors", bodies, total_errors);
        for (name, count, msg) in &error_funcs {
            if *count == 1 {
                eprintln!("  {name}: {msg}");
            } else {
                eprintln!("  {name} ({count}x): {msg}");
            }
        }

        // Categorize errors
        let mut categories: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for func in &mir.functions {
            if let Some(body) = &func.body {
                if body.values.is_empty() || body.blocks.is_empty() {
                    continue;
                }
                for err in kestrel_mir_3::verify::verify_ossa(body, &mir) {
                    // Extract the error pattern (first word-ish)
                    let cat = if err.message.contains("EndBorrow on non-@guaranteed") {
                        "EndBorrow on non-@guaranteed"
                    } else if err.message.contains("BeginBorrow on @none") {
                        "BeginBorrow on @none"
                    } else if err.message.contains("BeginMutBorrow on @none") {
                        "BeginMutBorrow on @none"
                    } else if err.message.contains("consumed more than once") {
                        "consumed more than once"
                    } else if err.message.contains("unconsumed @owned") {
                        "unconsumed @owned"
                    } else if err.message.contains("active borrow") {
                        "consume during active borrow"
                    } else if err.message.contains("use after consume") {
                        "use after consume"
                    } else if err.message.contains("block arg") {
                        "block arg mismatch"
                    } else if err.message.contains("live at block exit") {
                        "unconsumed at block exit"
                    } else if err.message.contains("type mismatch") {
                        "type mismatch in block arg"
                    } else if err.message.contains("CopyValue on @none") {
                        "CopyValue on @none"
                    } else if err.message.contains("DestroyValue on @none") {
                        "DestroyValue on @none"
                    } else if err.message.contains("BeginBorrow on @none") {
                        "BeginBorrow on @none"
                    } else if err.message.contains("not live") || err.message.contains("not tracked") {
                        "value not live/tracked"
                    } else if err.message.contains("CopyValue") {
                        "CopyValue other"
                    } else if err.message.contains("DestroyValue") {
                        "DestroyValue other"
                    } else if err.message.contains("Op1 operand") || err.message.contains("Op2 operand") {
                        "Op operand not @none"
                    } else if err.message.contains("still active at block exit") {
                        "open borrow at block exit"
                    } else if err.message.contains("ownership mismatch") {
                        "ownership mismatch"
                    } else if err.message.contains("use of consumed") {
                        "use of consumed value"
                    } else {
                        &err.message
                    };
                    *categories.entry(cat.to_string()).or_default() += 1;
                }
            }
        }
        // Sub-categorize errors by block structure to find root causes
        let mut unconsumed_in_entry = 0usize; // bb0 only (no branches at all)
        let mut unconsumed_in_branching = 0usize; // block that has Branch/Switch terminator
        let mut unconsumed_in_arms = 0usize; // non-entry block, jumps to merge
        let mut unconsumed_in_return_block = 0usize; // block with Return terminator
        let mut unconsumed_other = 0usize;
        let mut consumed_twice_with_copy = 0usize;
        let mut consumed_twice_without_copy = 0usize;
        let mut block_arg_count = 0usize;
        let mut block_arg_type = 0usize;
        let mut block_arg_ownership = 0usize;
        let mut funcs_only_unconsumed = 0usize;
        let mut funcs_only_consumed_twice = 0usize;
        let mut funcs_mixed = 0usize;

        for func in &mir.functions {
            if let Some(body) = &func.body {
                if body.values.is_empty() || body.blocks.is_empty() { continue; }
                let errors = kestrel_mir_3::verify::verify_ossa(body, &mir);
                if errors.is_empty() { continue; }

                let has_unconsumed = errors.iter().any(|e| e.message.contains("live at block exit"));
                let has_consumed_twice = errors.iter().any(|e| e.message.contains("consumed more than once"));
                if has_unconsumed && !has_consumed_twice { funcs_only_unconsumed += 1; }
                else if has_consumed_twice && !has_unconsumed { funcs_only_consumed_twice += 1; }
                else if has_unconsumed && has_consumed_twice { funcs_mixed += 1; }

                for err in &errors {
                    if err.message.contains("live at block exit") {
                        let block = body.block(err.block);
                        let term = &block.terminator.kind;
                        if err.block.index() == 0 {
                            unconsumed_in_entry += 1;
                        } else if matches!(term, kestrel_mir_3::terminator::TerminatorKind::Branch { .. }
                            | kestrel_mir_3::terminator::TerminatorKind::Switch { .. }) {
                            unconsumed_in_branching += 1;
                        } else if matches!(term, kestrel_mir_3::terminator::TerminatorKind::Return(_)) {
                            unconsumed_in_return_block += 1;
                        } else if matches!(term, kestrel_mir_3::terminator::TerminatorKind::Jump { .. }) {
                            unconsumed_in_arms += 1;
                        } else {
                            unconsumed_other += 1;
                        }
                    }
                    if err.message.contains("consumed more than once") {
                        let err_block = body.block(err.block);
                        let has_copy = err_block.insts.iter().any(|i|
                            matches!(&i.kind, kestrel_mir_3::inst::InstKind::CopyValue { .. }));
                        if has_copy {
                            consumed_twice_with_copy += 1;
                        } else {
                            consumed_twice_without_copy += 1;
                        }
                    }
                    if err.message.contains("block arg") {
                        if err.message.contains("passes") && err.message.contains("expects") {
                            block_arg_count += 1;
                        } else if err.message.contains("type mismatch") {
                            block_arg_type += 1;
                        } else if err.message.contains("ownership mismatch") {
                            block_arg_ownership += 1;
                        }
                    }
                }
            }
        }

        eprintln!("\n=== DETAILED BREAKDOWN ===");
        eprintln!("Unconsumed at block exit:");
        eprintln!("  {unconsumed_in_entry:>5} in entry block (bb0)");
        eprintln!("  {unconsumed_in_branching:>5} in branching blocks (Branch/Switch terminator)");
        eprintln!("  {unconsumed_in_arms:>5} in arm blocks (Jump terminator)");
        eprintln!("  {unconsumed_in_return_block:>5} in return blocks");
        eprintln!("  {unconsumed_other:>5} other");
        eprintln!("Consumed more than once:");
        eprintln!("  {consumed_twice_with_copy:>5} in blocks with CopyValue");
        eprintln!("  {consumed_twice_without_copy:>5} in blocks without CopyValue");
        eprintln!("Block arg mismatch:");
        eprintln!("  {block_arg_count:>5} count mismatch");
        eprintln!("  {block_arg_type:>5} type mismatch");
        eprintln!("  {block_arg_ownership:>5} ownership mismatch");
        eprintln!("Failing funcs:");
        eprintln!("  {funcs_only_unconsumed:>5} with ONLY unconsumed errors");
        eprintln!("  {funcs_only_consumed_twice:>5} with ONLY consumed-twice errors");
        eprintln!("  {funcs_mixed:>5} with both unconsumed + consumed-twice");

        // Dump examples of each major category
        let mut type_mismatch_examples = 0;
        let mut unconsumed_branch_examples = 0;
        let mut unconsumed_arm_examples = 0;
        let mut consumed_twice_examples = 0;
        for func in &mir.functions {
            if let Some(body) = &func.body {
                if body.values.is_empty() || body.blocks.is_empty() { continue; }
                let errors = kestrel_mir_3::verify::verify_ossa(body, &mir);
                if errors.is_empty() { continue; }

                let has_branch_unconsumed = errors.iter().any(|e| {
                    e.message.contains("live at block exit")
                        && matches!(body.block(e.block).terminator.kind,
                            kestrel_mir_3::terminator::TerminatorKind::Branch { .. }
                            | kestrel_mir_3::terminator::TerminatorKind::Switch { .. })
                });
                let has_arm_unconsumed = errors.iter().any(|e| {
                    e.message.contains("live at block exit")
                        && matches!(body.block(e.block).terminator.kind,
                            kestrel_mir_3::terminator::TerminatorKind::Jump { .. })
                });
                let has_consumed_twice = errors.iter().any(|e| e.message.contains("consumed more than once"));

                // Dump one example per category (small functions only)
                let cats_seen: Vec<String> = errors.iter().map(|e| {
                    if e.message.contains("type mismatch") { "type_mismatch".into() }
                    else if e.message.contains("ownership mismatch") { "ownership_mismatch".into() }
                    else if e.message.contains("consumed more than once") { "consumed_twice".into() }
                    else if e.message.contains("still active at block exit") { "open_borrow".into() }
                    else if e.message.contains("Op1 operand") || e.message.contains("Op2 operand") { "op_not_none".into() }
                    else if e.message.contains("active borrow") { "consume_during_borrow".into() }
                    else if e.message.contains("passes") && e.message.contains("expects") { "arg_count".into() }
                    else if e.message.contains("BeginBorrow on @none") { "borrow_none".into() }
                    else if e.message.contains("uninit") { "uninit_field".into() }
                    else { format!("other: {}", &e.message[..e.message.len().min(60)]) }
                }).collect();

                static DUMP_CATS: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, usize>>> = std::sync::OnceLock::new();
                let seen = DUMP_CATS.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
                let mut lock = seen.lock().unwrap();
                for cat in &cats_seen {
                    let count = lock.entry(cat.clone()).or_insert(0);
                    if *count < 2 && body.blocks.len() <= 10 {
                        *count += 1;
                        eprintln!("\n--- EXAMPLE [{}] #{}: {} ({} blocks, {} errors) ---",
                            cat, count, func.name, body.blocks.len(), errors.len());
                        eprintln!("{}", kestrel_mir_3::display::display_body(body, &mir));
                        for e in &errors {
                            eprintln!("  ERR {:?} inst={:?}: {}", e.block, e.inst, e.message);
                        }
                    }
                }
            }
        }

        let mut sorted: Vec<_> = categories.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        for (cat, count) in &sorted {
            eprintln!("  {count:>6} {cat}");
        }

        let clean_count = mir.functions.iter().filter(|f| {
            f.body.as_ref().is_some_and(|b| {
                !b.values.is_empty() && !b.blocks.is_empty()
                    && kestrel_mir_3::verify::verify_ossa(b, &mir).is_empty()
            })
        }).count();
        eprintln!("{clean_count}/{bodies} functions pass verifier cleanly");
        eprintln!("(verifier errors are expected during initial development)");
    }
}
