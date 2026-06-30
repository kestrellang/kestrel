// test: execution
// stdlib: true
// expect-stdout: -0000005|-000ff|00000007

// Regression (#171): a zero-padded format spec placed the minus sign AFTER
// the pad zeros ("000000-5") instead of before them ("-0000005"). The fix
// splits the sign (and any radix prefix) off the magnitude and emits it
// before the zero-padding. Covers a negative decimal, a negative hex value
// (sign still leads), and a positive value (unaffected).

module Test

@main
func main() {
    let n = -5;
    let h = -255;
    let p = 7;
    print("\(n:08)|\(h:06x)|\(p:08)");
}
