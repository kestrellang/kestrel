// test: execution
// stdlib: true
// expect-exit: 0
//
// A static method used as a function value captures nothing, so like a free
// function it satisfies every closure kind, including `escaping`. Today a free
// function passes but `H.twice` is classed as a normal (frame-view) closure
// and rejected with E624. Found 2026-11-01 building perch-2, whose routes
// register `app.on(Get("/:id"), Notes.read)` into an escaping slot.

module Test

struct H { static func twice(x: Int64) -> Int64 { x * 2 } }
func twiceFree(x: Int64) -> Int64 { x * 2 }

struct Store { var f: escaping (Int64) -> Int64 }
func store(f: escaping (Int64) -> Int64) -> Store { Store(f: f) }

@main
func main() -> Int64 {
    let a = store(twiceFree);
    let b = store(H.twice);
    (a.f)(2) + (b.f)(3) - 10
}
