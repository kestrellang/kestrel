// test: execution
// stdlib: true
// expect-exit: 0
//
// #215: a structural unit conformance `extend (): P` must support a static
// `-> Self` requirement (Self == (), so `make() -> ()` is a valid witness) and
// register associated-type bindings (`type Mark = …`), dispatched through
// generics. Before the fix: `make() -> ()` falsely got E458 "wrong return type"
// (Self normalized to Named(lang.()) not the structural Tuple([])), and `().Mark`
// failed ("no associated type 'Mark'") because assoc resolution had no case for
// the structural `()` type.

module Test

protocol Ident {
    type Mark
    static func make() -> Self
    func label() -> String
    func mark() -> Mark
}

extend (): Ident {
    type Mark = Int64;
    static func make() -> () { () }
    func label() -> String { "unit" }
    func mark() -> Int64 { 3 }
}

func makeLabelG[T]() -> String where T: Ident { T.make().label() } // static -> Self
func markG[T](x: T) -> T.Mark where T: Ident { x.mark() }          // assoc projection

@main
func main() -> lang.i32 {
    let u = ();
    if makeLabelG[()]() != "unit" { return 1 } // static `make() -> Self` witness
    if markG(u) != 3 { return 2 }              // `().Mark` resolves to Int64
    0
}
