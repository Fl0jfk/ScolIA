"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import { OFFICE_KIND_META, type OfficeKind } from "@/app/lib/office-types";
import { dash } from "@/app/lib/dashboard-brand";

type RecentRow = {
  kind?: OfficeKind;
  fileName: string;
  relPath: string;
  scope: string;
  shareId?: string | null;
  fileShareId?: string | null;
  sharedLabel?: string;
  touchedAt?: string;
  editUrl: string;
};

const ALL_KINDS: OfficeKind[] = ["writer", "calc", "impress"];

type Props = {
  /** Filtre optionnel (pages dédiées). Sans filtre = hub Bureautique. */
  kind?: OfficeKind;
};

export default function OfficeHubClient({ kind }: Props) {
  const suite = !kind;
  const router = useRouter();
  const [personal, setPersonal] = useState<RecentRow[]>([]);
  const [shared, setShared] = useState<RecentRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState<OfficeKind | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [searchResults, setSearchResults] = useState<RecentRow[] | null>(null);
  const [searching, setSearching] = useState(false);

  const kindParam = kind || "all";
  const title = suite ? "Bureautique" : OFFICE_KIND_META[kind].label;
  const description = suite
    ? "Créez et ouvrez vos documents, tableurs et présentations — enregistrés dans votre cloud."
    : "Fichiers ouverts dans le navigateur, enregistrés dans votre cloud personnel.";

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch(`/api/documents/office/recents?kind=${kindParam}`);
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
  }, [kindParam]);

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
          `/api/documents/office/recents?kind=${kindParam}&q=${encodeURIComponent(q)}`,
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
  }, [query, kindParam]);

  const onCreate = async (createKind: OfficeKind) => {
    setCreating(createKind);
    setError(null);
    try {
      const res = await fetch("/api/documents/office/new", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ kind: createKind }),
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
      setCreating(null);
    }
  };

  const emptyHint = useMemo(
    () => "Aucun document pour l’instant — créez-en un ci-dessus.",
    [],
  );

  const searchLabel = suite
    ? "Rechercher dans le cloud (texte, tableur, présentation)"
    : `Rechercher dans le cloud (${OFFICE_KIND_META[kind].label.toLowerCase()})`;

  return (
    <ModulePageShell tourModuleId="documents">
      <ModulePageHeader
        eyebrow="Services"
        title={title}
        description={description}
        actions={
          <div className="flex flex-wrap items-center gap-3">
            {!suite ? (
              <Link
                href="/documents/office"
                className={`text-sm font-semibold underline-offset-2 hover:underline ${dash.textMid}`}
              >
                Toute la bureautique
              </Link>
            ) : null}
            <Link
              href="/documents"
              className={`text-sm font-semibold underline-offset-2 hover:underline ${dash.textMid}`}
            >
              Cloud personnel
            </Link>
          </div>
        }
      />

      {error ? (
        <p className="mb-4 rounded-lg border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-800">
          {error}
        </p>
      ) : null}

      {suite ? (
        <div className="grid gap-3 sm:grid-cols-3">
          {ALL_KINDS.map((k) => {
            const meta = OFFICE_KIND_META[k];
            return (
              <button
                key={k}
                type="button"
                onClick={() => void onCreate(k)}
                disabled={creating !== null}
                className="flex flex-col items-start gap-2 rounded-2xl border border-slate-200 bg-white px-4 py-4 text-left shadow-sm transition hover:border-[var(--dash-primary)]/40 hover:shadow-md disabled:opacity-60"
              >
                <span className="text-2xl" aria-hidden>
                  {meta.emoji}
                </span>
                <span className={`text-sm font-semibold ${dash.ink}`}>{meta.label}</span>
                <span className="text-xs font-semibold text-[var(--dash-primary)]">
                  {creating === k ? "Création…" : meta.newLabel}
                </span>
              </button>
            );
          })}
        </div>
      ) : (
        <button
          type="button"
          onClick={() => void onCreate(kind)}
          disabled={creating !== null}
          className="w-full sm:w-auto rounded-xl bg-[var(--dash-primary)] px-5 py-3 text-sm font-semibold text-white shadow-sm hover:opacity-95 disabled:opacity-60"
        >
          {creating === kind ? "Création…" : OFFICE_KIND_META[kind].newLabel}
        </button>
      )}

      <div className="mt-6">
        <label className="block text-xs font-semibold uppercase tracking-wide text-slate-500">
          {searchLabel}
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
            showKind={suite}
          />
        ) : null}
      </div>

      {!searchResults ? (
        <>
          <FileList
            title="Mes derniers documents"
            rows={personal}
            empty={loading ? "Chargement…" : emptyHint}
            showKind={suite}
          />
          <FileList
            title="Mes documents partagés"
            rows={shared}
            empty={loading ? "Chargement…" : "Aucun document partagé pour l’instant."}
            showKind={suite}
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
  showKind,
}: {
  title: string;
  rows: RecentRow[];
  empty: string;
  showKind?: boolean;
}) {
  return (
    <section className="mt-8">
      <h2 className={`text-sm font-semibold ${dash.ink}`}>{title}</h2>
      {rows.length === 0 ? (
        <p className={`mt-2 text-sm ${dash.textMid}`}>{empty}</p>
      ) : (
        <ul className="mt-3 divide-y divide-slate-100 rounded-xl border border-slate-200 bg-white">
          {rows.map((row) => {
            const kindMeta = row.kind ? OFFICE_KIND_META[row.kind] : null;
            return (
              <li
                key={`${row.scope}-${row.shareId || ""}-${row.fileShareId || ""}-${row.relPath}`}
              >
                <Link
                  href={row.editUrl}
                  className="flex flex-col gap-0.5 px-4 py-3 hover:bg-slate-50 sm:flex-row sm:items-center sm:justify-between"
                >
                  <span className="flex min-w-0 items-center gap-2">
                    {showKind && kindMeta ? (
                      <span className="shrink-0 text-base" aria-hidden>
                        {kindMeta.emoji}
                      </span>
                    ) : null}
                    <span className={`truncate text-sm font-medium ${dash.ink}`}>
                      {row.fileName}
                    </span>
                  </span>
                  <span className={`text-xs ${dash.textMid}`}>
                    {showKind && kindMeta ? `${kindMeta.shortLabel} · ` : ""}
                    {row.sharedLabel || (row.scope === "personal" ? "Cloud perso" : "Partagé")}
                  </span>
                </Link>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
