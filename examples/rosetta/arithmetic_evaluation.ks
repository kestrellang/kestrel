// Rosetta Code: Arithmetic evaluation
// https://rosettacode.org/wiki/Arithmetic_evaluation
// A recursive-descent parser builds an AST, which is then evaluated.
module Main

indirect enum Expr {
    case Num(Float64)
    case Binary(op: Char, left: Expr, right: Expr)
}

struct Parser {
    var chars: Array[Char];
    var pos: Int64;

    init(parsing text: String) {
        self.chars = text.chars.iter().filter { not it.isWhitespace }.collect();
        self.pos = 0;
    }

    func peek() -> Char? {
        if self.pos < self.chars.count { .Some(self.chars(self.pos)) } else { .None }
    }

    // expr := term (('+' | '-') term)*
    mutating func expr() -> Expr {
        var left = self.term();
        while let .Some(c) = self.peek() {
            if c != '+' and c != '-' { break }
            self.pos = self.pos + 1;
            left = .Binary(op: c, left: left, right: self.term());
        }
        left
    }

    // term := factor (('*' | '/') factor)*
    mutating func term() -> Expr {
        var left = self.factor();
        while let .Some(c) = self.peek() {
            if c != '*' and c != '/' { break }
            self.pos = self.pos + 1;
            left = .Binary(op: c, left: left, right: self.factor());
        }
        left
    }

    // factor := number | '(' expr ')'
    mutating func factor() -> Expr {
        if self.peek() == .Some('(') {
            self.pos = self.pos + 1;
            let inner = self.expr();
            self.pos = self.pos + 1;
            return inner
        }
        var digits = StringBuilder();
        while let .Some(c) = self.peek() {
            if not c.isAsciiDigit and c != '.' { break }
            digits.append(char: c);
            self.pos = self.pos + 1;
        }
        .Num(Float64(parsing: digits.build()) ?? 0.0)
    }
}

func eval(e: Expr) -> Float64 {
    match e {
        .Num(x) => x,
        .Binary(op, l, r) => {
            let a = eval(l);
            let b = eval(r);
            match op {
                '+' => a + b,
                '-' => a - b,
                '*' => a * b,
                _ => a / b,
            }
        },
    }
}

@main
func main() {
    for text in ["(1+3)*7", "2*(3+4)*5", "1 + 2 * 3 - 4 / 8", "((2.5))*4", "100/(2*(3+2))"] {
        var parser = Parser(parsing: text);
        println("\(text) = \(eval(parser.expr()))");
    }
}
