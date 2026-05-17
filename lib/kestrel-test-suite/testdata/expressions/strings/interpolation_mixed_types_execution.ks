// test: execution
// stdlib: true
// expect-exit: 0

module Test

func main() -> lang.i64 {
    let name = "result";
    let value = 255;

    if "\(name): \(value) (hex: \(value:x))" != "result: 255 (hex: ff)" { return 1 }

    // Bool interpolation
    let flag = true;
    if "flag=\(flag)" != "flag=true" { return 2 }

    0
}
