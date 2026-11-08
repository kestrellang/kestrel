// Rosetta Code: Parsing/Shunting-yard algorithm
// https://rosettacode.org/wiki/Parsing/Shunting-yard_algorithm
module Main

func precedence(op: String) -> Int64 {
    match op {
        "^" => 4,
        "*" or "/" => 3,
        "+" or "-" => 2,
        _ => 0,
    }
}

func toRpn(infix: String) -> String {
    var output = Array[String]();
    var ops = Array[String]();
    for piece in infix.split(" ") {
        let token = piece.toOwned();
        if token == "(" {
            ops.append(token);
        } else if token == ")" {
            while let .Some(top) = ops.pop() {
                if top == "(" { break }
                output.append(top);
            }
        } else if precedence(token) > 0 {
            // Pop operators that bind tighter (or equal, for left-associative ones).
            while let .Some(top) = ops.last() {
                let p = precedence(top);
                let q = precedence(token);
                if p > q or (p == q and token != "^") {
                    output.append(ops.pop() ?? "");
                } else {
                    break
                }
            }
            ops.append(token);
        } else {
            output.append(token);
        }
    }
    while let .Some(op) = ops.pop() { output.append(op); }
    output.joined(" ")
}

@main
func main() {
    let infix = "3 + 4 * 2 / ( 1 - 5 ) ^ 2 ^ 3";
    println("infix: \(infix)");
    println("RPN:   \(toRpn(infix))");
}
