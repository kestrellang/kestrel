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
      className="relative bg-[var(--color-forest)] overflow-hidden flex-grow">
      {/* Subtle pattern - diagonal lines */}
      <div className="absolute inset-0 opacity-25">
        <div
          className="absolute inset-0"
          style={{
            backgroundImage: `repeating-linear-gradient(-45deg, transparent, transparent 24px, #1a3328 24px, #1a3328 27px)`,
          }}
        />
      </div>

      {/* Accent glow */}
      <div className="absolute top-1/4 -right-32 w-96 h-96 bg-[var(--color-gold)] opacity-20 blur-3xl rounded-full" />
      <div className="absolute bottom-1/4 -left-32 w-64 h-64 bg-[var(--color-cream)] opacity-10 blur-3xl rounded-full" />

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
            Take <span className="text-[var(--color-gold)]">Flight.</span>
          </h2>
          <p className="mt-4 text-xl text-white/70 font-serif">
            One command to install. One command to run.
          </p>
        </div>

        {/* Install command */}
        <div
          className={`max-w-4xl transition-all duration-1000 delay-100 ${
            isVisible ? "opacity-100 translate-y-0" : "opacity-0 translate-y-10"
          }`}>
          <div className="relative bg-[var(--color-slate)] rounded-xl p-5 group/install">
            <pre className="font-mono text-sm md:text-base text-gray-300 overflow-x-auto">
              <span className="text-[var(--color-gold)]">$ </span>
              {installCommand}
            </pre>
            <button
              onClick={copyInstallCommand}
              className="absolute top-1/2 -translate-y-1/2 right-4 p-2 bg-white/10 rounded-lg font-mono text-xs text-gray-400 hover:bg-white/20 hover:text-white transition-all opacity-0 group-hover/install:opacity-100">
              {copiedInstall ? (
                <Check className="w-4 h-4 text-[var(--color-gold)]" />
              ) : (
                <Copy className="w-4 h-4" />
              )}
            </button>
          </div>
          <p className="mt-3 text-white/50 font-mono text-xs">
            Installs kestrel and adds it to your PATH. Run kestrel --help to get started.
          </p>
        </div>

        {/* CTA */}
        <div
          className={`mt-12 transition-all duration-1000 delay-400 ${
            isVisible ? "opacity-100 translate-y-0" : "opacity-0 translate-y-10"
          }`}>
          <a
            href="https://github.com/jkpdino/kestrel"
            className="inline-flex items-center gap-3 px-8 py-4 bg-[var(--color-gold)] text-[var(--color-slate)] font-serif text-lg font-bold rounded-xl hover:bg-[var(--color-cream)]">
            <Github className="w-5 h-5" />
            View on GitHub
          </a>
          <p className="mt-4 text-white/60 font-mono text-sm">
            Open source. MIT licensed. Contributions welcome.
          </p>
        </div>
      </div>
    </section>
  );
}
