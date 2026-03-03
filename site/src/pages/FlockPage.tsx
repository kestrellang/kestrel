import { useEffect, useState } from "react";
import { Package, Search } from "lucide-react";
import Nav from "../components/Nav";
import Footer from "../components/Footer";
import PackageCard from "../components/PackageCard";

interface PackageInfo {
  org: string;
  name: string;
  description: string;
  latest_version: string;
  published_at: string;
  downloads: number;
}

const REGISTRY_URL = "https://registry.kestrel-lang.com";

export default function FlockPage() {
  const [query, setQuery] = useState("");
  const [packages, setPackages] = useState<PackageInfo[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    setLoading(true);
    const timer = setTimeout(() => {
      const url = query
        ? `${REGISTRY_URL}/api/v1/packages?q=${encodeURIComponent(query)}`
        : `${REGISTRY_URL}/api/v1/packages`;

      fetch(url)
        .then((res) => res.json())
        .then((data: { packages: PackageInfo[] }) =>
          setPackages(data.packages)
        )
        .catch(() => setPackages([]))
        .finally(() => setLoading(false));
    }, 300);

    return () => clearTimeout(timer);
  }, [query]);

  return (
    <div className="min-h-screen bg-[var(--bg-secondary)] flex flex-col">
      <Nav />

      {/* Main content */}
      <main className="flex-1 max-w-6xl w-full mx-auto px-6 py-12">
        <h1
          className="font-serif text-5xl font-bold text-[var(--color-slate)] mb-2"
          style={{ fontFamily: "var(--font-serif)" }}
        >
          Flock Packages
        </h1>
        <p className="text-[var(--color-slate-light)] font-serif text-lg mb-8">
          Browse the Kestrel package registry
        </p>

        {/* Search */}
        <div className="relative mb-8">
          <Search className="absolute left-4 top-1/2 -translate-y-1/2 w-5 h-5 text-[var(--color-slate-light)]" />
          <input
            type="text"
            placeholder="Search packages..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            className="w-full pl-12 pr-4 py-3 rounded-lg bg-white/60 dark:bg-white/5 border border-[var(--color-slate)]/10 focus:border-[var(--color-rust)]/50 focus:outline-none font-mono text-sm text-[var(--color-slate)] placeholder-[var(--color-slate-light)]"
          />
        </div>

        {/* Package list */}
        {loading ? (
          <div className="text-center py-16">
            <p className="font-mono text-sm text-[var(--color-slate-light)]">
              Loading packages...
            </p>
          </div>
        ) : packages.length === 0 ? (
          <div className="text-center py-16">
            <Package className="w-12 h-12 mx-auto mb-4 text-[var(--color-slate-light)] opacity-50" />
            <p className="font-serif text-lg text-[var(--color-slate-light)]">
              {query ? "No packages match your search" : "No packages published yet"}
            </p>
            <p className="font-mono text-sm text-[var(--color-slate-light)] mt-2">
              {query
                ? "Try a different search term"
                : "Be the first to publish with flock publish"}
            </p>
          </div>
        ) : (
          <div className="flex flex-col gap-4">
            {packages.map((pkg) => (
              <PackageCard
                key={`${pkg.org}/${pkg.name}`}
                org={pkg.org}
                name={pkg.name}
                description={pkg.description}
                latestVersion={pkg.latest_version}
                publishedAt={pkg.published_at}
                downloads={pkg.downloads}
              />
            ))}
          </div>
        )}
      </main>

      <Footer />
    </div>
  );
}
