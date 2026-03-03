import { Link } from "react-router";
import { ArrowDownToLine } from "lucide-react";

interface PackageCardProps {
  org: string;
  name: string;
  description: string;
  latestVersion: string;
  publishedAt: string;
  downloads: number;
}

export function formatRelativeDate(dateStr: string): string {
  const date = new Date(dateStr + "Z");
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffSecs = Math.floor(diffMs / 1000);
  const diffMins = Math.floor(diffSecs / 60);
  const diffHours = Math.floor(diffMins / 60);
  const diffDays = Math.floor(diffHours / 24);

  if (diffDays > 30) {
    return date.toLocaleDateString(undefined, {
      month: "short",
      day: "numeric",
      year: "numeric",
    });
  }
  if (diffDays > 0) return `${diffDays}d ago`;
  if (diffHours > 0) return `${diffHours}h ago`;
  if (diffMins > 0) return `${diffMins}m ago`;
  return "just now";
}

export function formatCount(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return n.toString();
}

export default function PackageCard({
  org,
  name,
  description,
  latestVersion,
  publishedAt,
  downloads,
}: PackageCardProps) {
  return (
    <Link
      to={`/flock/${org}/${name}`}
      className="block p-6 rounded-lg bg-white/60 dark:bg-white/5 border border-[var(--color-slate)]/10 hover:border-[var(--color-rust)]/30 transition-colors"
    >
      <div className="flex items-baseline justify-between mb-2">
        <h3 className="font-mono text-lg text-[var(--color-slate)]">
          <span className="text-[var(--color-slate-light)]">{org}/</span>
          {name}
        </h3>
        <span className="font-mono text-sm text-[var(--color-forest)]">
          v{latestVersion}
        </span>
      </div>
      {description && (
        <p className="text-[var(--color-slate-light)] font-serif text-sm mb-3">
          {description}
        </p>
      )}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <p className="font-mono text-xs text-[var(--color-slate-light)]">
            {formatRelativeDate(publishedAt)}
          </p>
          <span className="inline-flex items-center gap-1 font-mono text-xs text-[var(--color-slate-light)]">
            <ArrowDownToLine className="w-3 h-3" />
            {formatCount(downloads)}
          </span>
        </div>
        <code className="font-mono text-xs text-[var(--color-rust)] bg-[var(--color-rust)]/10 px-2 py-1 rounded">
          flock add {org}/{name}
        </code>
      </div>
    </Link>
  );
}
