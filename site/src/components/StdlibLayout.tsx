import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import { Link } from "react-router";
import { PanelLeftClose, PanelLeftOpen } from "lucide-react";
import StdlibSidebar from "./StdlibSidebar";
import ThemeToggle from "./ThemeToggle";

const STORAGE_KEY = "kestrel:stdlib:sidebarOpen";

export default function StdlibLayout({
  children,
  sidebar,
  sidebarLabel = "Stdlib",
}: {
  children: ReactNode;
  /// Optional drawer content. When omitted, the drawer shows the module
  /// tree — the default for the index and module pages. Item pages pass
  /// their own rustdoc-style TOC.
  sidebar?: ReactNode;
  /// Heading shown at the top of the drawer (defaults to "Stdlib").
  sidebarLabel?: string;
}) {
  const [open, setOpen] = useState<boolean>(() => {
    if (typeof window === "undefined") return true;
    const stored = window.localStorage.getItem(STORAGE_KEY);
    return stored === null ? true : stored === "1";
  });

  useEffect(() => {
    if (typeof window !== "undefined") {
      window.localStorage.setItem(STORAGE_KEY, open ? "1" : "0");
    }
  }, [open]);

  return (
    <div className="min-h-screen bg-[var(--bg-secondary)] flex flex-col">
      {/* Floating theme toggle — pinned to the top-right corner so the
          docs surface stays uncluttered. */}
      <div className="fixed top-4 right-6 z-40">
        <ThemeToggle />
      </div>

      <div className="flex-1 flex">
        {/* Sidebar — sticky column on the left, slides off-screen when
            collapsed. Houses both the Kestrel home link and the
            section label. */}
        <aside
          className={`hidden lg:flex sticky top-0 self-start h-screen w-60 shrink-0 transition-all duration-200 ease-out ${
            open ? "ml-0" : "-ml-60"
          }`}
        >
          <div className="flex w-full flex-col bg-white/70 dark:bg-white/[0.04]">
            <div className="flex items-center justify-between px-4 py-4">
              <Link
                to="/"
                className="font-serif text-2xl font-bold text-[var(--color-slate)]"
              >
                Kestrel
              </Link>
              <button
                onClick={() => setOpen(false)}
                className="p-1 rounded text-[var(--color-slate-light)] hover:text-[var(--color-rust)] hover:bg-black/5 dark:hover:bg-white/10 transition-colors"
                aria-label="Collapse sidebar"
                title="Collapse sidebar"
              >
                <PanelLeftClose className="w-4 h-4" />
              </button>
            </div>
            <div className="px-4 pb-2">
              <span className="font-mono text-base font-semibold text-[var(--color-slate)] truncate block">
                {sidebarLabel}
              </span>
            </div>
            <div className="sidebar-scroll flex-1 overflow-y-auto px-4 pb-4">
              {sidebar ?? <StdlibSidebar />}
            </div>
          </div>
        </aside>

        {/* Floating "open" affordance — only when drawer is closed. */}
        {!open && (
          <button
            onClick={() => setOpen(true)}
            className="hidden lg:flex fixed left-2 top-20 z-30 items-center justify-center w-9 h-9 rounded-lg bg-white/70 dark:bg-white/[0.04] text-[var(--color-slate-light)] hover:text-[var(--color-rust)] transition-colors"
            aria-label="Open sidebar"
            title="Open sidebar"
          >
            <PanelLeftOpen className="w-4 h-4" />
          </button>
        )}

        <div className="flex-1 px-6 pt-12 pb-8 min-w-0">
          <main className="max-w-4xl mx-auto min-w-0">{children}</main>
        </div>
      </div>
    </div>
  );
}
