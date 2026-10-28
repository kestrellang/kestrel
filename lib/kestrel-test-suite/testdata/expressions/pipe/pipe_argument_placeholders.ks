// test: execution
// stdlib: true
// expect-exit: 0

// Every `_` in the right-hand call's top-level arguments is the piped value,
// including `_` inside a larger argument. A `_` in a nested argument list
// keeps its placeholder meaning.

module Test

struct User {
    let name: String
    let id: Int64
    let age: Int64
}

func greet(name: String) -> String { "hi " + name }
func tag(u: User, id: Int64) -> Int64 { u.age * 1000 + id }
func plus(x: Int64) -> Int64 { x }
func applyTo(u: User, f: (User) -> Int64) -> Int64 { f(u) }

@main
func main() -> lang.i64 {
    let u = User(name: "ada", id: 7, age: 36);
    // `x |> f(_.name)` is `f(x.name)`
    if (u |> greet(_.name)) != "hi ada" { return 1 }
    // `x |> f(_ + 1)` is `f(x + 1)`
    if (4 |> plus(_ + 1)) != 5 { return 2 }
    // `x |> f(_, _.id)` is `f(x, x.id)`
    if (u |> tag(_, _.id)) != 36007 { return 3 }
    // nested: `_.age` inside `applyTo`'s own argument list stays a closure
    // over that argument; the top-level `_` is the piped value.
    if (u |> applyTo(_, plusAge(_.age))) != 36 { return 4 }
    0
}

func plusAge(f: (User) -> Int64) -> (User) -> Int64 { f }
