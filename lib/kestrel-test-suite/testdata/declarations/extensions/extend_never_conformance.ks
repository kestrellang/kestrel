// test: execution
// stdlib: false
// expect-exit: 0

// The never type `!` can conform to a protocol. `!` is uninhabited, so the
// method body is unreachable and can't be called — but the conformance must
// type-check and produce a witness (implementing type = `MirTy::Never`). This
// is a compile-and-run smoke test that the synthetic `lang.!` entity is wired.
module Main

protocol Tag { func tag() -> lang.i64 }

extend !: Tag { func tag() -> lang.i64 { 0 } }

@main
func main() -> lang.i64 { 0 }
