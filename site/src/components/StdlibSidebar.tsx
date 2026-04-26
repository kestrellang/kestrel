import { useEffect, useState } from "react";
import { Link, useLocation } from "react-router";
import { ChevronRight } from "lucide-react";

interface ModuleSummary {
  path: string;
  name: string;
  item_count: number;
}

interface ModuleIndex {
  modules: ModuleSummary[];
}

interface Item {
  kind: string;
  name: string;
  anchor: string;
}

interface ModulePage {
  path: string;
  items: Item[];
}

interface TreeNode {
  segment: string;
  fullPath: string;
  isModule: boolean;
  itemCount: number;
  children: TreeNode[];
}

const ITEM_KIND_ORDER = [
  "protocol",
  "struct",
  "enum",
  "typealias",
  "function",
];

const ITEM_KIND_TITLE: Record<string, string> = {
  protocol: "Protocols",
  struct: "Structs",
  enum: "Enums",
  typealias: "Type Aliases",
  function: "Functions",
};

function itemRank(kind: string): number {
  const i = ITEM_KIND_ORDER.indexOf(kind);
  return i === -1 ? ITEM_KIND_ORDER.length : i;
}

/// Build a nested tree from the flat list of dotted module paths so the
/// sidebar can render `std > io > file` as nested rows.
function buildTree(modules: ModuleSummary[]): TreeNode {
  const root: TreeNode = {
    segment: "",
    fullPath: "",
    isModule: false,
    itemCount: 0,
    children: [],
  };
  const byPath = new Map<string, TreeNode>();
  byPath.set("", root);

  for (const m of [...modules].sort((a, b) => a.path.localeCompare(b.path))) {
    const segments = m.path.split(".");
    let acc = "";
    let parent = root;
    for (let i = 0; i < segments.length; i++) {
      const seg = segments[i];
      acc = acc ? `${acc}.${seg}` : seg;
      let node = byPath.get(acc);
      if (!node) {
        node = {
          segment: seg,
          fullPath: acc,
          isModule: false,
          itemCount: 0,
          children: [],
        };
        byPath.set(acc, node);
        parent.children.push(node);
      }
      if (i === segments.length - 1) {
        node.isModule = true;
        node.itemCount = m.item_count;
      }
      parent = node;
    }
  }
  return root;
}

/// Group items by `kind` and return the canonical render order.
function groupItems(items: Item[]): [string, Item[]][] {
  const buckets = new Map<string, Item[]>();
  for (const it of items) {
    if (!buckets.has(it.kind)) buckets.set(it.kind, []);
    buckets.get(it.kind)!.push(it);
  }
  for (const list of buckets.values())
    list.sort((a, b) => a.name.localeCompare(b.name));
  return [...buckets.entries()].sort(
    (a, b) => itemRank(a[0]) - itemRank(b[0])
  );
}

function ItemRow({
  modulePath,
  item,
  depth,
  active,
}: {
  modulePath: string;
  item: Item;
  depth: number;
  active: boolean;
}) {
  // `min-w-0` on the link lets it shrink below its content width so
  // `truncate` can actually clip; `shrink-0` on the spacer keeps every
  // row's text starting at the same x.
  return (
    <div
      className="flex items-center gap-1"
      style={{ paddingLeft: `${depth * 12}px` }}
    >
      <span className="w-4 shrink-0" />
      <Link
        to={`/reference/stdlib/${modulePath}/${item.name}`}
        className={`font-mono text-sm py-0.5 truncate min-w-0 flex-1 ${
          active
            ? "text-[var(--color-rust)] font-semibold"
            : "text-[var(--color-rust)] hover:underline"
        }`}
        title={item.name}
      >
        {item.name}
      </Link>
    </div>
  );
}

function KindHeading({ kind, depth }: { kind: string; depth: number }) {
  return (
    <div
      className="flex items-center gap-1 mt-2"
      style={{ paddingLeft: `${depth * 12}px` }}
    >
      <span className="w-4" />
      <span className="font-mono text-[10px] uppercase tracking-wide text-[var(--color-slate-light)] py-0.5">
        {ITEM_KIND_TITLE[kind] || kind}
      </span>
    </div>
  );
}

function TreeRow({
  node,
  depth,
  activePath,
  activeItem,
  activeItems,
}: {
  node: TreeNode;
  depth: number;
  activePath: string;
  activeItem: string;
  activeItems: Item[] | null;
}) {
  // Auto-expand when this subtree contains the active path so the user
  // never has to manually click open the chain to see where they are.
  const containsActive =
    activePath === node.fullPath ||
    activePath.startsWith(node.fullPath + ".");
  const [open, setOpen] = useState(containsActive || depth === 0);

  useEffect(() => {
    if (containsActive) setOpen(true);
  }, [containsActive]);

  const isActive = activePath === node.fullPath;
  const hasItems = isActive && activeItems != null && activeItems.length > 0;
  const expandable = node.children.length > 0 || hasItems;
  const itemGroups = hasItems ? groupItems(activeItems!) : [];

  return (
    <div>
      <div
        className="flex items-center gap-1"
        style={{ paddingLeft: `${depth * 12}px` }}
      >
        {expandable ? (
          <button
            onClick={() => setOpen(!open)}
            className="p-0.5 shrink-0 text-[var(--color-slate-light)] hover:text-[var(--color-rust)]"
            aria-label={open ? "collapse" : "expand"}
          >
            <ChevronRight
              className={`w-3 h-3 transition-transform ${open ? "rotate-90" : ""}`}
            />
          </button>
        ) : (
          <span className="w-4 shrink-0" />
        )}
        {node.isModule ? (
          <Link
            to={`/reference/stdlib/${node.fullPath}`}
            className={`font-mono text-sm py-0.5 truncate ${
              isActive
                ? "text-[var(--color-rust)] font-semibold"
                : "text-[var(--color-slate)] hover:text-[var(--color-rust)]"
            }`}
            title={`${node.fullPath} · ${node.itemCount} items`}
          >
            {node.segment}
          </Link>
        ) : (
          <span className="font-mono text-sm text-[var(--color-slate-light)] py-0.5">
            {node.segment}
          </span>
        )}
      </div>
      {expandable && open && (
        <div>
          {node.children
            .sort((a, b) => a.segment.localeCompare(b.segment))
            .map((child) => (
              <TreeRow
                key={child.fullPath}
                node={child}
                depth={depth + 1}
                activePath={activePath}
                activeItem={activeItem}
                activeItems={activeItems}
              />
            ))}
          {itemGroups.map(([kind, list]) => (
            <div key={kind}>
              <KindHeading kind={kind} depth={depth + 1} />
              {list.map((it) => (
                <ItemRow
                  key={it.anchor}
                  modulePath={node.fullPath}
                  item={it}
                  depth={depth + 1}
                  active={activeItem === it.name}
                />
              ))}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export default function StdlibSidebar() {
  const [index, setIndex] = useState<ModuleIndex | null>(null);
  const [activeItems, setActiveItems] = useState<Item[] | null>(null);
  const location = useLocation();

  useEffect(() => {
    fetch("/stdlib/index.json")
      .then((r) => r.json())
      .then((data: ModuleIndex) => setIndex(data))
      .catch(() => {});
  }, []);

  // Active path = the module currently in the URL. URLs are:
  //   /reference/stdlib                       → ""
  //   /reference/stdlib/<module>              → <module>
  //   /reference/stdlib/<module>/<item>       → <module>, item = <item>
  const { activePath, activeItem } = (() => {
    const m = location.pathname.match(
      /^\/reference\/stdlib\/([^/]+)(?:\/([^/]+))?/
    );
    return {
      activePath: m ? m[1] : "",
      activeItem: m && m[2] ? decodeURIComponent(m[2]) : "",
    };
  })();

  // Pull the active module's items so they can be rendered inline under
  // the module row — gives the sidebar a docs.rs-style "where am I + what
  // else is here" view.
  useEffect(() => {
    if (!activePath) {
      setActiveItems(null);
      return;
    }
    let cancelled = false;
    fetch(`/stdlib/${activePath}.json`)
      .then((r) => (r.ok ? r.json() : null))
      .then((data: ModulePage | null) => {
        if (!cancelled) setActiveItems(data?.items ?? null);
      })
      .catch(() => {
        if (!cancelled) setActiveItems(null);
      });
    return () => {
      cancelled = true;
    };
  }, [activePath]);

  if (!index) {
    return (
      <p className="font-mono text-sm text-[var(--color-slate-light)]">
        Loading…
      </p>
    );
  }

  const tree = buildTree(index.modules);

  return (
    <nav className="flex flex-col">
      {tree.children
        .sort((a, b) => a.segment.localeCompare(b.segment))
        .map((child) => (
          <TreeRow
            key={child.fullPath}
            node={child}
            depth={0}
            activePath={activePath}
            activeItem={activeItem}
            activeItems={activeItems}
          />
        ))}
    </nav>
  );
}
