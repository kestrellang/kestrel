// test: execution
// stdlib: true
// expect-exit: 3

// Two conformances to the SAME parameterized protocol with DIFFERENT type args,
// whose witness methods differ only by their RETURN type (`produce() -> Out`).
// Both must be accepted, and `s.produce()` must dispatch by the expected
// (context) return type -> a = 1 (Producer[Int64]), b = 2 (Producer[Int32]);
// 1 + 2 = 3.
//
// This exercises three layers that must agree (previously each was wrong, giving
// a spurious E458 at the declaration, then mis-dispatch):
//   1. analyzer (conformance_completeness.rs) — among arity+label-matching impls,
//      pick the one whose return type matches THIS instantiation (not first-found);
//   2. solver (solver.rs) — with >1 conformance to the protocol, leave the proto
//      param a fresh var so the expected return type selects the instantiation,
//      instead of pinning it to the first conformance;
//   3. witness lowering (witness_lower.rs) — `prefer_source` when the protocol
//      args are concrete, so each witness binds its own extension's `produce`.
//
// Sibling coverage: multi_conformance_distinct_instantiation_no_conflict.ks
// pins the ARGUMENT-disambiguated declaration (no E412, but never executed).
// This file pins the RETURN-disambiguated case AND its runtime dispatch.
module Main

protocol Producer[Out] { func produce() -> Out }

struct S { var x: Int64 }

extend S: Producer[Int64] { func produce() -> Int64 { 1 } }
extend S: Producer[Int32] { func produce() -> Int32 { 2 } }

@main
func main() -> Int64 {
    let s = S(x: 0);
    let a: Int64 = s.produce();
    let b: Int32 = s.produce();
    a + Int64(from: b) // 1 + 2 = 3
}
