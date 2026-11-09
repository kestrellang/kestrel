// test: execution
// stdlib: true
// expect-exit: 0

module Main

// `+=`, `-=`, `*=` and `/=` on Float64 and Float32, as on the integers.

@main
func main() -> Int64 {
    var total = 0.0;
    for x in [1.5, 2.5, 4.0] {
        total += x;
    }
    if total != 8.0 { return 1 }
    total -= 2.0;
    total *= 3.0;
    total /= 4.0;
    if total != 4.5 { return 2 }

    var small: Float32 = 1.0;
    small += 0.5;
    small *= 2.0;
    if small != 3.0 { return 3 }
    0
}
