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
// HIR-level move tracker. Stage 7 of the memory-model rewrite stood up a
// MIR-level `kestrel-ownership::move_check` and intended to retire this
// one, but the MIR check's flat dataflow doesn't yet model `while-let`
// loops, pattern re-binding across iterations, or the other nuanced
// patterns this 956-line analyzer encodes. Both run in parallel today;
// the test harness dedupes overlapping E500/E501 emissions. Deleting
// this file is gated on the MIR check growing those features.
pub mod move_tracking;

// Wave 6: Pattern checks
pub mod exhaustiveness;
pub mod for_loop_pattern;
pub mod match_pattern;
pub mod refutable_pattern;

// Literal/lexing checks (E700-E799)
pub mod string_escape;
