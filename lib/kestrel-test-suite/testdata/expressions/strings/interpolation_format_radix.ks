// test: execution
// stdlib: true
// expect-exit: 0

module Test

@main
func main() -> lang.i64 {
    let val = 255;

    if "\(val:x)" != "ff" { return 1 }
    if "\(val:X)" != "FF" { return 2 }
    if "\(val:b)" != "11111111" { return 3 }
    if "\(val:o)" != "377" { return 4 }
    if "\(val:#x)" != "0xff" { return 5 }
    if "\(val:#X)" != "0xFF" { return 6 }
    if "\(val:#b)" != "0b11111111" { return 7 }
    if "\(val:#o)" != "0o377" { return 8 }

    0
}
