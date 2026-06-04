// test: execution
// stdlib: true
// expect-exit: 0

// Regression: 8-byte Named wrappers (CString, Int64, UInt64, Pointer[T]) used
// to be passed to an `@extern(.C)` function as the address of the wrapper
// aggregate instead of the inner scalar — `compile_extern_call_arg` only
// loaded the inner value when the flattened scalar width differed from the
// SSA pointer width, and pointer-width wrappers tunnelled straight through.
// With that bug `strlen(cstr)` walks arbitrary stack bytes and returns a
// length unrelated to the input string.
//
// See lib/kestrel-codegen-cranelift/src/rvalue/call.rs::compile_extern_call_arg.

module Test

@extern(.C, mangleName: "strlen")
func strlen(s: CString) -> lang.i64

@main
func main() -> lang.i64 {
    let cstr = "kestrel".toCString();
    let n = strlen(cstr);
    cstr.free();
    if lang.i64_ne(n, 7) { return 1 }
    0
}
