"use client";

import { useCallback, useEffect, useMemo, useState, type DragEvent } from "react";
import Link from "next/link";
import { useParams } from "next/navigation";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";

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
  classmates: Array<{ id: string; nom: string; prenom: string }>;
  meta: {
    sites: Array<{ siteId: string; label: string; kind: string | null }>;
    annees: Array<{ id: string; label: string; isCurrent: boolean }>;
    canEditStructure: boolean;
    canDecideAccess: boolean;
    profRestrictedView?: boolean;
    tiroirs: string[];
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
  stubs: {
    notes: { message: string } | null;
    vieScolaire: { message: string } | null;
  };
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
  };
};

type TabId = "synthese" | "famille" | "documents" | "scolarite" | "notes" | "sante" | "facturation";

const TIROIR_LABELS: Record<string, string> = {
  scolaire: "Scolaire",
  inscription: "Inscription",
  facturation: "Facturation",
  voyages: "Voyages",
  sante: "Santé",
  vie_scolaire: "Vie scolaire",
};

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
  const id = String(params.id || "");
  const [data, setData] = useState<DossierPayload | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [tab, setTab] = useState<TabId>("synthese");
  const [busy, setBusy] = useState(false);
  const [dragOver, setDragOver] = useState(false);

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

  const [scolForm, setScolForm] = useState({
    siteId: "",
    classe: "",
    anneeScolaireId: "",
    statut: "en_cours",
    demiPension: false,
    etablissementPrecedent: "",
  });

  const [uploadMeta, setUploadMeta] = useState({
    tiroir: "scolaire",
    confidentialite: "standard",
    title: "",
  });
  const [accessForm, setAccessForm] = useState<{
    documentId: string;
    durationDays: number;
    note: string;
  } | null>(null);

  const load = useCallback(async () => {
    setError(null);
    const res = await fetch(`/api/eleves/${id}/dossier`);
    if (!res.ok) {
      const j = (await res.json().catch(() => ({}))) as { error?: string };
      setError(j.error || `Erreur ${res.status}`);
      setData(null);
      return;
    }
    setData((await res.json()) as DossierPayload);
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
      { id: "documents", label: "Documents", show: s.has("documents") },
      { id: "notes", label: "Notes", show: s.has("notes") },
      { id: "sante", label: "Santé", show: s.has("sante") },
      { id: "facturation", label: "Facturation", show: s.has("facturation") },
    ];
    return list.filter((t) => t.show);
  }, [data]);

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

  async function mockBulletin() {
    const ok = await postAction({ action: "mock_bulletin", anneeLabel: "2025-2026" });
    if (ok) setTab("documents");
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

  async function createScolarite() {
    await postAction({
      action: "create_scolarite",
      siteId: scolForm.siteId || null,
      classe: scolForm.classe || null,
      anneeScolaireId: scolForm.anneeScolaireId || null,
      statut: scolForm.statut,
      demiPension: scolForm.demiPension,
      etablissementPrecedent: scolForm.etablissementPrecedent || null,
      closePrevious: true,
      updateEleveClasse: true,
    });
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
        <Link href="/eleves/dossiers" className="text-sm font-semibold text-indigo-600">
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
  const canEdit = data.meta.canEditStructure;

  return (
    <ModulePageShell maxWidthClass="max-w-5xl">
      <ModulePageHeader
        eyebrow="Dossier élève"
        title={`${e.prenom} ${e.nom}`}
        description={
          <span>
            {e.classe || "Classe à préciser"}
            {e.secteur ? ` · ${e.secteur}` : ""} · {e.status}
          </span>
        }
        actions={
          <Link href="/eleves/dossiers" className="text-sm font-bold text-indigo-600 hover:underline">
            Liste des dossiers
          </Link>
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

      {tab === "synthese" ? (
        <div className="grid gap-4 md:grid-cols-2">
          <section className="rounded-3xl border border-slate-200 bg-white p-6 shadow-sm">
            <h2 className="text-xs font-bold uppercase tracking-widest text-slate-400 mb-3">
              Identité
            </h2>
            <dl className="space-y-2 text-sm">
              <div className="flex justify-between gap-4">
                <dt className="text-slate-500">Né(e) le</dt>
                <dd className="font-semibold text-slate-900">{e.dateNaissance || "—"}</dd>
              </div>
              {!data.meta.profRestrictedView ? (
                <div className="flex justify-between gap-4">
                  <dt className="text-slate-500">Lieu</dt>
                  <dd className="font-semibold text-slate-900">{e.lieuNaissance || "—"}</dd>
                </div>
              ) : null}
              <div className="flex justify-between gap-4">
                <dt className="text-slate-500">Classe</dt>
                <dd className="font-semibold text-slate-900">
                  {e.classe || "—"}
                  {e.classe ? (
                    <Link
                      href={`/edt-classe?classe=${encodeURIComponent(e.classe)}`}
                      className="ml-2 text-xs font-bold text-indigo-600 hover:underline"
                    >
                      EDT classe
                    </Link>
                  ) : null}
                </dd>
              </div>
            </dl>
            {data.meta.profRestrictedView ? (
              <p className="mt-4 text-xs text-slate-500">
                Coordonnées élève et famille masquées (accès pédagogique).
              </p>
            ) : null}
          </section>
          <section className="rounded-3xl border border-slate-200 bg-white p-6 shadow-sm">
            <h2 className="text-xs font-bold uppercase tracking-widest text-slate-400 mb-3">
              Maintenant
            </h2>
            {data.enCoursMaintenant.activity ? (
              <div className="rounded-2xl border border-indigo-100 bg-indigo-50/70 px-4 py-3">
                <p className="text-[10px] font-black uppercase tracking-wide text-indigo-600">
                  En cours
                </p>
                <p className="mt-1 text-lg font-bold text-indigo-950">
                  {data.enCoursMaintenant.activity.subject}
                </p>
                <p className="text-sm text-indigo-900/90">
                  {data.enCoursMaintenant.activity.start}–{data.enCoursMaintenant.activity.end}
                  {data.enCoursMaintenant.activity.room
                    ? ` · ${data.enCoursMaintenant.activity.room}`
                    : ""}
                </p>
                <p className="mt-1 text-xs text-indigo-800/80">
                  {data.enCoursMaintenant.activity.teacherName}
                  {data.enCoursMaintenant.activity.kind === "remplacement"
                    ? " · remplacement"
                    : data.enCoursMaintenant.activity.weekType
                      ? ` · semaine ${data.enCoursMaintenant.activity.weekType}`
                      : ""}
                </p>
              </div>
            ) : (
              <p className="text-sm text-slate-600">
                {data.enCoursMaintenant.reason === "vacances" ||
                data.enCoursMaintenant.reason === "weekend" ||
                data.enCoursMaintenant.reason === "ferie"
                  ? data.enCoursMaintenant.label || "Pas de cours (calendrier)."
                  : data.enCoursMaintenant.reason === "pas_de_classe"
                    ? "Classe non renseignée — impossible de déduire l’emploi du temps."
                    : data.enCoursMaintenant.reason === "pas_edt"
                      ? "Aucun EDT prof renseigné pour cette classe."
                      : "Pas de cours en ce moment."}
              </p>
            )}
            {data.scolarites[0] ? (
              <p className="mt-3 text-sm">
                Demi-pension :{" "}
                <strong>{data.scolarites[0].demiPension ? "oui" : "non"}</strong>
              </p>
            ) : null}
          </section>
          {data.classmates.length > 0 ? (
            <section className="md:col-span-2 rounded-3xl border border-slate-200 bg-white p-6 shadow-sm">
              <h2 className="text-xs font-bold uppercase tracking-widest text-slate-400 mb-3">
                Classe — camarades
              </h2>
              <ul className="flex flex-wrap gap-2">
                {data.classmates.slice(0, 24).map((c) => (
                  <li key={c.id}>
                    <Link
                      href={`/eleves/dossier/${c.id}`}
                      className="inline-flex rounded-lg border border-slate-200 px-3 py-1.5 text-sm font-medium text-slate-700 hover:border-indigo-300"
                    >
                      {c.prenom} {c.nom}
                    </Link>
                  </li>
                ))}
              </ul>
            </section>
          ) : null}
        </div>
      ) : null}

      {tab === "scolarite" ? (
        <section className="space-y-4">
          <div className="rounded-3xl border border-slate-200 bg-white p-6 shadow-sm space-y-3">
            <h2 className="text-sm font-bold text-slate-800">Historique (même élève)</h2>
            {data.scolarites.length === 0 ? (
              <p className="text-sm text-slate-500">Aucune scolarité enregistrée.</p>
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
                      {s.classe || "Classe ?"}
                      {siteLabel ? ` · ${siteLabel}` : ""}
                      {anneeLabel ? ` · ${anneeLabel}` : ""}
                    </p>
                    <p className="text-slate-600">
                      Statut {s.statut}
                      {s.etablissementPrecedent ? ` · préc. ${s.etablissementPrecedent}` : ""}
                    </p>
                  </div>
                );
              })
            )}
          </div>

          {canEdit ? (
            <div className="rounded-3xl border border-slate-200 bg-white p-6 shadow-sm space-y-3">
              <h2 className="text-sm font-bold text-slate-800">
                Continuité — nouvelle scolarité (CM2→6e, 3e→2nde…)
              </h2>
              <p className="text-xs text-slate-500">
                Conserve le même dossier. La scolarité « en cours » précédente passe en terminée.
              </p>
              <div className="grid gap-3 sm:grid-cols-2">
                <label className="text-xs font-semibold text-slate-600">
                  Site
                  <select
                    value={scolForm.siteId}
                    onChange={(ev) => setScolForm((f) => ({ ...f, siteId: ev.target.value }))}
                    className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                  >
                    <option value="">—</option>
                    {data.meta.sites.map((s) => (
                      <option key={s.siteId} value={s.siteId}>
                        {s.label}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="text-xs font-semibold text-slate-600">
                  Année
                  <select
                    value={scolForm.anneeScolaireId}
                    onChange={(ev) =>
                      setScolForm((f) => ({ ...f, anneeScolaireId: ev.target.value }))
                    }
                    className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                  >
                    <option value="">—</option>
                    {data.meta.annees.map((a) => (
                      <option key={a.id} value={a.id}>
                        {a.label}
                        {a.isCurrent ? " (courante)" : ""}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="text-xs font-semibold text-slate-600">
                  Classe
                  <input
                    value={scolForm.classe}
                    onChange={(ev) => setScolForm((f) => ({ ...f, classe: ev.target.value }))}
                    className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                    placeholder="ex. 6A"
                  />
                </label>
                <label className="text-xs font-semibold text-slate-600">
                  Statut
                  <select
                    value={scolForm.statut}
                    onChange={(ev) => setScolForm((f) => ({ ...f, statut: ev.target.value }))}
                    className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                  >
                    <option value="en_cours">En cours</option>
                    <option value="prevue">Prévue</option>
                    <option value="terminee">Terminée</option>
                  </select>
                </label>
                <label className="text-xs font-semibold text-slate-600 sm:col-span-2">
                  Établissement précédent (hors groupe)
                  <input
                    value={scolForm.etablissementPrecedent}
                    onChange={(ev) =>
                      setScolForm((f) => ({ ...f, etablissementPrecedent: ev.target.value }))
                    }
                    className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                  />
                </label>
                <label className="flex items-center gap-2 text-sm text-slate-700">
                  <input
                    type="checkbox"
                    checked={scolForm.demiPension}
                    onChange={(ev) =>
                      setScolForm((f) => ({ ...f, demiPension: ev.target.checked }))
                    }
                  />
                  Demi-pension
                </label>
              </div>
              <button
                type="button"
                disabled={busy}
                onClick={() => void createScolarite()}
                className="rounded-xl bg-slate-900 px-4 py-2 text-xs font-bold text-white disabled:opacity-50"
              >
                Enregistrer la scolarité
              </button>
            </div>
          ) : null}
        </section>
      ) : null}

      {tab === "famille" ? (
        <section className="space-y-4">
          {data.foyers.length === 0 ? (
            <p className="text-sm text-slate-500 rounded-3xl border border-slate-200 bg-white p-6">
              Aucun foyer lié.
            </p>
          ) : (
            data.foyers.map((f) => (
              <div key={f.id} className="rounded-3xl border border-slate-200 bg-white p-6 shadow-sm">
                <h3 className="font-bold text-slate-900 mb-1">
                  {f.label}{" "}
                  <span className="text-xs font-medium text-slate-400">({f.relation})</span>
                </h3>
                {(f.adresse || f.ville) && (
                  <p className="text-xs text-slate-500 mb-2">
                    {[f.adresse, f.codePostal, f.ville].filter(Boolean).join(", ")}
                  </p>
                )}
                <p className="text-xs text-slate-500 mb-3">
                  Payeur : {f.payeurEstFoyer ? "foyer" : "responsable désigné"} ·{" "}
                  {f.responsables.length}/4 responsables
                </p>
                <ul className="space-y-2">
                  {f.responsables.map((r) => (
                    <li key={r.id} className="rounded-xl bg-slate-50 px-3 py-2 text-sm">
                      <span className="font-semibold">
                        {r.prenom} {r.nom}
                      </span>
                      <span className="text-slate-500">
                        {" "}
                        ·{" "}
                        {[
                          r.autoriteParentale ? "autorité" : null,
                          r.payeur ? "payeur" : null,
                          r.contactUrgence ? "urgence" : null,
                        ]
                          .filter(Boolean)
                          .join(", ") || "contact"}
                      </span>
                      <div className="text-slate-600 text-xs mt-1">
                        {r.email || "—"} · {r.telephone || "—"}
                      </div>
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
            <div className="flex flex-wrap items-center justify-between gap-3">
              <h2 className="text-sm font-bold text-slate-800">Documents du dossier</h2>
              <button
                type="button"
                disabled={busy}
                onClick={() => void mockBulletin()}
                className="rounded-xl bg-slate-900 px-4 py-2 text-xs font-bold text-white disabled:opacity-50"
              >
                {busy ? "…" : "Simuler bulletin Notes → dossier"}
              </button>
            </div>

            <div className="grid gap-3 sm:grid-cols-3">
              <label className="text-xs font-semibold text-slate-600">
                Tiroir
                <select
                  value={uploadMeta.tiroir}
                  onChange={(ev) => setUploadMeta((m) => ({ ...m, tiroir: ev.target.value }))}
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                >
                  {data.meta.tiroirs.map((t) => (
                    <option key={t} value={t}>
                      {TIROIR_LABELS[t] || t}
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
                  <option value="restreint">Restreint</option>
                  <option value="sante">Santé</option>
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

            {data.documents.length === 0 ? (
              <p className="text-sm text-slate-500">Aucun document.</p>
            ) : (
              <ul className="grid gap-3 sm:grid-cols-2">
                {data.documents.map((d) => {
                  const isImage =
                    d.canOpen &&
                    Boolean(d.fileUrl) &&
                    (d.mimeType?.startsWith("image/") ||
                      /\.(jpe?g|png|gif|webp)$/i.test(d.fileUrl || ""));
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

      {tab === "notes" && data.stubs.notes ? (
        <p className="rounded-3xl border border-slate-200 bg-white p-6 text-sm text-slate-600">
          {data.stubs.notes.message}
        </p>
      ) : null}
      {tab === "sante" ? (
        <p className="rounded-3xl border border-slate-200 bg-white p-6 text-sm text-slate-600">
          Section santé — documents tiroir « sante » / PAP (accès restreint).
        </p>
      ) : null}
      {tab === "facturation" ? (
        <p className="rounded-3xl border border-slate-200 bg-white p-6 text-sm text-slate-600">
          Section facturation familles (P7) — visible selon votre rôle uniquement.
        </p>
      ) : null}
    </ModulePageShell>
  );
}
