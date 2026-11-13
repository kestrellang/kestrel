// test: diagnostics
// stdlib: true

// A C function type `@extern(.C) (…) -> …` holds a C address, so only a
// function with a C entry point (`@export(.C)` or `@extern(.C)`) can be named
// as one; an ordinary function can't (E647) and a closure has no address at
// all (E624). `NullableCFunction[F]` needs a C function type `F` (E647).

module Test

type IntFn = @extern(.C) (Int64) -> Int64

@export(.C)
func exported(x: Int64) -> Int64 { x }

func plain(x: Int64) -> Int64 { x }

@extern(.C, mangleName: "labs")
func labs(x: Int64) -> Int64

func uses() {
    let ok: IntFn = exported;
    let alsoOk: IntFn = unsafe labs;
    let nullable = NullableCFunction(ok);
    let bad: IntFn = plain; // ERROR: 'plain' has no C entry point; a C function type needs a function declared '@export(.C)' or '@extern(.C)'
    let closure: IntFn = { (x: Int64) in x }; // ERROR: closure kind mismatch: expected a C function, found a normal closure
    let notC = NullableCFunction(plain); // ERROR: 'NullableCFunction' needs a C function type '@extern(.C) (…) -> …'
}
