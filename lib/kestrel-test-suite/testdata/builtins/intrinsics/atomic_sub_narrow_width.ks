// test: execution
// stdlib: false
// backends: cranelift,llvm
// expect-exit: 32

// Companion to atomic_add_narrow_width.ks — see that file for the history.

module Test

@main
func main() -> lang.i64 {
    var slot: lang.i32 = 40;
    let p = lang.ptr_to(slot);
    let old = lang.atomic_sub(p, 8);
    lang.cast_i32_i64(lang.ptr_read(p))
}
