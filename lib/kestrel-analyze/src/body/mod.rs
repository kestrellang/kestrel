//! Body-level analyzers — checks that operate on function/init bodies.

pub mod assignment;
pub mod condition_check;
pub mod dead_code;
pub mod exhaustive_return;
pub mod guard_let;
pub mod param_pattern;
pub mod type_check;

// Wave 5: Complex body checks
pub mod access_mode;
pub mod closure;
pub mod definite_assignment;
pub mod initializer;
pub mod move_tracking;

// Wave 6: Pattern checks
pub mod exhaustiveness;
pub mod for_loop_pattern;
pub mod match_pattern;
pub mod refutable_pattern;

// Literal/lexing checks (E700-E799)
pub mod string_escape;
