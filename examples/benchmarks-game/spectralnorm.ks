module Main

import std.system.Process

// A(i, j) = 1 / ((i + j) * (i + j + 1) / 2 + i + 1). The denominators are
// built in floating point (exact below 2^53), and the inner loop keeps 8
// independent partial sums, so LLVM's SLP vectorizer turns each step into
// packed adds, multiplies and divides.

// out = A * v
func mulAv(v: Slice[Float64], mutating out: Slice[Float64], n: Int64) {
    for i in 0..<n {
        let fi = Float64(from: i);
        var s0 = 0.0; var s1 = 0.0; var s2 = 0.0; var s3 = 0.0; var s4 = 0.0; var s5 = 0.0; var s6 = 0.0; var s7 = 0.0;
        var j: Int64 = 0;
        while j + 8 <= n {
            let fj = Float64(from: j);
            let a = fi + fj;
            s0 = s0 + unsafe v(unchecked: j + 0) / ((a + 0.0) * (a + 1.0) * 0.5 + fi + 1.0);
            s1 = s1 + unsafe v(unchecked: j + 1) / ((a + 1.0) * (a + 2.0) * 0.5 + fi + 1.0);
            s2 = s2 + unsafe v(unchecked: j + 2) / ((a + 2.0) * (a + 3.0) * 0.5 + fi + 1.0);
            s3 = s3 + unsafe v(unchecked: j + 3) / ((a + 3.0) * (a + 4.0) * 0.5 + fi + 1.0);
            s4 = s4 + unsafe v(unchecked: j + 4) / ((a + 4.0) * (a + 5.0) * 0.5 + fi + 1.0);
            s5 = s5 + unsafe v(unchecked: j + 5) / ((a + 5.0) * (a + 6.0) * 0.5 + fi + 1.0);
            s6 = s6 + unsafe v(unchecked: j + 6) / ((a + 6.0) * (a + 7.0) * 0.5 + fi + 1.0);
            s7 = s7 + unsafe v(unchecked: j + 7) / ((a + 7.0) * (a + 8.0) * 0.5 + fi + 1.0);
            j = j + 8;
        }
        var sum = ((s0 + s1) + (s2 + s3)) + ((s4 + s5) + (s6 + s7));
        while j < n {
            let fj = Float64(from: j);
            let a = fi + fj;
            sum = sum + unsafe v(unchecked: j) / (a * (a + 1.0) * 0.5 + fi + 1.0);
            j = j + 1;
        }
        unsafe out(unchecked: i) = sum;
    }
}

// out = A^T * v
func mulAtv(v: Slice[Float64], mutating out: Slice[Float64], n: Int64) {
    for i in 0..<n {
        let fi = Float64(from: i);
        var s0 = 0.0; var s1 = 0.0; var s2 = 0.0; var s3 = 0.0; var s4 = 0.0; var s5 = 0.0; var s6 = 0.0; var s7 = 0.0;
        var j: Int64 = 0;
        while j + 8 <= n {
            let fj = Float64(from: j);
            let a = fi + fj;
            s0 = s0 + unsafe v(unchecked: j + 0) / ((a + 0.0) * (a + 1.0) * 0.5 + fj + 0.0 + 1.0);
            s1 = s1 + unsafe v(unchecked: j + 1) / ((a + 1.0) * (a + 2.0) * 0.5 + fj + 1.0 + 1.0);
            s2 = s2 + unsafe v(unchecked: j + 2) / ((a + 2.0) * (a + 3.0) * 0.5 + fj + 2.0 + 1.0);
            s3 = s3 + unsafe v(unchecked: j + 3) / ((a + 3.0) * (a + 4.0) * 0.5 + fj + 3.0 + 1.0);
            s4 = s4 + unsafe v(unchecked: j + 4) / ((a + 4.0) * (a + 5.0) * 0.5 + fj + 4.0 + 1.0);
            s5 = s5 + unsafe v(unchecked: j + 5) / ((a + 5.0) * (a + 6.0) * 0.5 + fj + 5.0 + 1.0);
            s6 = s6 + unsafe v(unchecked: j + 6) / ((a + 6.0) * (a + 7.0) * 0.5 + fj + 6.0 + 1.0);
            s7 = s7 + unsafe v(unchecked: j + 7) / ((a + 7.0) * (a + 8.0) * 0.5 + fj + 7.0 + 1.0);
            j = j + 8;
        }
        var sum = ((s0 + s1) + (s2 + s3)) + ((s4 + s5) + (s6 + s7));
        while j < n {
            let fj = Float64(from: j);
            let a = fi + fj;
            sum = sum + unsafe v(unchecked: j) / (a * (a + 1.0) * 0.5 + fj + 1.0);
            j = j + 1;
        }
        unsafe out(unchecked: i) = sum;
    }
}

// out = A^T * A * v
func mulAtAv(v: Slice[Float64], mutating out: Slice[Float64], mutating tmp: Slice[Float64], n: Int64) {
    mulAv(v, tmp, n);
    mulAtv(tmp, out, n);
}

@main
func main() {
    let args = Process.arguments();
    let n = if args.count > 1 { Int64(parsing: args(1)) ?? 100 } else { 100 };
    var u = Array[Float64](repeating: 1.0, count: n);
    var v = Array[Float64](repeating: 0.0, count: n);
    var tmp = Array[Float64](repeating: 0.0, count: n);
    for _ in 0..<10 {
        mulAtAv(u.asSlice(), v.asMutableSlice(), tmp.asMutableSlice(), n);
        mulAtAv(v.asSlice(), u.asMutableSlice(), tmp.asMutableSlice(), n);
    }
    var vBv = 0.0;
    var vv = 0.0;
    for i in 0..<n {
        let ui = unsafe u(unchecked: i);
        let vi = unsafe v(unchecked: i);
        vBv = vBv + ui * vi;
        vv = vv + vi * vi;
    }
    println("\((vBv / vv).sqrt():.9)");
}
