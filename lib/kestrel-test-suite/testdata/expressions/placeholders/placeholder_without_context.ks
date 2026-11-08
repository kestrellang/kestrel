// test: diagnostics
// stdlib: true

// With no expected function type, nothing types an argument placeholder's
// parameter: the same error as the closure it stands for.

module Test

struct User {
    let name: String
}

func keep[T](x: T) -> T { x }

func test() {
    let f = keep(_.name); // ERROR: the type of this value must be known here to look up 'name'
    let g = keep({ it.name }); // ERROR: the type of this value must be known here to look up 'name'
}
