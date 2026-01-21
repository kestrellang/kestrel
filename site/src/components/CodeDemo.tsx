import { FileCode, Play } from "lucide-react";
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

enum Result[T, E] {
    case Ok(T)
    case Err(E)
}

func divide(a: Int, b: Int) -> Result[Int, String] {
    if b == 0 {
        return .Err("division by zero")
    }
    .Ok(a / b)
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
    var items: Array[T]

    mutating func push(item: T) {
        self.items.append(item)
    }

    mutating func pop() -> Option[T] {
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
    "true",
    "false",
    "public",
    "match",
    "extend",
    "it",
  ];
  const types = [
    "Int",
    "Float",
    "String",
    "Bool",
    "Array",
    "Option",
    "Result",
    "User",
    "Stack",
    "Serializable",
    "T",
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
      className="scroll-section bg-[var(--color-slate)]">
      {/* Subtle pattern */}
      <div className="absolute inset-0 opacity-[0.15]">
        <div
          className="absolute inset-0"
          style={{
            backgroundImage: `radial-gradient(circle at 2px 2px, white 1px, transparent 0)`,
            backgroundSize: "32px 32px",
          }}
        />
      </div>

      {/* Accent glow */}
      <div className="absolute top-1/4 -right-32 w-96 h-96 bg-[var(--color-rust)] opacity-10 blur-3xl rounded-full" />
      <div className="absolute bottom-1/4 -left-32 w-64 h-64 bg-[var(--color-forest)] opacity-10 blur-3xl rounded-full" />

      <div className="relative z-10 h-full flex flex-col justify-center px-6 md:px-12 lg:px-24 py-20">
        {/* Section header - left aligned */}
        <div
          className={`max-w-2xl mb-12 transition-all duration-1000 ${
            isVisible ? "opacity-100 translate-y-0" : "opacity-0 translate-y-10"
          }`}>
          <span className="font-mono text-[var(--color-rust-light)] text-sm uppercase tracking-widest">
            See It In Action
          </span>
          <h2 className="font-serif text-5xl md:text-6xl lg:text-7xl font-black text-white mt-4 tracking-tight">
            Expressive <span className="text-[var(--color-gold)]">by Default.</span>
          </h2>
          <p className="mt-4 text-xl text-white/60 font-mono">
            Clean syntax that reveals intent.
          </p>
        </div>

        {/* IDE-style editor */}
        <div
          className={`bg-[#1a2a3a] rounded-2xl shadow-2xl overflow-hidden transition-all duration-1000 delay-200 max-w-5xl border border-[#0d1a24] ${
            isVisible ? "opacity-100 translate-y-0" : "opacity-0 translate-y-10"
          }`}>
          <div className="flex flex-col">
            {/* Tab bar */}
            <div className="flex items-center justify-between pl-4 pr-3 py-2">
              <div className="flex items-center gap-2">
                {/* Traffic lights */}
                <div className="flex gap-2 mr-3">
                  <div className="w-3.5 h-3.5 rounded-full bg-[#ff5f57] hover:bg-[#ff3b30] transition-colors cursor-pointer" />
                  <div className="w-3.5 h-3.5 rounded-full bg-[#febc2e] hover:bg-[#f5a623] transition-colors cursor-pointer" />
                  <div className="w-3.5 h-3.5 rounded-full bg-[#28c840] hover:bg-[#1db954] transition-colors cursor-pointer" />
                </div>
                {/* Tabs */}
                {codeExamples.map((ex, i) => (
                  <button
                    key={i}
                    onClick={() => handleExampleChange(i)}
                    className={`flex items-center gap-2 px-3.5 py-2 rounded-lg font-mono text-xs transition-colors ${
                      i === currentExample
                        ? "bg-[#0d1a24] text-white"
                        : "text-white/50 hover:text-white/80"
                    }`}>
                    <FileCode className="w-4 h-4 flex-shrink-0" />
                    <span>{ex.filename}</span>
                  </button>
                ))}
              </div>
              {/* Run button */}
              <button
                onClick={runCode}
                className="p-2.5 rounded-full text-white/80 hover:text-white hover:bg-[#0d1a24] transition-colors"
                title="Run">
                <Play className="w-4 h-4 fill-current" />
              </button>
            </div>

            {/* Code area */}
            <div className="flex-1 flex flex-col min-w-0">
              {/* Code content */}
              <div className="flex-1 p-6 font-mono text-sm leading-relaxed min-h-[380px] overflow-x-auto">
                <pre className="text-gray-300 whitespace-pre-wrap">
                  {tokenize(example.code)}
                </pre>
              </div>

              {/* Output */}
              <div className="px-2 pb-2">
                <div className="bg-[#0d1a24] rounded-xl p-3 min-h-[80px] font-mono text-white">
                  <div className="flex items-center gap-2 mb-1">
                    <span className="text-white/50 text-[10px] font-semibold uppercase tracking-wider">Output</span>
                  </div>
                  {showOutput && (
                    <div className="mt-2">
                      {example.output.map((line, i) => (
                        <div
                          key={i}
                          className="text-sm"
                          style={{
                            animation: `fadeIn 0.3s ease-out ${i * 0.1}s forwards`,
                            opacity: 0,
                          }}>
                          {line}
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            </div>
          </div>

          {/* Mobile file selector */}
          <div className="md:hidden border-t border-white/10 p-2 flex gap-2 overflow-x-auto bg-[#0d1a24]">
            {codeExamples.map((ex, i) => (
              <button
                key={i}
                onClick={() => handleExampleChange(i)}
                className={`px-3 py-1.5 rounded-lg font-mono text-xs whitespace-nowrap transition-colors ${
                  i === currentExample
                    ? "bg-[var(--color-gold)] text-[var(--color-slate)]"
                    : "bg-white/10 text-white/50"
                }`}>
                {ex.title}
              </button>
            ))}
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
