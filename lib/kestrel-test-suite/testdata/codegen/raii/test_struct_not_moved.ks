// test: diagnostics
// stdlib: false

module Test

struct Resource {
    let value: lang.i64
}

func main() -> lang.i64 {
    let r = Resource(value: 42);
    let x = r.value;  // Copy field, don't move r
    x
}
