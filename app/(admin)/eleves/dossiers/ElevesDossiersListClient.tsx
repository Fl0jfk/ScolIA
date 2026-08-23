"use client";

import { useEffect, useMemo, useState } from "react";
import Link from "next/link";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";

type EleveRow = {
  id?: string;
  nom: string;
  prenom: string;
  classe?: string;
  ine?: string;
  folderName?: string;
};

type Preinsc = {
  id: string;
  nom: string;
  prenom: string;
  siteId: string | null;
  niveauVise: string | null;
  status: string;
  createdAt: string;
};

type AccessReq = {
  id: string;
  documentId: string;
  status: string;
  durationDays: number;
  note: string | null;
  createdAt: string;
  docTitle?: string;
  docTiroir?: string;
  eleveId?: string;
  eleveNom?: string;
  elevePrenom?: string;
};

export default function ElevesDossiersListClient() {
  const [eleves, setEleves] = useState<EleveRow[]>([]);
  const [preinsc, setPreinsc] = useState<Preinsc[]>([]);
  const [accessReqs, setAccessReqs] = useState<AccessReq[]>([]);
  const [canDecideAccess, setCanDecideAccess] = useState(false);
  const [q, setQ] = useState("");
  const [siteFilter, setSiteFilter] = useState("");
  const [tab, setTab] = useState<"dossiers" | "preinscriptions" | "acces">("dossiers");
  const [error, setError] = useState<string | null>(null);
  const [ids, setIds] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    void (async () => {
      try {
        const [er, pr, ar] = await Promise.all([
          fetch("/api/eleves"),
          fetch("/api/eleves/preinscriptions?status=pending"),
          fetch("/api/eleves/document-access-requests?status=pending"),
        ]);
        if (er.ok) {
          const j = (await er.json()) as { eleves: EleveRow[] };
          setEleves(j.eleves || []);
        }
        if (pr.ok) {
          const j = (await pr.json()) as { preinscriptions: Preinsc[] };
          setPreinsc(j.preinscriptions || []);
        }
        if (ar.ok) {
          const j = (await ar.json()) as { requests: AccessReq[]; canDecide: boolean };
          setAccessReqs(j.requests || []);
          setCanDecideAccess(Boolean(j.canDecide));
        }
        const dbRes = await fetch("/api/eleves/registry-ids");
        if (dbRes.ok) {
          const j = (await dbRes.json()) as { byFolder: Record<string, string> };
          setIds(j.byFolder || {});
        }
      } catch (e) {
        setError(e instanceof Error ? e.message : "Erreur chargement");
      }
    })();
  }, []);

  const filtered = useMemo(() => {
    const needle = q.trim().toLowerCase();
    return eleves.filter((e) => {
      if (!needle) return true;
      return `${e.nom} ${e.prenom} ${e.classe || ""}`.toLowerCase().includes(needle);
    });
  }, [eleves, q]);

  const preFiltered = useMemo(() => {
    if (!siteFilter) return preinsc;
    return preinsc.filter((p) => p.siteId === siteFilter);
  }, [preinsc, siteFilter]);

  const sites = useMemo(() => {
    const s = new Set<string>();
    for (const p of preinsc) if (p.siteId) s.add(p.siteId);
    return [...s].sort();
  }, [preinsc]);

  async function decide(id: string, action: "accept" | "reject") {
    const res = await fetch("/api/eleves/preinscriptions", {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ id, action }),
    });
    if (!res.ok) {
      const j = (await res.json().catch(() => ({}))) as { error?: string };
      setError(j.error || "Échec");
      return;
    }
    setPreinsc((prev) => prev.filter((p) => p.id !== id));
  }

  async function decideAccess(id: string, decision: "approved" | "rejected") {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/eleves/document-access-requests", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id, decision }),
      });
      const j = (await res.json().catch(() => ({}))) as { error?: string };
      if (!res.ok) throw new Error(j.error || "Échec");
      setAccessReqs((prev) => prev.filter((r) => r.id !== id));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  return (
    <ModulePageShell maxWidthClass="max-w-5xl">
      <ModulePageHeader
        title="Dossiers élèves"
        description="Hub anti-silos — une fiche, des vues par rôle."
        actions={
          <Link href="/preinscription" className="text-sm font-bold text-indigo-600 hover:underline">
            Formulaire public
          </Link>
        }
      />

      <div className="mb-4 flex flex-wrap gap-2">
        <button
          type="button"
          onClick={() => setTab("dossiers")}
          className={`rounded-xl px-4 py-2 text-sm font-bold border ${
            tab === "dossiers" ? "bg-slate-900 text-white" : "bg-white border-slate-200"
          }`}
        >
          Dossiers ({filtered.length})
        </button>
        <button
          type="button"
          onClick={() => setTab("preinscriptions")}
          className={`rounded-xl px-4 py-2 text-sm font-bold border ${
            tab === "preinscriptions" ? "bg-slate-900 text-white" : "bg-white border-slate-200"
          }`}
        >
          Préinscriptions ({preFiltered.length})
        </button>
        <button
          type="button"
          onClick={() => setTab("acces")}
          className={`rounded-xl px-4 py-2 text-sm font-bold border ${
            tab === "acces" ? "bg-slate-900 text-white" : "bg-white border-slate-200"
          }`}
        >
          Accès documents ({accessReqs.length})
        </button>
      </div>

      {error ? <p className="mb-3 text-sm text-red-600">{error}</p> : null}

      {tab === "dossiers" ? (
        <>
          <input
            value={q}
            onChange={(e) => setQ(e.target.value)}
            placeholder="Rechercher…"
            className="mb-4 w-full rounded-xl border border-slate-200 bg-white px-4 py-3 text-sm"
          />
          <ul className="divide-y divide-slate-100 rounded-3xl border border-slate-200 bg-white overflow-hidden">
            {filtered.slice(0, 100).map((e, i) => {
              const key = e.folderName || `${e.nom}_${e.prenom}`;
              const uuid = e.id || ids[key];
              return (
                <li key={`${key}-${i}`} className="flex items-center justify-between gap-3 px-4 py-3">
                  <div>
                    <p className="font-semibold text-slate-900">
                      {e.prenom} {e.nom}
                    </p>
                    <p className="text-xs text-slate-500">{e.classe || "—"}</p>
                  </div>
                  {uuid ? (
                    <Link
                      href={`/eleves/dossier/${uuid}`}
                      className="text-sm font-bold text-indigo-600 hover:underline"
                    >
                      Ouvrir
                    </Link>
                  ) : (
                    <span className="text-xs text-slate-400">UUID manquant</span>
                  )}
                </li>
              );
            })}
          </ul>
        </>
      ) : null}

      {tab === "preinscriptions" ? (
        <>
          <div className="mb-4 flex flex-wrap gap-2">
            <button
              type="button"
              onClick={() => setSiteFilter("")}
              className={`rounded-lg px-3 py-1.5 text-xs font-bold border ${
                !siteFilter ? "bg-slate-900 text-white" : "bg-white"
              }`}
            >
              Tous les sites
            </button>
            {sites.map((s) => (
              <button
                key={s}
                type="button"
                onClick={() => setSiteFilter(s)}
                className={`rounded-lg px-3 py-1.5 text-xs font-bold border ${
                  siteFilter === s ? "bg-slate-900 text-white" : "bg-white"
                }`}
              >
                {s}
              </button>
            ))}
          </div>
          <ul className="space-y-2">
            {preFiltered.map((p) => (
              <li
                key={p.id}
                className="flex flex-wrap items-center justify-between gap-3 rounded-2xl border border-slate-200 bg-white px-4 py-3"
              >
                <div>
                  <p className="font-semibold">
                    {p.prenom} {p.nom}
                  </p>
                  <p className="text-xs text-slate-500">
                    {p.siteId || "site ?"} · {p.niveauVise || "niveau ?"}
                  </p>
                </div>
                <div className="flex gap-2">
                  <button
                    type="button"
                    onClick={() => void decide(p.id, "accept")}
                    className="rounded-lg bg-emerald-600 px-3 py-1.5 text-xs font-bold text-white"
                  >
                    Accepter
                  </button>
                  <button
                    type="button"
                    onClick={() => void decide(p.id, "reject")}
                    className="rounded-lg bg-slate-200 px-3 py-1.5 text-xs font-bold text-slate-700"
                  >
                    Refuser
                  </button>
                </div>
              </li>
            ))}
            {preFiltered.length === 0 ? (
              <p className="text-sm text-slate-500">Aucune préinscription en attente.</p>
            ) : null}
          </ul>
        </>
      ) : null}

      {tab === "acces" ? (
        <ul className="space-y-2">
          {accessReqs.length === 0 ? (
            <p className="rounded-3xl border border-slate-200 bg-white p-6 text-sm text-slate-500">
              Aucune demande d’accès en attente.
            </p>
          ) : (
            accessReqs.map((r) => (
              <li
                key={r.id}
                className="flex flex-wrap items-center justify-between gap-3 rounded-2xl border border-slate-200 bg-white px-4 py-3"
              >
                <div>
                  <p className="font-semibold text-slate-900">
                    {r.docTitle || "Document"}
                    {r.elevePrenom ? (
                      <span className="font-normal text-slate-600">
                        {" "}
                        — {r.elevePrenom} {r.eleveNom}
                      </span>
                    ) : null}
                  </p>
                  <p className="text-xs text-slate-500">
                    {r.docTiroir || "tiroir"} · {r.durationDays} j
                    {r.note ? ` · ${r.note}` : ""}
                  </p>
                </div>
                <div className="flex gap-2 items-center">
                  {r.eleveId ? (
                    <Link
                      href={`/eleves/dossier/${r.eleveId}`}
                      className="text-xs font-bold text-indigo-600 hover:underline"
                    >
                      Dossier
                    </Link>
                  ) : null}
                  {canDecideAccess ? (
                    <>
                      <button
                        type="button"
                        disabled={busy}
                        onClick={() => void decideAccess(r.id, "approved")}
                        className="rounded-lg bg-emerald-600 px-3 py-1.5 text-xs font-bold text-white disabled:opacity-50"
                      >
                        Approuver
                      </button>
                      <button
                        type="button"
                        disabled={busy}
                        onClick={() => void decideAccess(r.id, "rejected")}
                        className="rounded-lg bg-slate-200 px-3 py-1.5 text-xs font-bold text-slate-700 disabled:opacity-50"
                      >
                        Refuser
                      </button>
                    </>
                  ) : (
                    <span className="text-xs font-bold text-amber-700">En attente</span>
                  )}
                </div>
              </li>
            ))
          )}
        </ul>
      ) : null}
    </ModulePageShell>
  );
}
