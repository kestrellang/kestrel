import { Check, Copy, Github } from "lucide-react";
import { useEffect, useRef, useState } from "react";

const installCommand = "curl --proto '=https' --tlsv1.2 -sSf https://kestrel-lang.com/install.sh | sh";

export default function GetStarted() {
  const [isVisible, setIsVisible] = useState(false);
  const [copiedInstall, setCopiedInstall] = useState(false);
  const sectionRef = useRef<HTMLElement>(null);

  useEffect(() => {
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setIsVisible(true);
        }
      },
      { threshold: 0.2 }
    );

    if (sectionRef.current) {
      observer.observe(sectionRef.current);
    }

    return () => observer.disconnect();
  }, []);

  const copyInstallCommand = async () => {
    await navigator.clipboard.writeText(installCommand);
    setCopiedInstall(true);
    setTimeout(() => setCopiedInstall(false), 2000);
  };

  return (
    <section
      ref={sectionRef}
      id="get-started"
      className="relative bg-[#16251f] overflow-hidden flex-grow">
      {/* Technical pattern */}
      <div className="absolute inset-0 opacity-[0.08]">
        <div
          className="absolute inset-0"
          style={{
            backgroundImage: `
              linear-gradient(rgba(244, 223, 180, 0.28) 1px, transparent 1px),
              linear-gradient(90deg, rgba(244, 223, 180, 0.28) 1px, transparent 1px),
              radial-gradient(circle at 2px 2px, rgba(244, 223, 180, 0.45) 1px, transparent 0)
            `,
            backgroundSize: "96px 96px, 96px 96px, 32px 32px",
          }}
        />
      </div>

      {/* Watercolor flight arc — the closing "take flight" gesture */}
      <div className="absolute inset-0 overflow-hidden pointer-events-none">
        <svg
          className="absolute inset-0 h-full w-full"
          viewBox="0 0 1440 900"
          preserveAspectRatio="none"
          aria-hidden="true">
          <defs>
            <filter id="getstarted-soften">
              <feGaussianBlur stdDeviation="6" />
            </filter>
            <linearGradient id="getstarted-arc-gold" x1="0" y1="1" x2="1" y2="0">
              <stop offset="0%" stopColor="#d4a844" stopOpacity="0" />
              <stop offset="55%" stopColor="#d4a844" stopOpacity="0.32" />
              <stop offset="100%" stopColor="#f4dfb4" stopOpacity="0.05" />
            </linearGradient>
            <linearGradient id="getstarted-arc-olive" x1="0" y1="1" x2="1" y2="0">
              <stop offset="0%" stopColor="#6f7f5d" stopOpacity="0" />
              <stop offset="50%" stopColor="#6f7f5d" stopOpacity="0.22" />
              <stop offset="100%" stopColor="#6f7f5d" stopOpacity="0" />
            </linearGradient>
            <linearGradient id="getstarted-horizon" x1="0" y1="1" x2="0" y2="0">
              <stop offset="0%" stopColor="#d4a844" stopOpacity="0.18" />
              <stop offset="100%" stopColor="#d4a844" stopOpacity="0" />
            </linearGradient>
          </defs>

          <g filter="url(#getstarted-soften)" opacity="0.85">
            <path
              d="M-220 820 C 320 700, 720 240, 1640 -60"
              fill="none"
              stroke="url(#getstarted-arc-gold)"
              strokeWidth="56"
              strokeLinecap="round"
            />
            <path
              d="M-180 880 C 360 780, 760 420, 1620 140"
              fill="none"
              stroke="url(#getstarted-arc-olive)"
              strokeWidth="42"
              strokeLinecap="round"
            />
          </g>

          <rect
            x="0"
            y="720"
            width="1440"
            height="180"
            fill="url(#getstarted-horizon)"
          />

          <g
            fill="none"
            stroke="#f4dfb4"
            strokeOpacity="0.18"
            strokeWidth="1"
            strokeDasharray="6 12">
            <path d="M-40 760 C 360 620, 720 240, 1500 20" />
          </g>
        </svg>
      </div>

      <div className="relative z-10 h-full flex flex-col justify-center px-6 md:px-12 lg:px-24 py-20">
        {/* Section header - left aligned */}
        <div
          className={`max-w-2xl mb-12 transition-all duration-1000 ${
            isVisible ? "opacity-100 translate-y-0" : "opacity-0 translate-y-10"
          }`}>
          <span className="font-mono text-[var(--color-gold)] text-sm uppercase tracking-widest">
            Get Started
          </span>
          <h2 className="font-serif text-5xl md:text-6xl lg:text-7xl font-black text-white mt-4 tracking-tight">
            Take <span className="text-[var(--color-gold)]">Flight</span>
          </h2>
          <p className="mt-4 text-xl text-white/70 font-serif">
            Install the toolchain and run your first Kestrel program
          </p>
        </div>

        {/* Install command */}
        <div
          className={`max-w-4xl transition-all duration-1000 delay-100 ${
            isVisible ? "opacity-100 translate-y-0" : "opacity-0 translate-y-10"
          }`}>
          <div className="relative bg-[#0f1b18] border border-[var(--color-gold)]/25 rounded-md p-5 pr-16">
            <pre className="font-mono text-sm md:text-base text-gray-300 overflow-x-auto">
              <span className="text-[var(--color-gold)]">$ </span>
              {installCommand}
            </pre>
            <button
              onClick={copyInstallCommand}
              aria-label={copiedInstall ? "Copied" : "Copy install command"}
              className="absolute top-1/2 -translate-y-1/2 right-3 p-2 rounded-md text-gray-400 bg-white/5 hover:bg-white/10 hover:text-white focus-visible:bg-white/10 focus-visible:text-white transition-colors">
              {copiedInstall ? (
                <Check className="w-4 h-4 text-[var(--color-gold)]" />
              ) : (
                <Copy className="w-4 h-4" />
              )}
            </button>
          </div>
          <p className="mt-3 text-white/50 font-mono text-xs">
            Installs Jessup, the Kestrel version manager, and the latest stable toolchain.{" "}
            <a
              href="/install.sh"
              className="text-[var(--color-gold)]/80 hover:text-[var(--color-gold)] underline underline-offset-2">
              View the script
            </a>
            .
          </p>
        </div>

        {/* CTA */}
        <div
          className={`mt-12 transition-all duration-1000 delay-400 ${
            isVisible ? "opacity-100 translate-y-0" : "opacity-0 translate-y-10"
          }`}>
          <a
            href="https://github.com/jkpdino/kestrel"
            className="inline-flex items-center gap-2.5 px-5 py-2.5 rounded-md border border-[var(--color-gold)]/45 text-[var(--color-gold)] bg-transparent font-mono text-sm hover:bg-[var(--color-gold)]/10 hover:border-[var(--color-gold)] transition-colors">
            <Github className="w-4 h-4" />
            Source on GitHub
          </a>
        </div>
      </div>
    </section>
  );
}
