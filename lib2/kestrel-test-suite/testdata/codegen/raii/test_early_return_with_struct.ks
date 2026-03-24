// test: diagnostics
// stdlib: false

module Test

struct Resource {
    let value: lang.i64
}

func test_early_return(x: lang.i64) -> lang.i64 {
    let r = Resource(value: x);
    if lang.i64_signed_gt(x, 50) {
        return lang.i64_add(r.value, 1);
    }
    r.value
}

func main() -> lang.i64 {
    test_early_return(60)
}
