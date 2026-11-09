module Main

import std.streams.(StandardInput, StandardOutput)

@main
func main() {
    // Complement table: IUPAC codes map to their complements, in upper case.
    var comp = Array[UInt8](repeating: 0, count: 256);
    for i in 0..<256 { unsafe comp(unchecked: i) = UInt8(truncating: i); }
    let from = "ACGTUMRWSYKVHDBNacgtumrwsykvhdbn";
    let to = "TGCAAKYWSRMBDHVNTGCAAKYWSRMBDHVN";
    var fromArr = Array[UInt8](); for b in from.bytes { fromArr.append(b); }
    var toArr = Array[UInt8](); for b in to.bytes { toArr.append(b); }
    for i in 0..<fromArr.count { comp(Int64(from: fromArr(i))) = toArr(i); }
    let ct = comp.asSlice();

    // All of standard input, read straight into one growing buffer.
    var data = Array[UInt8](capacity: 1 << 24);
    let _ = StandardInput().readAll(into: data);
    let len = data.count;
    let buf = data.asMutableSlice();
    var stdout = StandardOutput();
    // Output goes out in 1 MB pieces through the (already buffered) stdout.
    var outArr = Array[UInt8](repeating: 0, count: 1 << 20);
    let out = outArr.asMutableSlice();
    var o: Int64 = 0;
    var i: Int64 = 0;
    while i < len {
        // Write the header line as is.
        let h = i;
        while i < len and unsafe buf(unchecked: i) != 10 { i = i + 1; }
        i = i + 1;
        let _ = stdout.write(from: buf(h..<i));
        // Squeeze out the newlines in place, up to the next header.
        let start = i;
        var w = i;
        while i < len {
            let b = unsafe buf(unchecked: i);
            if b == 62 { break }
            unsafe buf(unchecked: w) = b;
            w = w + if b == 10 { 0 } else { 1 };
            i = i + 1;
        }
        // Emit the reverse complement in lines of 60.
        var j = w;
        while j > start {
            let k = if j - start < 60 { j - start } else { 60 };
            if o + 61 > out.count {
                let _ = stdout.write(from: out(0..<o));
                o = 0;
            }
            for t in 0..<k {
                unsafe out(unchecked: o + t) = unsafe ct(unchecked: Int64(from: unsafe buf(unchecked: j - 1 - t)));
            }
            unsafe out(unchecked: o + k) = 10;
            o = o + k + 1;
            j = j - k;
        }
        let _ = stdout.write(from: out(0..<o));
        o = 0;
    }
    let _ = stdout.flush();
}
