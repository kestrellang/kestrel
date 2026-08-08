// test: execution
// stdlib: false
// backends: cranelift,llvm
// expect-exit: 12

// `lang.atomic_add` is seeded generic over `T`, so a narrower pointee than
// i64 is well-typed. The cranelift backend hardcoded `ir::types::I64` as the
// access width, which the cranelift verifier rejected ("arg 1 has type i32,
// expected i64") — narrow atomics simply did not compile. The width now comes
// from the value operand, matching the LLVM backend. Pinned to both backends
// so they cannot diverge again.

module Test

@main
func main() -> lang.i64 {
    var slot: lang.i32 = 7;
    let p = lang.ptr_to(slot);
    let old = lang.atomic_add(p, 5);
    lang.cast_i32_i64(lang.ptr_read(p))
}
