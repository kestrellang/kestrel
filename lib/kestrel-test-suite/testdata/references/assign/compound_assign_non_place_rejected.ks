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

struct Holder {
    var v: Int64
    func peek() -> &Int64 { self.v }
}

func five() -> Int64 { 5 }

@main
func main() {
    var h = Holder(v: 1);
    five() += 1; // ERROR(E202)
    h.peek() += 1; // ERROR(E207)
}
