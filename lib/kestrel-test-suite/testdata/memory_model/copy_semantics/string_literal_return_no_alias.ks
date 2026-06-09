// test: execution
// stdlib: true
// expect-stdout: interp=01 append=01 hex=#3a7bd5

// Regression (#125): a function returning a string *literal* handed back a
// `String` that aliased shared storage instead of an owned value. `String` is a
// `CowBox[StringStorage]`-backed Cloneable; the MIR mono expand pass
// over-suppressed `CopyValue -> clone`, so a copied/returned literal `String`
// became a *bitwise alias* of the shared storage rather than a real clone. With
// >=2 live results the second clobbered the first: `"\(d(0))\(d(9))"` printed
// `11` instead of `01`, and a hex formatter turned `#3a7bd5` into `#3070d0`
// (low nibble always 0). Fixed by the expand.rs copy-elaboration cluster
// (narrow `skip_nominal` to clone impls only; key `not_copyable` per
// instantiation). Sibling of try_unwrap_noncopyable_payload_no_double_free.
module Test

func d(n: UInt8) -> String {
    match n { 0 => "0", _ => "1" }
}

func hexDigit(n: UInt8) -> String {
    match n {
        0 => "0", 1 => "1", 2 => "2", 3 => "3",
        4 => "4", 5 => "5", 6 => "6", 7 => "7",
        8 => "8", 9 => "9", 10 => "a", 11 => "b",
        12 => "c", 13 => "d", 14 => "e", _ => "f"
    }
}

func hexByte(b: UInt8) -> String {
    var s = "";
    s.append(hexDigit(b / 16));
    s.append(hexDigit(b % 16));
    return s;
}

@main
func main() -> lang.i32 {
    // Minimal repro: two live literal-returns combined via interpolation.
    let a = d(0);
    let b = d(9);

    // append form: two literal-returns appended into one buffer.
    var ap = "";
    ap.append(d(0));
    ap.append(d(9));

    // real-world symptom: composed hex formatter, three live bytes.
    var hex = "#";
    hex.append(hexByte(58));   // 0x3a
    hex.append(hexByte(123));  // 0x7b
    hex.append(hexByte(213));  // 0xd5

    print("interp=\(a)\(b) append=\(ap) hex=\(hex)");
    return 0;
}
