// test: execution
// backends: cranelift,llvm
// stdlib: true

// A `not Static` owner relaxes its own params wholesale: `Foo[T]: not
// Static { var v: T }` is instantiable with a non-Static argument without
// spelling `where T: not Static`.

module Test

struct Handle: not Static {
    var id: Int64
}

struct Foo[T]: not Static {
    var v: T
}

@main
func main() {
    let f = Foo(v: Handle(id: 9));
    if f.v.id != 9 {
        fatalError("not-Static owner should accept non-Static args");
    }
    print("ok");
}

// CHECK: ok
