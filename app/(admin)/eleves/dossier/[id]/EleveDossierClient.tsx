"use client";

import { useCallback, useEffect, useMemo, useRef, useState, type DragEvent } from "react";
import Link from "next/link";
import { useParams, useSearchParams } from "next/navigation";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import {
  CATEGORIE_TIROIRS,
  DOC_CATEGORIE_LABELS,
  TIROIR_LABELS,
  TIROIR_TO_CATEGORIE,
  type EleveDocCategorie,
} from "@/app/lib/eleve-doc-categories";
import EleveFinancesPanel from "@/app/components/eleves/EleveFinancesPanel";
import EleveDossierSidebar from "@/app/components/eleves/EleveDossierSidebar";
import { scolariteStatutLabel } from "@/app/lib/eleve-dossier-synthese";
import {
  formatFoyerFacturationLabel,
  formatFoyerPayeurDetail,
  responsableRoleTags,
} from "@/app/lib/foyer-display";
import {
  grilleFromMealDays,
  toggleGrilleCell,
  type EleveGrilleRepas,
  type EleveGrilleRepasDay,
  type MealDayKey,
} from "@/app/lib/eleve-grille-repas";

function dossiersListHrefFromRetour(retour: string | null, fallbackClasse?: string | null): string {
  if (retour) {
    const decoded = (() => {
      try {
        return decodeURIComponent(retour);
      } catch {
        return retour;
      }
    })();
    if (decoded.startsWith("/eleves/dossiers")) return decoded;
    if (decoded.startsWith("?")) return `/eleves/dossiers${decoded}`;
  }
  if (fallbackClasse?.trim()) {
    return `/eleves/dossiers?classe=${encodeURIComponent(fallbackClasse.trim())}`;
  }
  return "/eleves/dossiers";
}

function dossiersListHrefForClasse(classe: string | null | undefined): string {
  const cls = String(classe || "").trim();
  if (!cls) return "/eleves/dossiers";
  return `/eleves/dossiers?classe=${encodeURIComponent(cls)}`;
}

function formatDateNaissanceFr(value: string | null | undefined): string {
  const raw = String(value || "").trim();
  if (!raw) return "—";
  const iso = raw.slice(0, 10);
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(iso);
  if (m) return `${m[3]}/${m[2]}/${m[1]}`;
  return raw;
}

function IconUpload({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" className={className} fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
      <path d="M12 16V4m0 0l-4 4m4-4l4 4" strokeLinecap="round" strokeLinejoin="round" />
      <path d="M4 14v4a2 2 0 002 2h12a2 2 0 002-2v-4" strokeLinecap="round" />
    </svg>
  );
}

function IconFile({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" className={className} fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
      <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" strokeLinejoin="round" />
      <path d="M14 2v6h6" strokeLinejoin="round" />
    </svg>
  );
}

function IconLock({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" className={className} fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
      <rect x="4" y="11" width="16" height="10" rx="2" />
      <path d="M8 11V8a4 4 0 118 0v3" strokeLinecap="round" />
    </svg>
  );
}

type DossierPayload = {
  eleve: {
    id: string;
    nom: string;
    prenom: string;
    classe: string | null;
    dateNaissance: string | null;
    lieuNaissance: string | null;
    status: string;
    secteur: string | null;
    ine?: string | null;
    mef?: string | null;
    parentEmail?: string | null;
    parent1Email?: string | null;
    parent2Email?: string | null;
    parentPhone?: string | null;
    parent1Phone?: string | null;
    parent2Phone?: string | null;
  };
  sections: string[];
  scolarites: Array<{
    id: string;
    siteId: string | null;
    classe: string | null;
    statut: string;
    demiPension: boolean;
    etablissementPrecedent: string | null;
    anneeScolaireId: string | null;
  }>;
  groupes?: Array<{ id: string; code: string; libelle: string; type: string }>;
  foyers: Array<{
    id: string;
    label: string;
    adresse: string | null;
    codePostal: string | null;
    ville: string | null;
    payeurEstFoyer: boolean;
    relation: string;
    responsables: Array<{
      id: string;
      nom: string;
      prenom: string;
      email: string | null;
      telephone: string | null;
      autoriteParentale: boolean;
      contactUrgence: boolean;
      payeur: boolean;
      rang: number;
    }>;
  }>;
  documents: Array<{
    id: string;
    tiroir: string;
    title: string;
    canOpen: boolean;
    lockedReason: string | null;
    source: string;
    anneeLabel: string | null;
    mimeType: string | null;
    fileUrl: string | null;
    confidentialite: string;
  }>;
  classmates?: Array<{ id: string; nom: string; prenom: string }>;
  meta: {
    sites: Array<{ siteId: string; label: string; kind: string | null }>;
    annees: Array<{ id: string; label: string; isCurrent: boolean }>;
    canEditStructure: boolean;
    canDecideAccess: boolean;
    profRestrictedView?: boolean;
    tiroirs: string[];
    docCategories?: Array<"administratif" | "financier" | "sante">;
  };
  pendingAccessRequests: Array<{
    id: string;
    documentId: string;
    requesterUserId: string;
    durationDays: number;
    note: string | null;
    createdAt: string;
    docTitle: string;
  }>;
  enCoursMaintenant: {
    activity: {
      subject: string;
      room: string | null;
      start: string;
      end: string;
      teacherName: string;
      kind: "cours" | "remplacement";
      weekType: "A" | "B" | null;
    } | null;
    reason: string;
    label?: string;
    conflictCount?: number;
  };
  synthese?: {
    statusLabel: string;
    classeLabel: string | null;
    siteLabel: string | null;
    initials: string;
    photoUrl: string | null;
    mef?: string | null;
    ine?: string | null;
    groupesAcademiques?: Array<{ code: string; libelle: string; type: string }>;
    groupesInternes?: Array<{ code: string; libelle: string; type: string }>;
    restauration: {
      regime: "externe" | "demi_pension" | "interne";
      days: Array<{
        key: MealDayKey;
        label: string;
        midi: boolean;
        soir: boolean;
        etude: boolean;
        garderie: boolean;
        sortSeul: boolean;
      }>;
      repasParSemaine: number | null;
      inferred: boolean;
    };
    internat: { actif: boolean; roomLabel: string | null };
    notesTrimestre: { available: boolean; label: string; value: string; detail: string };
    absences: { available: boolean; label: string; value: string; detail: string };
    finances: { available: boolean; label: string; detail: string };
  };
  notes?: Array<{
    matiereLibelle: string;
    moyenne: string | null;
    nbNotes: number;
    periodeId?: string;
    periodeLibelle?: string;
    periodeStatut?: string;
  }>;
  competences?: Array<{
    domaineLibelle: string;
    itemLibelle: string;
    niveau: string | null;
    niveauLabel: string;
    periodeId: string;
  }>;
  absences?: Array<{
    id: string;
    dateDebut: string;
    type: string;
    statut: string;
    justifie: boolean;
    motif: string | null;
  }>;
  sanctions?: Array<{
    id: string;
    typeLibelle: string;
    dateSanction: string;
    motif: string | null;
    createdByNom: string | null;
  }>;
  carnet?: Array<{
    id: string;
    dateEntree: string;
    categorie: string;
    titre: string;
    corps: string;
    signeAt: string | null;
    signeParNom: string | null;
    createdByNom: string | null;
  }>;
};

type TabId =
  | "synthese"
  | "famille"
  | "documents"
  | "scolarite"
  | "finances"
  | "notes"
  | "vie_scolaire";

const emptyResp = {
  nom: "",
  prenom: "",
  email: "",
  telephone: "",
  autoriteParentale: true,
  contactUrgence: false,
  payeur: false,
};

export default function EleveDossierClient() {
  const params = useParams();
  const searchParams = useSearchParams();
  const id = String(params.id || "");
  const listHref = dossiersListHrefFromRetour(
    searchParams.get("retour"),
    searchParams.get("classe"),
  );
  const [data, setData] = useState<DossierPayload | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [tab, setTab] = useState<TabId>("synthese");
  const [focusFoyerId, setFocusFoyerId] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [dragOver, setDragOver] = useState(false);
  const [staleCache, setStaleCache] = useState(false);
  const dataRef = useRef<DossierPayload | null>(null);
  dataRef.current = data;

  const [foyerForm, setFoyerForm] = useState({
    label: "",
    adresse: "",
    codePostal: "",
    ville: "",
    payeurEstFoyer: true,
    relation: "principal",
    responsable: { ...emptyResp },
  });
  const [addRespFoyerId, setAddRespFoyerId] = useState<string | null>(null);
  const [addResp, setAddResp] = useState({ ...emptyResp });

  const [uploadMeta, setUploadMeta] = useState({
    tiroir: "scolaire",
    confidentialite: "standard",
    title: "",
  });
  const [docCategory, setDocCategory] = useState<EleveDocCategorie | "tous">("tous");
  const [accessForm, setAccessForm] = useState<{
    documentId: string;
    durationDays: number;
    note: string;
  } | null>(null);

  const load = useCallback(async (opts?: { silent?: boolean }) => {
    const cacheKey = `scola:eleve-dossier:${id}`;
    if (!opts?.silent) {
      setError(null);
      try {
        const raw = sessionStorage.getItem(cacheKey);
        if (raw) {
          const cached = JSON.parse(raw) as DossierPayload;
          if (cached?.eleve?.id === id) {
            setData(cached);
            setStaleCache(true);
          }
        }
      } catch {
        /* ignore cache corrompu */
      }
    }
    try {
      const res = await fetch(`/api/eleves/${id}/dossier`, { cache: "no-store" });
      const j = (await res.json().catch(() => ({}))) as {
        error?: string;
        detail?: string;
      } & Partial<DossierPayload>;
      if (!res.ok) {
        const detail = typeof j.detail === "string" && j.detail.trim() ? ` (${j.detail})` : "";
        if (!dataRef.current) {
          setError((j.error || `Erreur ${res.status}`) + detail);
          setData(null);
        } else {
          setError((j.error || `Erreur ${res.status}`) + detail);
        }
        return;
      }
      const payload = j as DossierPayload;
      setData(payload);
      setStaleCache(false);
      setError(null);
      try {
        sessionStorage.setItem(cacheKey, JSON.stringify(payload));
      } catch {
        /* quota / private mode */
      }
      const firstTiroir = payload.meta?.tiroirs?.[0];
      if (firstTiroir) {
        setUploadMeta((m) =>
          payload.meta.tiroirs.includes(m.tiroir) ? m : { ...m, tiroir: firstTiroir },
        );
      }
      const cats = payload.meta?.docCategories ?? [];
      setDocCategory((prev) => {
        if (prev === "tous") return cats.length === 1 ? cats[0]! : "tous";
        if (cats.includes(prev)) return prev;
        return cats.length === 1 ? cats[0]! : "tous";
      });
    } catch (e) {
      if (!dataRef.current) {
        setError(e instanceof Error ? e.message : "Erreur réseau");
        setData(null);
      }
    }
  }, [id]);

  useEffect(() => {
    void load();
  }, [load]);

  const tabs = useMemo(() => {
    if (!data) return [];
    const s = new Set(data.sections);
    const list: { id: TabId; label: string; show: boolean }[] = [
      { id: "synthese", label: "Synthèse", show: true },
      { id: "scolarite", label: "Scolarité", show: s.has("scolarite") },
      { id: "famille", label: "Famille", show: s.has("famille") },
      { id: "finances", label: "Finances", show: s.has("facturation") },
      { id: "notes", label: "Notes", show: s.has("notes") },
      { id: "vie_scolaire", label: "Vie scolaire", show: s.has("vie_scolaire") },
      { id: "documents", label: "Documents", show: s.has("documents") },
    ];
    return list.filter((t) => t.show);
  }, [data]);

  const allowedDocCategories = useMemo((): EleveDocCategorie[] => {
    if (!data?.meta.docCategories?.length) {
      const fromTiroirs = new Set<EleveDocCategorie>();
      for (const t of data?.meta.tiroirs ?? []) {
        const c = TIROIR_TO_CATEGORIE[t as keyof typeof TIROIR_TO_CATEGORIE];
        if (c) fromTiroirs.add(c);
      }
      return (["administratif", "financier", "sante"] as EleveDocCategorie[]).filter((c) =>
        fromTiroirs.has(c),
      );
    }
    return data.meta.docCategories;
  }, [data]);

  const uploadTiroirsForCategory = useMemo(() => {
    if (!data) return [] as string[];
    const active =
      docCategory === "tous"
        ? allowedDocCategories
        : allowedDocCategories.filter((c) => c === docCategory);
    const allowed = new Set(data.meta.tiroirs);
    const out: string[] = [];
    for (const cat of active) {
      for (const t of CATEGORIE_TIROIRS[cat]) {
        if (allowed.has(t)) out.push(t);
      }
    }
    return out.length ? out : data.meta.tiroirs;
  }, [data, docCategory, allowedDocCategories]);

  const filteredDocuments = useMemo(() => {
    if (!data) return [];
    if (docCategory === "tous") return data.documents;
    const tiroirs = new Set(CATEGORIE_TIROIRS[docCategory]);
    return data.documents.filter((d) => tiroirs.has(d.tiroir as keyof typeof TIROIR_TO_CATEGORIE));
  }, [data, docCategory]);

  useEffect(() => {
    if (!uploadTiroirsForCategory.length) return;
    if (!uploadTiroirsForCategory.includes(uploadMeta.tiroir)) {
      setUploadMeta((m) => ({ ...m, tiroir: uploadTiroirsForCategory[0]! }));
    }
  }, [uploadTiroirsForCategory, uploadMeta.tiroir]);

  async function postAction(payload: Record<string, unknown>) {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch(`/api/eleves/${id}/dossier`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      });
      const j = (await res.json().catch(() => ({}))) as { error?: string };
      if (!res.ok) throw new Error(j.error || `Erreur ${res.status}`);
      await load();
      return true;
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
      return false;
    } finally {
      setBusy(false);
    }
  }

  async function ensureFoyerFromContacts() {
    setBusy(true);
    setError(null);
    try {
      await postAction({ action: "ensure_foyer_from_contacts" });
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Impossible de créer le foyer");
    } finally {
      setBusy(false);
    }
  }

  async function createFoyer() {
    const ok = await postAction({
      action: "create_foyer",
      ...foyerForm,
      responsable: foyerForm.responsable,
    });
    if (ok) {
      setFoyerForm({
        label: "",
        adresse: "",
        codePostal: "",
        ville: "",
        payeurEstFoyer: true,
        relation: "principal",
        responsable: { ...emptyResp },
      });
    }
  }

  async function submitAddResp() {
    if (!addRespFoyerId) return;
    const ok = await postAction({
      action: "add_responsable",
      foyerId: addRespFoyerId,
      responsable: addResp,
    });
    if (ok) {
      setAddRespFoyerId(null);
      setAddResp({ ...emptyResp });
    }
  }

  async function uploadFiles(files: FileList | File[]) {
    const list = Array.from(files);
    if (list.length === 0) return;
    setBusy(true);
    setError(null);
    try {
      for (const file of list) {
        const prep = await fetch(`/api/eleves/${id}/documents/upload`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            fileName: file.name,
            fileType: file.type || "application/octet-stream",
            size: file.size,
          }),
        });
        const prepJ = (await prep.json().catch(() => ({}))) as {
          error?: string;
          uploadUrl?: string;
          s3Key?: string;
          fileUrl?: string;
        };
        if (!prep.ok || !prepJ.uploadUrl) {
          throw new Error(prepJ.error || "Préparation upload impossible");
        }
        const put = await fetch(prepJ.uploadUrl, {
          method: "PUT",
          headers: { "Content-Type": file.type || "application/octet-stream" },
          body: file,
        });
        if (!put.ok) throw new Error(`Upload S3 échoué (${put.status})`);

        const title =
          uploadMeta.title.trim() ||
          file.name.replace(/\.[^.]+$/, "").replace(/[_-]+/g, " ").trim() ||
          file.name;
        const reg = await fetch(`/api/eleves/${id}/dossier`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            action: "register_document",
            title,
            tiroir: uploadMeta.tiroir,
            confidentialite: uploadMeta.confidentialite,
            s3Key: prepJ.s3Key,
            fileUrl: prepJ.fileUrl,
            mimeType: file.type || "application/octet-stream",
            source: "upload",
          }),
        });
        const regJ = (await reg.json().catch(() => ({}))) as { error?: string };
        if (!reg.ok) throw new Error(regJ.error || "Enregistrement document échoué");
      }
      setUploadMeta((m) => ({ ...m, title: "" }));
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur upload");
    } finally {
      setBusy(false);
      setDragOver(false);
    }
  }

  function onDrop(e: DragEvent<HTMLDivElement>) {
    e.preventDefault();
    setDragOver(false);
    void uploadFiles(e.dataTransfer.files);
  }

  async function requestAccess() {
    if (!accessForm) return;
    const ok = await postAction({
      action: "request_document_access",
      documentId: accessForm.documentId,
      durationDays: accessForm.durationDays,
      note: accessForm.note || null,
    });
    if (ok) setAccessForm(null);
  }

  async function decideAccess(requestId: string, decision: "approved" | "rejected") {
    await postAction({ action: "decide_document_access", requestId, decision });
  }

  if (error && !data) {
    return (
      <ModulePageShell maxWidthClass="max-w-3xl">
        <p className="text-red-600">{error}</p>
        <Link href={listHref} className="text-sm font-semibold text-indigo-600">
          ← Retour liste
        </Link>
      </ModulePageShell>
    );
  }

  if (!data) {
    return (
      <ModulePageShell maxWidthClass="max-w-3xl">
        <p className="text-slate-500">Chargement du dossier…</p>
      </ModulePageShell>
    );
  }

  const e = data.eleve;
  const classeListHref = dossiersListHrefForClasse(e.classe);
  const canEdit = data.meta.canEditStructure;
  const synth = data.synthese;
  const liveNow = data.enCoursMaintenant;
  const statusLabel = synth?.statusLabel || e.status;
  const classeDisplay = synth?.classeLabel || e.classe || "Classe à préciser";
  const regime = synth?.restauration.regime ?? "externe";
  const regimeText =
    regime === "interne"
      ? "Interne"
      : regime === "demi_pension"
        ? "Demi-pensionnaire"
        : "Externe";

  function liveNowCopy(): { title: string; detail: string; tone: "live" | "idle" | "off" } {
    const live = liveNow;
    if (live.activity) {
      return {
        title: live.activity.subject,
        detail: [
          `${live.activity.start}–${live.activity.end}`,
          live.activity.room,
          live.activity.teacherName,
          live.activity.kind === "remplacement" ? "remplacement" : null,
          live.activity.weekType ? `sem. ${live.activity.weekType}` : null,
        ]
          .filter(Boolean)
          .join(" · "),
        tone: "live",
      };
    }
    if (live.reason === "weekend") {
      return { title: "Week-end", detail: live.label || "Pas de cours.", tone: "off" };
    }
    if (live.reason === "vacances" || live.reason === "ferie" || live.reason === "conge") {
      return {
        title: live.label || "Calendrier scolaire",
        detail: "Pas de cours aujourd’hui.",
        tone: "off",
      };
    }
    if (live.reason === "pas_de_classe") {
      return {
        title: "Classe non renseignée",
        detail: "Impossible de déduire l’emploi du temps.",
        tone: "idle",
      };
    }
    if (live.reason === "pas_edt") {
      return {
        title: "Pas de cours en ce moment",
        detail: "Aucun EDT renseigné pour cette classe.",
        tone: "idle",
      };
    }
    return {
      title: "Pas de cours en ce moment",
      detail: live.label || "Créneau libre ou hors emploi du temps.",
      tone: "idle",
    };
  }

  const nowView = liveNowCopy();

  function navigateToDossier(eleveId: string) {
    const retour = searchParams.get("retour");
    if (!retour) return `/eleves/dossier/${eleveId}`;
    return `/eleves/dossier/${eleveId}?retour=${encodeURIComponent(retour)}`;
  }

  function openFoyerFinances(foyerId: string) {
    setFocusFoyerId(foyerId);
    setTab("finances");
  }

  return (
    <ModulePageShell maxWidthClass="max-w-6xl">
      <Link
        href={listHref}
        className="mb-4 inline-flex items-center gap-1.5 text-sm font-semibold text-slate-600 transition hover:text-indigo-700"
      >
        <span aria-hidden>←</span>
        Retour à la liste des dossiers
      </Link>
      <ModulePageHeader
        eyebrow="Dossier élève"
        title={`${e.prenom} ${e.nom}`}
        description={
          <span className="inline-flex flex-wrap items-center gap-x-2 gap-y-1">
            {staleCache ? (
              <span className="rounded-full bg-amber-50 px-2 py-0.5 text-xs font-semibold text-amber-800">
                Mise à jour…
              </span>
            ) : null}
            {e.classe ? (
              <Link
                href={listHref}
                className="text-base font-bold text-indigo-700 hover:underline"
              >
                {classeDisplay}
              </Link>
            ) : (
              <span className="text-base font-bold text-slate-700">{classeDisplay}</span>
            )}
            <span className="text-slate-400">·</span>
            <span className="rounded-full bg-emerald-50 px-2.5 py-0.5 text-xs font-bold text-emerald-800">
              {statusLabel}
            </span>
            <span className="rounded-full bg-slate-100 px-2.5 py-0.5 text-xs font-bold text-slate-700">
              {regimeText}
            </span>
          </span>
        }
      />

      <div className="mb-6 flex flex-wrap gap-2">
        {tabs.map((t) => (
          <button
            key={t.id}
            type="button"
            onClick={() => setTab(t.id)}
            className={`rounded-xl px-4 py-2 text-sm font-bold border transition ${
              tab === t.id
                ? "bg-slate-900 text-white border-slate-900"
                : "bg-white text-slate-600 border-slate-200 hover:border-slate-400"
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>

      {error ? (
        <p className="mb-4 rounded-xl bg-red-50 px-3 py-2 text-sm text-red-700">{error}</p>
      ) : null}

      <div className="lg:grid lg:grid-cols-[minmax(220px,260px)_1fr] lg:gap-8 lg:items-start">
        <div className="hidden lg:block lg:sticky lg:top-4">
          <EleveDossierSidebar
            currentEleveId={id}
            classe={e.classe}
            classmates={data.classmates ?? []}
            dossierHref={navigateToDossier}
          />
        </div>

        <div className="min-w-0">
      {tab === "synthese" ? (
        <div className="space-y-4">
          {/* Identité + maintenant */}
          <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm sm:p-6">
            <div className="flex flex-col gap-5 sm:flex-row sm:items-start">
              <div className="shrink-0">
                {synth?.photoUrl ? (
                  // eslint-disable-next-line @next/next/no-img-element
                  <img
                    src={synth.photoUrl}
                    alt=""
                    className="h-24 w-24 rounded-2xl object-cover ring-1 ring-slate-200"
                  />
                ) : (
                  <div
                    className="flex h-24 w-24 items-center justify-center rounded-2xl bg-gradient-to-br from-slate-700 to-slate-900 text-2xl font-black tracking-wide text-white shadow-inner"
                    aria-hidden
                  >
                    {synth?.initials || `${e.prenom.charAt(0)}${e.nom.charAt(0)}`.toUpperCase()}
                  </div>
                )}
              </div>
              <div className="min-w-0 flex-1">
                <p className="text-xs font-bold uppercase tracking-widest text-slate-400">
                  Identité
                </p>
                <h2 className="mt-1 text-2xl font-black text-slate-900">
                  {e.prenom} {e.nom}
                </h2>
                {e.classe ? (
                  <Link
                    href={classeListHref}
                    className="mt-2 inline-flex items-center gap-2 rounded-2xl border border-indigo-200 bg-indigo-50 px-4 py-2 text-xl font-black text-indigo-900 transition hover:border-indigo-400 hover:bg-indigo-100"
                    title="Voir tous les élèves de cette classe"
                  >
                    {e.classe}
                    {synth?.siteLabel ? (
                      <span className="text-sm font-bold text-indigo-700/80">
                        · {synth.siteLabel}
                      </span>
                    ) : null}
                    <span className="text-xs font-bold uppercase tracking-wide text-indigo-600">
                      Classe →
                    </span>
                  </Link>
                ) : (
                  <p className="mt-2 text-lg font-bold text-slate-500">Classe non renseignée</p>
                )}
                <dl className="mt-4 grid gap-2 text-sm sm:grid-cols-2">
                  <div className="flex justify-between gap-3 rounded-xl bg-slate-50 px-3 py-2">
                    <dt className="text-slate-500">Né(e) le</dt>
                    <dd className="font-semibold text-slate-900">
                      {formatDateNaissanceFr(e.dateNaissance)}
                    </dd>
                  </div>
                  {!data.meta.profRestrictedView ? (
                    <div className="flex justify-between gap-3 rounded-xl bg-slate-50 px-3 py-2">
                      <dt className="text-slate-500">Lieu</dt>
                      <dd className="font-semibold text-slate-900">{e.lieuNaissance || "—"}</dd>
                    </div>
                  ) : null}
                  <div className="flex justify-between gap-3 rounded-xl bg-slate-50 px-3 py-2">
                    <dt className="text-slate-500">Statut</dt>
                    <dd className="font-semibold text-slate-900">{statusLabel}</dd>
                  </div>
                  <div className="flex justify-between gap-3 rounded-xl bg-slate-50 px-3 py-2">
                    <dt className="text-slate-500">Restauration</dt>
                    <dd className="font-semibold text-slate-900">{regimeText}</dd>
                  </div>
                  {synth?.mef ? (
                    <div className="flex justify-between gap-3 rounded-xl bg-slate-50 px-3 py-2 sm:col-span-2">
                      <dt className="text-slate-500">MEF</dt>
                      <dd className="font-mono font-semibold text-slate-900">{synth.mef}</dd>
                    </div>
                  ) : null}
                  {!data.meta.profRestrictedView && synth?.ine ? (
                    <div className="flex justify-between gap-3 rounded-xl bg-slate-50 px-3 py-2">
                      <dt className="text-slate-500">INE</dt>
                      <dd className="font-mono font-semibold text-slate-900">{synth.ine}</dd>
                    </div>
                  ) : null}
                </dl>
                {(synth?.groupesAcademiques?.length || synth?.groupesInternes?.length) ? (
                  <div className="mt-4 space-y-3">
                    {synth.groupesAcademiques && synth.groupesAcademiques.length > 0 ? (
                      <div>
                        <p className="text-xs font-bold uppercase tracking-wide text-slate-400">
                          Options académiques
                        </p>
                        <ul className="mt-2 flex flex-wrap gap-2">
                          {synth.groupesAcademiques.map((g) => (
                            <li
                              key={`${g.code}-${g.type}`}
                              className="rounded-xl border border-indigo-100 bg-indigo-50 px-3 py-1.5 text-xs font-semibold text-indigo-900"
                            >
                              {g.libelle}
                              <span className="ml-1 font-mono text-indigo-600/70">({g.code})</span>
                            </li>
                          ))}
                        </ul>
                      </div>
                    ) : null}
                    {synth.groupesInternes && synth.groupesInternes.length > 0 ? (
                      <div>
                        <p className="text-xs font-bold uppercase tracking-wide text-slate-400">
                          Options internes & activités
                        </p>
                        <ul className="mt-2 flex flex-wrap gap-2">
                          {synth.groupesInternes.map((g) => (
                            <li
                              key={`${g.code}-${g.type}`}
                              className="rounded-xl border border-emerald-100 bg-emerald-50 px-3 py-1.5 text-xs font-semibold text-emerald-900"
                            >
                              {g.libelle}
                              {g.type !== "autre" ? (
                                <span className="ml-1 text-emerald-700/70">({g.type})</span>
                              ) : null}
                            </li>
                          ))}
                        </ul>
                      </div>
                    ) : null}
                  </div>
                ) : null}
                {data.meta.profRestrictedView ? (
                  <p className="mt-3 text-xs text-slate-500">
                    Coordonnées élève et famille masquées (accès pédagogique).
                  </p>
                ) : null}
              </div>
            </div>
          </section>

          <section
            className={`rounded-3xl border p-5 shadow-sm sm:p-6 ${
              nowView.tone === "live"
                ? "border-indigo-200 bg-indigo-50/80"
                : nowView.tone === "off"
                  ? "border-slate-200 bg-slate-50"
                  : "border-slate-200 bg-white"
            }`}
          >
            <h2 className="text-xs font-bold uppercase tracking-widest text-slate-500">
              Maintenant
            </h2>
            <p
              className={`mt-2 text-2xl font-black ${
                nowView.tone === "live" ? "text-indigo-950" : "text-slate-900"
              }`}
            >
              {nowView.title}
            </p>
            <p
              className={`mt-1 text-sm ${
                nowView.tone === "live" ? "text-indigo-900/90" : "text-slate-600"
              }`}
            >
              {nowView.detail}
            </p>
            {(liveNow.conflictCount ?? 0) > 1 ? (
              <p className="mt-2 text-xs font-semibold text-amber-800">
                {liveNow.conflictCount} créneaux EDT coïncident — conflit de
                saisie à corriger.
              </p>
            ) : null}
            {e.classe ? (
              <Link
                href={`/edt-classe?classe=${encodeURIComponent(e.classe)}`}
                className="mt-3 inline-block text-xs font-bold text-indigo-700 hover:underline"
              >
                Voir l’emploi du temps de la classe
              </Link>
            ) : null}
          </section>

          {/* Repas semaine */}
          <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm sm:p-6">
            <div className="mb-4 flex flex-wrap items-end justify-between gap-2">
              <div>
                <h2 className="text-xs font-bold uppercase tracking-widest text-slate-400">
                  Restauration — semaine
                </h2>
                <p className="mt-1 text-lg font-bold text-slate-900">{regimeText}</p>
                {synth?.internat.actif && synth.internat.roomLabel ? (
                  <p className="text-xs text-slate-500">Chambre {synth.internat.roomLabel}</p>
                ) : null}
              </div>
              {synth?.restauration.inferred ? (
                <p className="text-[11px] text-slate-400">
                  Jours déduits — cliquez pour figer la grille
                  {synth.restauration.repasParSemaine != null
                    ? ` (${synth.restauration.repasParSemaine} midi/sem.)`
                    : ""}
                </p>
              ) : (
                <p className="text-[11px] text-emerald-700 font-semibold">Grille saisie</p>
              )}
            </div>
            {synth?.restauration ? (
              <div className="overflow-x-auto">
                <table className="w-full min-w-[320px] text-center text-sm">
                  <thead>
                    <tr>
                      <th className="pb-2 text-left text-[10px] font-bold uppercase tracking-wide text-slate-400">
                        Couche
                      </th>
                      {synth.restauration.days.map((d) => (
                        <th
                          key={d.key}
                          className="pb-2 text-[11px] font-bold uppercase text-slate-500"
                        >
                          {d.label}
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {(
                      [
                        { field: "midi" as const, label: "Midi", on: "bg-amber-500", titleOn: "Repas midi" },
                        { field: "soir" as const, label: "Soir", on: "bg-indigo-500", titleOn: "Repas soir" },
                        { field: "etude" as const, label: "Étude", on: "bg-teal-500", titleOn: "Étude" },
                        {
                          field: "garderie" as const,
                          label: "Garderie",
                          on: "bg-rose-400",
                          titleOn: "Garderie",
                        },
                        {
                          field: "sortSeul" as const,
                          label: "Sort seul",
                          on: "bg-slate-700",
                          titleOn: "Sort seul",
                        },
                      ] as const
                    ).map((row) => (
                      <tr key={row.field}>
                        <td className="py-1.5 text-left text-xs font-semibold text-slate-600">
                          {row.label}
                        </td>
                        {synth.restauration.days.map((d) => {
                          const active = Boolean(d[row.field]);
                          const title = active ? row.titleOn : `Pas de ${row.label.toLowerCase()}`;
                          if (!canEdit) {
                            return (
                              <td key={`${d.key}-${row.field}`} className="py-1.5">
                                <span
                                  className={`inline-block h-3.5 w-3.5 rounded-full ${
                                    active ? row.on : "bg-slate-200"
                                  }`}
                                  title={title}
                                />
                              </td>
                            );
                          }
                          return (
                            <td key={`${d.key}-${row.field}`} className="py-1.5">
                              <button
                                type="button"
                                disabled={busy}
                                title={`${title} — cliquer pour basculer`}
                                onClick={() => {
                                  const base: EleveGrilleRepas = grilleFromMealDays(
                                    synth.restauration.days,
                                  );
                                  const next = toggleGrilleCell(
                                    base,
                                    d.key,
                                    row.field as keyof EleveGrilleRepasDay,
                                  );
                                  void postAction({
                                    action: "update_grille_repas",
                                    grilleRepas: next,
                                    scolariteId:
                                      data.scolarites.find((s) => s.statut === "en_cours")?.id ||
                                      data.scolarites[0]?.id,
                                  });
                                }}
                                className={`inline-flex h-7 w-7 items-center justify-center rounded-full transition disabled:opacity-50 ${
                                  active
                                    ? `${row.on} ring-2 ring-offset-1 ring-slate-300`
                                    : "bg-slate-200 hover:bg-slate-300"
                                }`}
                              >
                                <span className="sr-only">{title}</span>
                              </button>
                            </td>
                          );
                        })}
                      </tr>
                    ))}
                  </tbody>
                </table>
                {canEdit ? (
                  <p className="mt-3 text-[11px] text-slate-500">
                    Enregistrement immédiat. La grille alimente Passage (cantine) et la facturation
                    repas.
                  </p>
                ) : null}
              </div>
            ) : (
              <p className="text-sm text-slate-500">Informations restauration indisponibles.</p>
            )}
          </section>

          {/* Notes + absences (facturation uniquement dans l’onglet Finances) */}
          <div className="grid gap-4 md:grid-cols-2">
            <section
              className={`rounded-3xl border p-5 ${
                synth?.notesTrimestre.available
                  ? "border-indigo-100 bg-indigo-50/50"
                  : "border-dashed border-slate-200 bg-slate-50/80"
              }`}
            >
              <h2 className="text-xs font-bold uppercase tracking-widest text-slate-400">
                {synth?.notesTrimestre.label || "Notes — trimestre en cours"}
              </h2>
              <p
                className={`mt-3 text-2xl font-black ${
                  synth?.notesTrimestre.available ? "text-indigo-900" : "text-slate-300"
                }`}
              >
                {synth?.notesTrimestre.value || "—"}
              </p>
              <p className="mt-1 text-sm text-slate-500">
                {synth?.notesTrimestre.detail ||
                  "Moyennes et alertes pédagogiques dès le module Notes."}
              </p>
            </section>
            <section
              className={`rounded-3xl border p-5 ${
                synth?.absences.available && (synth.absences.value !== "0" && synth.absences.value !== "—")
                  ? "border-amber-200 bg-amber-50/80"
                  : synth?.absences.available
                    ? "border-emerald-100 bg-emerald-50/50"
                    : "border-dashed border-slate-200 bg-slate-50/80"
              }`}
            >
              <h2 className="text-xs font-bold uppercase tracking-widest text-slate-400">
                Absences & retards
              </h2>
              <p
                className={`mt-3 text-2xl font-black ${
                  synth?.absences.available && synth.absences.value !== "0"
                    ? "text-amber-900"
                    : synth?.absences.available
                      ? "text-emerald-800"
                      : "text-slate-300"
                }`}
              >
                {synth?.absences.value || "—"}
              </p>
              <p className="mt-1 text-sm text-slate-500">
                {synth?.absences.label || "Résumé vie scolaire"}
                {synth?.absences.detail ? ` · ${synth.absences.detail}` : ""}
              </p>
            </section>
          </div>
        </div>
      ) : null}

      {tab === "scolarite" ? (
        <section className="space-y-4">
          <div className="rounded-3xl border border-slate-200 bg-white p-6 shadow-sm space-y-3">
            <h2 className="text-sm font-bold text-slate-800">Historique scolarité</h2>
            {data.scolarites.length === 0 ? (
              <p className="text-sm text-slate-500">Aucune année enregistrée.</p>
            ) : (
              data.scolarites.map((s) => {
                const siteLabel =
                  data.meta.sites.find((x) => x.siteId === s.siteId)?.label || s.siteId;
                const anneeLabel =
                  data.meta.annees.find((a) => a.id === s.anneeScolaireId)?.label || null;
                return (
                  <div
                    key={s.id}
                    className="rounded-2xl border border-slate-100 bg-slate-50 px-4 py-3 text-sm"
                  >
                    <p className="font-bold text-slate-900">
                      {anneeLabel || "Année ?"}
                      {s.classe ? ` · ${s.classe}` : ""}
                      {siteLabel ? ` · ${siteLabel}` : ""}
                    </p>
                    <p className="text-slate-600">
                      {scolariteStatutLabel(s.statut)}
                      {s.demiPension ? " · Demi-pension" : ""}
                      {s.etablissementPrecedent ? ` · Provenance : ${s.etablissementPrecedent}` : ""}
                    </p>
                  </div>
                );
              })
            )}
          </div>

          <div className="rounded-3xl border border-slate-200 bg-white p-6 shadow-sm space-y-3">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <h2 className="text-sm font-bold text-slate-800">Groupes pédagogiques</h2>
              <Link
                href="/groupes-pedagogiques"
                className="rounded-xl border border-slate-200 px-3 py-1.5 text-xs font-bold text-slate-700 hover:bg-slate-50"
              >
                Gérer les groupes
              </Link>
            </div>
            {(data.groupes || []).length === 0 ? (
              <p className="text-sm text-slate-500">
                Aucun groupe (LV2, options, demi-groupes). Affectation manuelle depuis le module
                Groupes.
              </p>
            ) : (
              <ul className="flex flex-wrap gap-2">
                {(data.groupes || []).map((g) => (
                  <li
                    key={g.id}
                    className="rounded-xl border border-indigo-100 bg-indigo-50 px-3 py-2 text-xs"
                  >
                    <span className="font-mono font-bold text-indigo-900">{g.code}</span>
                    <span className="text-indigo-800 ml-2">{g.libelle}</span>
                    {g.type !== "autre" ? (
                      <span className="text-indigo-600/70 ml-1">({g.type})</span>
                    ) : null}
                  </li>
                ))}
              </ul>
            )}
          </div>
        </section>
      ) : null}

      {tab === "famille" ? (
        <section className="space-y-4">
          {!data.meta.profRestrictedView &&
          data.foyers.length === 0 &&
          (e.parent1Email ||
            e.parent2Email ||
            e.parentEmail ||
            e.parent1Phone ||
            e.parent2Phone ||
            e.parentPhone) ? (
            <div className="rounded-3xl border border-indigo-200 bg-indigo-50/50 p-6 shadow-sm space-y-3">
              <h3 className="font-bold text-slate-900">Coordonnées parents sur la fiche</h3>
              <p className="text-xs text-slate-600">
                Saisies à l’inscription par l’établissement — le foyer se crée manuellement quand vous
                validez la scolarité (pas à l’import Siècle / Rectorat).
              </p>
              <ul className="space-y-2 text-sm text-slate-700">
                {(e.parent1Email || e.parentEmail || e.parent1Phone || e.parentPhone) && (
                  <li className="rounded-xl bg-white px-3 py-2">
                    <span className="font-semibold">Responsable 1</span>
                    <div className="text-xs text-slate-600 mt-1">
                      {e.parent1Email || e.parentEmail || "—"} ·{" "}
                      {e.parent1Phone || e.parentPhone || "—"}
                    </div>
                  </li>
                )}
                {(e.parent2Email || e.parent2Phone) && (
                  <li className="rounded-xl bg-white px-3 py-2">
                    <span className="font-semibold">Responsable 2</span>
                    <div className="text-xs text-slate-600 mt-1">
                      {e.parent2Email || "—"} · {e.parent2Phone || "—"}
                    </div>
                  </li>
                )}
              </ul>
              {canEdit ? (
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => void ensureFoyerFromContacts()}
                  className="rounded-xl bg-indigo-600 px-4 py-2 text-xs font-bold text-white disabled:opacity-50"
                >
                  Créer le foyer à partir de ces coordonnées
                </button>
              ) : null}
            </div>
          ) : null}
          {data.foyers.length === 0 ? (
            <p className="text-sm text-slate-500 rounded-3xl border border-slate-200 bg-white p-6">
              Aucun foyer lié.
              {canEdit
                ? " Créez le foyer manuellement ci-dessous (inscription privée — source de vérité = l’établissement)."
                : ""}
            </p>
          ) : (
            data.foyers.map((f) => (
              <div key={f.id} className="rounded-3xl border border-slate-200 bg-white p-6 shadow-sm">
                <div className="flex flex-wrap items-start justify-between gap-2 mb-3">
                  <div>
                    <h3 className="font-bold text-slate-900">{formatFoyerFacturationLabel(f)}</h3>
                    {f.relation !== "principal" ? (
                      <span className="mt-1 inline-block rounded-full bg-slate-100 px-2 py-0.5 text-[10px] font-bold uppercase text-slate-600">
                        {f.relation}
                      </span>
                    ) : null}
                  </div>
                  {data.sections.includes("facturation") ? (
                    <button
                      type="button"
                      onClick={() => openFoyerFinances(f.id)}
                      className="rounded-xl border border-indigo-200 bg-indigo-50 px-3 py-1.5 text-xs font-bold text-indigo-700 hover:bg-indigo-100"
                    >
                      IBAN · facturation →
                    </button>
                  ) : null}
                </div>
                <p className="text-xs text-slate-600 mb-2">{formatFoyerPayeurDetail(f)}</p>
                {(f.adresse || f.ville) && (
                  <p className="text-xs text-slate-500 mb-3">
                    {[f.adresse, f.codePostal, f.ville].filter(Boolean).join(", ")}
                  </p>
                )}
                <p className="text-xs font-semibold text-slate-500 mb-2">Responsables</p>
                <ul className="space-y-2">
                  {f.responsables.map((r) => (
                    <li key={r.id}>
                      <button
                        type="button"
                        onClick={() => openFoyerFinances(f.id)}
                        className="w-full rounded-xl bg-slate-50 px-3 py-2 text-left text-sm transition hover:bg-indigo-50 hover:ring-1 hover:ring-indigo-100"
                      >
                        <span className="font-semibold text-slate-900">
                          {r.prenom} {r.nom}
                        </span>
                        <span className="text-slate-500">
                          {" "}
                          ·{" "}
                          {responsableRoleTags(r, f.payeurEstFoyer).join(", ") || "contact"}
                        </span>
                        <div className="text-slate-600 text-xs mt-1">
                          {r.email || "—"} · {r.telephone || "—"}
                        </div>
                      </button>
                    </li>
                  ))}
                </ul>
                {canEdit && f.responsables.length < 4 ? (
                  <button
                    type="button"
                    className="mt-3 text-xs font-bold text-indigo-600"
                    onClick={() => {
                      setAddRespFoyerId(f.id);
                      setAddResp({ ...emptyResp });
                    }}
                  >
                    + Ajouter un responsable
                  </button>
                ) : null}
              </div>
            ))
          )}

          {canEdit && addRespFoyerId ? (
            <div className="rounded-3xl border border-indigo-200 bg-indigo-50/40 p-6 space-y-3">
              <h3 className="text-sm font-bold text-slate-900">Nouveau responsable</h3>
              <div className="grid gap-3 sm:grid-cols-2">
                <input
                  placeholder="Nom"
                  value={addResp.nom}
                  onChange={(ev) => setAddResp((r) => ({ ...r, nom: ev.target.value }))}
                  className="rounded-xl border border-slate-200 px-3 py-2 text-sm"
                />
                <input
                  placeholder="Prénom"
                  value={addResp.prenom}
                  onChange={(ev) => setAddResp((r) => ({ ...r, prenom: ev.target.value }))}
                  className="rounded-xl border border-slate-200 px-3 py-2 text-sm"
                />
                <input
                  placeholder="Email"
                  value={addResp.email}
                  onChange={(ev) => setAddResp((r) => ({ ...r, email: ev.target.value }))}
                  className="rounded-xl border border-slate-200 px-3 py-2 text-sm"
                />
                <input
                  placeholder="Téléphone"
                  value={addResp.telephone}
                  onChange={(ev) => setAddResp((r) => ({ ...r, telephone: ev.target.value }))}
                  className="rounded-xl border border-slate-200 px-3 py-2 text-sm"
                />
              </div>
              <div className="flex flex-wrap gap-4 text-sm">
                <label className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={addResp.autoriteParentale}
                    onChange={(ev) =>
                      setAddResp((r) => ({ ...r, autoriteParentale: ev.target.checked }))
                    }
                  />
                  Autorité parentale
                </label>
                <label className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={addResp.contactUrgence}
                    onChange={(ev) =>
                      setAddResp((r) => ({ ...r, contactUrgence: ev.target.checked }))
                    }
                  />
                  Urgence
                </label>
                <label className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={addResp.payeur}
                    onChange={(ev) => setAddResp((r) => ({ ...r, payeur: ev.target.checked }))}
                  />
                  Payeur
                </label>
              </div>
              <div className="flex gap-2">
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => void submitAddResp()}
                  className="rounded-xl bg-slate-900 px-4 py-2 text-xs font-bold text-white"
                >
                  Enregistrer
                </button>
                <button
                  type="button"
                  onClick={() => setAddRespFoyerId(null)}
                  className="rounded-xl border border-slate-200 px-4 py-2 text-xs font-bold"
                >
                  Annuler
                </button>
              </div>
            </div>
          ) : null}

          {canEdit ? (
            <div className="rounded-3xl border border-slate-200 bg-white p-6 shadow-sm space-y-3">
              <h2 className="text-sm font-bold text-slate-800">Créer un foyer</h2>
              <div className="grid gap-3 sm:grid-cols-2">
                <input
                  placeholder="Libellé foyer"
                  value={foyerForm.label}
                  onChange={(ev) => setFoyerForm((f) => ({ ...f, label: ev.target.value }))}
                  className="rounded-xl border border-slate-200 px-3 py-2 text-sm sm:col-span-2"
                />
                <input
                  placeholder="Adresse"
                  value={foyerForm.adresse}
                  onChange={(ev) => setFoyerForm((f) => ({ ...f, adresse: ev.target.value }))}
                  className="rounded-xl border border-slate-200 px-3 py-2 text-sm sm:col-span-2"
                />
                <input
                  placeholder="Code postal"
                  value={foyerForm.codePostal}
                  onChange={(ev) => setFoyerForm((f) => ({ ...f, codePostal: ev.target.value }))}
                  className="rounded-xl border border-slate-200 px-3 py-2 text-sm"
                />
                <input
                  placeholder="Ville"
                  value={foyerForm.ville}
                  onChange={(ev) => setFoyerForm((f) => ({ ...f, ville: ev.target.value }))}
                  className="rounded-xl border border-slate-200 px-3 py-2 text-sm"
                />
                <input
                  placeholder="Nom responsable"
                  value={foyerForm.responsable.nom}
                  onChange={(ev) =>
                    setFoyerForm((f) => ({
                      ...f,
                      responsable: { ...f.responsable, nom: ev.target.value },
                    }))
                  }
                  className="rounded-xl border border-slate-200 px-3 py-2 text-sm"
                />
                <input
                  placeholder="Prénom responsable"
                  value={foyerForm.responsable.prenom}
                  onChange={(ev) =>
                    setFoyerForm((f) => ({
                      ...f,
                      responsable: { ...f.responsable, prenom: ev.target.value },
                    }))
                  }
                  className="rounded-xl border border-slate-200 px-3 py-2 text-sm"
                />
                <input
                  placeholder="Email"
                  value={foyerForm.responsable.email}
                  onChange={(ev) =>
                    setFoyerForm((f) => ({
                      ...f,
                      responsable: { ...f.responsable, email: ev.target.value },
                    }))
                  }
                  className="rounded-xl border border-slate-200 px-3 py-2 text-sm"
                />
                <input
                  placeholder="Téléphone"
                  value={foyerForm.responsable.telephone}
                  onChange={(ev) =>
                    setFoyerForm((f) => ({
                      ...f,
                      responsable: { ...f.responsable, telephone: ev.target.value },
                    }))
                  }
                  className="rounded-xl border border-slate-200 px-3 py-2 text-sm"
                />
              </div>
              <div className="flex flex-wrap gap-4 text-sm">
                <label className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={foyerForm.payeurEstFoyer}
                    onChange={(ev) =>
                      setFoyerForm((f) => ({ ...f, payeurEstFoyer: ev.target.checked }))
                    }
                  />
                  Payeur = foyer
                </label>
                <label className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={foyerForm.responsable.autoriteParentale}
                    onChange={(ev) =>
                      setFoyerForm((f) => ({
                        ...f,
                        responsable: { ...f.responsable, autoriteParentale: ev.target.checked },
                      }))
                    }
                  />
                  Autorité parentale
                </label>
                <label className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={foyerForm.responsable.contactUrgence}
                    onChange={(ev) =>
                      setFoyerForm((f) => ({
                        ...f,
                        responsable: { ...f.responsable, contactUrgence: ev.target.checked },
                      }))
                    }
                  />
                  Urgence
                </label>
              </div>
              <button
                type="button"
                disabled={busy}
                onClick={() => void createFoyer()}
                className="rounded-xl bg-slate-900 px-4 py-2 text-xs font-bold text-white disabled:opacity-50"
              >
                Créer le foyer
              </button>
            </div>
          ) : null}
        </section>
      ) : null}

      {tab === "finances" ? (
        <EleveFinancesPanel
          eleveId={id}
          canEdit={Boolean(data.meta.canEditStructure)}
          focusFoyerId={focusFoyerId}
        />
      ) : null}

      {tab === "notes" ? (
        <section className="space-y-4">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <h2 className="text-sm font-bold text-slate-900">Moyennes par matière</h2>
              <p className="text-xs text-slate-500">
                Données issues du module Notes — périodes et devoirs en cours.
              </p>
            </div>
            <div className="flex flex-wrap gap-2">
              {Array.from(
                new Map(
                  (data.notes || [])
                    .filter((n) => n.periodeId)
                    .map((n) => [n.periodeId, n.periodeLibelle || "Bulletin"]),
                ).entries(),
              ).map(([periodeId, label]) => (
                <a
                  key={periodeId}
                  href={`/api/notes/bulletins/pdf?eleveId=${id}&periodeId=${periodeId}`}
                  target="_blank"
                  rel="noreferrer"
                  className="rounded-xl border border-indigo-200 bg-indigo-50 px-3 py-2 text-xs font-bold text-indigo-700 hover:bg-indigo-100"
                >
                  PDF {label}
                </a>
              ))}
              <Link
                href="/notes/saisie"
                className="rounded-xl border border-slate-200 bg-white px-3 py-2 text-xs font-bold text-slate-700 hover:bg-slate-50"
              >
                Ouvrir la saisie
              </Link>
              <Link
                href="/notes/competences"
                className="rounded-xl border border-slate-200 bg-white px-3 py-2 text-xs font-bold text-slate-700 hover:bg-slate-50"
              >
                Compétences LSU
              </Link>
            </div>
          </div>
          {(data.notes || []).length === 0 ? (
            <div className="rounded-3xl border border-dashed border-slate-200 bg-slate-50/60 p-8 text-center">
              <p className="text-sm font-semibold text-slate-700">Aucune moyenne enregistrée</p>
              <p className="mt-1 text-xs text-slate-500">
                Les moyennes apparaîtront dès qu&apos;un devoir sera saisi pour cet élève.
              </p>
            </div>
          ) : (
            <div className="overflow-hidden rounded-3xl border border-slate-200 bg-white">
              <table className="w-full text-left text-sm">
                <thead className="bg-slate-50 text-xs font-bold uppercase tracking-wide text-slate-500">
                  <tr>
                    <th className="px-4 py-3">Période</th>
                    <th className="px-4 py-3">Matière</th>
                    <th className="px-4 py-3 text-right">Moyenne</th>
                    <th className="px-4 py-3 text-right">Notes</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-100">
                  {(data.notes || []).map((n, idx) => (
                    <tr key={`${n.matiereLibelle}-${n.periodeLibelle || idx}`}>
                      <td className="px-4 py-3 text-slate-600">{n.periodeLibelle || "—"}</td>
                      <td className="px-4 py-3 font-semibold text-slate-900">{n.matiereLibelle}</td>
                      <td className="px-4 py-3 text-right font-bold tabular-nums text-slate-900">
                        {n.moyenne ?? "—"}
                      </td>
                      <td className="px-4 py-3 text-right tabular-nums text-slate-500">{n.nbNotes}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
          {(data.competences || []).length > 0 ? (
            <div className="overflow-hidden rounded-3xl border border-slate-200 bg-white">
              <h3 className="px-4 pt-4 text-sm font-bold text-slate-900">Compétences (LSU)</h3>
              <table className="w-full text-left text-sm">
                <thead className="bg-slate-50 text-xs font-bold uppercase tracking-wide text-slate-500">
                  <tr>
                    <th className="px-4 py-3">Domaine</th>
                    <th className="px-4 py-3">Item</th>
                    <th className="px-4 py-3 text-right">Maîtrise</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-100">
                  {(data.competences || []).map((c, idx) => (
                    <tr key={`${c.itemLibelle}-${idx}`}>
                      <td className="px-4 py-3 text-slate-600">{c.domaineLibelle}</td>
                      <td className="px-4 py-3 font-semibold text-slate-900">{c.itemLibelle}</td>
                      <td className="px-4 py-3 text-right tabular-nums text-slate-900">
                        {c.niveau ? `${c.niveau} — ${c.niveauLabel}` : "—"}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : null}
        </section>
      ) : null}

      {tab === "vie_scolaire" ? (
        <section className="space-y-6">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <h2 className="text-sm font-bold text-slate-900">Absences & retards</h2>
              <p className="text-xs text-slate-500">Issus des appels de classe — suivi CPE.</p>
            </div>
            <div className="flex flex-wrap gap-2">
              <Link
                href="/vie-scolaire/presence?tab=absences"
                className="rounded-xl border border-slate-200 bg-white px-3 py-2 text-xs font-bold text-slate-700 hover:bg-slate-50"
              >
                Suivi absences
              </Link>
              <Link
                href="/vie-scolaire/sanctions"
                className="rounded-xl border border-slate-200 bg-white px-3 py-2 text-xs font-bold text-slate-700 hover:bg-slate-50"
              >
                Sanctions
              </Link>
              <Link
                href="/vie-scolaire/carnet"
                className="rounded-xl border border-slate-200 bg-white px-3 py-2 text-xs font-bold text-slate-700 hover:bg-slate-50"
              >
                Carnet
              </Link>
            </div>
          </div>

          {(data.absences || []).length === 0 ? (
            <div className="rounded-3xl border border-dashed border-slate-200 bg-slate-50/60 p-6 text-center">
              <p className="text-sm font-semibold text-slate-700">Aucune absence enregistrée</p>
            </div>
          ) : (
            <div className="overflow-hidden rounded-3xl border border-slate-200 bg-white">
              <table className="w-full text-left text-sm">
                <thead className="bg-slate-50 text-xs font-bold uppercase tracking-wide text-slate-500">
                  <tr>
                    <th className="px-4 py-3">Date</th>
                    <th className="px-4 py-3">Type</th>
                    <th className="px-4 py-3">Statut</th>
                    <th className="px-4 py-3">Motif</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-100">
                  {(data.absences || []).map((a) => (
                    <tr key={a.id}>
                      <td className="px-4 py-3 text-slate-700">
                        {a.dateDebut ? new Date(a.dateDebut).toLocaleDateString("fr-FR") : "—"}
                      </td>
                      <td className="px-4 py-3 font-semibold text-slate-900">{a.type}</td>
                      <td className="px-4 py-3">
                        <span
                          className={`rounded-full px-2 py-0.5 text-xs font-bold ${
                            a.statut === "a_traiter"
                              ? "bg-amber-50 text-amber-900"
                              : a.justifie
                                ? "bg-emerald-50 text-emerald-800"
                                : "bg-slate-100 text-slate-700"
                          }`}
                        >
                          {a.statut}
                        </span>
                      </td>
                      <td className="px-4 py-3 text-slate-600">{a.motif || "—"}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}

          <div>
            <h2 className="text-sm font-bold text-slate-900 mb-3">Sanctions actives</h2>
            {(data.sanctions || []).length === 0 ? (
              <div className="rounded-3xl border border-dashed border-slate-200 bg-slate-50/60 p-6 text-center">
                <p className="text-sm font-semibold text-slate-700">Aucune sanction active</p>
              </div>
            ) : (
              <ul className="space-y-2">
                {(data.sanctions || []).map((s) => (
                  <li
                    key={s.id}
                    className="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm"
                  >
                    <p className="font-bold text-slate-900">
                      {s.typeLibelle}
                      <span className="ml-2 font-normal text-slate-500">
                        {s.dateSanction
                          ? new Date(s.dateSanction).toLocaleDateString("fr-FR")
                          : ""}
                      </span>
                    </p>
                    {s.motif ? <p className="text-xs text-slate-600 mt-1">{s.motif}</p> : null}
                    {s.createdByNom ? (
                      <p className="text-xs text-slate-400 mt-1">par {s.createdByNom}</p>
                    ) : null}
                  </li>
                ))}
              </ul>
            )}
          </div>

          <div>
            <h2 className="text-sm font-bold text-slate-900 mb-3">Carnet de correspondance</h2>
            {(data.carnet || []).length === 0 ? (
              <div className="rounded-3xl border border-dashed border-slate-200 bg-slate-50/60 p-6 text-center">
                <p className="text-sm font-semibold text-slate-700">Aucune entrée carnet</p>
              </div>
            ) : (
              <ul className="space-y-2">
                {(data.carnet || []).map((c) => (
                  <li
                    key={c.id}
                    className="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm"
                  >
                    <div className="flex flex-wrap items-start justify-between gap-2">
                      <p className="font-bold text-slate-900">
                        {c.titre}
                        <span className="ml-2 font-normal text-slate-500">
                          {c.dateEntree ? new Date(c.dateEntree).toLocaleDateString("fr-FR") : ""}
                        </span>
                      </p>
                      <span
                        className={`rounded-full px-2 py-0.5 text-[11px] font-bold ${
                          c.signeAt
                            ? "bg-emerald-50 text-emerald-800"
                            : "bg-amber-50 text-amber-900"
                        }`}
                      >
                        {c.signeAt ? "Signé" : "En attente"}
                      </span>
                    </div>
                    <p className="text-xs text-slate-600 mt-1 whitespace-pre-wrap">{c.corps}</p>
                    {c.createdByNom ? (
                      <p className="text-xs text-slate-400 mt-1">par {c.createdByNom}</p>
                    ) : null}
                  </li>
                ))}
              </ul>
            )}
          </div>
        </section>
      ) : null}

      {tab === "documents" ? (
        <section className="space-y-4">
          {data.pendingAccessRequests.length > 0 ? (
            <div className="rounded-3xl border border-amber-200 bg-amber-50/60 p-6 space-y-3">
              <h2 className="text-sm font-bold text-amber-900">Demandes d’accès en attente</h2>
              <ul className="space-y-2">
                {data.pendingAccessRequests.map((r) => (
                  <li
                    key={r.id}
                    className="flex flex-wrap items-center justify-between gap-2 rounded-xl bg-white px-3 py-2 text-sm border border-amber-100"
                  >
                    <div>
                      <p className="font-semibold">{r.docTitle}</p>
                      <p className="text-xs text-slate-500">
                        {r.durationDays} j · {r.note || "sans motif"}
                      </p>
                    </div>
                    <div className="flex gap-2">
                      <button
                        type="button"
                        disabled={busy}
                        onClick={() => void decideAccess(r.id, "approved")}
                        className="rounded-lg bg-emerald-700 px-3 py-1.5 text-xs font-bold text-white"
                      >
                        Approuver
                      </button>
                      <button
                        type="button"
                        disabled={busy}
                        onClick={() => void decideAccess(r.id, "rejected")}
                        className="rounded-lg border border-slate-200 px-3 py-1.5 text-xs font-bold"
                      >
                        Refuser
                      </button>
                    </div>
                  </li>
                ))}
              </ul>
            </div>
          ) : null}

          <div className="rounded-3xl border border-slate-200 bg-white p-6 shadow-sm space-y-4">
            <div className="flex flex-wrap items-end justify-between gap-3">
              <h2 className="text-sm font-bold text-slate-800">Documents du dossier</h2>
              <p className="text-xs text-slate-500">
                Classés automatiquement : administratif, financier / comptable, santé.
              </p>
            </div>

            {allowedDocCategories.length > 0 ? (
              <div className="flex flex-wrap gap-2 border-b border-slate-100 pb-3">
                {allowedDocCategories.length > 1 ? (
                  <button
                    type="button"
                    onClick={() => setDocCategory("tous")}
                    className={`rounded-lg px-3 py-1.5 text-xs font-bold transition ${
                      docCategory === "tous"
                        ? "bg-slate-900 text-white"
                        : "bg-slate-100 text-slate-600 hover:bg-slate-200"
                    }`}
                  >
                    Tous
                  </button>
                ) : null}
                {allowedDocCategories.map((cat) => (
                  <button
                    key={cat}
                    type="button"
                    onClick={() => setDocCategory(cat)}
                    className={`rounded-lg px-3 py-1.5 text-xs font-bold transition ${
                      docCategory === cat
                        ? "bg-slate-900 text-white"
                        : "bg-slate-100 text-slate-600 hover:bg-slate-200"
                    }`}
                  >
                    {DOC_CATEGORIE_LABELS[cat]}
                  </button>
                ))}
              </div>
            ) : null}

            <div className="grid gap-3 sm:grid-cols-3">
              <label className="text-xs font-semibold text-slate-600">
                Tiroir
                <select
                  value={uploadMeta.tiroir}
                  onChange={(ev) => setUploadMeta((m) => ({ ...m, tiroir: ev.target.value }))}
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                >
                  {uploadTiroirsForCategory.map((t) => (
                    <option key={t} value={t}>
                      {TIROIR_LABELS[t] || t}
                      {TIROIR_TO_CATEGORIE[t as keyof typeof TIROIR_TO_CATEGORIE]
                        ? ` · ${DOC_CATEGORIE_LABELS[TIROIR_TO_CATEGORIE[t as keyof typeof TIROIR_TO_CATEGORIE]]}`
                        : ""}
                    </option>
                  ))}
                </select>
              </label>
              <label className="text-xs font-semibold text-slate-600">
                Confidentialité
                <select
                  value={uploadMeta.confidentialite}
                  onChange={(ev) =>
                    setUploadMeta((m) => ({ ...m, confidentialite: ev.target.value }))
                  }
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                >
                  <option value="standard">Standard</option>
                  {!data.meta.profRestrictedView ? (
                    <>
                      <option value="restreint">Restreint</option>
                      {allowedDocCategories.includes("sante") ? (
                        <option value="sante">Santé</option>
                      ) : null}
                    </>
                  ) : null}
                </select>
              </label>
              <label className="text-xs font-semibold text-slate-600">
                Titre (optionnel)
                <input
                  value={uploadMeta.title}
                  onChange={(ev) => setUploadMeta((m) => ({ ...m, title: ev.target.value }))}
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                  placeholder="Sinon = nom fichier"
                />
              </label>
            </div>

            <div
              onDragOver={(ev) => {
                ev.preventDefault();
                setDragOver(true);
              }}
              onDragLeave={() => setDragOver(false)}
              onDrop={onDrop}
              className={`rounded-2xl border-2 border-dashed px-4 py-10 text-center transition ${
                dragOver
                  ? "border-indigo-400 bg-indigo-50/50"
                  : "border-slate-200 bg-slate-50/50"
              }`}
            >
              <IconUpload className="mx-auto mb-2 h-8 w-8 text-slate-400" />
              <p className="text-sm font-semibold text-slate-700">
                Glisser-déposer des fichiers ici
              </p>
              <p className="mt-1 text-xs text-slate-500">PDF, images… max 25 Mo</p>
              <label className="mt-4 inline-flex cursor-pointer rounded-xl border border-slate-200 bg-white px-4 py-2 text-xs font-bold text-slate-700">
                Choisir des fichiers
                <input
                  type="file"
                  multiple
                  className="hidden"
                  onChange={(ev) => {
                    if (ev.target.files) void uploadFiles(ev.target.files);
                    ev.target.value = "";
                  }}
                />
              </label>
            </div>

            {filteredDocuments.length === 0 ? (
              <p className="text-sm text-slate-500">Aucun document dans cette catégorie.</p>
            ) : (
              <ul className="grid gap-3 sm:grid-cols-2">
                {filteredDocuments.map((d) => {
                  const isImage =
                    d.canOpen &&
                    Boolean(d.fileUrl) &&
                    (d.mimeType?.startsWith("image/") ||
                      /\.(jpe?g|png|gif|webp)$/i.test(d.fileUrl || ""));
                  const cat = TIROIR_TO_CATEGORIE[d.tiroir as keyof typeof TIROIR_TO_CATEGORIE];
                  return (
                    <li
                      key={d.id}
                      className="flex gap-3 rounded-2xl border border-slate-100 bg-white p-3 text-sm"
                    >
                      <div className="flex h-16 w-16 shrink-0 items-center justify-center overflow-hidden rounded-xl bg-slate-100">
                        {isImage && d.fileUrl ? (
                          <img
                            src={d.fileUrl}
                            alt=""
                            className="h-full w-full object-cover"
                          />
                        ) : d.canOpen ? (
                          <IconFile className="h-7 w-7 text-slate-400" />
                        ) : (
                          <IconLock className="h-7 w-7 text-amber-500" />
                        )}
                      </div>
                      <div className="min-w-0 flex-1">
                        <p className="font-semibold text-slate-900 truncate">{d.title}</p>
                        <p className="text-xs text-slate-500">
                          {cat ? `${DOC_CATEGORIE_LABELS[cat]} · ` : ""}
                          {TIROIR_LABELS[d.tiroir] || d.tiroir} · {d.confidentialite}
                          {d.anneeLabel ? ` · ${d.anneeLabel}` : ""}
                        </p>
                        <div className="mt-2">
                          {d.canOpen ? (
                            d.fileUrl ? (
                              <a
                                href={d.fileUrl}
                                target="_blank"
                                rel="noreferrer"
                                className="text-xs font-bold text-emerald-700 hover:underline"
                              >
                                Ouvrir
                              </a>
                            ) : (
                              <span className="text-xs font-bold text-emerald-700">Accessible</span>
                            )
                          ) : (
                            <button
                              type="button"
                              className="text-xs font-bold text-amber-700 hover:underline"
                              onClick={() =>
                                setAccessForm({
                                  documentId: d.id,
                                  durationDays: 1,
                                  note: "",
                                })
                              }
                            >
                              Présent — demander l’accès
                            </button>
                          )}
                        </div>
                      </div>
                    </li>
                  );
                })}
              </ul>
            )}
          </div>

          {accessForm ? (
            <div className="rounded-3xl border border-amber-200 bg-white p-6 space-y-3 shadow-sm">
              <h3 className="text-sm font-bold text-slate-900">Demande d’accès direction</h3>
              <label className="block text-xs font-semibold text-slate-600">
                Durée (1–7 jours)
                <select
                  value={accessForm.durationDays}
                  onChange={(ev) =>
                    setAccessForm((f) =>
                      f ? { ...f, durationDays: Number(ev.target.value) } : f,
                    )
                  }
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                >
                  {[1, 2, 3, 4, 5, 6, 7].map((n) => (
                    <option key={n} value={n}>
                      {n} jour{n > 1 ? "s" : ""}
                    </option>
                  ))}
                </select>
              </label>
              <label className="block text-xs font-semibold text-slate-600">
                Motif
                <textarea
                  value={accessForm.note}
                  onChange={(ev) =>
                    setAccessForm((f) => (f ? { ...f, note: ev.target.value } : f))
                  }
                  rows={2}
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                />
              </label>
              <div className="flex gap-2">
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => void requestAccess()}
                  className="rounded-xl bg-slate-900 px-4 py-2 text-xs font-bold text-white"
                >
                  Envoyer
                </button>
                <button
                  type="button"
                  onClick={() => setAccessForm(null)}
                  className="rounded-xl border border-slate-200 px-4 py-2 text-xs font-bold"
                >
                  Annuler
                </button>
              </div>
            </div>
          ) : null}
        </section>
      ) : null}
        </div>
      </div>
    </ModulePageShell>
  );
}
