import { Play } from "lucide-react";
import { useEffect, useRef, useState } from "react";

interface CodeExample {
  title: string;
  filename: string;
  description: string;
  code: string;
  output: string[];
}

const codeExamples: CodeExample[] = [
  {
    title: "Protocols",
    filename: "json.ks",
    description: "Define behavior, not inheritance",
    code: `// Define behavior, not inheritance

protocol Serializable {
    func toJson() -> String
}

struct User {
    let name: String
    let email: String
}

extend User: Serializable {
    func toJson() -> String {
        "{ \\"name\\": \\"\(self.name)\\", \\"email\\": \\"\(self.email)\\" }"
    }
}

let user = User(name: "Alice", email: "alice@example.com")
print(user.toJson())`,
    output: ['{ "name": "Alice", "email": "alice@example.com" }'],
  },
  {
    title: "Pattern Matching",
    filename: "result.ks",
    description: "Exhaustive matching, no forgotten cases",
    code: `// Exhaustive matching, no forgotten cases

func divide(a: Int, b: Int) -> Int throws String {
    if b == 0 {
        throw "division by zero"
    }
    a / b  // automatically promoted to .Ok
}

match divide(a: 10, b: 2) {
    .Ok(value)  => print("Result: \(value)"),
    .Err(msg)   => print("Error: \(msg)")
}`,
    output: ["Result: 5"],
  },
  {
    title: "Generics",
    filename: "stack.ks",
    description: "Full parametric polymorphism",
    code: `// Full parametric polymorphism

struct Stack[T] {
    var items: [T]

    mutating func push(item: T) {
        self.items.append(item)
    }

    mutating func pop() -> T? {
        self.items.pop()
    }
}

var stack = Stack[Int](items: [])
stack.push(item: 1)
stack.push(item: 2)
print(stack.pop())  // Some(2)`,
    output: ["Some(2)"],
  },
  {
    title: "Closures",
    filename: "filter.ks",
    description: "First-class functions",
    code: `// First-class functions with trailing closure syntax

let numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]

let evens = numbers
    .filter { it % 2 == 0 }
    .map { it * it }

// Equivalent to:
// .filter { (it) in it % 2 == 0 }
// .map { (it) in it * it }

for n in evens {
    print(n)
}`,
    output: ["4", "16", "36", "64", "100"],
  },
];

function tokenize(code: string): React.ReactNode[] {
  const keywords = [
    "struct",
    "enum",
    "case",
    "protocol",
    "func",
    "let",
    "var",
    "fn",
    "if",
    "else",
    "for",
    "in",
    "while",
    "return",
    "import",
    "init",
    "static",
    "mutating",
    "self",
    "Self",
    "true",
    "false",
    "public",
    "internal",
    "private",
    "match",
    "extend",
    "extension",
    "it",
    "deinit",
    "type",
    "module",
    "guard",
    "loop",
    "break",
    "continue",
    "try",
    "where",
    "as",
    "throw",
    "throws",
  ];
  const types = [
    "Int",
    "Float",
    "Float64",
    "Float32",
    "String",
    "Bool",
    "Array",
    "Option",
    "Result",
    "User",
    "Stack",
    "Serializable",
    "Error",
    "Void",
    "T",
    "U",
    "E",
  ];

  const tokens: React.ReactNode[] = [];
  let current = "";
  let i = 0;
  let key = 0;

  const pushCurrent = () => {
    if (current) {
      if (keywords.includes(current)) {
        tokens.push(
          <span key={key++} className="token-keyword">
            {current}
          </span>
        );
      } else if (types.includes(current)) {
        tokens.push(
          <span key={key++} className="token-type">
            {current}
          </span>
        );
      } else if (/^\d+(\.\d+)?$/.test(current)) {
        tokens.push(
          <span key={key++} className="token-number">
            {current}
          </span>
        );
      } else {
        tokens.push(<span key={key++}>{current}</span>);
      }
      current = "";
    }
  };

  while (i < code.length) {
    const char = code[i];

    // String literals
    if (char === '"') {
      pushCurrent();
      let str = '"';
      i++;
      while (i < code.length && code[i] !== '"') {
        str += code[i];
        i++;
      }
      str += '"';
      i++;
      tokens.push(
        <span key={key++} className="token-string">
          {str}
        </span>
      );
      continue;
    }

    // Comments
    if (char === "/" && code[i + 1] === "/") {
      pushCurrent();
      let comment = "";
      while (i < code.length && code[i] !== "\n") {
        comment += code[i];
        i++;
      }
      tokens.push(
        <span key={key++} className="token-comment">
          {comment}
        </span>
      );
      continue;
    }

    // Punctuation and operators
    if (/[{}()\[\]:;.,=<>+\-*/%|&!?@]/.test(char)) {
      pushCurrent();
      if (char === "-" && code[i + 1] === ">") {
        tokens.push(
          <span key={key++} className="token-operator">
            {"->"}
          </span>
        );
        i += 2;
        continue;
      }
      if (char === "=" && code[i + 1] === ">") {
        tokens.push(
          <span key={key++} className="token-operator">
            {"=>"}
          </span>
        );
        i += 2;
        continue;
      }
      if (char === "|" && code[i + 1] === "|") {
        tokens.push(
          <span key={key++} className="token-operator">
            ||
          </span>
        );
        i += 2;
        continue;
      }
      if (char === "?" && code[i + 1] === "?") {
        tokens.push(
          <span key={key++} className="token-operator">
            ??
          </span>
        );
        i += 2;
        continue;
      }
      tokens.push(
        <span key={key++} className="token-punctuation">
          {char}
        </span>
      );
      i++;
      continue;
    }

    // Whitespace
    if (/\s/.test(char)) {
      pushCurrent();
      tokens.push(<span key={key++}>{char}</span>);
      i++;
      continue;
    }

    // Identifiers
    current += char;
    i++;
  }

  pushCurrent();
  return tokens;
}

export default function CodeDemo() {
  const [currentExample, setCurrentExample] = useState(0);
  const [showOutput, setShowOutput] = useState(false);
  const [isVisible, setIsVisible] = useState(false);
  const sectionRef = useRef<HTMLElement>(null);

  useEffect(() => {
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting && !isVisible) {
          setIsVisible(true);
        }
      },
      { threshold: 0.3 }
    );

    if (sectionRef.current) {
      observer.observe(sectionRef.current);
    }

    return () => observer.disconnect();
  }, [isVisible]);

  const handleExampleChange = (index: number) => {
    setCurrentExample(index);
    setShowOutput(false);
  };

  const example = codeExamples[currentExample];

  const runCode = () => {
    setShowOutput(true);
  };

  return (
    <section
      ref={sectionRef}
      className="scroll-section bg-[#172536]">
      <div className="relative z-10 min-h-full grid grid-cols-1 lg:grid-cols-12">
        <div className="lg:col-span-5 min-h-[45vh] lg:min-h-screen flex items-center px-6 md:px-12 lg:px-16 xl:px-24 py-16 lg:py-0">
          <div
            className={`max-w-xl transition-all duration-1000 ${
              isVisible ? "opacity-100 translate-y-0" : "opacity-0 translate-y-10"
            }`}
          >
            <span className="font-mono text-[var(--color-rust-light)] text-sm uppercase tracking-widest">
              Code Sample
            </span>
            <h2 className="font-serif text-5xl md:text-6xl lg:text-7xl font-black text-white mt-4 tracking-tight">
              Source That <span className="text-[var(--color-gold)]">Stays Close</span>
            </h2>
            <p className="mt-4 text-xl text-white/60 font-mono">
              Readable syntax, predictable output
            </p>
          </div>
        </div>

        <div className="lg:col-span-7 min-h-[55vh] lg:min-h-screen bg-[#101b28] flex items-stretch">
          <div
            className={`w-full flex flex-col px-6 md:px-12 lg:px-12 xl:px-16 py-10 lg:py-14 transition-all duration-1000 delay-200 ${
              isVisible ? "opacity-100 translate-x-0" : "opacity-0 translate-x-10"
            }`}
          >
            <div className="flex items-center justify-between border-b border-white/10 pb-4">
              <div className="flex min-w-0 flex-1 items-center gap-2 overflow-x-auto pr-4">
                {codeExamples.map((ex, i) => (
                  <button
                    key={i}
                    onClick={() => handleExampleChange(i)}
                    className={`shrink-0 rounded-md px-3 py-2 font-mono text-xs transition-colors ${
                      i === currentExample
                        ? "bg-[var(--color-gold)]/12 text-[var(--color-gold)]"
                        : "text-white/45 hover:bg-white/[0.06] hover:text-white/70"
                    }`}
                    title={ex.description}
                  >
                    {ex.filename}
                  </button>
                ))}
              </div>
              <button
                onClick={runCode}
                className="inline-flex items-center gap-2 rounded-md border border-[var(--color-gold)]/45 px-4 py-2 font-mono text-xs text-[var(--color-gold)] hover:bg-[var(--color-gold)]/10 hover:border-[var(--color-gold)] transition-colors"
                title="Run"
              >
                <Play className="w-3.5 h-3.5 fill-current" />
                run
              </button>
            </div>

            <div className="flex-1 min-h-0 overflow-x-auto py-8">
              <pre className="font-mono text-xs md:text-sm leading-relaxed text-gray-300 whitespace-pre-wrap">
                {tokenize(example.code)}
              </pre>
            </div>

            <div className="border-t border-white/10 pt-4 font-mono text-white">
              <div className="flex items-center justify-between">
                <span className="text-[10px] font-semibold uppercase tracking-[0.24em] text-white/35">
                  Output
                </span>
                {!showOutput && (
                  <span className="text-xs text-white/30">waiting for run</span>
                )}
              </div>
              <div className="mt-3 min-h-[72px] text-sm text-white/85">
                {showOutput &&
                  example.output.map((line, i) => (
                    <div
                      key={i}
                      style={{
                        animation: `fadeIn 0.3s ease-out ${i * 0.1}s forwards`,
                        opacity: 0,
                      }}
                    >
                      {line}
                    </div>
                  ))}
              </div>
            </div>
          </div>
        </div>
      </div>

      <style>{`
        @keyframes fadeIn {
          from { opacity: 0; transform: translateY(5px); }
          to { opacity: 1; transform: translateY(0); }
        }
      `}</style>
    </section>
  );
}
