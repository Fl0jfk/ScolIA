"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";

type EleveRow = {
  id: string;
  nom: string;
  prenom: string;
  classe: string | null;
  classeLabel?: string | null;
  status: string;
  siteId: string | null;
  siteLabel?: string | null;
  folderName: string;
  ine: string | null;
};

type SiteOption = { siteId: string; label: string };

type ClassOption = {
  value: string;
  label: string;
  siteId: string | null;
  siteLabel: string | null;
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

const STATUS_OPTIONS = [
  { value: "", label: "Tous statuts" },
  { value: "inscrit", label: "En cours" },
  { value: "preinscrit", label: "Préinscrit" },
  { value: "ancien", label: "Ancien" },
  { value: "archive", label: "Archivé" },
];

export default function ElevesDossiersListClient() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const [eleves, setEleves] = useState<EleveRow[]>([]);
  const [preinsc, setPreinsc] = useState<Preinsc[]>([]);
  const [accessReqs, setAccessReqs] = useState<AccessReq[]>([]);
  const [canDecideAccess, setCanDecideAccess] = useState(false);
  const [canViewFullHub, setCanViewFullHub] = useState(false);
  const [profScoped, setProfScoped] = useState(false);
  const [sites, setSites] = useState<SiteOption[]>([]);
  const [siteLabelById, setSiteLabelById] = useState<Record<string, string>>({});
  const [classOptions, setClassOptions] = useState<ClassOption[]>([]);
  const [listMessage, setListMessage] = useState<string | null>(null);
  const [metaReady, setMetaReady] = useState(false);
  const [q, setQ] = useState(() => searchParams.get("q")?.trim() || "");
  const [siteFilter, setSiteFilter] = useState("");
  const [classeFilter, setClasseFilter] = useState(
    () => searchParams.get("classe")?.trim() || "",
  );
  const [statusFilter, setStatusFilter] = useState("");
  const [preSiteFilter, setPreSiteFilter] = useState("");
  const [tab, setTab] = useState<"dossiers" | "preinscriptions" | "acces">("dossiers");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    const nextClasse = searchParams.get("classe")?.trim() || "";
    const nextQ = searchParams.get("q")?.trim() || "";
    if (nextClasse) setClasseFilter(nextClasse);
    if (nextQ) setQ(nextQ);
  }, [searchParams]);

  const loadDossiers = useCallback(async () => {
    const res = await fetch("/api/eleves/dossiers/list", { cache: "no-store" });
    if (!res.ok) {
      const j = (await res.json().catch(() => ({}))) as { error?: string };
      throw new Error(j.error || `Erreur ${res.status}`);
    }
    const j = (await res.json()) as {
      eleves: EleveRow[];
      canViewFullHub: boolean;
      profScoped: boolean;
      assignedClasses: string[];
      sites: SiteOption[];
      siteLabelById?: Record<string, string>;
      classOptions?: ClassOption[];
      message?: string;
    };
    setEleves(j.eleves || []);
    setCanViewFullHub(Boolean(j.canViewFullHub));
    setProfScoped(Boolean(j.profScoped));
    setSites(j.sites || []);
    setSiteLabelById(j.siteLabelById || {});
    setClassOptions(j.classOptions || []);
    setListMessage(j.message ?? null);
    setMetaReady(true);
  }, []);

  useEffect(() => {
    void (async () => {
      try {
        setError(null);
        await loadDossiers();
      } catch (e) {
        setError(e instanceof Error ? e.message : "Erreur chargement");
        setMetaReady(true);
      }
    })();
  }, [loadDossiers]);

  useEffect(() => {
    if (!canViewFullHub) return;
    void (async () => {
      try {
        const [pr, ar] = await Promise.all([
          fetch("/api/eleves/preinscriptions?status=pending"),
          fetch("/api/eleves/document-access-requests?status=pending"),
        ]);
        if (pr.ok) {
          const j = (await pr.json()) as { preinscriptions: Preinsc[] };
          setPreinsc(j.preinscriptions || []);
        }
        if (ar.ok) {
          const j = (await ar.json()) as { requests: AccessReq[]; canDecide: boolean };
          setAccessReqs(j.requests || []);
          setCanDecideAccess(Boolean(j.canDecide));
        }
      } catch {
        /* onglets admin secondaires */
      }
    })();
  }, [canViewFullHub]);

  const hasActiveSearch = Boolean(
    q.trim() || classeFilter || siteFilter || statusFilter,
  );

  const filtered = useMemo(() => {
    if (!hasActiveSearch) return [];
    const needle = q.trim().toLowerCase();
    return eleves.filter((e) => {
      if (siteFilter && e.siteId !== siteFilter) return false;
      if (classeFilter && e.classe !== classeFilter) return false;
      if (statusFilter && e.status !== statusFilter) return false;
      if (!needle) return true;
      return `${e.nom} ${e.prenom} ${e.classe || ""} ${e.classeLabel || ""} ${e.ine || ""} ${e.siteLabel || ""}`
        .toLowerCase()
        .includes(needle);
    });
  }, [eleves, q, siteFilter, classeFilter, statusFilter, hasActiveSearch]);

  const classOptionsForSite = useMemo(() => {
    if (!siteFilter) return classOptions;
    return classOptions.filter((c) => !c.siteId || c.siteId === siteFilter);
  }, [classOptions, siteFilter]);

  const preFiltered = useMemo(() => {
    if (!preSiteFilter) return preinsc;
    return preinsc.filter((p) => p.siteId === preSiteFilter);
  }, [preinsc, preSiteFilter]);

  const preSites = useMemo(() => {
    const ids = new Set<string>();
    for (const p of preinsc) if (p.siteId) ids.add(p.siteId);
    return [...ids].sort((a, b) => {
      const la = siteLabelById[a] || a;
      const lb = siteLabelById[b] || b;
      return la.localeCompare(lb, "fr", { sensitivity: "base" });
    });
  }, [preinsc, siteLabelById]);

  const statusLabel = (status: string): string => {
    const hit = STATUS_OPTIONS.find((o) => o.value === status);
    return hit?.label || status;
  };

  function clearSearch() {
    setQ("");
    setSiteFilter("");
    setClasseFilter("");
    setStatusFilter("");
  }

  async function decide(id: string, action: "accept" | "reject") {
    const res = await fetch("/api/eleves/preinscriptions", {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ id, action }),
    });
    const j = (await res.json().catch(() => ({}))) as { error?: string; eleve?: { id: string } };
    if (!res.ok) {
      setError(j.error || "Échec");
      return;
    }
    setPreinsc((prev) => prev.filter((p) => p.id !== id));
    if (action === "accept" && j.eleve?.id) {
      router.push(`/eleves/dossier/${j.eleve.id}`);
    }
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
        description={
          profScoped
            ? "Recherchez un élève de vos classes — fiche pédagogique uniquement."
            : "Recherchez un élève, ou gérez les préinscriptions."
        }
        actions={
          canViewFullHub ? (
            <Link href="/preinscription" className="text-sm font-bold text-indigo-600 hover:underline">
              Formulaire public
            </Link>
          ) : null
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
          {hasActiveSearch ? `Dossiers (${filtered.length})` : "Dossiers"}
        </button>
        {canViewFullHub ? (
          <>
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
          </>
        ) : null}
      </div>

      {error ? <p className="mb-3 text-sm text-red-600">{error}</p> : null}
      {listMessage ? <p className="mb-3 text-sm text-amber-700">{listMessage}</p> : null}

      {tab === "dossiers" ? (
        <div className="space-y-4">
          <div className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm sm:p-6">
            <label htmlFor="dossier-search" className="mb-2 block text-sm font-bold text-slate-800">
              Rechercher un élève
            </label>
            <input
              id="dossier-search"
              value={q}
              onChange={(e) => setQ(e.target.value)}
              placeholder="Nom, prénom, classe, INE…"
              autoComplete="off"
              className="w-full rounded-2xl border border-slate-200 bg-slate-50 px-5 py-4 text-base text-slate-900 outline-none transition focus:border-slate-400 focus:bg-white focus:ring-2 focus:ring-slate-200"
            />
            <div className="mt-3 flex flex-wrap gap-2">
              {canViewFullHub ? (
                <>
                  <select
                    value={siteFilter}
                    onChange={(e) => {
                      setSiteFilter(e.target.value);
                      setClasseFilter("");
                    }}
                    className="rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm"
                    aria-label="Établissement"
                  >
                    <option value="">Tous les établissements</option>
                    {sites.map((s) => (
                      <option key={s.siteId} value={s.siteId}>
                        {s.label || s.siteId}
                      </option>
                    ))}
                  </select>
                  <select
                    value={statusFilter}
                    onChange={(e) => setStatusFilter(e.target.value)}
                    className="rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm"
                    aria-label="Statut"
                  >
                    {STATUS_OPTIONS.map((o) => (
                      <option key={o.value || "all"} value={o.value}>
                        {o.label}
                      </option>
                    ))}
                  </select>
                </>
              ) : null}
              <select
                value={classeFilter}
                onChange={(e) => setClasseFilter(e.target.value)}
                className="rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm"
                aria-label="Classe"
              >
                <option value="">{profScoped ? "Toutes mes classes" : "Toutes les classes"}</option>
                {classOptionsForSite.map((c) => (
                  <option key={c.value} value={c.value}>
                    {c.label}
                  </option>
                ))}
              </select>
              {hasActiveSearch ? (
                <button
                  type="button"
                  onClick={clearSearch}
                  className="rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm font-semibold text-slate-600 hover:bg-slate-50"
                >
                  Effacer
                </button>
              ) : null}
            </div>
          </div>

          {!metaReady ? (
            <p className="px-2 text-center text-sm text-slate-500">Chargement…</p>
          ) : !hasActiveSearch ? (
            <div className="rounded-3xl border border-dashed border-slate-200 bg-slate-50/80 px-6 py-12 text-center">
              <p className="text-base font-semibold text-slate-800">Aucun dossier affiché</p>
              <p className="mt-2 text-sm text-slate-500">
                Tapez un nom, ou choisissez une classe / un établissement pour afficher les résultats.
              </p>
            </div>
          ) : (
            <ul className="divide-y divide-slate-100 overflow-hidden rounded-3xl border border-slate-200 bg-white">
              {filtered.slice(0, 200).map((e) => (
                <li key={e.id} className="flex items-center justify-between gap-3 px-4 py-3">
                  <div>
                    <p className="font-semibold text-slate-900">
                      {e.prenom} {e.nom}
                    </p>
                    <p className="text-xs text-slate-500">
                      {e.classeLabel || e.classe || "—"}
                      {canViewFullHub && e.status ? ` · ${statusLabel(e.status)}` : ""}
                      {canViewFullHub && e.siteLabel ? ` · ${e.siteLabel}` : ""}
                    </p>
                  </div>
                  <Link
                    href={`/eleves/dossier/${e.id}`}
                    className="text-sm font-bold text-indigo-600 hover:underline"
                  >
                    Ouvrir
                  </Link>
                </li>
              ))}
              {filtered.length === 0 ? (
                <li className="px-4 py-8 text-center text-sm text-slate-500">
                  Aucun élève ne correspond à cette recherche.
                </li>
              ) : null}
              {filtered.length > 200 ? (
                <li className="px-4 py-3 text-center text-xs text-slate-400">
                  Affichage limité à 200 résultats — affinez la recherche.
                </li>
              ) : null}
            </ul>
          )}
        </div>
      ) : null}

      {tab === "preinscriptions" && canViewFullHub ? (
        <>
          <div className="mb-4 flex flex-wrap gap-2">
            <button
              type="button"
              onClick={() => setPreSiteFilter("")}
              className={`rounded-lg px-3 py-1.5 text-xs font-bold border ${
                !preSiteFilter ? "bg-slate-900 text-white" : "bg-white"
              }`}
            >
              Tous les établissements
            </button>
            {preSites.map((s) => (
              <button
                key={s}
                type="button"
                onClick={() => setPreSiteFilter(s)}
                className={`rounded-lg px-3 py-1.5 text-xs font-bold border ${
                  preSiteFilter === s ? "bg-slate-900 text-white" : "bg-white"
                }`}
              >
                {siteLabelById[s] || s}
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
                    {p.siteId ? siteLabelById[p.siteId] || p.siteId : "Établissement ?"} ·{" "}
                    {p.niveauVise || "niveau ?"}
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

      {tab === "acces" && canViewFullHub ? (
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
