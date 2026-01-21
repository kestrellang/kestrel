import { Github, Heart } from "lucide-react";

export default function Footer() {
  return (
    <footer className="bg-[var(--color-slate)] py-12 relative overflow-hidden">
      {/* Subtle top border */}
      <div className="absolute top-0 left-0 right-0 h-px bg-gradient-to-r from-transparent via-[var(--color-rust)] to-transparent" />

      <div className="max-w-6xl mx-auto px-6">
        <div className="flex flex-col md:flex-row justify-between items-center gap-8">
          {/* Brand */}
          <div className="text-center md:text-left">
            <h3 className="font-serif text-3xl font-bold text-white mb-2">
              Kestrel
            </h3>
            <p className="text-gray-400 font-serif italic text-sm max-w-xs">
              A programming language for humans.
            </p>
          </div>

          {/* GitHub CTA */}
          <a
            href="https://github.com/jkpdino/kestrel"
            className="inline-flex items-center gap-3 px-6 py-3 bg-white/10 text-white font-mono text-sm rounded-lg hover:bg-[var(--color-rust)]">
            <Github className="w-5 h-5" />
            Star on GitHub
          </a>
        </div>

        {/* Bottom bar */}
        <div className="mt-10 pt-6 border-t border-white/10 flex flex-col md:flex-row justify-between items-center gap-4">
          <p className="text-gray-500 font-mono text-xs">
            © {new Date().getFullYear()} Kestrel. MIT License.
          </p>
          <p className="text-gray-400 font-serif italic text-sm inline-flex items-center gap-1">
            Made with <Heart className="w-3 h-3 text-red-400 fill-red-400" /> by
            jkpdino
          </p>
        </div>
      </div>
    </footer>
  );
}
