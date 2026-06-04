//! Body-level analyzers — checks that operate on function/init bodies.

pub mod assignment;
pub mod condition_check;
pub mod dead_code;
pub mod exhaustive_return;
pub mod guard;
pub mod param_pattern;
pub mod type_check;

// Wave 5: Complex body checks
pub mod access_mode;
pub mod closure;
pub mod definite_assignment;
pub mod initializer;
// HIR-level move/use-after-move checker (E500/E501). Restored from the
// pre-MIR-rewrite (0.15) tree: it models `while-let` loops, pattern re-binding
// across iterations, and other control-flow shapes the OSSA verifier currently
// only catches as ICE-shaped "copy of non-Copyable" failures.
// TODO(move-checker): migrate this to a MIR/OSSA-level pass (the retired
// `kestrel-ownership::move_check`) once that check models the control-flow
// patterns this one encodes; the MIR pass is the single source of truth for
// ownership and would emit these diagnostics without a duplicate HIR dataflow.
pub mod move_tracking;

// Wave 6: Pattern checks
pub mod exhaustiveness;
pub mod for_loop_pattern;
pub mod match_pattern;
pub mod refutable_pattern;

// Literal/lexing checks (E700-E799)
pub mod string_escape;
