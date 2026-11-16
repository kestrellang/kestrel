// test: diagnostics
// stdlib: false
//
// A feature binding may name a role alias, which names the box: the check
// follows the chain to the struct (`AnyBox` -> `Shared` -> `GoodBox`).

module Test

@builtin(.Box)
protocol Box {}

struct GoodBox[T] {}
extend GoodBox[T]: Box {}

type Shared[T] = GoodBox[T]

@builtin(.AnyBox)
type AnyBox[T] = Shared[T]
