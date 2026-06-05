// test: execution
// stdlib: false
// expect-exit: 42

// An intrinsic `lang.*` type can conform to a protocol. Dispatch works through
// a generic function — i.e. via WITNESS dispatch at monomorphization, which
// requires the witness's implementing type to match the primitive self repr
// (`lang.i64` → `MirTy::I64`), not the nominal `lang.i64` entity.
module Main

protocol Doubler { func double() -> lang.i64 }

extend lang.i64: Doubler {
    func double() -> lang.i64 { return lang.i64_mul(self, 2); }
}

func doubleIt[T](x: T) -> lang.i64 where T: Doubler { x.double() }

@main
func main() -> lang.i64 {
    let x: lang.i64 = 21;
    doubleIt(x)
}
