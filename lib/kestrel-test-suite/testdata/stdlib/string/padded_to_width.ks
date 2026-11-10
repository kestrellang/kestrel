// test: execution
// stdlib: true
// expect-stdout: [ab   ]\n[00042]\n[**ab***]\n[*ab*]\n[hello]\n[é  ]\n[  é]\n

// `padded(toWidth:with:side:)` pads with a space after the text by default.
// `.Leading` pads before, `.Center` on both sides with the odd one after.
// Width counts graphemes: "é" written as `e` + U+0301 is one wide.
module Test

@main
func main() -> Int64 {
    println("[\("ab".padded(toWidth: 5))]");
    println("[\("42".padded(toWidth: 5, with: '0', side: .Leading))]");
    println("[\("ab".padded(toWidth: 7, with: '*', side: .Center))]");
    println("[\("ab".padded(toWidth: 4, with: '*', side: .Center))]");
    println("[\("hello".padded(toWidth: 3))]");
    let combining = "e\u{301}";
    println("[\(combining.padded(toWidth: 3))]");
    println("[\(combining.padded(toWidth: 3, side: .Leading))]");
    0
}
