import { Link, useLocation } from "react-router";
import { BookOpen, Github, Package } from "lucide-react";
import ThemeToggle from "./ThemeToggle";

export default function Nav() {
  const location = useLocation();
  const isHome = location.pathname === "/";
  const isFlockActive = location.pathname.startsWith("/flock");

  return (
    <nav className="flex items-center justify-between px-6 py-6">
      {isHome ? (
        <div />
      ) : (
        <Link
          to="/"
          className="font-serif text-2xl font-bold text-[var(--color-slate)]"
        >
          Kestrel
        </Link>
      )}
      <div className="flex items-center gap-2">
        <span className="hidden sm:inline-flex items-center px-3 py-1.5 rounded-full bg-[var(--color-forest)]/10 text-[var(--color-forest)] font-mono text-xs font-medium">
          v0.15
        </span>
        <Link
          to="/flock"
          className={`inline-flex items-center gap-2 px-3 py-2 rounded-full transition-colors font-mono text-sm ${
            isFlockActive
              ? "text-[var(--color-rust)] bg-[var(--color-rust)]/10"
              : "text-[var(--color-slate)] hover:text-[#f5deb3] hover:bg-[var(--color-rust)]"
          }`}
          title="Packages"
        >
          <Package className="w-5 h-5" />
          {isFlockActive && "Flock"}
        </Link>
        <a
          href="https://github.com/jkpdino/kestrel/tree/main/docs"
          className="p-3 rounded-full text-[var(--color-slate)] hover:text-[#f5deb3] hover:bg-[var(--color-rust)] transition-colors"
          title="Documentation"
        >
          <BookOpen className="w-5 h-5" />
        </a>
        <ThemeToggle />
        <a
          href="https://github.com/jkpdino/kestrel"
          className="p-3 rounded-full text-[var(--color-slate)] hover:text-[#f5deb3] hover:bg-[var(--color-rust)] transition-colors"
          title="GitHub"
        >
          <Github className="w-6 h-6" />
        </a>
      </div>
    </nav>
  );
}
