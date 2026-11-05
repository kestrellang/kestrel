// test: diagnostics
// stdlib: true

// `CFunction(f)` needs a function with a C entry point: an `@export(.C)` or
// `@extern(.C)` one. An ordinary function or a closure has none (E646).

module Test

@export(.C)
func exported(x: Int64) -> Int64 { x }

func plain(x: Int64) -> Int64 { x }

@extern(.C, mangleName: "labs")
func labs(x: Int64) -> Int64

func uses() {
    let ok = CFunction(exported);
    let alsoOk = unsafe CFunction(labs);
    let bad = CFunction(plain); // ERROR: needs a function declared '@export(.C)'
    let closure: (Int64) -> Int64 = { (x: Int64) in x };
    let alsoBad = CFunction(closure); // ERROR: needs a function declared '@export(.C)'
}
