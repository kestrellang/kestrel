// test: diagnostics
// stdlib: false

// E499: a borrow names an existing PLACE — rvalues (literals, call
// results, arithmetic) have none.
module Test

func make() -> lang.i64 { 7 }

func f() {
    let a = &5; // ERROR(E499)
    let b = &make(); // ERROR(E499)
    let c = &lang.i64_add(1, 2); // ERROR(E499)
}
