// test: diagnostics
// stdlib: true

// References 2b: nesting stays banned — `& &T` is E487 even where a
// single ref would be a legal type argument.
module Test

func f() {
    let o: Optional[& &Int64] = .None; // ERROR(E487)
}
