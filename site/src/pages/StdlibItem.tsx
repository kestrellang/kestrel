import { useEffect, useState } from "react";
import { Link, useParams } from "react-router";
import { ChevronRight, Package } from "lucide-react";
import StdlibLayout from "../components/StdlibLayout";
import ItemSidebar from "../components/ItemSidebar";
import MarkdownBody from "../components/MarkdownBody";
import { HighlightedCode } from "../components/Highlight";

interface Item {
  kind: string;
  name: string;
  anchor: string;
  signature: string;
  doc: string;
  source_path?: string;
  member_groups?: MemberGroup[];
}

interface MemberGroup {
  kind: "direct" | "protocol" | string;
  label?: string | null;
  source_path?: string | null;
  members: Item[];
}

interface ModulePage {
  path: string;
  name: string;
  submodules: string[];
  items: Item[];
}

const CATEGORY_ORDER = [
  "case",
  "field",
  "typealias",
  "initializer",
  "function",
  "subscript",
];

const CATEGORY_TITLE: Record<string, string> = {
  case: "Cases",
  field: "Properties",
  typealias: "Associated Types",
  initializer: "Initializers",
  function: "Methods",
  subscript: "Subscripts",
};

function categoryRank(kind: string): number {
  const i = CATEGORY_ORDER.indexOf(kind);
  return i === -1 ? CATEGORY_ORDER.length : i;
}

function MemberRow({ item, anchorPrefix }: { item: Item; anchorPrefix: string }) {
  const id = `${anchorPrefix}${item.anchor}`;
  return (
    <details
      open
      id={id}
      className="member-row py-3 border-b border-[var(--color-slate)]/10 last:border-0 scroll-mt-24"
    >
      <summary className="cursor-pointer list-none flex items-start gap-2">
        <ChevronRight className="member-chevron w-4 h-4 mt-1 shrink-0 text-[var(--color-slate-light)] transition-transform" />
        <code className="font-mono text-base whitespace-pre-wrap break-words min-w-0 flex-1">
          <HighlightedCode code={item.signature} />
        </code>
      </summary>
      {item.doc && (
        <div className="mt-2 ml-6">
          <MarkdownBody source={item.doc} compact />
        </div>
      )}
    </details>
  );
}

/// Render a single MemberGroup: a heading (Direct, or the protocol name)
/// followed by its members subgrouped by category.
function MemberGroupBlock({ group }: { group: MemberGroup }) {
  // Bucket the group's members by category for the inner subheadings.
  const buckets = new Map<string, Item[]>();
  for (const m of group.members) {
    if (!buckets.has(m.kind)) buckets.set(m.kind, []);
    buckets.get(m.kind)!.push(m);
  }
  const categories = [...buckets.entries()].sort(
    (a, b) => categoryRank(a[0]) - categoryRank(b[0])
  );

  const isProtocol = group.kind === "protocol";
  const anchorPrefix = isProtocol ? `${group.label}-` : "";

  return (
    <section className="mb-10">
      {isProtocol && (
        <h2
          id={`protocol-${group.label}`}
          className="font-mono text-xl text-[var(--color-slate)] mb-4 pb-1 border-b border-[var(--color-slate)]/15 scroll-mt-24"
        >
          <span className="text-[var(--color-slate-light)] text-sm uppercase tracking-wide mr-2">
            Implements
          </span>
          {group.source_path ? (
            <Link
              to={`/reference/stdlib/${group.source_path
                .split(".")
                .slice(0, -1)
                .join(".")}/${group.label}`}
              className="text-[var(--color-rust)] hover:underline"
            >
              {group.label}
            </Link>
          ) : (
            <span>{group.label}</span>
          )}
        </h2>
      )}

      {categories.map(([kind, items]) => (
        <div key={kind} className="mb-6">
          <h3 className="font-mono text-sm font-semibold text-[var(--color-slate-light)] uppercase tracking-wide mb-2">
            {CATEGORY_TITLE[kind] || kind}
          </h3>
          <div>
            {items.map((m) => (
              <MemberRow
                key={`${m.kind}-${m.name}-${m.anchor}`}
                item={m}
                anchorPrefix={anchorPrefix}
              />
            ))}
          </div>
        </div>
      ))}
    </section>
  );
}

export default function StdlibItem() {
  const params = useParams();
  const modulePath = params.modulePath || "";
  const itemName = params.itemName || "";
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

  const item = page?.items.find((it) => it.name === itemName);

  return (
    <StdlibLayout
      sidebarLabel={item ? item.name : modulePath}
      sidebar={item ? <ItemSidebar item={item} /> : undefined}
    >
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
      ) : !item ? (
        <div className="text-center py-16">
          <Package className="w-12 h-12 mx-auto mb-4 text-[var(--color-slate-light)] opacity-50" />
          <p className="font-serif text-lg text-[var(--color-slate-light)]">
            {itemName} not found in {modulePath}
          </p>
        </div>
      ) : (
        <div>
          <nav className="flex items-center gap-1 mb-4 font-mono text-xs text-[var(--color-slate-light)]">
            <Link
              to="/reference/stdlib"
              className="hover:text-[var(--color-rust)]"
            >
              stdlib
            </Link>
            <ChevronRight className="w-3 h-3" />
            <Link
              to={`/reference/stdlib/${modulePath}`}
              className="hover:text-[var(--color-rust)]"
            >
              {modulePath}
            </Link>
          </nav>

          <h1 className="font-mono text-2xl text-[var(--color-slate)] mb-3">
            {item.name}
          </h1>

          {item.signature && (
            <pre className="font-mono text-base bg-white/70 dark:bg-white/[0.04] rounded-lg p-4 mb-4 overflow-x-auto whitespace-pre-wrap">
              <HighlightedCode code={item.signature} />
            </pre>
          )}

          {item.doc && (
            <div className="mb-8">
              <MarkdownBody
                source={item.doc}
                compact={
                  item.kind !== "struct" &&
                  item.kind !== "enum" &&
                  item.kind !== "protocol" &&
                  item.kind !== "extension"
                }
              />
            </div>
          )}

          {(item.member_groups ?? []).map((group) => (
            <MemberGroupBlock
              key={`${group.kind}-${group.label ?? "direct"}`}
              group={group}
            />
          ))}

          {item.source_path && (
            <p className="mt-8 font-mono text-xs text-[var(--color-slate-light)]">
              Defined in{" "}
              <span className="text-[var(--color-slate)]">
                {item.source_path}
              </span>
            </p>
          )}
        </div>
      )}
    </StdlibLayout>
  );
}
