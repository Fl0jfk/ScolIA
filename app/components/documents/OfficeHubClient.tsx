"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import { OFFICE_KIND_META, type OfficeKind } from "@/app/lib/office-types";
import { dash } from "@/app/lib/dashboard-brand";

type RecentRow = {
  fileName: string;
  relPath: string;
  scope: string;
  shareId?: string | null;
  fileShareId?: string | null;
  sharedLabel?: string;
  touchedAt?: string;
  editUrl: string;
};

export default function OfficeHubClient({ kind }: { kind: OfficeKind }) {
  const meta = OFFICE_KIND_META[kind];
  const router = useRouter();
  const [personal, setPersonal] = useState<RecentRow[]>([]);
  const [shared, setShared] = useState<RecentRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [searchResults, setSearchResults] = useState<RecentRow[] | null>(null);
  const [searching, setSearching] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch(`/api/documents/office/recents?kind=${kind}`);
      const data = await res.json();
      if (!res.ok) {
        setError(data.error || "Chargement impossible.");
        return;
      }
      setPersonal(data.personal || []);
      setShared(data.shared || []);
    } catch {
      setError("Erreur réseau.");
    } finally {
      setLoading(false);
    }
  }, [kind]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    const q = query.trim();
    if (!q) {
      setSearchResults(null);
      return;
    }
    const t = setTimeout(async () => {
      setSearching(true);
      try {
        const res = await fetch(
          `/api/documents/office/recents?kind=${kind}&q=${encodeURIComponent(q)}`,
        );
        const data = await res.json();
        if (res.ok) setSearchResults(data.results || []);
      } catch {
        /* ignore */
      } finally {
        setSearching(false);
      }
    }, 280);
    return () => clearTimeout(t);
  }, [query, kind]);

  const onCreate = async () => {
    setCreating(true);
    setError(null);
    try {
      const res = await fetch("/api/documents/office/new", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ kind }),
      });
      const data = await res.json();
      if (!res.ok) {
        setError(data.error || "Création impossible.");
        return;
      }
      router.push(data.editUrl);
    } catch {
      setError("Erreur réseau.");
    } finally {
      setCreating(false);
    }
  };

  const emptyHint = useMemo(
    () => "Aucun document pour l’instant — créez-en un ci-dessus.",
    [],
  );

  return (
    <ModulePageShell tourModuleId="documents">
      <ModulePageHeader
        eyebrow="Services"
        title={meta.label}
        description="Fichiers ouverts dans le navigateur, enregistrés dans votre cloud personnel."
        actions={
          <Link
            href="/documents"
            className={`text-sm font-semibold underline-offset-2 hover:underline ${dash.textMid}`}
          >
            Cloud personnel
          </Link>
        }
      />

      {error ? (
        <p className="mb-4 rounded-lg border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-800">
          {error}
        </p>
      ) : null}

      <button
        type="button"
        onClick={() => void onCreate()}
        disabled={creating}
        className="w-full sm:w-auto rounded-xl bg-[var(--dash-primary)] px-5 py-3 text-sm font-semibold text-white shadow-sm hover:opacity-95 disabled:opacity-60"
      >
        {creating ? "Création…" : meta.newLabel}
      </button>

      <div className="mt-6">
        <label className="block text-xs font-semibold uppercase tracking-wide text-slate-500">
          Rechercher dans le cloud ({meta.label.toLowerCase()})
        </label>
        <input
          type="search"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Nom du fichier…"
          className="mt-1 w-full max-w-lg rounded-lg border border-slate-200 bg-white px-3 py-2 text-sm"
        />
        {searching ? <p className={`mt-2 text-xs ${dash.textMid}`}>Recherche…</p> : null}
        {searchResults ? (
          <FileList
            title="Résultats"
            rows={searchResults}
            empty="Aucun fichier trouvé."
          />
        ) : null}
      </div>

      {!searchResults ? (
        <>
          <FileList
            title="Mes derniers documents"
            rows={personal}
            empty={loading ? "Chargement…" : emptyHint}
          />
          <FileList
            title="Mes documents partagés"
            rows={shared}
            empty={loading ? "Chargement…" : "Aucun document partagé pour l’instant."}
          />
        </>
      ) : null}
    </ModulePageShell>
  );
}

function FileList({
  title,
  rows,
  empty,
}: {
  title: string;
  rows: RecentRow[];
  empty: string;
}) {
  return (
    <section className="mt-8">
      <h2 className={`text-sm font-semibold ${dash.ink}`}>{title}</h2>
      {rows.length === 0 ? (
        <p className={`mt-2 text-sm ${dash.textMid}`}>{empty}</p>
      ) : (
        <ul className="mt-3 divide-y divide-slate-100 rounded-xl border border-slate-200 bg-white">
          {rows.map((row) => (
            <li key={`${row.scope}-${row.shareId || ""}-${row.fileShareId || ""}-${row.relPath}`}>
              <Link
                href={row.editUrl}
                className="flex flex-col gap-0.5 px-4 py-3 hover:bg-slate-50 sm:flex-row sm:items-center sm:justify-between"
              >
                <span className={`text-sm font-medium ${dash.ink}`}>{row.fileName}</span>
                <span className={`text-xs ${dash.textMid}`}>
                  {row.sharedLabel || (row.scope === "personal" ? "Cloud perso" : "Partagé")}
                </span>
              </Link>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
