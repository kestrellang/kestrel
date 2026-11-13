// Rosetta Code: Parsing/RPN calculator algorithm
// https://rosettacode.org/wiki/Parsing/RPN_calculator_algorithm
module Main

func evaluate(expr: String) -> Float64 {
    var stack = Array[Float64]();
    println("token  stack");
    for piece in expr.split(by: " ") {
        let token = piece.toOwned();
        if let some x = Float64(parsing: token) {
            stack.append(x);
        } else {
            let b = stack.popLast() ?? 0.0;
            let a = stack.popLast() ?? 0.0;
            let r: Float64 = match token {
                "+" => a + b,
                "-" => a - b,
                "*" => a * b,
                "/" => a / b,
                "^" => a.pow(b),
                _ => 0.0,
            };
            stack.append(r);
        }
        println("\(token:<6) \(stack)");
    }
    stack.popLast() ?? 0.0
}

@main
func main() {
    println("result: \(evaluate("3 4 2 * 1 5 - 2 3 ^ ^ / +"))");
}
