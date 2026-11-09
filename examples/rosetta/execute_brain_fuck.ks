// Rosetta Code: Execute Brain****
// https://rosettacode.org/wiki/Execute_Brain****
module Main

func run(program: String) -> String {
    let code = program.bytes.iter().collect();
    // Precompute matching brackets.
    var jump = Array[Int64](repeating: 0, count: code.count);
    var stack = Array[Int64]();
    for (i, op) in code.iter().enumerate() {
        if op == 91 { stack.append(i); }
        if op == 93 {
            let open = stack.pop() ?? 0;
            jump(i) = open;
            jump(open) = i;
        }
    }
    var tape = Array[UInt8](repeating: 0, count: 30000);
    var ptr: Int64 = 0;
    var pc: Int64 = 0;
    var out = Array[UInt8]();
    while pc < code.count {
        match code(pc) {
            62 => ptr = ptr + 1,                           // >
            60 => ptr = ptr - 1,                           // <
            43 => tape(ptr) = tape(ptr).addWrapping(1),    // +
            45 => tape(ptr) = tape(ptr).subtractWrapping(1), // -
            46 => out.append(tape(ptr)),                   // .
            91 => if tape(ptr) == 0 { pc = jump(pc); },    // [
            93 => if tape(ptr) != 0 { pc = jump(pc); },    // ]
            _ => {},
        }
        pc = pc + 1;
    }
    String(fromUtf8Lossy: out)
}

@main
func main() {
    let hello = "++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.";
    print(run(hello));
}
