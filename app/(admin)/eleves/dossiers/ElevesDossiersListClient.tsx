"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import { schoolClassesMatch } from "@/app/lib/school-classes-catalog";
import { personMatchesSearchQuery } from "@/app/lib/person-name-search";
import {
  DOCUMENT_ACCESS_DURATION_OPTIONS,
  documentAccessDurationLabel,
} from "@/app/lib/eleve-document-access-duration";

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
  photoUrl?: string | null;
  hasPap?: boolean;
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
  eleveClasse?: string | null;
  requesterName?: string | null;
  requesterEmail?: string | null;
};

const STATUS_OPTIONS = [
  { value: "", label: "Tous statuts" },
  { value: "inscrit", label: "Scolarisé" },
  { value: "preinscrit", label: "Préinscription" },
  { value: "ancien", label: "Ancien" },
];

export default function ElevesDossiersListClient() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const [eleves, setEleves] = useState<EleveRow[]>([]);
  const [preinsc, setPreinsc] = useState<Preinsc[]>([]);
  const [accessReqs, setAccessReqs] = useState<AccessReq[]>([]);
  const [canDecideAccess, setCanDecideAccess] = useState(false);
  const [canViewFullHub, setCanViewFullHub] = useState(false);
  const [canManagePreinscriptions, setCanManagePreinscriptions] = useState(false);
  const [canOpenDetail, setCanOpenDetail] = useState(true);
  const [profScoped, setProfScoped] = useState(false);
  const [sites, setSites] = useState<SiteOption[]>([]);
  const [siteLabelById, setSiteLabelById] = useState<Record<string, string>>({});
  const [classOptions, setClassOptions] = useState<ClassOption[]>([]);
  const [listMessage, setListMessage] = useState<string | null>(null);
  const [metaReady, setMetaReady] = useState(false);
  const [q, setQ] = useState(() => searchParams.get("q")?.trim() || "");
  const [siteFilter, setSiteFilter] = useState(() => searchParams.get("site")?.trim() || "");
  const [classeFilter, setClasseFilter] = useState(
    () => searchParams.get("classe")?.trim() || "",
  );
  const [statusFilter, setStatusFilter] = useState(
    () => searchParams.get("status")?.trim() || "inscrit",
  );
  const [preSiteFilter, setPreSiteFilter] = useState("");
  const [tab, setTab] = useState<"dossiers" | "preinscriptions" | "acces">(() => {
    const t = searchParams.get("tab");
    if (t === "preinscriptions" || t === "acces") return t;
    return "dossiers";
  });
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    const nextClasse = searchParams.get("classe")?.trim() || "";
    const nextQ = searchParams.get("q")?.trim() || "";
    const nextSite = searchParams.get("site")?.trim() || "";
    const nextStatus = searchParams.get("status")?.trim() || "";
    const nextTab = searchParams.get("tab");
    setClasseFilter(nextClasse);
    setQ(nextQ);
    setSiteFilter(nextSite);
    setStatusFilter(nextStatus);
    if (nextTab === "preinscriptions" || nextTab === "acces" || nextTab === "dossiers") {
      setTab(nextTab);
    }
  }, [searchParams]);

  useEffect(() => {
    if (tab === "preinscriptions" && !canManagePreinscriptions) setTab("dossiers");
    if (tab === "acces" && (!canViewFullHub || !canOpenDetail)) setTab("dossiers");
  }, [tab, canManagePreinscriptions, canViewFullHub, canOpenDetail]);

  const listQueryString = useMemo(() => {
    const p = new URLSearchParams();
    if (q.trim()) p.set("q", q.trim());
    if (classeFilter) p.set("classe", classeFilter);
    if (siteFilter) p.set("site", siteFilter);
    if (statusFilter) p.set("status", statusFilter);
    if (tab !== "dossiers") p.set("tab", tab);
    const s = p.toString();
    return s ? `?${s}` : "";
  }, [q, classeFilter, siteFilter, statusFilter, tab]);

  useEffect(() => {
    if (!metaReady) return;
    const target = `/eleves/dossiers${listQueryString}`;
    const current = `${window.location.pathname}${window.location.search}`;
    if (current !== target) {
      router.replace(target, { scroll: false });
    }
  }, [listQueryString, metaReady, router]);

  const dossierHref = useCallback(
    (eleveId: string) => {
      const retour = listQueryString || "";
      if (!retour) return `/eleves/dossier/${eleveId}`;
      return `/eleves/dossier/${eleveId}?retour=${encodeURIComponent(retour)}`;
    },
    [listQueryString],
  );

  const loadDossiers = useCallback(async () => {
    const res = await fetch("/api/eleves/dossiers/list", { cache: "no-store" });
    if (!res.ok) {
      const j = (await res.json().catch(() => ({}))) as { error?: string };
      throw new Error(j.error || `Erreur ${res.status}`);
    }
    const j = (await res.json()) as {
      eleves: EleveRow[];
      canViewFullHub: boolean;
      canManagePreinscriptions?: boolean;
      canOpenDetail?: boolean;
      profScoped: boolean;
      assignedClasses: string[];
      sites: SiteOption[];
      siteLabelById?: Record<string, string>;
      classOptions?: ClassOption[];
      message?: string;
    };
    setEleves(j.eleves || []);
    setCanViewFullHub(Boolean(j.canViewFullHub));
    setCanManagePreinscriptions(Boolean(j.canManagePreinscriptions));
    setCanOpenDetail(j.canOpenDetail !== false);
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
    if (!canViewFullHub && !canManagePreinscriptions) return;
    void (async () => {
      try {
        const fetches: Promise<Response>[] = [];
        if (canManagePreinscriptions) {
          fetches.push(fetch("/api/eleves/preinscriptions?status=pending"));
        }
        if (canViewFullHub && canOpenDetail) {
          fetches.push(fetch("/api/eleves/document-access-requests?status=pending"));
        }
        const results = await Promise.all(fetches);
        let idx = 0;
        if (canManagePreinscriptions) {
          const pr = results[idx++]!;
          if (pr.ok) {
            const j = (await pr.json()) as { preinscriptions: Preinsc[] };
            setPreinsc(j.preinscriptions || []);
          }
        }
        if (canViewFullHub && canOpenDetail) {
          const ar = results[idx++]!;
          if (ar.ok) {
            const j = (await ar.json()) as { requests: AccessReq[]; canDecide: boolean };
            setAccessReqs(j.requests || []);
            setCanDecideAccess(Boolean(j.canDecide));
          }
        }
      } catch {
        /* onglets admin secondaires */
      }
    })();
  }, [canViewFullHub, canManagePreinscriptions, canOpenDetail]);

  const hasActiveSearch = Boolean(
    q.trim() || classeFilter || siteFilter || statusFilter,
  );

  const filtered = useMemo(() => {
    if (!hasActiveSearch) return [];
    const needle = q.trim().toLowerCase();
    return eleves.filter((e) => {
      if (siteFilter && e.siteId !== siteFilter) return false;
      if (classeFilter && !schoolClassesMatch(e.classe, classeFilter)) return false;
      if (statusFilter && e.status !== statusFilter) return false;
      if (!needle) return true;
      return personMatchesSearchQuery(
        {
          nom: e.nom,
          prenom: e.prenom,
          extras: [e.classe, e.classeLabel, e.ine, e.siteLabel],
        },
        needle,
      );
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
    setStatusFilter("inscrit");
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
      router.push(dossierHref(j.eleve.id));
    }
  }

  async function decideAccess(
    id: string,
    decision: "approved" | "rejected",
    durationDays?: number,
  ) {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/eleves/document-access-requests", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id, decision, durationDays }),
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
            : canManagePreinscriptions
              ? "Recherchez un élève, ou gérez les préinscriptions."
              : "Recherchez un élève dans le référentiel."
        }
        actions={
          canManagePreinscriptions ? (
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
        {canManagePreinscriptions ? (
          <button
            type="button"
            onClick={() => setTab("preinscriptions")}
            className={`rounded-xl px-4 py-2 text-sm font-bold border ${
              tab === "preinscriptions" ? "bg-slate-900 text-white" : "bg-white border-slate-200"
            }`}
          >
            Préinscriptions ({preFiltered.length})
          </button>
        ) : null}
        {canViewFullHub && canOpenDetail ? (
          <button
            type="button"
            onClick={() => setTab("acces")}
            className={`rounded-xl px-4 py-2 text-sm font-bold border ${
              tab === "acces" ? "bg-slate-900 text-white" : "bg-white border-slate-200"
            }`}
          >
            Accès documents ({accessReqs.length})
          </button>
        ) : null}
      </div>

      {error ? <p className="mb-3 text-sm text-red-600">{error}</p> : null}
      {listMessage ? <p className="mb-3 text-sm text-amber-700">{listMessage}</p> : null}

      {tab === "dossiers" ? (
        <div className="space-y-4">
          <div className="rounded-3xl border border-slate-200/80 bg-gradient-to-br from-white via-white to-slate-50 p-5 shadow-sm sm:p-6">
            <label htmlFor="dossier-search" className="mb-2 block text-sm font-bold text-slate-800">
              Rechercher un élève
            </label>
            <input
              id="dossier-search"
              value={q}
              onChange={(e) => setQ(e.target.value)}
              placeholder="Nom, prénom, classe, INE…"
              autoComplete="off"
              className="w-full rounded-2xl border border-slate-200 bg-white px-5 py-4 text-base text-slate-900 shadow-inner outline-none transition focus:border-indigo-300 focus:ring-2 focus:ring-indigo-100"
            />
            <div className="mt-3 flex flex-wrap items-center gap-2">
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
              {hasActiveSearch && metaReady ? (
                <span className="ml-auto text-xs font-semibold text-slate-500">
                  {filtered.length} résultat{filtered.length > 1 ? "s" : ""}
                </span>
              ) : null}
            </div>
          </div>

          {!metaReady ? (
            <p className="px-2 text-center text-sm text-slate-500">Chargement…</p>
          ) : !hasActiveSearch ? (
            <div className="rounded-3xl border border-dashed border-slate-200 bg-slate-50/80 px-6 py-12 text-center">
              <p className="text-base font-semibold text-slate-800">Affinez la recherche</p>
              <p className="mt-2 text-sm text-slate-500">
                Tapez un nom, choisissez un établissement ou une classe, ou incluez les anciens élèves via le
                filtre statut.
              </p>
            </div>
          ) : (
            <ul className="overflow-hidden rounded-3xl border border-slate-200/80 bg-white shadow-sm">
              {filtered.slice(0, 200).map((e) => {
                const initials = `${e.prenom.charAt(0)}${e.nom.charAt(0)}`.toUpperCase() || "?";
                const rowBody = (
                  <>
                      {e.photoUrl ? (
                        // eslint-disable-next-line @next/next/no-img-element
                        <img
                          src={e.photoUrl}
                          alt=""
                          className="h-12 w-12 shrink-0 rounded-2xl object-cover ring-1 ring-slate-200 sm:h-14 sm:w-14"
                        />
                      ) : (
                        <div
                          className="flex h-12 w-12 shrink-0 items-center justify-center rounded-2xl bg-gradient-to-br from-slate-600 to-slate-800 text-sm font-black tracking-wide text-white shadow-inner sm:h-14 sm:w-14 sm:text-base"
                          aria-hidden
                        >
                          {initials}
                        </div>
                      )}
                      <div className="min-w-0 flex-1">
                        <div className="flex flex-wrap items-baseline gap-x-2 gap-y-0.5">
                          <p className="truncate text-base font-semibold text-slate-900">
                            {e.prenom} {e.nom}
                          </p>
                          {e.hasPap ? (
                            <span className="rounded-full bg-rose-600 px-2 py-0.5 text-[10px] font-black uppercase tracking-wide text-white">
                              PAP
                            </span>
                          ) : null}
                          {canViewFullHub && e.status ? (
                            <span className="rounded-full bg-slate-100 px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide text-slate-600">
                              {statusLabel(e.status)}
                            </span>
                          ) : null}
                        </div>
                        <div className="mt-1 flex flex-wrap items-center gap-x-2 gap-y-0.5 text-sm text-slate-600">
                          <span className="font-medium text-slate-800">
                            {e.classeLabel || e.classe || "Classe non renseignée"}
                          </span>
                          {canViewFullHub && e.siteLabel ? (
                            <>
                              <span className="text-slate-300" aria-hidden>
                                ·
                              </span>
                              <span>{e.siteLabel}</span>
                            </>
                          ) : null}
                          {canViewFullHub && e.ine ? (
                            <>
                              <span className="text-slate-300" aria-hidden>
                                ·
                              </span>
                              <span className="font-mono text-xs text-slate-500">INE {e.ine}</span>
                            </>
                          ) : null}
                        </div>
                      </div>
                      {canOpenDetail ? (
                        <span
                          className="shrink-0 text-slate-300 transition group-hover:translate-x-0.5 group-hover:text-indigo-600"
                          aria-hidden
                        >
                          →
                        </span>
                      ) : null}
                  </>
                );
                return (
                  <li key={e.id} className="border-b border-slate-100 last:border-b-0">
                    {canOpenDetail ? (
                      <Link
                        href={dossierHref(e.id)}
                        className="group flex items-center gap-3 px-3 py-3 transition hover:bg-slate-50/90 sm:gap-4 sm:px-4 sm:py-3.5"
                      >
                        {rowBody}
                      </Link>
                    ) : (
                      <div
                        className="flex items-center gap-3 px-3 py-3 sm:gap-4 sm:px-4 sm:py-3.5"
                        title="Consultation liste uniquement — ouverture du dossier réservée à d’autres rôles"
                      >
                        {rowBody}
                      </div>
                    )}
                  </li>
                );
              })}
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

      {tab === "preinscriptions" && canManagePreinscriptions ? (
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

      {tab === "acces" && canViewFullHub && canOpenDetail ? (
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
                        {r.eleveClasse ? ` (${r.eleveClasse})` : ""}
                      </span>
                    ) : null}
                  </p>
                  <p className="text-xs text-slate-500">
                    {r.requesterName || r.requesterEmail
                      ? `Demandeur : ${r.requesterName || r.requesterEmail} · `
                      : ""}
                    {r.docTiroir || "tiroir"} · {documentAccessDurationLabel(r.durationDays)}
                    {r.note ? ` · ${r.note}` : ""}
                  </p>
                </div>
                <div className="flex flex-wrap gap-2 items-center">
                  {r.eleveId ? (
                    <Link
                      href={dossierHref(r.eleveId)}
                      className="text-xs font-bold text-indigo-600 hover:underline"
                    >
                      Dossier
                    </Link>
                  ) : null}
                  {canDecideAccess ? (
                    <>
                      <select
                        id={`list-decide-duration-${r.id}`}
                        defaultValue={r.durationDays}
                        className="rounded-lg border border-slate-200 px-2 py-1.5 text-xs"
                        aria-label="Durée d’accès"
                      >
                        {DOCUMENT_ACCESS_DURATION_OPTIONS.map((o) => (
                          <option key={o.days} value={o.days}>
                            {o.label}
                          </option>
                        ))}
                      </select>
                      <button
                        type="button"
                        disabled={busy}
                        onClick={() => {
                          const sel = document.getElementById(
                            `list-decide-duration-${r.id}`,
                          ) as HTMLSelectElement | null;
                          void decideAccess(
                            r.id,
                            "approved",
                            sel ? Number(sel.value) : r.durationDays,
                          );
                        }}
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
