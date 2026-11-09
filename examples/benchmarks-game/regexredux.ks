module Main

import std.memory.(RawPointer, Pointer)
import std.ffi.(malloc, memcpy)
import std.streams.(StandardInput)

// PCRE2 with its JIT, the same engine the fastest C and Rust programs use.
@extern(.C, mangleName: "pcre2_compile_8")
func pcreCompile(pattern: RawPointer, len: Int64, options: UInt32, err: RawPointer, errOffset: RawPointer, ctx: RawPointer) -> RawPointer
@extern(.C, mangleName: "pcre2_jit_compile_8")
func pcreJitCompile(code: RawPointer, options: UInt32) -> Int32
@extern(.C, mangleName: "pcre2_match_data_create_8")
func pcreMatchDataCreate(n: UInt32, ctx: RawPointer) -> RawPointer
@extern(.C, mangleName: "pcre2_jit_match_8")
func pcreJitMatch(code: RawPointer, subject: RawPointer, len: Int64, start: Int64, options: UInt32, md: RawPointer, ctx: RawPointer) -> Int32
@extern(.C, mangleName: "pcre2_get_ovector_pointer_8")
func pcreOvector(md: RawPointer) -> RawPointer
@extern(.C, mangleName: "pcre2_code_free_8")
func pcreCodeFree(code: RawPointer)

func bytesOf(s: String) -> Array[UInt8] {
    var a = Array[UInt8]();
    for b in s.bytes { a.append(b); }
    a
}

struct Regex {
    var code: RawPointer
    var md: RawPointer
    var ov: Pointer[Int64]

    init(pattern: String) {
        let p = bytesOf(pattern);
        let scratch = unsafe malloc(16);
        self.code = unsafe pcreCompile(p.asSlice().pointer.asRaw(), p.count, 0, scratch, scratch.offset(by: 8), RawPointer.nullPointer());
        let _ = unsafe pcreJitCompile(self.code, 1);
        self.md = unsafe pcreMatchDataCreate(4, RawPointer.nullPointer());
        self.ov = unsafe pcreOvector(self.md).cast[Int64]();
    }

    // Finds the next match at or after `start`; returns false when none.
    func find(subject: Slice[UInt8], start: Int64) -> Bool {
        unsafe pcreJitMatch(self.code, subject.pointer.asRaw(), subject.count, start, 0, self.md, RawPointer.nullPointer()) >= 0
    }

    func matchStart() -> Int64 { unsafe self.ov.read() }
    func matchEnd() -> Int64 { unsafe self.ov.offset(by: 1).read() }

    func count(subject: Slice[UInt8]) -> Int64 {
        var n: Int64 = 0;
        var pos: Int64 = 0;
        while self.find(subject, pos) {
            n = n + 1;
            pos = self.matchEnd();
        }
        n
    }
}

// A byte buffer with a separate length, so appends are plain memcpys.
struct Buf {
    var data: Array[UInt8]
    var len: Int64
}

// Replaces every match of `re` in `src` with `rep`. The output never grows
// past 1.5x the input (the longest replacement is 3 bytes for a 2-byte match).
func replaceAll(re: Regex, src: Slice[UInt8], rep: Array[UInt8]) -> Buf {
    var out = Buf(data: Array[UInt8](repeating: 0, count: src.count + src.count / 2 + 16), len: 0);
    let d = out.data.asMutableSlice().pointer.asRaw();
    let s = src.pointer.asRaw();
    let r = rep.asSlice().pointer.asRaw();
    let rn = rep.count;
    var o: Int64 = 0;
    var pos: Int64 = 0;
    while re.find(src, pos) {
        let m = re.matchStart();
        let _ = unsafe memcpy(d.offset(by: o), s.offset(by: pos), m - pos);
        o = o + m - pos;
        let _ = unsafe memcpy(d.offset(by: o), r, rn);
        o = o + rn;
        pos = re.matchEnd();
    }
    let _ = unsafe memcpy(d.offset(by: o), s.offset(by: pos), src.count - pos);
    out.len = o + src.count - pos;
    out
}

@main
func main() {
    var input = Array[UInt8]();
    input.reserveCapacity(1 << 26);
    var chunk = Array[UInt8](repeating: 0, count: 1 << 20);
    var stdin = StandardInput();
    loop {
        let n = match stdin.read(into: chunk.asMutableSlice()) { .Ok(n) => n, .Err(_) => 0 };
        if n == 0 { break }
        input.append(contentsOf: chunk(0..<n));
    }
    let inputLen = input.count;
    let seq = replaceAll(Regex(">.*\\n|\\n"), input.asSlice(), Array[UInt8]());
    let seqLen = seq.len;

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
        let re = Regex(v);
        println("\(v) \(re.count(seq.data(0..<seq.len)))");
    }

    let pats = ["tHa[Nt]", "aND|caN|Ha[DS]|WaS", "a[NSt]|BY", "<[^>]*>", "\\|[^|][^|]*\\|"];
    let reps = ["<4>", "<3>", "<2>", "|", "-"];
    var cur = seq;
    for i in 0..<pats.count {
        cur = replaceAll(Regex(pats(i)), cur.data(0..<cur.len), bytesOf(reps(i)));
    }
    println("");
    println("\(inputLen)");
    println("\(seqLen)");
    println("\(cur.len)");
}
