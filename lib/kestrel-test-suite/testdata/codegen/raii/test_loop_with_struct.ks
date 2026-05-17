// test: diagnostics
// stdlib: false

module Test

struct Resource {
    let value: lang.i64
}

func main() -> lang.i64 {
    var sum = 0;
    var i = 0;
    while lang.i64_signed_lt(i, 3) {
        let r = Resource(value: lang.i64_mul(i, 10));
        sum = lang.i64_add(sum, r.value);
        i = lang.i64_add(i, 1);
    }
    sum  // 0 + 10 + 20 = 30
}
