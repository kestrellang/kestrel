// test: diagnostics
// stdlib: false

module Test
func bitwiseOps() {
    let _and = lang.i64_and(0b1100, 0b1010);
    let _or = lang.i64_or(0b1100, 0b1010);
    let _xor = lang.i64_xor(0b1100, 0b1010);
    let _not = lang.i64_not(0b1010);
    let _shl = lang.i64_shl(1, 4);
}
