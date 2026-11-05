// test: diagnostics
// stdlib: false
//
// E641: an `unsafe` that covers no unsafe use is reported. A use marks only
// the innermost `unsafe` around it, so an outer one wrapping an inner one is
// unnecessary too.

module Test

unsafe func poke() -> lang.i64 { 1 }

func safeOne() -> lang.i64 { 2 }

func a() -> lang.i64 {
    unsafe safeOne() // WARN: unnecessary `unsafe`: it covers no unsafe operation
}

func b() -> lang.i64 {
    unsafe { // WARN: unnecessary `unsafe`: it covers no unsafe operation
        unsafe poke()
    }
}
