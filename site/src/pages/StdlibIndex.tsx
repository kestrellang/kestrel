import { useEffect, useState } from "react";
import { Link } from "react-router";
import { Package } from "lucide-react";
import StdlibLayout from "../components/StdlibLayout";

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

  // Top-level modules only (segments without a dot) — they're the entry
  // points; everything else is reachable via the sidebar tree.
  const topLevel = index?.modules.filter((m) => !m.path.includes(".")) ?? [];
  const totalItems =
    index?.modules.reduce((sum, m) => sum + m.item_count, 0) ?? 0;

  return (
    <StdlibLayout>
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
        <>
          <div className="mb-6">
            <p className="font-mono text-xs text-[var(--color-slate-light)] uppercase tracking-wide">
              Reference
            </p>
            <h1 className="font-serif text-4xl font-bold text-[var(--color-slate)] mb-2">
              Kestrel Standard Library
            </h1>
            <p className="text-[var(--color-slate-light)] font-mono text-sm">
              {index.modules.length} modules · {totalItems} items
            </p>
          </div>

          <section className="mb-8">
            <h2 className="font-mono text-lg text-[var(--color-slate)] mb-2 pb-1 border-b border-[var(--color-slate)]/15">
              Packages
            </h2>
            <table className="w-full font-mono text-base">
              <tbody>
                {topLevel.map((m) => (
                  <tr
                    key={m.path}
                    className="border-b border-[var(--color-slate)]/5 last:border-0"
                  >
                    <td className="py-2 pr-4 whitespace-nowrap align-top">
                      <Link
                        to={`/reference/stdlib/${m.path}`}
                        className="text-[var(--color-rust)] hover:underline"
                      >
                        {m.path}
                      </Link>
                    </td>
                    <td className="py-2 text-[var(--color-slate-light)] font-sans">
                      The Kestrel standard library.
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </section>
        </>
      )}
    </StdlibLayout>
  );
}
