import { ArrowRight, BookOpen, ChevronDown } from "lucide-react";
import { useEffect, useState } from "react";
import Nav from "./Nav";

export default function Hero() {
  const [isVisible, setIsVisible] = useState(false);

  useEffect(() => {
    setIsVisible(true);
  }, []);

  return (
    <section className="scroll-section bg-[#f8f6f1]">
      {/* Top nav */}
      <div className="absolute top-0 left-0 right-0 z-20">
        <Nav />
      </div>

      {/* Full background image - scaled down to show more of the trail */}
      <div className="absolute inset-0 flex items-center justify-center translate-x-32 -translate-y-2 md:translate-x-52 md:-translate-y-4 lg:translate-x-72 lg:-translate-y-4">
        <svg
          className="absolute h-[78%] w-[78%] overflow-visible -translate-x-2 -translate-y-20 md:-translate-x-4 md:-translate-y-32 lg:-translate-x-6 lg:-translate-y-40"
          viewBox="0 0 1000 600"
          preserveAspectRatio="xMidYMid meet"
          aria-hidden="true"
        >
          <defs>
            <marker
              id="hero-guide-arrow"
              viewBox="0 0 10 10"
              refX="9"
              refY="5"
              markerWidth="5"
              markerHeight="5"
              orient="auto-start-reverse"
            >
              <path d="M0 0 L10 5 L0 10 Z" fill="none" stroke="context-stroke" strokeOpacity="0.2" strokeWidth="1.5" strokeLinejoin="round" />
            </marker>
            {/* Knock holes in the vertical line at every dashed-circle intersection */}
            <mask id="hero-line-break" maskUnits="userSpaceOnUse" x="0" y="0" width="1000" height="900">
              <rect x="0" y="0" width="1000" height="900" fill="white" />
              <circle cx="430" cy="180" r="9" fill="black" />
              <circle cx="430" cy="260" r="9" fill="black" />
              <circle cx="430" cy="436" r="9" fill="black" />
              <circle cx="430" cy="516" r="9" fill="black" />
            </mask>
          </defs>
          <g
            className="dark:hidden"
            fill="#2c3e50"
            stroke="#2c3e50"
            strokeOpacity="0.16"
            fillOpacity="0.32"
            strokeWidth="1"
          >
            <path
              d="M430 140 L 430 730"
              fill="none"
              strokeDasharray="7 9"
              mask="url(#hero-line-break)"
              markerStart="url(#hero-guide-arrow)"
              markerEnd="url(#hero-guide-arrow)"
            />
            <path
              d="M 667 686 A 252 252 0 1 0 193 686"
              fill="none"
              strokeDasharray="7 9"
              markerStart="url(#hero-guide-arrow)"
              markerEnd="url(#hero-guide-arrow)"
            />
            <circle cx="430" cy="348" r="88" fill="none" strokeDasharray="7 9" />
            <circle cx="430" cy="348" r="168" fill="none" strokeDasharray="7 9" />
            <circle cx="430" cy="348" r="3.5" fill="none" />
            <circle cx="430" cy="180" r="2.5" fill="none" />
            <circle cx="430" cy="260" r="2.5" fill="none" />
            <circle cx="430" cy="436" r="2.5" fill="none" />
            <circle cx="430" cy="516" r="2.5" fill="none" />
            <circle cx="343" cy="363" r="2.5" fill="none" />
            <circle cx="517" cy="363" r="2.5" fill="none" />
            <circle cx="272" cy="404" r="2.5" fill="none" />
            <circle cx="588" cy="404" r="2.5" fill="none" />
          </g>
          <g
            className="hidden dark:block"
            fill="#d4a844"
            stroke="#d4a844"
            strokeOpacity="0.22"
            fillOpacity="0.45"
            strokeWidth="1"
          >
            <path
              d="M430 140 L 430 730"
              fill="none"
              strokeDasharray="7 9"
              mask="url(#hero-line-break)"
              markerStart="url(#hero-guide-arrow)"
              markerEnd="url(#hero-guide-arrow)"
            />
            <path
              d="M 667 686 A 252 252 0 1 0 193 686"
              fill="none"
              strokeDasharray="7 9"
              markerStart="url(#hero-guide-arrow)"
              markerEnd="url(#hero-guide-arrow)"
            />
            <circle cx="430" cy="348" r="88" fill="none" strokeDasharray="7 9" />
            <circle cx="430" cy="348" r="168" fill="none" strokeDasharray="7 9" />
            <circle cx="430" cy="348" r="3.5" fill="none" />
            <circle cx="430" cy="180" r="2.5" fill="none" />
            <circle cx="430" cy="260" r="2.5" fill="none" />
            <circle cx="430" cy="436" r="2.5" fill="none" />
            <circle cx="430" cy="516" r="2.5" fill="none" />
            <circle cx="343" cy="363" r="2.5" fill="none" />
            <circle cx="517" cy="363" r="2.5" fill="none" />
            <circle cx="272" cy="404" r="2.5" fill="none" />
            <circle cx="588" cy="404" r="2.5" fill="none" />
          </g>
        </svg>
        <img
          src="/kestrel-bird.png"
          alt=""
          className="relative z-10 max-w-[58%] max-h-[74%] object-contain"
        />
      </div>

      <div className="relative z-10 h-full flex flex-col justify-center px-6 md:px-12 lg:px-24 -translate-y-16 md:-translate-y-24">
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
            className={`mt-1 text-sm md:text-lg text-[var(--color-slate)] dark:text-white font-sans font-light uppercase tracking-[0.18em] md:tracking-[0.26em] transition-all duration-1000 delay-100 ${
              isVisible
                ? "opacity-100 translate-y-0"
                : "opacity-0 translate-y-10"
            }`}>
            Systems programming, refined
          </p>

          {/* Sub-tagline */}
          <p
            className={`mt-3 text-lg md:text-xl text-[var(--color-rust)] font-mono transition-all duration-1000 delay-200 ${
              isVisible
                ? "opacity-100 translate-y-0"
                : "opacity-0 translate-y-10"
            }`}>
            Low-level control. High-level clarity.
          </p>

          <div
            className={`mt-8 flex flex-wrap items-center gap-4 transition-all duration-1000 delay-300 ${
              isVisible
                ? "opacity-100 translate-y-0"
                : "opacity-0 translate-y-10"
            }`}>
            <a
              href="#get-started"
              className="inline-flex items-center gap-3 px-6 py-3 rounded-md bg-[var(--color-rust)] text-white font-sans text-sm font-medium hover:bg-[var(--color-rust-dark)] transition-colors"
            >
              Get Started
              <ArrowRight className="w-5 h-5" />
            </a>
            <a
              href="/reference/stdlib"
              className="inline-flex items-center gap-3 px-6 py-3 rounded-md border border-[var(--color-slate)]/35 dark:border-white/45 text-[var(--color-slate)] dark:text-white font-sans text-sm font-medium bg-transparent hover:border-[var(--color-rust)] hover:text-[var(--color-rust)] dark:hover:text-[var(--color-gold)] transition-colors"
            >
              Read the docs
              <BookOpen className="w-5 h-5" />
            </a>
          </div>
        </div>
      </div>

      {/* Scroll indicator */}
      <div
        className={`absolute bottom-6 left-1/2 z-20 -translate-x-1/2 transition-all duration-1000 delay-400 ${
          isVisible ? "opacity-100" : "opacity-0"
        }`}>
        <div className="flex flex-col items-center text-[var(--color-forest)] dark:text-[var(--color-gold)]">
          <span className="text-sm font-mono font-semibold uppercase tracking-[0.32em] mb-2">
            scroll to explore
          </span>
          <div className="flex flex-col items-center -space-y-2 animate-bounce">
            <ChevronDown className="w-4 h-4" strokeWidth={2.5} />
            <ChevronDown className="w-4 h-4" strokeWidth={2.5} />
          </div>
        </div>
      </div>
    </section>
  );
}
