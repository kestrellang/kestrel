pub mod clone_shim;
pub mod copy_check;
pub mod copy_propagation;
pub mod drop_fix;
pub mod drop_shim;
pub mod layout;
pub mod thunk;

use crate::MirModule;
use crate::item::TargetConfig;
use crate::verify::VerifyError;

/// A single observable point in the MIR (OSSA) lowering → codegen pipeline.
///
/// The variant order IS the pipeline order (and equals [`Stage::ORDER`]); the
/// derived `Ord` is what powers the `stop >= Stage::X` gating in
/// [`run_pipeline_until`]. This is the single source of truth for stage
/// identity, naming, and ordering — shared by `kestrel dump mir -s <stage>`.
///
/// The first seven stages are pre-mono and produce a [`MirModule`]; the last
/// three are post-mono and produce a `mono::MonoModule` (the post-mono passes
/// are orchestrated in `kestrel-compiler`, not here).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    /// Raw lowering output — no passes, no verify.
    Raw,
    /// After `drop_fix::fix_drop_behaviors`.
    DropFix,
    /// After `thunk::run_thunk_pass`.
    Thunk,
    /// After `drop_shim::synthesize_drop_shims`.
    DropShim,
    /// After `clone_shim::synthesize_clone_shims`.
    CloneShim,
    /// After `layout::run_layout_pass`.
    Layout,
    /// After `verify_ossa` + `copy_check` — the default `kestrel dump mir` point.
    Verify,
    /// After `mono::monomorphize`.
    Mono,
    /// After `copy_propagation::eliminate_redundant_copies`.
    CopyProp,
    /// After `mono::expand::expand_destroy_copy` (+ `verify_mono`) — feeds codegen.
    Expand,
}

impl Stage {
    /// All stages in pipeline order. Must match the variant declaration order.
    pub const ORDER: [Stage; 10] = [
        Stage::Raw,
        Stage::DropFix,
        Stage::Thunk,
        Stage::DropShim,
        Stage::CloneShim,
        Stage::Layout,
        Stage::Verify,
        Stage::Mono,
        Stage::CopyProp,
        Stage::Expand,
    ];

    /// Stable kebab-case name (matches the `--stage` flag spelling).
    pub fn name(self) -> &'static str {
        match self {
            Stage::Raw => "raw",
            Stage::DropFix => "drop-fix",
            Stage::Thunk => "thunk",
            Stage::DropShim => "drop-shim",
            Stage::CloneShim => "clone-shim",
            Stage::Layout => "layout",
            Stage::Verify => "verify",
            Stage::Mono => "mono",
            Stage::CopyProp => "copy-prop",
            Stage::Expand => "expand",
        }
    }

    /// Parse a kebab-case stage name (inverse of [`Stage::name`]).
    pub fn from_name(s: &str) -> Option<Stage> {
        Stage::ORDER.into_iter().find(|st| st.name() == s)
    }

    /// True for the seven stages that produce a [`MirModule`] (`Raw..=Verify`).
    pub fn is_pre_mono(self) -> bool {
        self <= Stage::Verify
    }

    /// True for the three stages that produce a `mono::MonoModule`.
    pub fn is_post_mono(self) -> bool {
        !self.is_pre_mono()
    }
}

/// Run the full pre-codegen OSSA pipeline:
/// drop_fix → thunk → drop_shim → clone_shim → layout → ossa_verify.
pub fn run_pipeline(
    module: &mut MirModule,
    target: &TargetConfig,
    next_entity: &mut u32,
) -> Vec<VerifyError> {
    run_pipeline_until(module, target, next_entity, Stage::Verify)
}

/// Run the pre-mono pipeline, stopping after `stop` (inclusive).
///
/// Each pass is gated on `stop >= Stage::<pass>`, so e.g. `Stage::Layout` runs
/// everything through layout but skips verify. Verify (`verify_ossa` +
/// `copy_check`) runs only at `Stage::Verify`; earlier stops return no errors,
/// letting callers dump a not-yet-verified (possibly malformed) module — the
/// whole point of inspecting intermediate stages.
///
/// Panics in debug builds if `stop` is a post-mono stage (those are run by
/// `kestrel-compiler`'s `monomorphize_mir_until`, not here).
pub fn run_pipeline_until(
    module: &mut MirModule,
    target: &TargetConfig,
    next_entity: &mut u32,
    stop: Stage,
) -> Vec<VerifyError> {
    debug_assert!(
        stop.is_pre_mono(),
        "run_pipeline_until got post-mono stage {stop:?}"
    );

    if stop >= Stage::DropFix {
        drop_fix::fix_drop_behaviors(module);
    }
    if stop >= Stage::Thunk {
        thunk::run_thunk_pass(module, next_entity);
    }
    if stop >= Stage::DropShim {
        drop_shim::synthesize_drop_shims(module, next_entity);
    }
    if stop >= Stage::CloneShim {
        clone_shim::synthesize_clone_shims(module, next_entity);
    }
    if stop >= Stage::Layout {
        layout::run_layout_pass(module, target);
    }
    if stop < Stage::Verify {
        return Vec::new();
    }

    let mut errors = Vec::new();
    for func in module.functions.values() {
        if let Some(body) = &func.body {
            if body.values.is_empty() || body.blocks.is_empty() {
                continue;
            }
            let func_errors = crate::verify::verify_ossa(body, module, &func.name, func.entity);
            errors.extend(func_errors);
        }
    }
    // Flags every CopyValue/CopyAddr of a non-Copyable value. WIP: the MIR
    // lowering still copies non-Copyable values by design in mainstream paths
    // (closures called per-element, resource handles like File passed around),
    // so this currently fails ~550 tests until the lowering's move/copy
    // decision is fixed to borrow/move those instead of copying.
    errors.extend(copy_check::check_copies(module));
    // References stage 1: the root rule for ret_borrow functions — returned
    // borrows must root at Param/Static/PointerDerived (user diagnostics
    // E494-E496, not ICEs).
    errors.extend(crate::verify::check_escapes(module));
    errors
}
