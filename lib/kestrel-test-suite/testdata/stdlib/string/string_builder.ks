// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    // ---- Append String ----
    var b = std.text.StringBuilder();
    b.append("hello");
    b.append(" ");
    b.append("world");
    let s = b.build();
    if s.isEqual(to: "hello world") == false { return 1 }

    // ---- Append StringSlice ----
    var b2 = std.text.StringBuilder();
    let source: std.text.String = "hello world";
    let slice = source.asSlice();
    b2.append(slice);
    let s2 = b2.build();
    if s2.isEqual(to: "hello world") == false { return 2 }

    // ---- Append subslice ----
    var b3 = std.text.StringBuilder();
    let sub = source.asSlice().subslice(from: 0, to: 5);
    b3.append(sub);
    b3.append(" ");
    let sub2 = source.asSlice().subslice(from: 6, to: 11);
    b3.append(sub2);
    let s3 = b3.build();
    if s3.isEqual(to: "hello world") == false { return 3 }

    // ---- Mix String and StringSlice appends ----
    var b4 = std.text.StringBuilder();
    b4.append("prefix:");
    b4.append(source.asSlice());
    let s4 = b4.build();
    if s4.isEqual(to: "prefix:hello world") == false { return 4 }

    // ---- Builder reuse after build ----
    var b5 = std.text.StringBuilder();
    b5.append("first");
    let r1 = b5.build();
    if r1.isEqual(to: "first") == false { return 5 }

    b5.append("second");
    let r2 = b5.build();
    if r2.isEqual(to: "second") == false { return 6 }

    // ---- Empty append ----
    var b6 = std.text.StringBuilder();
    let emptySlice = std.text.String().asSlice();
    b6.append(emptySlice);
    b6.append("ok");
    let s6 = b6.build();
    if s6.isEqual(to: "ok") == false { return 7 }

    // ---- appendChar ----
    var b7 = std.text.StringBuilder();
    b7.append(char: 'A');
    b7.append(char: 'B');
    let s7 = b7.build();
    if s7.isEqual(to: "AB") == false { return 8 }

    0
}
