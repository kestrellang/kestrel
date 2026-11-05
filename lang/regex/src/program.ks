module regex

// Compiling the syntax tree to a program for the Pike VM: a list of
// instructions where `split` forks a thread and `save` records a capture
// position.

let opChar: Int64 = 0;
let opClass: Int64 = 1;
let opAny: Int64 = 2;
let opAnyNotNewline: Int64 = 3;
let opSplit: Int64 = 4;
let opJump: Int64 = 5;
let opSave: Int64 = 6;
let opAssert: Int64 = 7;
let opMatch: Int64 = 8;

/// The most instructions a pattern may compile to, so counted repeats
/// can't blow up memory (`(a{1000}){1000}`).
let maxInstructions: Int64 = 200000;

struct Inst {
    var op: Int64;
    /// Char: the code point. Class: its index. Split and jump: the
    /// preferred target. Save: the slot. Assert: the kind.
    var x: Int64;
    /// Char: the other case, or the same code point. Split: the second
    /// target.
    var y: Int64;

    init(op op: Int64, x x: Int64, y y: Int64) {
        self.op = op;
        self.x = x;
        self.y = y;
    }
}

struct Program {
    var insts: Array[Inst];
    var classes: Array[CharClass];
    /// Capture groups including group 0 (the whole match).
    var groupCount: Int64;
    /// Set when every match must start at the beginning of the text.
    var anchoredStart: Bool;
    /// A byte every match must start with, or -1.
    var firstByte: Int64;
}

struct Compiler {
    let nodes: Array[Node];
    var insts: Array[Inst];
    var tooBig: Bool;

    init(nodes nodes: Array[Node]) {
        self.nodes = nodes;
        self.insts = [];
        self.tooBig = false;
    }

    mutating func emit(op: Int64, x: Int64, y: Int64) -> Int64 {
        if self.insts.count >= maxInstructions {
            self.tooBig = true;
            return self.insts.count - 1;
        }
        self.insts.append(Inst(op: op, x: x, y: y));
        self.insts.count - 1
    }

    mutating func patch(at: Int64, x: Int64, y: Int64) {
        if at < 0 or at >= self.insts.count { return; }
        var inst = self.insts(at);
        inst.x = x;
        inst.y = y;
        self.insts(at) = inst;
    }

    var next: Int64 { self.insts.count }

    mutating func compile(index: Int64) {
        if self.tooBig { return; }
        let node = self.nodes(index);
        let kind = node.kind;
        if kind == nodeEmpty { return; }
        if kind == nodeLiteral {
            var other = node.value;
            if node.fold { other = otherCase(node.value); }
            let _ = self.emit(opChar, node.value, other);
            return;
        }
        if kind == nodeClass {
            let _ = self.emit(opClass, node.value, 0);
            return;
        }
        if kind == nodeAny {
            var op = opAnyNotNewline;
            if node.dotAll { op = opAny; }
            let _ = self.emit(op, 0, 0);
            return;
        }
        if kind == nodeAssert {
            let _ = self.emit(opAssert, node.value, 0);
            return;
        }
        if kind == nodeGroup {
            let capture = node.value;
            if capture >= 0 { let _ = self.emit(opSave, capture * 2, 0); }
            self.compile(node.children(0));
            if capture >= 0 { let _ = self.emit(opSave, capture * 2 + 1, 0); }
            return;
        }
        if kind == nodeConcat {
            for child in node.children { self.compile(child); }
            return;
        }
        if kind == nodeAlternate {
            // split L1, next; L1: a; jump end; next: split L2, ...; last
            var jumps: [Int64] = [];
            let last = node.children.count - 1;
            for i in 0..=last {
                if i == last {
                    self.compile(node.children(i));
                } else {
                    let split = self.emit(opSplit, 0, 0);
                    self.compile(node.children(i));
                    jumps.append(self.emit(opJump, 0, 0));
                    self.patch(split, split + 1, self.next);
                }
            }
            for j in jumps { self.patch(j, self.next, 0); }
            return;
        }
        self.compileRepeat(node);
    }

    mutating func compileRepeat(node: Node) {
        let child = node.children(0);
        let min = node.value;
        let max = node.max;
        for _ in 0..<min { self.compile(child); }
        if max < 0 {
            // loop: split body, out (greedy) or out, body (lazy)
            let split = self.emit(opSplit, 0, 0);
            self.compile(child);
            let _ = self.emit(opJump, split, 0);
            self.patchSplit(split, split + 1, self.next, node.greedy);
            return;
        }
        // Each optional copy: split body, out.
        var splits: [Int64] = [];
        for _ in min..<max {
            splits.append(self.emit(opSplit, 0, 0));
            self.compile(child);
        }
        let out = self.next;
        for s in splits { self.patchSplit(s, s + 1, out, node.greedy); }
    }

    /// Points a split at `body` and `out`, preferring `body` when greedy.
    mutating func patchSplit(at: Int64, body: Int64, out: Int64, greedy: Bool) {
        if greedy {
            self.patch(at, body, out);
        } else {
            self.patch(at, out, body);
        }
    }
}

/// The first node that must match, skipping groups and taking the head
/// of concatenations.
func leadingNode(nodes: Array[Node], root: Int64) -> Node {
    var node = nodes(root);
    loop {
        if node.kind == nodeGroup or (node.kind == nodeConcat and node.children.count > 0) {
            node = nodes(node.children(0));
        } else if node.kind == nodeRepeat and node.value > 0 {
            node = nodes(node.children(0));
        } else {
            return node;
        }
    }
}

func compileProgram(parser: Parser, root: Int64) -> Result[Program, RegexError] {
    var compiler = Compiler(nodes: parser.nodes);
    let _ = compiler.emit(opSave, 0, 0);
    compiler.compile(root);
    let _ = compiler.emit(opSave, 1, 0);
    let _ = compiler.emit(opMatch, 0, 0);
    if compiler.tooBig {
        return .Err(RegexError(message: "pattern too large", position: 0));
    }
    let lead = leadingNode(parser.nodes, root);
    let anchored = lead.kind == nodeAssert and lead.value == assertTextStart;
    var firstByte: Int64 = -1;
    if lead.kind == nodeLiteral and not lead.fold and lead.value < 128 { firstByte = lead.value; }
    .Ok(Program(insts: compiler.insts, classes: parser.classes, groupCount: parser.names.count, anchoredStart: anchored, firstByte: firstByte))
}
