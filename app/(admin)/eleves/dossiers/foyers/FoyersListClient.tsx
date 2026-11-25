"use client";

import Link from "next/link";
import { useCallback, useEffect, useState } from "react";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import ResponsableEditor from "./ResponsableEditor";

type Item = {
  id: string;
  label: string;
  responsables: Array<{
    id: string;
    nom: string;
    prenom: string;
    email: string | null;
    telephone: string | null;
    autoriteParentale: boolean;
    compteFamille: boolean;
  }>;
  enfants: Array<{ id: string; nom: string; prenom: string; classe: string | null }>;
};

const PAGE = 50;

export default function FoyersListClient() {
  const [q, setQ] = useState("");
  const [debounced, setDebounced] = useState("");
  const [offset, setOffset] = useState(0);
  const [items, setItems] = useState<Item[]>([]);
  const [total, setTotal] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [editId, setEditId] = useState<string | null>(null);

  useEffect(() => {
    const t = setTimeout(() => {
      setDebounced(q);
      setOffset(0);
    }, 300);
    return () => clearTimeout(t);
  }, [q]);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch(
        `/api/eleves/foyers?q=${encodeURIComponent(debounced)}&limit=${PAGE}&offset=${offset}`,
        { cache: "no-store" },
      );
      const j = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(j.error || "Chargement impossible.");
      setItems(j.items ?? []);
      setTotal(j.total ?? 0);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur.");
    } finally {
      setLoading(false);
    }
  }, [debounced, offset]);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <ModulePageShell maxWidthClass="max-w-5xl">
      <ModulePageHeader
        title="Foyers"
        description="Responsables légaux et enfants rattachés — lecture seule."
        actions={
          <Link href="/eleves/dossiers/foyers/import" className="text-sm font-bold text-indigo-600 hover:underline">
            Importer les foyers (Excel)
          </Link>
        }
      />
      <input
        value={q}
        onChange={(e) => setQ(e.target.value)}
        placeholder="Rechercher un responsable, un e-mail, un enfant…"
        className="mb-4 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
      />
      {error ? <p className="mb-3 text-sm text-rose-600">{error}</p> : null}
      <p className="mb-3 text-xs text-slate-500">
        {loading ? "Chargement…" : `${total} foyer(s)`}
      </p>
      <ul className="space-y-3">
        {items.map((f) => (
          <li key={f.id} className="rounded-2xl border border-slate-200 bg-white p-4 shadow-sm">
            <h3 className="font-bold text-slate-900">{f.label}</h3>
            <div className="mt-2 grid gap-3 sm:grid-cols-2">
              <div>
                <p className="text-xs font-semibold text-slate-500">Responsables</p>
                <ul className="mt-1 space-y-1 text-sm">
                  {f.responsables.map((r) => (
                    <li key={r.id}>
                      {editId === r.id ? (
                        <ResponsableEditor
                          resp={r}
                          onCancel={() => setEditId(null)}
                          onSaved={() => {
                            setEditId(null);
                            void load();
                          }}
                        />
                      ) : (
                        <>
                          <span className="font-semibold">
                            {r.prenom} {r.nom}
                          </span>
                          <span className="block text-xs text-slate-600">
                            {r.email || "—"} · {r.telephone || "—"}
                          </span>
                          <button
                            type="button"
                            className="text-xs font-bold text-indigo-600"
                            onClick={() => setEditId(r.id)}
                          >
                            Modifier
                          </button>
                        </>
                      )}
                    </li>
                  ))}
                </ul>
              </div>
              <div>
                <p className="text-xs font-semibold text-slate-500">Enfants ({f.enfants.length})</p>
                <ul className="mt-1 space-y-1 text-sm">
                  {f.enfants.map((e) => (
                    <li key={e.id}>
                      <Link
                        href={`/eleves/dossier/${e.id}`}
                        className="font-semibold text-indigo-600 hover:underline"
                      >
                        {e.prenom} {e.nom}
                      </Link>
                      <span className="text-xs text-slate-500"> · {e.classe || "—"}</span>
                    </li>
                  ))}
                </ul>
              </div>
            </div>
          </li>
        ))}
      </ul>
      <div className="mt-4 flex items-center gap-3 text-sm">
        <button
          type="button"
          disabled={offset === 0}
          onClick={() => setOffset(Math.max(0, offset - PAGE))}
          className="rounded-xl border border-slate-200 px-3 py-1.5 disabled:opacity-40"
        >
          Précédent
        </button>
        <button
          type="button"
          disabled={offset + PAGE >= total}
          onClick={() => setOffset(offset + PAGE)}
          className="rounded-xl border border-slate-200 px-3 py-1.5 disabled:opacity-40"
        >
          Suivant
        </button>
      </div>
    </ModulePageShell>
  );
}
