// test: execution
// backends: cranelift,llvm
// stdlib: true

// Zero-breakage canary: ordinary generics over ordinary (Static) types
// run unchanged with the implicit `T: Static` bound injected everywhere.

module Test

func pick[T](a: T, b: T, takeFirst: Bool) -> T {
    if takeFirst { a } else { b }
}

@main
func main() {
    let xs = [1, 2, 3].map { it * 10 };
    let n = pick(xs(0), xs(2), false);
    if n != 30 {
        fatalError("generic plumbing broke under the Static bound");
    }
    let s = pick("left", "right", true);
    if s != "left" {
        fatalError("string generic broke under the Static bound");
    }
    print("ok");
}

// CHECK: ok
