// test: execution
// stdlib: true
// expect-stdout: true 5 3\nplain\n

module Test

import std.ffi.(CString, malloc, free)

// C strings (file names, environment values) aren't always UTF-8.
// `String(from: CString)` used to copy the bytes unchecked, making an
// invalid String; now invalid bytes become U+FFFD.
@main
func main() {
    let raw = unsafe malloc(4);
    let p = raw.cast[UInt8]();
    unsafe p.write(0x61);
    unsafe p.offset(by: 1).write(0xFF);
    unsafe p.offset(by: 2).write(0x62);
    unsafe p.offset(by: 3).write(0);
    let s = String(from: CString(raw: p));
    println("\(s == "a\u{FFFD}b") \(s.bytes.count) \(s.chars.count)");
    unsafe free(raw);
    let plain = "plain".toCString();
    println(String(from: plain));
    plain.free();
}
