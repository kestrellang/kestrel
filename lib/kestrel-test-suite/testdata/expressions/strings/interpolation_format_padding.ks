// test: execution
// stdlib: true
// expect-exit: 0

module Test

func main() -> lang.i64 {
    let val = 42;

    // Zero-padded hex
    if "\(val:08x)" != "0000002a" { return 1 }

    // Right-aligned with width
    if "\(val:>6)" != "    42" { return 2 }

    // Left-aligned with width
    if "\(val:<6)" != "42    " { return 3 }

    // Center-aligned with width
    if "\(val:^6)" != "  42  " { return 4 }

    // Custom fill character
    if "\(val:*>6)" != "****42" { return 5 }

    0
}
