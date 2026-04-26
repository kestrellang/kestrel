// test: diagnostics
// stdlib: false

module Main

func apply(f: (lang.i64) -> lang.i64, x: lang.i64) -> lang.i64 {
    f(x)
}

func main() -> lang.i64 {
    let double = { (x: lang.i64) in lang.i64_mul(x, 2) };
    let addOne = { (x: lang.i64) in lang.i64_add(x, 1) };

    let a = apply(double, 5);
    let b = apply(addOne, 10);

    lang.i64_add(a, b)
}
