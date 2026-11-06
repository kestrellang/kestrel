// test: diagnostics

// Payload sub-patterns bind by position, like call arguments. A written label
// must name the payload at its position; a swapped, unknown, or duplicate
// label is an error (it used to be ignored, silently binding by position).

module Test

enum E {
    case A(x: Int64, y: Int64)
    case B(Int64)
}

func swapped(e: E) -> Int64 {
    match e {
        .A(y: b, x: a) => a + b, // ERROR: wrong argument label
        .B(n) => n
    }
}

func unknown(e: E) -> Int64 {
    match e {
        .A(z: a, y: b) => a + b, // ERROR: wrong argument label
        .B(n) => n
    }
}

func duplicate(e: E) -> Int64 {
    match e {
        .A(x: a, x: b) => a + b, // ERROR: wrong argument label
        .B(n) => n
    }
}
