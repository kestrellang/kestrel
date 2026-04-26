import { useEffect, useState } from "react";
import { Link, useParams } from "react-router";
import Markdown from "react-markdown";
import { ArrowLeft, Package } from "lucide-react";
import Nav from "../components/Nav";
import Footer from "../components/Footer";

interface Item {
  kind: string;
  name: string;
  anchor: string;
  signature: string;
  doc: string;
  source_path?: string;
  members?: Item[];
}

interface ModulePage {
  path: string;
  name: string;
  submodules: string[];
  items: Item[];
}

const KIND_ORDER = [
  "protocol",
  "struct",
  "enum",
  "typealias",
  "function",
  "extension",
];

function kindRank(kind: string): number {
  const i = KIND_ORDER.indexOf(kind);
  return i === -1 ? KIND_ORDER.length : i;
}

function ItemBlock({ item }: { item: Item }) {
  return (
    <article
      id={item.anchor}
      className="p-5 rounded-lg bg-white/60 dark:bg-white/5 border border-[var(--color-slate)]/10 scroll-mt-24"
    >
      <div className="flex items-baseline gap-3 mb-3">
        <span className="font-mono text-xs uppercase tracking-wide px-2 py-0.5 rounded bg-[var(--color-rust)]/10 text-[var(--color-rust)]">
          {item.kind}
        </span>
        <h3 className="font-mono text-xl text-[var(--color-slate)]">
          {item.name}
        </h3>
      </div>
      {item.signature && (
        <pre className="font-mono text-sm bg-black/10 dark:bg-white/10 rounded-lg p-4 mb-3 overflow-x-auto whitespace-pre-wrap">
          <code>{item.signature}</code>
        </pre>
      )}
      {item.doc && (
        <div className="prose prose-sm max-w-none text-[var(--color-slate)]">
          <Markdown
            components={{
              p: ({ children }) => (
                <p className="text-sm leading-relaxed mb-2">{children}</p>
              ),
              code: ({ className, children }) => {
                const isBlock = className?.includes("language-");
                return isBlock ? (
                  <code className="block font-mono text-xs bg-black/10 dark:bg-white/10 rounded p-3 mb-2 overflow-x-auto whitespace-pre">
                    {children}
                  </code>
                ) : (
                  <code className="font-mono text-xs bg-black/10 dark:bg-white/10 rounded px-1 py-0.5">
                    {children}
                  </code>
                );
              },
              ul: ({ children }) => (
                <ul className="list-disc list-inside mb-2 space-y-0.5 text-sm">
                  {children}
                </ul>
              ),
              li: ({ children }) => <li className="text-sm">{children}</li>,
              a: ({ href, children }) => (
                <a
                  href={href}
                  className="text-[var(--color-rust)] hover:underline"
                >
                  {children}
                </a>
              ),
            }}
          >
            {item.doc}
          </Markdown>
        </div>
      )}
      {item.members && item.members.length > 0 && (
        <details className="mt-4">
          <summary className="cursor-pointer font-mono text-xs text-[var(--color-slate-light)] hover:text-[var(--color-rust)]">
            {item.members.length} member
            {item.members.length === 1 ? "" : "s"}
          </summary>
          <div className="mt-3 flex flex-col gap-3 pl-4 border-l-2 border-[var(--color-slate)]/10">
            {item.members.map((m) => (
              <ItemBlock key={`${m.kind}-${m.name}`} item={m} />
            ))}
          </div>
        </details>
      )}
    </article>
  );
}

export default function StdlibModule() {
  const params = useParams();
  const path = params["*"] || params.modulePath || "";
  const [page, setPage] = useState<ModulePage | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setPage(null);
    setError(null);
    fetch(`/stdlib/${path}.json`)
      .then((r) => {
        if (!r.ok) throw new Error(`module not found: ${path}`);
        return r.json();
      })
      .then((data: ModulePage) => setPage(data))
      .catch((e) => setError((e as Error).message));
  }, [path]);

  // Group items by kind for a sidebar TOC.
  const groupedItems = (() => {
    if (!page) return [] as [string, Item[]][];
    const buckets = new Map<string, Item[]>();
    for (const it of page.items) {
      if (!buckets.has(it.kind)) buckets.set(it.kind, []);
      buckets.get(it.kind)!.push(it);
    }
    return [...buckets.entries()].sort(
      (a, b) => kindRank(a[0]) - kindRank(b[0])
    );
  })();

  return (
    <div className="min-h-screen bg-[var(--bg-secondary)] flex flex-col">
      <Nav />
      <main className="flex-1 max-w-6xl w-full mx-auto px-6 py-12">
        <Link
          to="/reference/stdlib"
          className="inline-flex items-center gap-2 text-[var(--color-slate-light)] hover:text-[var(--color-rust)] font-mono text-sm mb-6 transition-colors"
        >
          <ArrowLeft className="w-4 h-4" />
          Back to stdlib
        </Link>

        {error ? (
          <div className="text-center py-16">
            <Package className="w-12 h-12 mx-auto mb-4 text-[var(--color-slate-light)] opacity-50" />
            <p className="font-serif text-lg text-[var(--color-slate-light)]">
              {error}
            </p>
          </div>
        ) : !page ? (
          <p className="font-mono text-sm text-[var(--color-slate-light)]">
            Loading…
          </p>
        ) : (
          <>
            <h1 className="font-mono text-3xl text-[var(--color-slate)] mb-2">
              {page.path}
            </h1>
            <p className="text-[var(--color-slate-light)] font-mono text-sm mb-8">
              {page.items.length} item{page.items.length === 1 ? "" : "s"}
              {page.submodules.length > 0 &&
                ` · ${page.submodules.length} submodule${
                  page.submodules.length === 1 ? "" : "s"
                }`}
            </p>

            <div className="grid grid-cols-1 lg:grid-cols-[1fr_220px] gap-6">
              <div className="flex flex-col gap-6">
                {page.submodules.length > 0 && (
                  <section>
                    <h2 className="font-mono text-lg text-[var(--color-slate)] mb-3">
                      Submodules
                    </h2>
                    <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
                      {page.submodules.map((sub) => (
                        <Link
                          key={sub}
                          to={`/reference/stdlib/${sub}`}
                          className="p-3 rounded-lg bg-white/60 dark:bg-white/5 border border-[var(--color-slate)]/10 hover:border-[var(--color-rust)]/40 transition-colors font-mono text-sm text-[var(--color-rust)]"
                        >
                          {sub}
                        </Link>
                      ))}
                    </div>
                  </section>
                )}

                {groupedItems.map(([kind, items]) => (
                  <section key={kind}>
                    <h2 className="font-mono text-lg text-[var(--color-slate)] mb-3 capitalize">
                      {kind}s
                    </h2>
                    <div className="flex flex-col gap-4">
                      {items.map((it) => (
                        <ItemBlock key={it.anchor} item={it} />
                      ))}
                    </div>
                  </section>
                ))}
              </div>

              <aside className="hidden lg:block">
                <div className="sticky top-6 p-4 rounded-lg bg-white/60 dark:bg-white/5 border border-[var(--color-slate)]/10">
                  <p className="font-mono text-xs text-[var(--color-slate-light)] mb-2">
                    On this page
                  </p>
                  <nav className="flex flex-col gap-1">
                    {groupedItems.flatMap(([_, items]) =>
                      items.map((it) => (
                        <a
                          key={it.anchor}
                          href={`#${it.anchor}`}
                          className="font-mono text-xs text-[var(--color-slate)] hover:text-[var(--color-rust)] truncate"
                        >
                          {it.name}
                        </a>
                      ))
                    )}
                  </nav>
                </div>
              </aside>
            </div>
          </>
        )}
      </main>
      <Footer />
    </div>
  );
}
