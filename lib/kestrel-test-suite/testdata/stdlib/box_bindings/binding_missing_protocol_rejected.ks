// test: diagnostics
// stdlib: false
//
// E470: a box binding names a box that lacks the protocols its feature calls.
// `AnyBox` needs `Box`; `ClosureBox` needs `SharedBox`. The binding is
// checked where the alias is declared, not at the lowering site that would
// otherwise ICE on the missing requirement.

module Test

@builtin(.Box)
protocol Box {}

@builtin(.SharedBox)
protocol SharedBox: Box {}

struct PlainBox[T] {}
struct OnlyBox[T] {}
extend OnlyBox[T]: Box {}

@builtin(.AnyBox)
type AnyBox[T] = PlainBox[T] // ERROR: @builtin(.AnyBox) binds 'PlainBox', which does not conform to 'Box'

@builtin(.ClosureBox)
type ClosureBox[T] = OnlyBox[T] // ERROR: @builtin(.ClosureBox) binds 'OnlyBox', which does not conform to 'SharedBox'
