// test: diagnostics

// A match arm that binds a non-copyable payload by value moves it out of the
// scrutinee. Using the scrutinee again afterwards is a use after move (it
// used to pass the checker and fail OSSA verification).

module Test

struct R: not Copyable { var v: Int64; }

func twice(consuming o: Optional[R]) -> Int64 {
    let a = match o { .Some(r) => r.v, .None => 0 };
    match o { .Some(r) => r.v, .None => 0 } // ERROR: value 'o' may have been moved
}

func borrowOnly(consuming o: Optional[R]) -> Int64 {
    let a = match o { .Some(&r) => r.v, .None => 0 };
    match o { .Some(r) => r.v + a, .None => a }
}
