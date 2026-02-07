import { Check, Code2, Cpu, Lock, ShieldCheck } from "lucide-react";
import type { ComponentType } from "react";
import { useEffect, useRef, useState } from "react";

interface FeaturePoint {
  text: string;
}

interface Feature {
  Icon: ComponentType<{ className?: string; strokeWidth?: number }>;
  title: string;
  tagline: string;
  points: FeaturePoint[];
  code: string;
  filename: string;
  color: "forest" | "rust" | "gold" | "slate";
}

const features: Feature[] = [
  {
    Icon: Code2,
    title: "Powerful Generics",
    tagline: "Write it once. Use it everywhere.",
    points: [
      { text: "No copy-paste code—one function works for any type that fits" },
      { text: "The compiler catches mistakes before production does" },
      { text: "Extend types you didn't write, even from dependencies" },
    ],
    color: "forest",
    filename: "map.ks",
    code: `func map[T, U](array: [T], transform: (T) -> U) -> [U] {
    var result: [U] = []
    for item in array {
        result.append(transform(item))
    }
    result
}

// Works with any type
let numbers = [1, 2, 3].map { it * 2 }
let names = users.map { it.name }`,
  },
  {
    Icon: Cpu,
    title: "Zero-Cost Abstractions",
    tagline: "Clean code that compiles to what you'd write by hand.",
    points: [
      { text: "High-level APIs, low-level performance" },
      { text: "Swap allocators or runtimes without rewriting business logic" },
      { text: "No hidden overhead—pay only for what you use" },
    ],
    color: "rust",
    filename: "orders.ks",
    code: `// High-level code...
let total = orders
    .filter { it.status == .Completed }
    .map { it.total }
    .reduce(0) { sum, n in sum + n }

// ...compiles to a single loop
// No intermediate allocations
// No closure overhead`,
  },
  {
    Icon: ShieldCheck,
    title: "Error Handling Done Right",
    tagline: "If it can fail, you'll know—and so will your code.",
    points: [
      { text: "Null doesn't exist. Missing values are explicit with Option" },
      { text: "Functions that fail say so in their signature" },
      { text: "The compiler won't let you forget a case" },
    ],
    color: "gold",
    filename: "user.ks",
    code: `func findUser(id: Int) -> User? {
    users.first { it.id == id }
}

// The compiler ensures you handle None
match findUser(id: 42) {
    .Some(user) => print("Hello, \\(user.name)"),
    .None       => print("User not found")
}`,
  },
  {
    Icon: Lock,
    title: "Memory Safety",
    tagline: "Ownership rules out the bugs. You ship the features.",
    points: [
      { text: "No segfaults—use-after-free caught at compile time" },
      { text: "Predictable cleanup when values go out of scope" },
      { text: "Safe defaults; opt into manual control when you need it" },
    ],
    color: "forest",
    filename: "file.ks",
    code: `struct File {
    let handle: FileHandle

    // Automatic cleanup when File goes out of scope
    deinit {
        handle.close()
    }
}

func process() {
    let file = File.open("data.txt")
    // use file...
}   // file.deinit called here—always`,
  },
];

function tokenize(code: string): React.ReactNode[] {
  const keywords = [
    "struct", "enum", "case", "protocol", "func", "let", "var",
    "if", "else", "for", "in", "while", "return", "match",
    "extend", "extension", "it", "self", "Self", "deinit", "init",
    "true", "false", "type", "import", "module", "public", "internal",
    "private", "guard", "loop", "break", "continue", "try", "where",
    "mutating", "static", "as", "throw", "throws",
  ];
  const types = [
    "Int", "String", "Bool", "Array", "Option", "Result", "User", "File",
    "FileHandle", "T", "U", "E", "Error", "Float64", "Float32", "Void",
  ];

  const tokens: React.ReactNode[] = [];
  let current = "";
  let i = 0;
  let key = 0;

  const pushCurrent = () => {
    if (current) {
      if (keywords.includes(current)) {
        tokens.push(<span key={key++} className="token-keyword">{current}</span>);
      } else if (types.includes(current)) {
        tokens.push(<span key={key++} className="token-type">{current}</span>);
      } else if (/^\d+(\.\d+)?$/.test(current)) {
        tokens.push(<span key={key++} className="token-number">{current}</span>);
      } else {
        tokens.push(<span key={key++}>{current}</span>);
      }
      current = "";
    }
  };

  while (i < code.length) {
    const char = code[i];

    if (char === '"') {
      pushCurrent();
      let str = '"';
      i++;
      while (i < code.length && code[i] !== '"') {
        if (code[i] === '\\' && i + 1 < code.length) {
          str += code[i] + code[i + 1];
          i += 2;
        } else {
          str += code[i];
          i++;
        }
      }
      str += '"';
      i++;
      tokens.push(<span key={key++} className="token-string">{str}</span>);
      continue;
    }

    if (char === "/" && code[i + 1] === "/") {
      pushCurrent();
      let comment = "";
      while (i < code.length && code[i] !== "\n") {
        comment += code[i];
        i++;
      }
      tokens.push(<span key={key++} className="token-comment">{comment}</span>);
      continue;
    }

    if (/[{}()\[\]:;.,=<>+\-*/%|&!?@]/.test(char)) {
      pushCurrent();
      if (char === "-" && code[i + 1] === ">") {
        tokens.push(<span key={key++} className="token-operator">{"->"}</span>);
        i += 2;
        continue;
      }
      if (char === "=" && code[i + 1] === ">") {
        tokens.push(<span key={key++} className="token-operator">{"=>"}</span>);
        i += 2;
        continue;
      }
      if (char === "=" && code[i + 1] === "=") {
        tokens.push(<span key={key++} className="token-operator">{"=="}</span>);
        i += 2;
        continue;
      }
      tokens.push(<span key={key++} className="token-punctuation">{char}</span>);
      i++;
      continue;
    }

    if (/\s/.test(char)) {
      pushCurrent();
      tokens.push(<span key={key++}>{char}</span>);
      i++;
      continue;
    }

    current += char;
    i++;
  }

  pushCurrent();
  return tokens;
}

const colorMap = {
  forest: {
    bg: "bg-[#e8f0ec]",
    accent: "text-[var(--color-forest)]",
    iconBg: "bg-[var(--color-forest)]/10",
    checkBg: "bg-[var(--color-forest)]/10",
  },
  rust: {
    bg: "bg-[#faf3f0]",
    accent: "text-[var(--color-rust)]",
    iconBg: "bg-[var(--color-rust)]/10",
    checkBg: "bg-[var(--color-rust)]/10",
  },
  gold: {
    bg: "bg-[#faf8f0]",
    accent: "text-[var(--color-gold)]",
    iconBg: "bg-[var(--color-gold)]/15",
    checkBg: "bg-[var(--color-gold)]/15",
  },
  slate: {
    bg: "bg-[#f0f2f4]",
    accent: "text-[var(--color-slate)]",
    iconBg: "bg-[var(--color-slate)]/10",
    checkBg: "bg-[var(--color-slate)]/10",
  },
};

// Intro section - "Fundamentals, Done Right"
function FeaturesIntro() {
  const [isVisible, setIsVisible] = useState(false);
  const sectionRef = useRef<HTMLElement>(null);

  useEffect(() => {
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setIsVisible(true);
        }
      },
      { threshold: 0.3 }
    );

    if (sectionRef.current) {
      observer.observe(sectionRef.current);
    }

    return () => observer.disconnect();
  }, []);

  return (
    <section
      ref={sectionRef}
      className="scroll-section bg-[#f0ebe3]"
    >
      {/* Hexagon pattern */}
      <div className="absolute inset-0 opacity-[0.06]">
        <svg className="absolute inset-0 w-full h-full">
          <defs>
            <pattern id="hexagons-intro" width="56" height="100" patternUnits="userSpaceOnUse" patternTransform="scale(0.5)">
              <path
                d="M28 66L0 50L0 16L28 0L56 16L56 50L28 66L28 100"
                fill="none"
                stroke="#8b6914"
                strokeWidth="1"
              />
              <path
                d="M28 0L28 34L0 50L0 84L28 100L56 84L56 50L28 34"
                fill="none"
                stroke="#8b6914"
                strokeWidth="1"
              />
            </pattern>
          </defs>
          <rect width="100%" height="100%" fill="url(#hexagons-intro)" />
        </svg>
      </div>

      <div className="relative z-10 h-full flex flex-col justify-center px-6 md:px-12 lg:px-24">
        <div
          className={`max-w-3xl transition-all duration-1000 ${
            isVisible ? "opacity-100 translate-y-0" : "opacity-0 translate-y-10"
          }`}
        >
          <span className="font-mono text-[var(--color-forest)] text-sm uppercase tracking-widest">
            Why Kestrel?
          </span>
          <h2 className="font-serif text-5xl md:text-6xl lg:text-7xl font-black text-[var(--color-slate)] mt-4 tracking-tight">
            Systems Programming, <span className="text-[var(--color-rust)]">Refined.</span>
          </h2>
          <p className="mt-6 text-xl md:text-2xl text-[var(--color-slate-light)] font-mono max-w-2xl">
            The good parts, without the baggage.
          </p>
        </div>

        {/* Feature preview pills */}
        <div
          className={`mt-12 flex flex-wrap gap-3 transition-all duration-1000 delay-300 ${
            isVisible ? "opacity-100 translate-y-0" : "opacity-0 translate-y-10"
          }`}
        >
          {features.map((feature, i) => (
            <div
              key={i}
              className="flex items-center gap-2 px-4 py-2 rounded-full bg-white/60 border border-[var(--color-slate)]/10"
            >
              <feature.Icon className="w-4 h-4 text-[var(--color-slate-light)]" strokeWidth={1.5} />
              <span className="font-mono text-sm text-[var(--color-slate)]">{feature.title}</span>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

// Individual feature section
function FeatureSection({ feature, index }: { feature: Feature; index: number }) {
  const [isVisible, setIsVisible] = useState(false);
  const sectionRef = useRef<HTMLElement>(null);
  const colors = colorMap[feature.color];

  useEffect(() => {
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setIsVisible(true);
        }
      },
      { threshold: 0.3 }
    );

    if (sectionRef.current) {
      observer.observe(sectionRef.current);
    }

    return () => observer.disconnect();
  }, []);

  return (
    <section
      ref={sectionRef}
      className={`scroll-section ${colors.bg} overflow-y-auto`}
    >
      {/* Subtle pattern overlay */}
      <div className="absolute inset-0 opacity-[0.03]">
        <div
          className="absolute inset-0"
          style={{
            backgroundImage: `radial-gradient(circle at 2px 2px, currentColor 1px, transparent 0)`,
            backgroundSize: "24px 24px",
          }}
        />
      </div>

      <div className="relative z-10 min-h-full flex items-center px-6 md:px-12 lg:px-24 py-12 lg:py-0">
        <div className="w-full max-w-6xl mx-auto grid grid-cols-1 lg:grid-cols-2 gap-8 lg:gap-16 items-center">
          {/* Text content */}
          <div className="space-y-6">
            {/* Icon + label */}
            <div
              className={`flex items-center gap-3 transition-all duration-700 ${
                isVisible ? "opacity-100 translate-x-0" : "opacity-0 -translate-x-10"
              }`}
            >
              <div className={`w-12 h-12 rounded-xl ${colors.iconBg} flex items-center justify-center`}>
                <feature.Icon className={`w-6 h-6 ${colors.accent}`} strokeWidth={1.5} />
              </div>
              <span className={`font-mono text-sm ${colors.accent} uppercase tracking-wider`}>
                0{index + 1}
              </span>
            </div>

            {/* Title */}
            <h3
              className={`font-serif text-4xl md:text-5xl font-black text-[var(--color-slate)] tracking-tight transition-all duration-700 delay-100 ${
                isVisible ? "opacity-100 translate-y-0" : "opacity-0 translate-y-10"
              }`}
            >
              {feature.title}
            </h3>

            {/* Tagline */}
            <p
              className={`text-xl ${colors.accent} font-mono transition-all duration-700 delay-150 ${
                isVisible ? "opacity-100 translate-y-0" : "opacity-0 translate-y-10"
              }`}
            >
              {feature.tagline}
            </p>

            {/* Points */}
            <ul className="space-y-4 pt-2">
              {feature.points.map((point, i) => (
                <li
                  key={i}
                  className={`flex items-start gap-3 transition-all duration-500 ${
                    isVisible ? "opacity-100 translate-x-0" : "opacity-0 -translate-x-10"
                  }`}
                  style={{ transitionDelay: `${200 + i * 100}ms` }}
                >
                  <div className={`w-6 h-6 rounded-full ${colors.checkBg} flex items-center justify-center flex-shrink-0 mt-0.5`}>
                    <Check className={`w-3.5 h-3.5 ${colors.accent}`} strokeWidth={2.5} />
                  </div>
                  <span className="text-[var(--color-slate-light)] font-mono text-sm leading-relaxed">
                    {point.text}
                  </span>
                </li>
              ))}
            </ul>
          </div>

          {/* Code visual */}
          <div
            className={`mt-8 lg:mt-0 transition-all duration-700 delay-300 ${
              isVisible ? "opacity-100 translate-x-0" : "opacity-0 translate-x-10"
            }`}
          >
            <div className="bg-[#1a2a3a] rounded-2xl shadow-2xl overflow-hidden border border-[#0d1a24]">
              {/* Window chrome */}
              <div className="flex items-center pl-4 pr-3 py-2">
                <div className="flex gap-2 mr-3">
                  <div className="w-3.5 h-3.5 rounded-full bg-[#ff5f57]" />
                  <div className="w-3.5 h-3.5 rounded-full bg-[#febc2e]" />
                  <div className="w-3.5 h-3.5 rounded-full bg-[#28c840]" />
                </div>
                <span className="px-3.5 py-2 font-mono text-sm text-white/40">{feature.filename}</span>
              </div>

              {/* Code */}
              <div className="px-6 pb-6 font-mono text-sm leading-relaxed overflow-x-auto">
                <pre className="text-gray-300 whitespace-pre">
                  {tokenize(feature.code)}
                </pre>
              </div>
            </div>
          </div>
        </div>
      </div>

    </section>
  );
}

// Export all sections as a fragment
export default function Features() {
  return (
    <>
      <FeaturesIntro />
      {features.map((feature, index) => (
        <FeatureSection key={index} feature={feature} index={index} />
      ))}
    </>
  );
}
