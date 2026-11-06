// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

module Test

// A closure that captures a BORROWED generic parameter by view, then consumes
// it (passes it to a `consuming` parameter), must copy: the capture slot
// aliases the caller's value, and the closure may run again. It used to be
// moved out (`take`) because the read was the local's only one — which made
// `Deque[String].pushBack` store an alias of the caller's string (crash on
// the next pop) and would empty the slot before a second call.

func consume[T](consuming v: T) -> Int64 { 1 }

func twice(body: () -> Int64) -> Int64 { body() + body() }

func viaClosure[T](x: T) -> Int64 {
    twice { consume(x) }
}

@main
func main() -> Int64 {
    let s = "heap-string-\(42)";
    if viaClosure(s) != 2 { return 1 }
    if viaClosure(s) != 2 { return 2 }
    // The caller's string is intact after both calls.
    if s != "heap-string-42" { return 3 }
    0
}
