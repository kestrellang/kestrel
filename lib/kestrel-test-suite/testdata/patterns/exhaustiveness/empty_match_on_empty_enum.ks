// test: diagnostics

// An enum with no cases has no values, so a `match` with no arms is
// exhaustive on it (like `Never`). An empty match on any other type is not.

module Test

enum Empty {}

func absurd(e: Empty) -> Int64 {
    match e { }
}

func notEmpty(b: Bool) -> Int64 {
    match b { } // ERROR: empty match on inhabited type
}
