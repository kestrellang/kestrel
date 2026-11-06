module regex

// The Pike VM: runs every possible match in lockstep, one input character
// at a time, so a search takes time proportional to the text length times
// the program size, whatever the pattern. Threads are kept in priority
// order, which gives Perl's leftmost-first match semantics.

/// The threads alive at one text position, at most one per instruction.
struct ThreadList {
    var pcs: Array[Int64];
    var caps: Array[Array[Int64]];
    var count: Int64;
    /// `seen(pc) == generation` when `pc` is already on the list.
    var seen: Array[Int64];
    var generation: Int64;

    init(size size: Int64) {
        self.pcs = Array[Int64](repeating: 0, count: size);
        let empty: [Int64] = [];
        self.caps = Array[Array[Int64]](repeating: empty, count: size);
        self.count = 0;
        self.seen = Array[Int64](repeating: -1, count: size);
        self.generation = 0;
    }

    mutating func clear() {
        self.count = 0;
        self.generation = self.generation + 1;
    }
}

struct Searcher {
    let program: Program;
    let text: Array[UInt8];
    var current: ThreadList;
    var following: ThreadList;
    /// The work stack for following jumps, splits and saves.
    var stackPcs: Array[Int64];
    var stackCaps: Array[Array[Int64]];

    init(program program: Program, text text: Array[UInt8]) {
        self.program = program;
        self.text = text;
        let size = program.insts.count;
        self.current = ThreadList(size: size);
        self.following = ThreadList(size: size);
        self.stackPcs = [];
        self.stackCaps = [];
    }

    func isWordByte(at: Int64) -> Bool {
        if at < 0 or at >= self.text.count { return false; }
        let b = self.text(at);
        (b >= 48 and b <= 57) or (b >= 65 and b <= 90) or (b >= 97 and b <= 122) or b == 95
    }

    func assertionHolds(kind: Int64, pos: Int64) -> Bool {
        let n = self.text.count;
        if kind == assertTextStart { return pos == 0; }
        if kind == assertTextEnd { return pos == n; }
        if kind == assertLineStart { return pos == 0 or self.text(pos - 1) == 10; }
        if kind == assertLineEnd { return pos == n or self.text(pos) == 10; }
        let boundary = self.isWordByte(pos - 1) != self.isWordByte(pos);
        if kind == assertWordBoundary { return boundary; }
        not boundary
    }

    /// Adds the thread at `pc` to `following` (when `intoFollowing`) or
    /// `current`, after following every jump, split, save and assertion
    /// it can take at `pos` without reading input.
    mutating func addThread(intoFollowing: Bool, pc: Int64, caps: Array[Int64], pos: Int64) {
        self.stackPcs.append(pc);
        self.stackCaps.append(caps);
        while self.stackPcs.count > 0 {
            guard let .Some(at) = self.stackPcs.pop() else { return; }
            guard let .Some(threadCaps) = self.stackCaps.pop() else { return; }
            if intoFollowing {
                if self.following.seen(at) == self.following.generation { continue; }
                self.following.seen(at) = self.following.generation;
            } else {
                if self.current.seen(at) == self.current.generation { continue; }
                self.current.seen(at) = self.current.generation;
            }
            let inst = self.program.insts(at);
            let op = inst.op;
            if op == opJump {
                self.push(inst.x, threadCaps);
            } else if op == opSplit {
                // Pushed second, popped first: `x` has priority.
                self.push(inst.y, threadCaps);
                self.push(inst.x, threadCaps);
            } else if op == opSave {
                var updated: Array[Int64] = threadCaps;
                let slot = inst.x;
                updated(slot) = pos;
                self.push(at + 1, updated);
            } else if op == opAssert {
                if self.assertionHolds(inst.x, pos) { self.push(at + 1, threadCaps); }
            } else if intoFollowing {
                let i = self.following.count;
                self.following.pcs(i) = at;
                self.following.caps(i) = threadCaps;
                self.following.count = i + 1;
            } else {
                let i = self.current.count;
                self.current.pcs(i) = at;
                self.current.caps(i) = threadCaps;
                self.current.count = i + 1;
            }
        }
    }

    mutating func push(pc: Int64, caps: Array[Int64]) {
        self.stackPcs.append(pc);
        self.stackCaps.append(caps);
    }

    func matchesChar(inst: Inst, c: Int64) -> Bool {
        let op = inst.op;
        if op == opChar { return c == inst.x or c == inst.y; }
        if op == opClass { return self.program.classes(inst.x).contains(c); }
        if op == opAny { return true; }
        if op == opAnyNotNewline { return c != 10; }
        false
    }

    /// The capture slots of the leftmost-first match starting at or after
    /// `start`, or `.None`. With `earliest`, stops at the first match
    /// found (for `isMatch`), whose slots may not be the leftmost-first.
    mutating func search(start: Int64, earliest: Bool) -> Array[Int64]? {
        let n = self.text.count;
        let empty = Array[Int64](repeating: -1, count: self.program.groupCount * 2);
        var matched: Array[Int64]? = .None;
        var pos = start;
        self.current.clear();
        loop {
            if matched.isNone and (not self.program.anchoredStart or pos == 0) {
                if self.current.count == 0 and self.program.firstByte >= 0 {
                    // No thread alive: jump to where a match could start.
                    let b = UInt8(truncating: self.program.firstByte);
                    while pos < n and self.text(pos) != b { pos = pos + 1; }
                    if pos >= n { return matched; }
                }
                self.addThread(false, 0, empty, pos);
            }
            if self.current.count == 0 {
                // Nothing alive: done if a match was found or none can
                // start later; otherwise try the next position.
                if matched.isSome or self.program.anchoredStart or pos >= n { return matched; }
                pos = pos + decodeUtf8(self.text, pos).1;
                self.current.clear();
                continue;
            }
            var c: Int64 = -1;
            var len: Int64 = 1;
            if pos < n {
                let decoded = decodeUtf8(self.text, pos);
                c = decoded.0;
                len = decoded.1;
            }
            self.following.clear();
            for i in 0..<self.current.count {
                let pc = self.current.pcs(i);
                let inst = self.program.insts(pc);
                if inst.op == opMatch {
                    matched = .Some(self.current.caps(i));
                    if earliest { return matched; }
                    // Lower-priority threads can't win any more.
                    break;
                }
                if pos < n and self.matchesChar(inst, c) {
                    self.addThread(true, pc + 1, self.current.caps(i), pos + len);
                }
            }
            if pos >= n { return matched; }
            pos = pos + len;
            let swap = self.current;
            self.current = self.following;
            self.following = swap;
        }
    }
}
