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
    tagline: "Reusable code, concrete performance",
    points: [
      { text: "One function works across every type that satisfies its constraints" },
      { text: "Type relationships stay explicit at the call site and in the signature" },
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
    tagline: "Clear intent, direct machine work",
    points: [
      { text: "High-level APIs stay close to the generated code" },
      { text: "Swap allocators or runtimes without rewriting business logic" },
      { text: "No hidden overhead, only abstractions that earn their keep" },
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
    title: "Explicit Failure",
    tagline: "No null, no hidden exceptions",
    points: [
      { text: "Missing values are explicit with Optional" },
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
    tagline: "Visible lifetimes without ceremony",
    points: [
      { text: "Use-after-free caught at compile time" },
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
    codeBg: "bg-[#dce8e2] dark:bg-[#1c2d27]",
    accent: "text-[var(--color-forest)]",
    iconBg: "bg-[var(--color-forest)]/10",
    checkBg: "bg-[var(--color-forest)]/10",
  },
  rust: {
    bg: "bg-[#faf3f0]",
    codeBg: "bg-[#f0e2dc] dark:bg-[#2b211e]",
    accent: "text-[var(--color-rust)]",
    iconBg: "bg-[var(--color-rust)]/10",
    checkBg: "bg-[var(--color-rust)]/10",
  },
  gold: {
    bg: "bg-[#faf8f0]",
    codeBg: "bg-[#eee8d6] dark:bg-[#2b271d]",
    accent: "text-[var(--color-gold)]",
    iconBg: "bg-[var(--color-gold)]/15",
    checkBg: "bg-[var(--color-gold)]/15",
  },
  slate: {
    bg: "bg-[#f0f2f4]",
    codeBg: "bg-[#e1e6ea] dark:bg-[#25282e]",
    accent: "text-[var(--color-slate)]",
    iconBg: "bg-[var(--color-slate)]/10",
    checkBg: "bg-[var(--color-slate)]/10",
  },
};

// Intro section
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
      {/* Watercolor flight-path layer */}
      <div className="absolute inset-0 overflow-hidden">
        <svg
          className="absolute inset-0 h-full w-full"
          viewBox="0 0 1440 900"
          preserveAspectRatio="none"
          aria-hidden="true"
        >
          <defs>
            <filter id="features-ribbon-soften">
              <feGaussianBlur stdDeviation="4.5" />
            </filter>
            <linearGradient id="features-ribbon-rust" x1="0" y1="0" x2="1" y2="0">
              <stop offset="0%" stopColor="#c45a3b" stopOpacity="0" />
              <stop offset="42%" stopColor="#c45a3b" stopOpacity="0.16" />
              <stop offset="100%" stopColor="#d4a844" stopOpacity="0.04" />
            </linearGradient>
            <linearGradient id="features-ribbon-olive" x1="0" y1="0" x2="1" y2="0">
              <stop offset="0%" stopColor="#6f7f5d" stopOpacity="0" />
              <stop offset="48%" stopColor="#6f7f5d" stopOpacity="0.14" />
              <stop offset="100%" stopColor="#2d5a47" stopOpacity="0.03" />
            </linearGradient>
            <linearGradient id="features-ribbon-gold" x1="0" y1="0" x2="1" y2="0">
              <stop offset="0%" stopColor="#d4a844" stopOpacity="0" />
              <stop offset="45%" stopColor="#d4a844" stopOpacity="0.13" />
              <stop offset="100%" stopColor="#c45a3b" stopOpacity="0.03" />
            </linearGradient>
          </defs>

          <g filter="url(#features-ribbon-soften)" opacity="0.55">
            <path
              d="M-180 620 C 160 500, 360 230, 700 290 C 950 334, 1138 482, 1580 230"
              fill="none"
              stroke="url(#features-ribbon-rust)"
              strokeWidth="76"
              strokeLinecap="round"
            />
            <path
              d="M-150 735 C 190 650, 410 470, 680 410 C 960 348, 1190 560, 1580 438"
              fill="none"
              stroke="url(#features-ribbon-olive)"
              strokeWidth="60"
              strokeLinecap="round"
            />
            <path
              d="M-190 360 C 150 310, 330 170, 610 230 C 840 280, 1040 390, 1560 94"
              fill="none"
              stroke="url(#features-ribbon-gold)"
              strokeWidth="38"
              strokeLinecap="round"
            />
          </g>

          <g
            fill="none"
            stroke="#2c3e50"
            strokeOpacity="0.1"
            strokeWidth="1"
            strokeDasharray="8 10"
          >
            <path d="M86 730 C 270 520, 402 412, 642 356" />
            <path d="M212 190 C 452 260, 682 330, 958 244" />
            <path d="M134 452 L 512 452 L 512 174" />
            <circle cx="512" cy="452" r="118" />
            <circle cx="512" cy="452" r="214" />
          </g>
          <g fill="#2c3e50" opacity="0.12">
            <circle cx="134" cy="452" r="3" />
            <circle cx="512" cy="452" r="3" />
            <circle cx="512" cy="174" r="3" />
            <circle cx="958" cy="244" r="3" />
          </g>
        </svg>
      </div>

      <div className="relative z-10 h-full flex flex-col justify-center px-6 md:px-12 lg:px-24">
        <div
          className={`max-w-3xl transition-all duration-1000 ${
            isVisible ? "opacity-100 translate-y-0" : "opacity-0 translate-y-10"
          }`}
        >
          <span className="font-mono text-[var(--color-forest)] text-sm uppercase tracking-widest">
            Why Kestrel
          </span>
          <h2 className="font-serif text-5xl md:text-6xl lg:text-7xl font-black text-[var(--color-slate)] mt-4 tracking-tight">
            Control Without <span className="text-[var(--color-rust)]">Ceremony</span>
          </h2>
          <p className="mt-6 text-xl md:text-2xl text-[var(--color-slate-light)] font-mono max-w-2xl">
            Fast, explicit, readable from the first line to the metal
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
  const codeOnLeft = index % 2 === 0;

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

      <div className="relative z-10 min-h-full grid grid-cols-1 lg:grid-cols-12">
        {/* Code rail */}
        <aside
          className={`min-h-[42vh] lg:col-span-5 lg:min-h-screen ${colors.codeBg} flex items-center ${
            codeOnLeft ? "lg:order-1" : "lg:order-2"
          }`}
        >
          <div
            className={`w-full px-6 md:px-12 lg:px-8 xl:px-10 py-12 transition-all duration-700 delay-300 ${
              isVisible
                ? "opacity-100 translate-x-0"
                : codeOnLeft
                  ? "opacity-0 -translate-x-10"
                  : "opacity-0 translate-x-10"
            }`}
          >
            <div className="mb-6 flex items-center justify-between border-b border-[var(--color-slate)]/10 dark:border-white/10 pb-3">
              <span className="font-mono text-xs uppercase tracking-[0.24em] text-[var(--color-slate-light)] dark:text-white/40">
                source
              </span>
              <span className="font-mono text-sm text-[var(--color-gold)]">
                {feature.filename}
              </span>
            </div>
            <pre className="font-mono text-xs md:text-sm leading-relaxed text-[var(--color-slate)] dark:text-gray-300 whitespace-pre overflow-x-auto">
              {tokenize(feature.code)}
            </pre>
          </div>
        </aside>

        {/* Text content */}
        <div
          className={`lg:col-span-7 min-h-[58vh] lg:min-h-screen flex items-center px-6 md:px-12 lg:px-20 xl:px-24 py-12 lg:py-0 ${
            codeOnLeft ? "lg:order-2" : "lg:order-1"
          }`}
        >
          <div className="max-w-3xl space-y-6">
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
