import { useEffect, useState } from "react";
import { Link } from "react-router";
import { BookOpen, Package } from "lucide-react";
import Nav from "../components/Nav";
import Footer from "../components/Footer";

interface ModuleSummary {
  path: string;
  name: string;
  item_count: number;
}

interface ModuleIndex {
  modules: ModuleSummary[];
}

export default function StdlibIndex() {
  const [index, setIndex] = useState<ModuleIndex | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetch("/stdlib/index.json")
      .then((r) => {
        if (!r.ok) throw new Error(`failed to load: ${r.status}`);
        return r.json();
      })
      .then((data: ModuleIndex) => setIndex(data))
      .catch((e) => setError((e as Error).message));
  }, []);

  // Group modules by their top-level segment so the index reads as a tree.
  const grouped = (() => {
    if (!index) return [];
    const buckets = new Map<string, ModuleSummary[]>();
    for (const m of index.modules) {
      const top = m.path.split(".")[0];
      if (!buckets.has(top)) buckets.set(top, []);
      buckets.get(top)!.push(m);
    }
    return [...buckets.entries()].sort((a, b) => a[0].localeCompare(b[0]));
  })();

  return (
    <div className="min-h-screen bg-[var(--bg-secondary)] flex flex-col">
      <Nav />
      <main className="flex-1 max-w-6xl w-full mx-auto px-6 py-12">
        <div className="flex items-center gap-3 mb-2">
          <BookOpen className="w-8 h-8 text-[var(--color-rust)]" />
          <h1 className="font-serif text-5xl font-bold text-[var(--color-slate)]">
            Stdlib Reference
          </h1>
        </div>
        <p className="text-[var(--color-slate-light)] font-serif text-lg mb-8">
          API documentation for the Kestrel standard library
        </p>

        {error ? (
          <div className="text-center py-16">
            <Package className="w-12 h-12 mx-auto mb-4 text-[var(--color-slate-light)] opacity-50" />
            <p className="font-serif text-lg text-[var(--color-slate-light)]">
              {error}
            </p>
          </div>
        ) : !index ? (
          <p className="font-mono text-sm text-[var(--color-slate-light)]">
            Loading…
          </p>
        ) : (
          <div className="flex flex-col gap-8">
            {grouped.map(([top, modules]) => (
              <section key={top}>
                <h2 className="font-mono text-2xl text-[var(--color-slate)] mb-4">
                  {top}
                </h2>
                <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
                  {modules.map((m) => (
                    <Link
                      key={m.path}
                      to={`/reference/stdlib/${m.path}`}
                      className="p-4 rounded-lg bg-white/60 dark:bg-white/5 border border-[var(--color-slate)]/10 hover:border-[var(--color-rust)]/40 transition-colors"
                    >
                      <div className="font-mono text-sm text-[var(--color-rust)] mb-1">
                        {m.path}
                      </div>
                      <div className="font-mono text-xs text-[var(--color-slate-light)]">
                        {m.item_count} item{m.item_count === 1 ? "" : "s"}
                      </div>
                    </Link>
                  ))}
                </div>
              </section>
            ))}
          </div>
        )}
      </main>
      <Footer />
    </div>
  );
}
