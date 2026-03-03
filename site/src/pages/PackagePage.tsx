import { useEffect, useState } from "react";
import { Link, useParams } from "react-router";
import Markdown from "react-markdown";
import {
  ArrowLeft,
  ArrowDownToLine,
  ExternalLink,
  Github,
  Package,
  Copy,
  Check,
  Scale,
  User,
  Globe,
  FileText,
  Building2,
  Terminal,
} from "lucide-react";
import Nav from "../components/Nav";
import Footer from "../components/Footer";
import { formatCount } from "../components/PackageCard";

interface PackageData {
  name: string;
  org: string;
  description: string;
  license: string;
  repository: string;
  author: string;
  website: string;
  documentation: string;
  downloads: number;
  versions: string[];
}

interface VersionData {
  name: string;
  org: string;
  version: string;
  checksum: string;
  dependencies: Record<string, string>;
  downloads: number;
  readme: string;
}

interface DailyDownload {
  date: string;
  count: number;
}

interface DownloadStats {
  daily: DailyDownload[];
  total: number;
}

interface Dependent {
  org: string;
  name: string;
  description: string;
  version: string;
}

const REGISTRY_URL = "https://registry.kestrel-lang.com";

function DownloadChart({ data }: { data: DailyDownload[] }) {
  if (data.length === 0) return null;

  const maxCount = Math.max(...data.map((d) => d.count), 1);
  const W = 248;
  const H = 60;
  const gap = 2;
  const barWidth = Math.max(3, Math.floor(W / data.length) - gap);
  const barRadius = Math.min(2, barWidth / 3);

  return (
    <div className="p-4 rounded-lg bg-white/60 dark:bg-white/5 border border-[var(--color-slate)]/10">
      <p className="font-mono text-xs text-[var(--color-slate-light)] mb-2">
        Downloads (30 days)
      </p>
      <svg
        viewBox={`0 0 ${W} ${H + 16}`}
        width={W}
        height={H + 16}
        className="w-full h-auto"
      >
        {data.map((d, i) => {
          const barHeight = Math.max(2, (d.count / maxCount) * H);
          const x = (i / data.length) * W;
          const w = Math.max(2, W / data.length - gap);
          return (
            <rect
              key={d.date}
              x={x}
              y={H - barHeight}
              width={w}
              height={barHeight}
              rx={barRadius}
              className="fill-[var(--color-rust)]"
              opacity={0.8}
            >
              <title>
                {d.date}: {d.count} downloads
              </title>
            </rect>
          );
        })}
        <text
          x={0}
          y={H + 12}
          className="fill-[var(--color-slate-light)]"
          style={{ fontSize: 9, fontFamily: "var(--font-mono)" }}
        >
          {data[0].date}
        </text>
        <text
          x={W}
          y={H + 12}
          textAnchor="end"
          className="fill-[var(--color-slate-light)]"
          style={{ fontSize: 9, fontFamily: "var(--font-mono)" }}
        >
          {data[data.length - 1].date}
        </text>
      </svg>
    </div>
  );
}

type Tab = "readme" | "dependencies" | "dependents" | "versions";

export default function PackagePage() {
  const { org, pkg } = useParams();
  const [packageData, setPackageData] = useState<PackageData | null>(null);
  const [selectedVersion, setSelectedVersion] = useState<VersionData | null>(
    null
  );
  const [downloadStats, setDownloadStats] = useState<DownloadStats | null>(
    null
  );
  const [dependents, setDependents] = useState<Dependent[]>([]);
  const [depDescriptions, setDepDescriptions] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [copiedToml, setCopiedToml] = useState(false);
  const [copiedCmd, setCopiedCmd] = useState(false);
  const [activeTab, setActiveTab] = useState<Tab>("readme");

  useEffect(() => {
    const fetchData = async () => {
      try {
        const res = await fetch(
          `${REGISTRY_URL}/api/v1/packages/${org}/${pkg}`
        );
        if (!res.ok) throw new Error("Package not found");
        const data: PackageData = await res.json();
        setPackageData(data);

        // Fetch version data and download stats in parallel
        const promises: Promise<void>[] = [];

        if (data.versions.length > 0) {
          promises.push(
            fetch(
              `${REGISTRY_URL}/api/v1/packages/${org}/${pkg}/${data.versions[0]}`
            )
              .then((r) => r.json())
              .then((vData: VersionData) => setSelectedVersion(vData))
          );
        }

        promises.push(
          fetch(
            `${REGISTRY_URL}/api/v1/packages/${org}/${pkg}/downloads?days=30`
          )
            .then((r) => r.json())
            .then((stats: DownloadStats) => setDownloadStats(stats))
            .catch(() => {})
        );

        promises.push(
          fetch(
            `${REGISTRY_URL}/api/v1/packages/${org}/${pkg}/dependents`
          )
            .then((r) => r.json())
            .then((data: { dependents: Dependent[] }) =>
              setDependents(data.dependents)
            )
            .catch(() => {})
        );

        await Promise.all(promises);
      } catch (e) {
        setError((e as Error).message);
      } finally {
        setLoading(false);
      }
    };

    fetchData();
  }, [org, pkg]);

  // Fetch descriptions for dependencies
  useEffect(() => {
    if (!selectedVersion) return;
    const deps = Object.keys(selectedVersion.dependencies);
    if (deps.length === 0) return;

    Promise.all(
      deps.map((dep) => {
        const [depOrg, depPkg] = dep.split("/");
        return fetch(`${REGISTRY_URL}/api/v1/packages/${depOrg}/${depPkg}`)
          .then((r) => (r.ok ? r.json() : null))
          .then((data) => [dep, data?.description || ""] as const)
          .catch(() => [dep, ""] as const);
      })
    ).then((results) => {
      const descs: Record<string, string> = {};
      for (const [name, desc] of results) {
        if (desc) descs[name] = desc;
      }
      setDepDescriptions(descs);
    });
  }, [selectedVersion]);

  const handleCopyToml = () => {
    navigator.clipboard.writeText(
      `${org}/${pkg} = "${packageData?.versions[0] || "0.1.0"}"`
    );
    setCopiedToml(true);
    setTimeout(() => setCopiedToml(false), 2000);
  };

  const handleCopyCmd = () => {
    navigator.clipboard.writeText(`flock add ${org}/${pkg}`);
    setCopiedCmd(true);
    setTimeout(() => setCopiedCmd(false), 2000);
  };

  return (
    <div className="min-h-screen bg-[var(--bg-secondary)] flex flex-col">
      <Nav />

      <main className="flex-1 max-w-6xl w-full mx-auto px-6 py-12">
        {/* Back link */}
        <Link
          to="/flock"
          className="inline-flex items-center gap-2 text-[var(--color-slate-light)] hover:text-[var(--color-rust)] font-mono text-sm mb-8 transition-colors"
        >
          <ArrowLeft className="w-4 h-4" />
          Back to packages
        </Link>

        {loading ? (
          <div className="text-center py-16">
            <p className="font-mono text-sm text-[var(--color-slate-light)]">
              Loading...
            </p>
          </div>
        ) : error ? (
          <div className="text-center py-16">
            <Package className="w-12 h-12 mx-auto mb-4 text-[var(--color-slate-light)] opacity-50" />
            <p className="font-serif text-lg text-[var(--color-slate-light)]">
              {error}
            </p>
          </div>
        ) : packageData ? (
          <>
            {/* Header */}
            <div className="mb-8">
              <h1 className="font-mono text-3xl text-[var(--color-slate)] mb-2">
                <span className="text-[var(--color-slate-light)]">
                  {packageData.org}/
                </span>
                {packageData.name}
              </h1>
              {packageData.description && (
                <p className="text-[var(--color-slate-light)] font-serif text-lg">
                  {packageData.description}
                </p>
              )}
            </div>

            {/* Tab bar — above grid so sidebar aligns with content */}
            <div className="flex gap-1 mb-4">
              {(["readme", "dependencies", "dependents", "versions"] as Tab[]).map(
                (tab) => (
                  <button
                    key={tab}
                    onClick={() => setActiveTab(tab)}
                    className={`px-4 py-2 rounded-lg font-mono text-sm transition-colors border ${
                      activeTab === tab
                        ? "bg-white/60 dark:bg-white/10 text-[var(--color-slate)] border-[var(--color-slate)]/10"
                        : "text-[var(--color-slate-light)] hover:text-[var(--color-slate)] border-transparent"
                    }`}
                  >
                    {tab === "readme"
                      ? "README"
                      : tab === "dependencies"
                        ? `Dependencies${selectedVersion ? ` (${Object.keys(selectedVersion.dependencies).length})` : ""}`
                        : tab === "dependents"
                          ? `Used By${dependents.length > 0 ? ` (${dependents.length})` : ""}`
                          : `Versions (${packageData.versions.length})`}
                  </button>
                )
              )}
            </div>

            {/* Two-column layout */}
            <div className="grid grid-cols-1 lg:grid-cols-[1fr_280px] gap-4">
              {/* Left column — Tab content */}
              <div>
                {activeTab === "readme" ? (
                  <div className="p-6 rounded-lg bg-white/60 dark:bg-white/5 border border-[var(--color-slate)]/10 min-h-[200px]">
                    {selectedVersion?.readme ? (
                      <div className="readme-content text-[var(--color-slate)]">
                        <Markdown
                          components={{
                            h1: ({ children }) => (
                              <h1 className="text-2xl font-bold mb-4 mt-6 first:mt-0 text-[var(--color-slate)]">{children}</h1>
                            ),
                            h2: ({ children }) => (
                              <h2 className="text-xl font-semibold mb-3 mt-5 text-[var(--color-slate)]">{children}</h2>
                            ),
                            h3: ({ children }) => (
                              <h3 className="text-lg font-semibold mb-2 mt-4 text-[var(--color-slate)]">{children}</h3>
                            ),
                            p: ({ children }) => (
                              <p className="text-base leading-relaxed mb-3">{children}</p>
                            ),
                            code: ({ className, children }) => {
                              const isBlock = className?.includes("language-");
                              return isBlock ? (
                                <code className="block font-mono text-sm bg-black/10 dark:bg-white/10 rounded-lg p-4 mb-3 overflow-x-auto whitespace-pre">
                                  {children}
                                </code>
                              ) : (
                                <code className="font-mono text-sm bg-black/10 dark:bg-white/10 rounded px-1.5 py-0.5">
                                  {children}
                                </code>
                              );
                            },
                            pre: ({ children }) => (
                              <pre className="mb-3">{children}</pre>
                            ),
                            ul: ({ children }) => (
                              <ul className="list-disc list-inside mb-3 space-y-1">{children}</ul>
                            ),
                            ol: ({ children }) => (
                              <ol className="list-decimal list-inside mb-3 space-y-1">{children}</ol>
                            ),
                            li: ({ children }) => (
                              <li className="text-base leading-relaxed">{children}</li>
                            ),
                            a: ({ href, children }) => (
                              <a href={href} className="text-[var(--color-rust)] hover:underline" target="_blank" rel="noopener noreferrer">{children}</a>
                            ),
                            blockquote: ({ children }) => (
                              <blockquote className="border-l-3 border-[var(--color-slate)]/20 pl-4 italic mb-3">{children}</blockquote>
                            ),
                            hr: () => (
                              <hr className="border-[var(--color-slate)]/10 my-6" />
                            ),
                          }}
                        >
                          {selectedVersion.readme}
                        </Markdown>
                      </div>
                    ) : (
                      <p className="text-[var(--color-slate-light)] italic">
                        No README available
                      </p>
                    )}
                  </div>
                ) : activeTab === "dependencies" ? (
                  <div className="py-2 px-2 rounded-lg bg-white/60 dark:bg-white/5 border border-[var(--color-slate)]/10 min-h-[200px]">
                    {selectedVersion &&
                    Object.keys(selectedVersion.dependencies).length > 0 ? (
                      Object.entries(selectedVersion.dependencies).map(
                        ([dep, ver]) => (
                          <Link
                            key={dep}
                            to={`/flock/${dep}`}
                            className="flex items-center justify-between py-3 px-4 border-b border-[var(--color-slate)]/5 last:border-0 hover:bg-black/5 dark:hover:bg-white/5 rounded transition-colors"
                          >
                            <div>
                              <span className="font-mono text-sm text-[var(--color-rust)]">
                                {dep}
                              </span>
                              {depDescriptions[dep] && (
                                <p className="text-xs text-[var(--color-slate-light)] mt-0.5">
                                  {depDescriptions[dep]}
                                </p>
                              )}
                            </div>
                            <span className="font-mono text-xs text-[var(--color-slate-light)]">
                              v{ver}
                            </span>
                          </Link>
                        )
                      )
                    ) : (
                      <p className="text-[var(--color-slate-light)] font-serif italic">
                        No dependencies
                      </p>
                    )}
                  </div>
                ) : activeTab === "dependents" ? (
                  <div className="py-2 px-2 rounded-lg bg-white/60 dark:bg-white/5 border border-[var(--color-slate)]/10 min-h-[200px]">
                    {dependents.length > 0 ? (
                      dependents.map((dep) => (
                        <Link
                          key={`${dep.org}/${dep.name}`}
                          to={`/flock/${dep.org}/${dep.name}`}
                          className="flex items-center justify-between py-3 px-4 border-b border-[var(--color-slate)]/5 last:border-0 hover:bg-black/5 dark:hover:bg-white/5 rounded transition-colors"
                        >
                          <div>
                            <span className="font-mono text-sm text-[var(--color-rust)]">
                              {dep.org}/{dep.name}
                            </span>
                            {dep.description && (
                              <p className="text-xs text-[var(--color-slate-light)] mt-0.5">
                                {dep.description}
                              </p>
                            )}
                          </div>
                          <span className="font-mono text-xs text-[var(--color-slate-light)]">
                            v{dep.version}
                          </span>
                        </Link>
                      ))
                    ) : (
                      <p className="text-[var(--color-slate-light)] font-serif italic">
                        No packages depend on this one yet
                      </p>
                    )}
                  </div>
                ) : (
                  <div className="py-2 px-2 rounded-lg bg-white/60 dark:bg-white/5 border border-[var(--color-slate)]/10">
                    {packageData.versions.map((ver) => (
                      <div
                        key={ver}
                        className="flex items-center justify-between py-3 px-4 border-b border-[var(--color-slate)]/5 last:border-0"
                      >
                        <span className="font-mono text-sm text-[var(--color-slate)]">
                          v{ver}
                        </span>
                        {ver === packageData.versions[0] && (
                          <span className="font-mono text-xs px-2 py-0.5 rounded bg-[var(--color-forest)]/10 text-[var(--color-forest)]">
                            latest
                          </span>
                        )}
                      </div>
                    ))}
                  </div>
                )}
              </div>

              {/* Right column — Metadata + Install */}
              <div className="flex flex-col gap-4">
                {/* Metadata */}
                <div className="p-4 rounded-lg bg-white/60 dark:bg-white/5 border border-[var(--color-slate)]/10">
                  <div className="flex flex-col gap-3">
                    {/* Organization — always shown */}
                    <div className="flex items-center gap-3">
                      <Building2 className="w-4 h-4 text-[var(--color-slate-light)] shrink-0" />
                      <div>
                        <p className="font-mono text-xs text-[var(--color-slate-light)]">
                          Organization
                        </p>
                        <Link
                          to={`/flock/${packageData.org}`}
                          className="font-mono text-sm text-[var(--color-rust)] hover:underline"
                        >
                          {packageData.org}
                        </Link>
                      </div>
                    </div>
                    {/* Downloads */}
                    <div className="flex items-center gap-3">
                      <ArrowDownToLine className="w-4 h-4 text-[var(--color-slate-light)] shrink-0" />
                      <div>
                        <p className="font-mono text-xs text-[var(--color-slate-light)]">
                          Downloads
                        </p>
                        <p className="font-mono text-sm text-[var(--color-slate)]">
                          {formatCount(packageData.downloads)}
                        </p>
                      </div>
                    </div>
                    {packageData.author && (
                      <div className="flex items-center gap-3">
                        <User className="w-4 h-4 text-[var(--color-slate-light)] shrink-0" />
                        <div>
                          <p className="font-mono text-xs text-[var(--color-slate-light)]">
                            Author
                          </p>
                          <p className="font-mono text-sm text-[var(--color-slate)]">
                            {packageData.author}
                          </p>
                        </div>
                      </div>
                    )}
                    {packageData.license && (
                      <div className="flex items-center gap-3">
                        <Scale className="w-4 h-4 text-[var(--color-slate-light)] shrink-0" />
                        <div>
                          <p className="font-mono text-xs text-[var(--color-slate-light)]">
                            License
                          </p>
                          <p className="font-mono text-sm text-[var(--color-slate)]">
                            {packageData.license}
                          </p>
                        </div>
                      </div>
                    )}
                    {packageData.repository && (
                      <div className="flex items-center gap-3">
                        <Github className="w-4 h-4 text-[var(--color-slate-light)] shrink-0" />
                        <div>
                          <p className="font-mono text-xs text-[var(--color-slate-light)]">
                            Repository
                          </p>
                          <a
                            href={packageData.repository}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="font-mono text-sm text-[var(--color-rust)] hover:underline inline-flex items-center gap-1"
                          >
                            Source
                            <ExternalLink className="w-3 h-3" />
                          </a>
                        </div>
                      </div>
                    )}
                    {packageData.website && (
                      <div className="flex items-center gap-3">
                        <Globe className="w-4 h-4 text-[var(--color-slate-light)] shrink-0" />
                        <div>
                          <p className="font-mono text-xs text-[var(--color-slate-light)]">
                            Website
                          </p>
                          <a
                            href={packageData.website}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="font-mono text-sm text-[var(--color-rust)] hover:underline inline-flex items-center gap-1"
                          >
                            Visit
                            <ExternalLink className="w-3 h-3" />
                          </a>
                        </div>
                      </div>
                    )}
                    {packageData.documentation && (
                      <div className="flex items-center gap-3">
                        <FileText className="w-4 h-4 text-[var(--color-slate-light)] shrink-0" />
                        <div>
                          <p className="font-mono text-xs text-[var(--color-slate-light)]">
                            Documentation
                          </p>
                          <a
                            href={packageData.documentation}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="font-mono text-sm text-[var(--color-rust)] hover:underline inline-flex items-center gap-1"
                          >
                            Docs
                            <ExternalLink className="w-3 h-3" />
                          </a>
                        </div>
                      </div>
                    )}
                  </div>
                </div>

                {/* Install card */}
                <div className="p-4 rounded-lg bg-white/60 dark:bg-white/5 border border-[var(--color-slate)]/10">
                  <div className="flex items-center justify-between mb-1">
                    <p className="font-mono text-xs text-[var(--color-slate-light)]">
                      flock.toml
                    </p>
                    <button
                      onClick={handleCopyToml}
                      className="p-1.5 rounded text-[var(--color-slate-light)] hover:text-[var(--color-rust)] hover:bg-black/5 dark:hover:bg-white/10 transition-colors"
                      title="Copy dependency string"
                    >
                      {copiedToml ? (
                        <Check className="w-3 h-3 text-[var(--color-forest)]" />
                      ) : (
                        <Copy className="w-3 h-3" />
                      )}
                    </button>
                  </div>
                  <code className="font-mono text-sm text-[var(--color-slate)] block overflow-x-auto whitespace-nowrap">
                    {org}/{pkg} = "{packageData.versions[0] || "0.1.0"}"
                  </code>

                  <div className="border-t border-[var(--color-slate)]/10 mt-3 pt-3">
                    <div className="flex items-center justify-between mb-1">
                      <p className="font-mono text-xs text-[var(--color-slate-light)]">
                        CLI
                      </p>
                      <button
                        onClick={handleCopyCmd}
                        className="p-1.5 rounded text-[var(--color-slate-light)] hover:text-[var(--color-rust)] hover:bg-black/5 dark:hover:bg-white/10 transition-colors"
                        title="Copy command"
                      >
                        {copiedCmd ? (
                          <Check className="w-3 h-3 text-[var(--color-forest)]" />
                        ) : (
                          <Copy className="w-3 h-3" />
                        )}
                      </button>
                    </div>
                    <code className="font-mono text-sm text-[var(--color-slate)] inline-flex items-center gap-1.5 overflow-x-auto whitespace-nowrap">
                      <Terminal className="w-3.5 h-3.5 text-[var(--color-slate-light)] shrink-0" />
                      flock add {org}/{pkg}
                    </code>
                  </div>
                </div>

                {/* Download chart */}
                {downloadStats && downloadStats.daily.length > 0 && (
                  <DownloadChart data={downloadStats.daily} />
                )}
              </div>
            </div>
          </>
        ) : null}
      </main>

      <Footer />
    </div>
  );
}
