import { useEffect, useState } from "react";
import { Link, useParams } from "react-router";
import { ChevronRight, Package } from "lucide-react";
import StdlibLayout from "../components/StdlibLayout";
import MarkdownBody from "../components/MarkdownBody";
import { HighlightedCode } from "../components/Highlight";

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
];

const KIND_TITLE: Record<string, string> = {
  protocol: "Protocols",
  struct: "Structs",
  enum: "Enums",
  typealias: "Type Aliases",
  function: "Functions",
};

function kindRank(kind: string): number {
  const i = KIND_ORDER.indexOf(kind);
  return i === -1 ? KIND_ORDER.length : i;
}

/// Module-page item row. Mirrors the rustdoc-style collapsible
/// `<details>` row used on item pages: chevron + signature in the
/// summary, full doc body when expanded. Clicking the name jumps to
/// the item's detail page.
function ItemRow({
  modulePath,
  item,
}: {
  modulePath: string;
  item: Item;
}) {
  return (
    <details
      open
      id={item.anchor}
      className="member-row py-3 border-b border-[var(--color-slate)]/10 last:border-0 scroll-mt-24"
    >
      <summary className="cursor-pointer list-none flex items-start gap-2">
        <ChevronRight className="member-chevron w-4 h-4 mt-1 shrink-0 text-[var(--color-slate-light)] transition-transform" />
        <Link
          to={`/reference/stdlib/${modulePath}/${item.name}`}
          className="font-mono text-base whitespace-pre-wrap break-words min-w-0 flex-1 hover:underline"
          onClick={(e) => e.stopPropagation()}
        >
          <HighlightedCode code={item.signature} />
        </Link>
      </summary>
      {item.doc && (
        <div className="mt-2 ml-6">
          <MarkdownBody source={item.doc} compact />
        </div>
      )}
    </details>
  );
}

export default function StdlibModule() {
  const params = useParams();
  const modulePath = params.modulePath || "";
  const [page, setPage] = useState<ModulePage | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setPage(null);
    setError(null);
    fetch(`/stdlib/${modulePath}.json`)
      .then((r) => {
        if (!r.ok) throw new Error(`module not found: ${modulePath}`);
        return r.json();
      })
      .then((data: ModulePage) => setPage(data))
      .catch((e) => setError((e as Error).message));
  }, [modulePath]);

  const groupedItems = (() => {
    if (!page) return [] as [string, Item[]][];
    const buckets = new Map<string, Item[]>();
    for (const it of page.items) {
      if (!buckets.has(it.kind)) buckets.set(it.kind, []);
      buckets.get(it.kind)!.push(it);
    }
    for (const list of buckets.values())
      list.sort((a, b) => a.name.localeCompare(b.name));
    return [...buckets.entries()].sort(
      (a, b) => kindRank(a[0]) - kindRank(b[0])
    );
  })();

  return (
    <StdlibLayout>
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
          <div className="mb-6">
            <p className="font-mono text-xs text-[var(--color-slate-light)] uppercase tracking-wide">
              Module
            </p>
            <h1 className="font-mono text-3xl text-[var(--color-slate)]">
              {page.path}
            </h1>
          </div>

          {page.submodules.length > 0 && (
            <section className="mb-10">
              <h2 className="font-mono text-xl text-[var(--color-slate)] mb-4 pb-1 border-b border-[var(--color-slate)]/15">
                Modules
              </h2>
              <ul className="flex flex-col">
                {page.submodules.map((sub) => (
                  <li
                    key={sub}
                    className="py-2 border-b border-[var(--color-slate)]/10 last:border-0"
                  >
                    <Link
                      to={`/reference/stdlib/${sub}`}
                      className="font-mono text-base text-[var(--color-rust)] hover:underline"
                    >
                      {sub.split(".").pop()}
                    </Link>
                    <span className="ml-3 font-mono text-sm text-[var(--color-slate-light)]">
                      {sub}
                    </span>
                  </li>
                ))}
              </ul>
            </section>
          )}

          {groupedItems.map(([kind, items]) => (
            <section key={kind} className="mb-10">
              <h2 className="font-mono text-xl text-[var(--color-slate)] mb-4 pb-1 border-b border-[var(--color-slate)]/15">
                {KIND_TITLE[kind] || kind}
              </h2>
              <div>
                {items.map((it) => (
                  <ItemRow
                    key={`${kind}-${it.name}-${it.anchor}`}
                    modulePath={page.path}
                    item={it}
                  />
                ))}
              </div>
            </section>
          ))}

          {groupedItems.length === 0 && page.submodules.length === 0 && (
            <p className="font-serif italic text-[var(--color-slate-light)]">
              This module has no public items.
            </p>
          )}
        </>
      )}
    </StdlibLayout>
  );
}
