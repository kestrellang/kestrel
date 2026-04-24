// test: diagnostics
// stdlib: false

module Test

struct Resource {
    let value: lang.i64
}

func main() -> lang.i64 {
    let cond = true;
    var result = 0;
    if cond {
        let r = Resource(value: 42);
        result = r.value;
    }
    result
}
