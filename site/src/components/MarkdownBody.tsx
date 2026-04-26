/// Shared markdown renderer for stdlib doc bodies. Used on item pages
/// (full doc) and module pages (per-item collapsible summaries).
///
/// `compact` scales heading sizes down one tier so doc-body `# Examples`
/// sections don't visually compete with a page's own item header.

import Markdown from "react-markdown";
import { HighlightedCode } from "./Highlight";

export default function MarkdownBody({
  source,
  compact = false,
}: {
  source: string;
  compact?: boolean;
}) {
  const h1Class = compact
    ? "font-serif font-semibold text-xl text-[var(--color-slate)] mt-6 mb-2 pb-1 border-b border-[var(--color-slate)]/15 first:mt-0"
    : "font-serif font-semibold text-2xl text-[var(--color-slate)] mt-8 mb-3 pb-1 border-b border-[var(--color-slate)]/15 first:mt-0";
  const h2Class = compact
    ? "font-serif font-semibold text-lg text-[var(--color-slate)] mt-5 mb-2 first:mt-0"
    : "font-serif font-semibold text-2xl text-[var(--color-slate)] mt-7 mb-3 first:mt-0";
  const h3Class = compact
    ? "font-serif font-semibold text-base text-[var(--color-slate)] mt-4 mb-2 first:mt-0"
    : "font-serif font-semibold text-xl text-[var(--color-slate)] mt-6 mb-2 first:mt-0";
  const h4Class = compact
    ? "font-serif font-semibold text-base text-[var(--color-slate)] mt-3 mb-1 first:mt-0"
    : "font-serif font-semibold text-lg text-[var(--color-slate)] mt-5 mb-2 first:mt-0";

  return (
    <div className="text-lg font-light leading-relaxed text-[var(--color-slate)]">
      <Markdown
        components={{
          h1: ({ children }) => <h1 className={h1Class}>{children}</h1>,
          h2: ({ children }) => <h2 className={h2Class}>{children}</h2>,
          h3: ({ children }) => <h3 className={h3Class}>{children}</h3>,
          h4: ({ children }) => <h4 className={h4Class}>{children}</h4>,
          p: ({ children }) => (
            <p className="leading-relaxed mb-3">{children}</p>
          ),
          // Markdown block code arrives as <pre><code class="language-?">...</code></pre>.
          // Skip the default `pre` wrapper (HighlightedCode adds its own
          // styling) and detect "block" by either a `language-` class or
          // a newline in the content — stdlib doc fences usually omit a
          // language tag, so falling back to "has newline" matters.
          pre: ({ children }) => <>{children}</>,
          code: ({ className, children }) => {
            const text = String(children).replace(/\n$/, "");
            const isBlock =
              (className?.includes("language-") ?? false) ||
              text.includes("\n");
            if (isBlock) {
              return (
                <HighlightedCode
                  code={text}
                  className="block font-mono text-sm bg-white/70 dark:bg-white/[0.04] rounded p-3 mb-2 overflow-x-auto whitespace-pre"
                />
              );
            }
            return (
              <code className="font-mono text-sm bg-white/70 dark:bg-white/[0.04] rounded px-1 py-0.5">
                {children}
              </code>
            );
          },
          ul: ({ children }) => (
            <ul className="list-disc list-inside mb-3 space-y-1 text-lg font-light">
              {children}
            </ul>
          ),
          li: ({ children }) => (
            <li className="text-lg font-light">{children}</li>
          ),
          a: ({ href, children }) => (
            <a href={href} className="text-[var(--color-rust)] hover:underline">
              {children}
            </a>
          ),
        }}
      >
        {source}
      </Markdown>
    </div>
  );
}
