import { ChevronDown, Github } from "lucide-react";
import { useEffect, useState } from "react";

export default function Hero() {
  const [isVisible, setIsVisible] = useState(false);

  useEffect(() => {
    setIsVisible(true);
  }, []);

  return (
    <section className="scroll-section bg-[#f8f6f1]">
      {/* GitHub icon in top right */}
      <a
        href="https://github.com/jkpdino/kestrel"
        className="absolute top-6 right-6 z-20 p-3 rounded-full text-[var(--color-slate)] hover:text-[#f5deb3] hover:bg-[var(--color-rust)]">
        <Github className="w-6 h-6" />
      </a>

      {/* Full background image - scaled down to show more of the trail */}
      <div className="absolute inset-0 flex items-center justify-center">
        <img
          src="/kestrel.jpeg"
          alt=""
          className="max-w-[85%] max-h-[85%] object-contain"
        />
      </div>

      {/* Gradient overlays for text readability */}
      <div className="absolute inset-0 bg-gradient-to-r from-[#f8f6f1] via-[#f8f6f1]/80 to-transparent" />
      <div className="absolute inset-0 bg-gradient-to-b from-[#f8f6f1]/50 via-transparent to-[#f8f6f1]/70" />

      <div className="relative z-10 h-full flex flex-col justify-center px-6 md:px-12 lg:px-24">
        <div className="max-w-2xl">
          {/* Main title */}
          <h1
            className={`font-serif text-7xl md:text-8xl lg:text-9xl font-black text-[var(--color-slate)] tracking-tight transition-all duration-1000 ${
              isVisible
                ? "opacity-100 translate-y-0"
                : "opacity-0 translate-y-10"
            }`}
            style={{ fontFamily: "var(--font-serif)" }}>
            Kestrel
          </h1>

          {/* Value prop */}
          <p
            className={`mt-4 text-xl md:text-2xl text-[var(--color-slate-light)] font-serif transition-all duration-1000 delay-100 ${
              isVisible
                ? "opacity-100 translate-y-0"
                : "opacity-0 translate-y-10"
            }`}>
            Systems programming, refined.
          </p>

          {/* Sub-tagline */}
          <p
            className={`mt-3 text-lg md:text-xl text-[var(--color-rust)] font-mono transition-all duration-1000 delay-200 ${
              isVisible
                ? "opacity-100 translate-y-0"
                : "opacity-0 translate-y-10"
            }`}>
            Bare metal power. Zero mental overhead.
          </p>
        </div>

        {/* Scroll indicator */}
        <div
          className={`absolute bottom-8 left-1/2 -translate-x-1/2 transition-all duration-1000 delay-400 ${
            isVisible ? "opacity-100" : "opacity-0"
          }`}>
          <div className="flex flex-col items-center text-[var(--color-forest)]">
            <span className="text-sm font-mono mb-2">scroll to explore</span>
            <ChevronDown className="w-5 h-5 text-[var(--color-forest)]" />
          </div>
        </div>
      </div>
    </section>
  );
}
