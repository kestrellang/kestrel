// test: diagnostics
// stdlib: true

// Call-shaped compound-assign LHS is admitted syntactically (it may
// return `&mutating T`) and validated by the assignment analyzer: a call
// returning a plain VALUE is a temporary, not a place — E202. A
// `&T`-returning call is E207 (mutating use of a shared ref), not E202.
// NOTE: `arr(0) += 1` is NO LONGER rejected — subscripts with get/set
// run the stage-1.5 writeback (accessor_writeback_* execution tests);
// an immutable BASE is rejected by the access-mode analyzer instead
// (compound_assign_immutable_base_rejected.ks).
module Test

func five() -> Int64 { 5 }

@main
func main() {
    var arr = [1, 2, 3];
    five() += 1; // ERROR(E202)
    arr.at(index: 0) += 1; // ERROR(E207)
}
