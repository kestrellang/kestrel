// test: execution
// stdlib: true
// backends: cranelift,llvm

module Test

struct Resource: not Copyable {
    var value: Int64
}

@main
func main() -> lang.i64 {
    let optional: Optional[Resource] = .Some(Resource(value: 41));
    match optional.asRef() {
        .Some(value) => if value.value != 41 { return 1 },
        .None => return 2
    };

    // The projection borrows; the payload can still be moved afterward.
    let mapped = optional.map { it.value + 1 };
    if mapped.unwrap() != 42 { return 3 }

    let ok: Result[Resource, String] = .Ok(Resource(value: 7));
    match ok.okRef() {
        .Some(value) => if value.value != 7 { return 4 },
        .None => return 5
    };
    if ok.map({ (value) in value.value }).unwrap() != 7 { return 6 }

    let err: Result[Int64, String] = .Err("broken");
    match err.errRef() {
        .Some(value) => if value.isEqual(to: "broken") == false { return 7 },
        .None => return 8
    };
    if err.unwrap(orElse: { (_) in 9 }) != 9 { return 9 }

    0
}
