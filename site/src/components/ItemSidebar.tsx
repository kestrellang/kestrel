/// Item-page sidebar: docs.rs-style nav showing the current item's
/// member groups (Direct + each conformed protocol), each group
/// subdivided by category (Methods / Cases / Fields / etc.), with
/// anchor links to scroll the page to each member.

interface Item {
  kind: string;
  name: string;
  anchor: string;
  member_groups?: MemberGroup[];
}

interface MemberGroup {
  kind: string;
  label?: string | null;
  members: Item[];
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

function rank(kind: string): number {
  const i = CATEGORY_ORDER.indexOf(kind);
  return i === -1 ? CATEGORY_ORDER.length : i;
}

export default function ItemSidebar({ item }: { item: Item }) {
  const groups = item.member_groups ?? [];

  return (
    <div className="flex flex-col gap-5">
      {groups.map((group) => {
        const isProtocol = group.kind === "protocol";
        const groupHref = `#protocol-${group.label}`;
        const anchorPrefix = isProtocol ? `${group.label}-` : "";

        const buckets = new Map<string, Item[]>();
        for (const m of group.members) {
          if (!buckets.has(m.kind)) buckets.set(m.kind, []);
          buckets.get(m.kind)!.push(m);
        }
        for (const list of buckets.values())
          list.sort((a, b) => a.name.localeCompare(b.name));
        const categories = [...buckets.entries()].sort(
          (a, b) => rank(a[0]) - rank(b[0])
        );

        return (
          <div key={`${group.kind}-${group.label ?? "direct"}`}>
            {isProtocol && (
              <a
                href={groupHref}
                className="block mb-2 font-mono text-sm font-semibold text-[var(--color-slate)] hover:underline truncate"
                title={group.label ?? ""}
              >
                {group.label}
              </a>
            )}

            {categories.map(([kind, members]) => (
              <section key={kind} className="mb-3">
                <h4 className="font-mono text-xs uppercase tracking-wide text-[var(--color-slate-light)] mb-1">
                  {CATEGORY_TITLE[kind] || kind}
                </h4>
                <ul className="flex flex-col">
                  {members.map((m) => (
                    <li key={m.anchor}>
                      <a
                        href={`#${anchorPrefix}${m.anchor}`}
                        className="block font-mono text-sm py-0.5 text-[var(--color-rust)] hover:underline truncate"
                        title={m.name}
                      >
                        {m.name}
                      </a>
                    </li>
                  ))}
                </ul>
              </section>
            ))}
          </div>
        );
      })}

      {groups.length === 0 && (
        <p className="font-mono text-sm text-[var(--color-slate-light)] italic">
          No members.
        </p>
      )}
    </div>
  );
}
