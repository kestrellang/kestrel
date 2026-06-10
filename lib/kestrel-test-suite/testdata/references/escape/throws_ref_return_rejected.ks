// test: diagnostics
// stdlib: false

// E-REF-17: a ref can never ride inside an effect/sugar wrapper — `throws`
// desugars the return to `Result[&T, E]`, a ref-in-enum-payload backdoor
// the ret_borrow ABI cannot express. (The explicit `Optional[&T]` /
// `Result[&T, E]` annotations stay E485 via the stage-0.5 walk —
// rejected_syntax/generic_arg_ref_rejected.ks.)
module Test

struct Err {}

func bad(x: lang.i64) -> &lang.i64 throws Err { x } // ERROR(E490)
