module Main

import regex.(Regex)
import std.streams.(StandardInput)

// regex-redux using Kestrel's own pure-Kestrel regex package (a Pike VM).
@main
func main() {
    var bytes = Array[UInt8]();
    var input = StandardInput();
    let _ = input.readAll(into: bytes);
    let text = String(fromUtf8Lossy: bytes.asSlice());
    let inputLen = bytes.count;
    let strip = match Regex.compile(">.*\\n|\\n") { .Ok(r) => r, .Err(_) => fatalError("bad pattern") };
    let seq = strip.replacingAll(in: text, with: "");
    let seqLen = seq.bytes.count;
    let variants = [
        "agggtaaa|tttaccct",
        "[cgt]gggtaaa|tttaccc[acg]",
        "a[act]ggtaaa|tttacc[agt]t",
        "ag[act]gtaaa|tttac[agt]ct",
        "agg[act]taaa|ttta[agt]cct",
        "aggg[acg]aaa|ttt[cgt]ccct",
        "agggt[cgt]aa|tt[acg]accct",
        "agggta[cgt]a|t[acg]taccct",
        "agggtaa[cgt]|[acg]ttaccct",
    ];
    for v in variants {
        let re = match Regex.compile(v) { .Ok(r) => r, .Err(_) => fatalError("bad pattern") };
        println("\(v) \(re.matches(in: seq).count)");
    }
    let pats = ["tHa[Nt]", "aND|caN|Ha[DS]|WaS", "a[NSt]|BY", "<[^>]*>", "\\|[^|][^|]*\\|"];
    let reps = ["<4>", "<3>", "<2>", "|", "-"];
    var cur = seq;
    for i in 0..<pats.count {
        let re = match Regex.compile(pats(i)) { .Ok(r) => r, .Err(_) => fatalError("bad pattern") };
        cur = re.replacingAll(in: cur, with: reps(i));
    }
    println("");
    println("\(inputLen)");
    println("\(seqLen)");
    println("\(cur.bytes.count)");
}
