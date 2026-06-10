// test: diagnostics
// stdlib: true

// Call-shaped compound-assign LHS is admitted syntactically (it may
// return `&mutating T`) and validated by the assignment analyzer: a call
// returning a plain VALUE is a temporary, not a place — E202. Subscript
// reads (`arr(0)`) are value calls too; writeback compound assignment is
// the stage-1.5 call-as-place item. A `&T`-returning call is E207
// (mutating use of a shared ref), not E202.
module Test

func five() -> Int64 { 5 }

@main
func main() {
    var arr = [1, 2, 3];
    five() += 1; // ERROR(E202)
    arr(0) += 10; // ERROR(E202)
    arr.at(index: 0) += 1; // ERROR(E207)
}
