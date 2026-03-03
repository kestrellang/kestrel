import { useEffect, useState } from "react";
import { Link, useParams } from "react-router";
import { ArrowLeft, Building2, Package } from "lucide-react";
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

interface OrgData {
  name: string;
  packages: PackageInfo[];
}

const REGISTRY_URL = "https://registry.kestrel-lang.com";

export default function OrgPage() {
  const { org } = useParams();
  const [orgData, setOrgData] = useState<OrgData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetch(`${REGISTRY_URL}/api/v1/orgs/${org}`)
      .then((res) => {
        if (!res.ok) throw new Error("Organization not found");
        return res.json();
      })
      .then((data: OrgData) => setOrgData(data))
      .catch((e) => setError((e as Error).message))
      .finally(() => setLoading(false));
  }, [org]);

  const totalDownloads = orgData?.packages.reduce((sum, p) => sum + p.downloads, 0) ?? 0;

  return (
    <div className="min-h-screen bg-[var(--bg-secondary)] flex flex-col">
      <Nav />

      <main className="flex-1 max-w-6xl w-full mx-auto px-6 py-12">
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
            <Building2 className="w-12 h-12 mx-auto mb-4 text-[var(--color-slate-light)] opacity-50" />
            <p className="font-serif text-lg text-[var(--color-slate-light)]">
              {error}
            </p>
          </div>
        ) : orgData ? (
          <>
            <div className="mb-8">
              <div className="flex items-center gap-3 mb-2">
                <Building2 className="w-8 h-8 text-[var(--color-slate)]" />
                <h1 className="font-mono text-3xl text-[var(--color-slate)]">
                  {orgData.name}
                </h1>
              </div>
              <p className="text-[var(--color-slate-light)] font-mono text-sm">
                {orgData.packages.length} package{orgData.packages.length !== 1 ? "s" : ""}
                {totalDownloads > 0 && (
                  <span className="ml-3">
                    {totalDownloads} total download{totalDownloads !== 1 ? "s" : ""}
                  </span>
                )}
              </p>
            </div>

            {orgData.packages.length === 0 ? (
              <div className="text-center py-16">
                <Package className="w-12 h-12 mx-auto mb-4 text-[var(--color-slate-light)] opacity-50" />
                <p className="font-serif text-lg text-[var(--color-slate-light)]">
                  No packages published yet
                </p>
              </div>
            ) : (
              <div className="flex flex-col gap-4">
                {orgData.packages.map((pkg) => (
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
          </>
        ) : null}
      </main>

      <Footer />
    </div>
  );
}
