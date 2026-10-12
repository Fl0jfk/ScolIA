"use client";

import { useCallback, useEffect, useId, useMemo, useState } from "react";
import PartenariatSignaturePad from "@/app/components/partenariats/PartenariatSignaturePad";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModuleCard from "@/app/components/module-chrome/ModuleCard";
import ModuleEmptyState from "@/app/components/module-chrome/ModuleEmptyState";
import ModuleTabFallback from "@/app/components/module-chrome/ModuleTabFallback";
import ModuleTabNav from "@/app/components/module-chrome/ModuleTabNav";
import SstFicheDetailModal from "@/app/components/personnel/SstFicheDetailModal";
import { dash } from "@/app/lib/dashboard-brand";
import { canSubmitSstEmargement } from "@/app/lib/sst-registre/emarger-body";
import {
  SST_FICHE_STATUS_LABELS,
  type SstConsultationDto,
  type SstFicheDetail,
  type SstFicheListItem,
  type SstMyStatus,
  type SstSuiviPayload,
} from "@/app/lib/sst-registre/types";

type ContentPayload = {
  presentation: {
    title: string;
    paragraphs: string[];
  };
  sommaire: string[];
  reglementation: {
    title: string;
    intro: string;
    texts: string[];
    objectifsTitle: string;
    objectifs: string[];
  };
  notice: {
    title: string;
    qui: string;
    comment: string;
    ou: string;
    quiConsulte: string;
    quandIntro: string;
    quand: string[];
    commentCompleterTitle: string;
    commentCompleter: string[];
    quiInformerTitle: string;
    quiInformer: string;
    frequence: string;
  };
  etablissement: {
    nom: string;
    adresse: string;
    telephone: string;
    directions: Array<{ label: string; directorName: string }>;
    noteReferents: string;
  };
  emargementTexte: string;
  urgences: Array<{ label: string; value: string }>;
};

type MePayload = SstMyStatus & { content: ContentPayload };

type ViewMode = "accueil" | "fiche" | "suivi";

const ILLEGIBLE_RESPONSE = "Réponse serveur illisible.";
const NETWORK_ERROR = "Connexion impossible. Réessayez.";

async function readJsonResponse<T>(res: Response): Promise<T> {
  try {
    return (await res.json()) as T;
  } catch {
    throw new Error(ILLEGIBLE_RESPONSE);
  }
}

function userFacingActionError(e: unknown, fallback: string): string {
  if (e instanceof Error) {
    if (e.message === ILLEGIBLE_RESPONSE) return ILLEGIBLE_RESPONSE;
    if (e.message === "Failed to fetch") return NETWORK_ERROR;
    return e.message || fallback;
  }
  return fallback;
}

function todayIso(): string {
  const parts = new Intl.DateTimeFormat("en-CA", {
    timeZone: "Europe/Paris",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).formatToParts(new Date());
  const y = parts.find((p) => p.type === "year")?.value ?? "2026";
  const m = parts.find((p) => p.type === "month")?.value ?? "01";
  const d = parts.find((p) => p.type === "day")?.value ?? "01";
  return `${y}-${m}-${d}`;
}

function statusBadgeClass(status: string): string {
  if (status === "cloturee") return "bg-emerald-50 text-emerald-800 ring-emerald-200";
  if (status === "ouverte") return "bg-amber-50 text-amber-900 ring-amber-200";
  return "bg-[color:var(--dash-soft)] text-[var(--dash-ink)] ring-black/10";
}

function InlineFeedback({
  error,
  success,
}: {
  error?: string | null;
  success?: string | null;
}) {
  if (!error && !success) return null;
  if (error) {
    return (
      <p role="alert" className="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700">
        {error}
      </p>
    );
  }
  return (
    <p role="status" className="rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-800">
      {success}
    </p>
  );
}

export default function RhSstRegistreClient({ mode = "staff" }: { mode?: "staff" | "pilotage" }) {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [me, setMe] = useState<MePayload | null>(null);
  const [view, setView] = useState<ViewMode>(mode === "pilotage" ? "suivi" : "accueil");
  const [signatureDataUrl, setSignatureDataUrl] = useState<string | null>(null);
  const [attestationAcknowledged, setAttestationAcknowledged] = useState(false);
  const [remarques, setRemarques] = useState("");
  const [signing, setSigning] = useState(false);
  const [signOk, setSignOk] = useState(false);

  const [fiches, setFiches] = useState<SstFicheListItem[]>([]);
  const [fichesLoading, setFichesLoading] = useState(false);
  const [fichesError, setFichesError] = useState<string | null>(null);
  const [suivi, setSuivi] = useState<SstSuiviPayload | null>(null);
  const [suiviLoading, setSuiviLoading] = useState(false);
  const [suiviError, setSuiviError] = useState<string | null>(null);
  const [selectedFiche, setSelectedFiche] = useState<SstFicheDetail | null>(null);

  const [ficheForm, setFicheForm] = useState({
    observedDate: todayIso(),
    observedTime: "",
    lieu: "",
    observations: "",
    suggestions: "",
  });
  const [ficheSignature, setFicheSignature] = useState<string | null>(null);
  const [savingFiche, setSavingFiche] = useState(false);
  const [ficheFormError, setFicheFormError] = useState<string | null>(null);
  const [ficheFormSuccess, setFicheFormSuccess] = useState<string | null>(null);

  const [advanceNotes, setAdvanceNotes] = useState("");
  const [advanceSignature, setAdvanceSignature] = useState<string | null>(null);
  const [advancing, setAdvancing] = useState(false);

  const [consultComments, setConsultComments] = useState("");
  const [consultSignature, setConsultSignature] = useState<string | null>(null);
  const [savingConsult, setSavingConsult] = useState(false);

  const [actionError, setActionError] = useState<string | null>(null);
  const [actionSuccess, setActionSuccess] = useState<string | null>(null);

  const emargementSigLabelId = useId();
  const emargementSigHintId = useId();
  const attestationCheckboxId = useId();
  const ficheSigLabelId = useId();
  const ficheSigHintId = useId();
  const consultSigLabelId = useId();
  const consultSigHintId = useId();

  const loadMe = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/rh/sst", { cache: "no-store" });
      let json: MePayload & { error?: string };
      try {
        json = (await res.json()) as MePayload & { error?: string };
      } catch {
        throw new Error("Réponse serveur illisible");
      }
      if (!res.ok) throw new Error(json.error || "Registre SST indisponible");
      setMe(json);
      setSignOk(json.signed);
    } catch (e) {
      setMe(null);
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  const loadFiches = useCallback(async () => {
    setFichesLoading(true);
    setFichesError(null);
    try {
      const res = await fetch("/api/rh/sst/fiches", { cache: "no-store" });
      let json: { fiches?: SstFicheListItem[]; error?: string };
      try {
        json = (await res.json()) as { fiches?: SstFicheListItem[]; error?: string };
      } catch {
        throw new Error("Réponse serveur illisible");
      }
      if (!res.ok) throw new Error(json.error || "Impossible de charger les fiches");
      setFiches(json.fiches ?? []);
    } catch (e) {
      setFichesError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setFichesLoading(false);
    }
  }, []);

  const loadSuivi = useCallback(async () => {
    if (!me?.canManage && mode !== "pilotage") return;
    setSuiviLoading(true);
    setSuiviError(null);
    try {
      const res = await fetch("/api/rh/sst/suivi", { cache: "no-store" });
      let json: SstSuiviPayload & { error?: string };
      try {
        json = (await res.json()) as SstSuiviPayload & { error?: string };
      } catch {
        throw new Error("Réponse serveur illisible");
      }
      if (!res.ok) throw new Error(json.error || "Impossible de charger le suivi");
      setSuivi(json);
      setFiches(json.fiches);
    } catch (e) {
      setSuivi(null);
      setSuiviError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSuiviLoading(false);
    }
  }, [me?.canManage, mode]);

  useEffect(() => {
    void loadMe();
  }, [loadMe]);

  useEffect(() => {
    if (!me) return;
    void loadFiches();
    if (me.canManage || mode === "pilotage") void loadSuivi();
  }, [me, loadFiches, loadSuivi, mode]);

  const pendingPeople = useMemo(
    () => (suivi?.people ?? []).filter((p) => !p.signed),
    [suivi],
  );

  const tabItems = useMemo((): Array<{ id: ViewMode; label: string }> => {
    const tabs: Array<{ id: ViewMode; label: string }> = [
      { id: "accueil", label: "Registre" },
      { id: "fiche", label: "Déposer une fiche" },
    ];
    if (me?.canManage || mode === "pilotage") {
      tabs.push({ id: "suivi", label: "Suivi & émargements" });
    }
    return tabs;
  }, [me?.canManage, mode]);

  async function submitEmargement() {
    setActionError(null);
    setActionSuccess(null);
    const canSubmit = canSubmitSstEmargement({
      signing,
      signatureDataUrl,
      attestationAcknowledged,
    });
    if (!canSubmit) {
      setActionError("Cochez l’attestation ou signez dans le cadre.");
      return;
    }
    setSigning(true);
    try {
      const body = attestationAcknowledged
        ? { attestationAcknowledged: true, remarques }
        : { signatureDataUrl, remarques };
      const res = await fetch("/api/rh/sst/emarger", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      const json = await readJsonResponse<{ error?: string }>(res);
      if (!res.ok) throw new Error(json.error || "Échec signature");
      setSignOk(true);
      setSignatureDataUrl(null);
      setAttestationAcknowledged(false);
      setActionSuccess("Émargement enregistré pour cette année scolaire.");
      await loadMe();
      if (me?.canManage) await loadSuivi();
    } catch (e) {
      setActionError(userFacingActionError(e, "Impossible d’enregistrer l’émargement."));
    } finally {
      setSigning(false);
    }
  }

  async function submitFiche() {
    setFicheFormError(null);
    setFicheFormSuccess(null);
    if (!ficheForm.observations.trim()) {
      setFicheFormError("Indiquez vos observations.");
      return;
    }
    setSavingFiche(true);
    try {
      const res = await fetch("/api/rh/sst/fiches", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          ...ficheForm,
          signatureDataUrl: ficheSignature || undefined,
        }),
      });
      const json = await readJsonResponse<{ error?: string; fiche?: { numero: number } }>(res);
      if (!res.ok) throw new Error(json.error || "Échec création");
      setFicheForm({
        observedDate: todayIso(),
        observedTime: "",
        lieu: "",
        observations: "",
        suggestions: "",
      });
      setFicheSignature(null);
      setView("accueil");
      await loadFiches();
      await loadMe();
      if (me?.canManage) await loadSuivi();
      setFicheFormSuccess(`Fiche n°${json.fiche?.numero ?? "—"} enregistrée.`);
    } catch (e) {
      setFicheFormError(userFacingActionError(e, "Impossible d’enregistrer la fiche."));
    } finally {
      setSavingFiche(false);
    }
  }

  async function openFiche(id: string) {
    setActionError(null);
    setActionSuccess(null);
    try {
      const res = await fetch(`/api/rh/sst/fiches/${id}`, { cache: "no-store" });
      const json = await readJsonResponse<{ fiche?: SstFicheDetail; error?: string }>(res);
      if (!res.ok || !json.fiche) {
        setActionError(json.error || "Fiche introuvable");
        return;
      }
      setSelectedFiche(json.fiche);
      setAdvanceNotes("");
      setAdvanceSignature(null);
    } catch (e) {
      setActionError(userFacingActionError(e, "Impossible d’ouvrir la fiche."));
    }
  }

  async function advanceFiche() {
    if (!selectedFiche) return;
    setActionError(null);
    setActionSuccess(null);
    setAdvancing(true);
    try {
      const res = await fetch(`/api/rh/sst/fiches/${selectedFiche.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: "advance",
          notes: advanceNotes,
          signatureDataUrl: advanceSignature || undefined,
        }),
      });
      const json = await readJsonResponse<{ error?: string; fiche?: SstFicheDetail }>(res);
      if (!res.ok) throw new Error(json.error || "Échec");
      if (json.fiche) setSelectedFiche(json.fiche);
      setAdvanceNotes("");
      setAdvanceSignature(null);
      await loadFiches();
      await loadSuivi();
      setActionSuccess("Étape validée.");
    } catch (e) {
      setActionError(userFacingActionError(e, "Impossible de valider l’étape."));
    } finally {
      setAdvancing(false);
    }
  }

  async function submitConsultation() {
    setActionError(null);
    setActionSuccess(null);
    setSavingConsult(true);
    try {
      const res = await fetch("/api/rh/sst/consultations", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          comments: consultComments,
          signatureDataUrl: consultSignature || undefined,
        }),
      });
      const json = await readJsonResponse<{ error?: string }>(res);
      if (!res.ok) throw new Error(json.error || "Échec");
      setConsultComments("");
      setConsultSignature(null);
      await loadSuivi();
      setActionSuccess("Consultation enregistrée.");
    } catch (e) {
      setActionError(userFacingActionError(e, "Impossible d’enregistrer la consultation."));
    } finally {
      setSavingConsult(false);
    }
  }

  if (loading) {
    return <ModuleTabFallback />;
  }
  if (error || !me) {
    return (
      <ModuleCard bodyClassName="p-8 text-center">
        <p role="alert" className="text-sm text-rose-600">{error || "Indisponible"}</p>
        <ModuleButton variant="secondary" className="mt-4" onClick={() => void loadMe()}>
          Réessayer
        </ModuleButton>
      </ModuleCard>
    );
  }

  const content = me.content;
  const emargementCanSubmit = canSubmitSstEmargement({
    signing,
    signatureDataUrl,
    attestationAcknowledged,
  });

  return (
    <div className="space-y-5">
      <header className="rounded-[1.35rem] border border-black/6 bg-white p-5 sm:p-6">
        <p className={`text-[10px] font-semibold uppercase tracking-[0.18em] ${dash.textMid}`}>
          Santé &amp; sécurité au travail
        </p>
        <h2 className="mt-1 text-2xl font-black text-slate-900">{me.campagne.title}</h2>
        <p className="mt-2 max-w-3xl text-sm text-slate-600">
          Document officiel de l&apos;établissement : présentation, réglementation, notice,
          informations pratiques, émargement annuel et fiches de signalement.
        </p>
        <ModuleTabNav
          className="mt-4"
          tabs={tabItems}
          active={view}
          onChange={(id) => {
            setView(id);
            if (id === "suivi") void loadSuivi();
          }}
          badges={{ suivi: suivi?.pendingCount ?? 0 }}
        />
      </header>

      <InlineFeedback
        error={selectedFiche ? null : actionError}
        success={selectedFiche ? null : actionSuccess}
      />

      {view === "accueil" ? (
        <div className="space-y-4">
          {!signOk ? (
            <div className="rounded-2xl border border-amber-200 bg-amber-50/80 px-4 py-3 text-sm text-amber-950">
              <strong>À faire pour {me.campagne.anneeLabel} :</strong> lisez le registre ci-dessous,
              puis signez l&apos;émargement en bas de page.
            </div>
          ) : (
            <div className="rounded-2xl border border-emerald-200 bg-emerald-50/70 px-4 py-3 text-sm text-emerald-900">
              Émargement enregistré pour {me.campagne.anneeLabel}
              {me.emargement?.signedAt
                ? ` — le ${new Date(me.emargement.signedAt).toLocaleString("fr-FR")}`
                : ""}
              .
            </div>
          )}

          <ModuleCard bodyClassName="p-5">
            <h3 className="font-black text-slate-900">{content.presentation.title}</h3>
            <div className="mt-3 space-y-3 text-sm leading-relaxed text-slate-700">
              {content.presentation.paragraphs.map((p) => (
                <p key={p.slice(0, 40)}>{p}</p>
              ))}
            </div>
            <p className="mt-4 text-xs font-bold uppercase tracking-wide text-slate-500">Sommaire</p>
            <ol className="mt-2 list-decimal space-y-1 pl-5 text-sm text-slate-700">
              {content.sommaire.map((item) => (
                <li key={item}>{item}</li>
              ))}
            </ol>
          </ModuleCard>

          <div className="grid gap-4 lg:grid-cols-2">
            <ModuleCard bodyClassName="p-5">
              <h3 className="font-black text-slate-900">{content.reglementation.title}</h3>
              <p className="mt-2 text-sm text-slate-700">{content.reglementation.intro}</p>
              <ul className="mt-3 list-disc space-y-1 pl-5 text-sm text-slate-700">
                {content.reglementation.texts.map((t) => (
                  <li key={t}>{t}</li>
                ))}
              </ul>
              <p className="mt-4 text-xs font-bold uppercase tracking-wide text-slate-500">
                {content.reglementation.objectifsTitle}
              </p>
              <ul className="mt-1 list-disc space-y-1 pl-5 text-sm text-slate-700">
                {content.reglementation.objectifs.map((t) => (
                  <li key={t}>{t}</li>
                ))}
              </ul>
            </ModuleCard>

            <ModuleCard bodyClassName="p-5">
              <h3 className="font-black text-slate-900">Informations sur l&apos;établissement</h3>
              <dl className="mt-3 space-y-2 text-sm text-slate-700">
                <div>
                  <dt className="font-bold text-slate-900">Nom</dt>
                  <dd>{content.etablissement.nom}</dd>
                </div>
                <div>
                  <dt className="font-bold text-slate-900">Adresse</dt>
                  <dd>{content.etablissement.adresse}</dd>
                </div>
                {content.etablissement.telephone ? (
                  <div>
                    <dt className="font-bold text-slate-900">Téléphone</dt>
                    <dd>{content.etablissement.telephone}</dd>
                  </div>
                ) : null}
                {content.etablissement.directions.length > 0 ? (
                  <div>
                    <dt className="font-bold text-slate-900">Directions / cheffes d&apos;établissement</dt>
                    <dd>
                      <ul className="mt-1 list-disc space-y-0.5 pl-5">
                        {content.etablissement.directions.map((d) => (
                          <li key={`${d.label}-${d.directorName}`}>
                            {d.label} : {d.directorName}
                          </li>
                        ))}
                      </ul>
                    </dd>
                  </div>
                ) : null}
                <div>
                  <dt className="font-bold text-slate-900">Référents</dt>
                  <dd>{content.etablissement.noteReferents}</dd>
                </div>
              </dl>
              <p className="mt-4 text-xs font-bold uppercase tracking-wide text-slate-500">
                Services d&apos;urgence
              </p>
              <ul className="mt-1 flex flex-wrap gap-2 text-sm">
                {content.urgences.map((u) => (
                  <li
                    key={u.label}
                    className="rounded-lg border border-slate-200 bg-slate-50 px-2.5 py-1 font-bold text-slate-800"
                  >
                    {u.label} {u.value}
                  </li>
                ))}
              </ul>
            </ModuleCard>
          </div>

          <ModuleCard bodyClassName="p-5">
            <h3 className="font-black text-slate-900">{content.notice.title}</h3>
            <dl className="mt-3 grid gap-4 text-sm text-slate-700 sm:grid-cols-2">
              <div>
                <dt className="font-bold text-slate-900">Qui peut renseigner ?</dt>
                <dd className="mt-1">{content.notice.qui}</dd>
              </div>
              <div>
                <dt className="font-bold text-slate-900">Comment ?</dt>
                <dd className="mt-1">{content.notice.comment}</dd>
              </div>
              <div>
                <dt className="font-bold text-slate-900">Où le trouver ?</dt>
                <dd className="mt-1">{content.notice.ou}</dd>
              </div>
              <div>
                <dt className="font-bold text-slate-900">Qui consulte ?</dt>
                <dd className="mt-1">{content.notice.quiConsulte}</dd>
              </div>
              <div className="sm:col-span-2">
                <dt className="font-bold text-slate-900">Quand le compléter ?</dt>
                <dd className="mt-1">
                  {content.notice.quandIntro}
                  <ul className="mt-1 list-disc space-y-1 pl-5">
                    {content.notice.quand.map((q) => (
                      <li key={q}>{q}</li>
                    ))}
                  </ul>
                </dd>
              </div>
              <div className="sm:col-span-2">
                <dt className="font-bold text-slate-900">{content.notice.commentCompleterTitle}</dt>
                <dd className="mt-1">
                  <ul className="list-disc space-y-1 pl-5">
                    {content.notice.commentCompleter.map((q) => (
                      <li key={q}>{q}</li>
                    ))}
                  </ul>
                </dd>
              </div>
              <div>
                <dt className="font-bold text-slate-900">{content.notice.quiInformerTitle}</dt>
                <dd className="mt-1">{content.notice.quiInformer}</dd>
              </div>
              <div>
                <dt className="font-bold text-slate-900">Fréquence de mise à jour</dt>
                <dd className="mt-1">{content.notice.frequence}</dd>
              </div>
            </dl>
            <ModuleButton variant="secondary" className="mt-4 text-xs" onClick={() => setView("fiche")}>
              Déposer une fiche de signalement
            </ModuleButton>
          </ModuleCard>

          {!signOk ? (
            <ModuleCard bodyClassName="p-5 border-amber-200 bg-amber-50/40">
              <p className="text-[10px] font-semibold uppercase tracking-[0.18em] text-amber-800">
                Émargement — année {me.campagne.anneeLabel}
              </p>
              <h3 className="mt-1 font-black text-slate-900">Prise de connaissance</h3>
              <p className="mt-2 text-sm text-slate-700">{content.emargementTexte}</p>
              <div className="mt-4 flex items-start gap-2">
                <input
                  id={attestationCheckboxId}
                  type="checkbox"
                  checked={attestationAcknowledged}
                  onChange={(e) => {
                    const checked = e.target.checked;
                    setAttestationAcknowledged(checked);
                    if (checked) setSignatureDataUrl(null);
                  }}
                  className="mt-1 h-4 w-4 accent-[color:var(--dash-ink)]"
                />
                <label htmlFor={attestationCheckboxId} className="text-sm font-semibold text-slate-800">
                  J&apos;atteste avoir pris connaissance du registre
                </label>
              </div>
              <div className="mt-4">
                <p id={emargementSigLabelId} className="mb-1 text-xs font-bold text-slate-600">
                  Signature manuscrite (optionnel si attestation cochée)
                </p>
                <p id={emargementSigHintId} className="mb-2 text-xs text-slate-500">
                  Souris, doigt ou stylet. Au clavier ou avec un lecteur d&apos;écran, cochez l&apos;attestation
                  ci-dessus.
                </p>
                <PartenariatSignaturePad
                  labelId={emargementSigLabelId}
                  describedById={emargementSigHintId}
                  onChange={(url) => {
                    setSignatureDataUrl(url);
                    if (url) setAttestationAcknowledged(false);
                  }}
                />
              </div>
              <label className="mt-3 block" htmlFor="sst-emargement-remarques">
                <span className="text-xs font-bold text-slate-600">Remarques (optionnel)</span>
                <input
                  id="sst-emargement-remarques"
                  value={remarques}
                  onChange={(e) => setRemarques(e.target.value)}
                  className={`mt-1 ${dash.field}`}
                  maxLength={500}
                />
              </label>
              <ModuleButton
                variant="primary"
                className="mt-4 text-xs"
                disabled={!emargementCanSubmit}
                onClick={() => void submitEmargement()}
              >
                {signing ? "Enregistrement…" : "Je signe pour cette année scolaire"}
              </ModuleButton>
            </ModuleCard>
          ) : null}

          <ModuleCard bodyClassName="p-5">
            <div className="mb-3 flex items-center justify-between gap-2">
              <h3 className="font-black text-slate-900">Fiches du registre</h3>
              <span className="text-xs text-slate-500">
                {me.openFichesCount} ouverte{me.openFichesCount > 1 ? "s" : ""}
              </span>
            </div>
            {fichesLoading ? (
              <ModuleTabFallback />
            ) : fichesError ? (
              <div className="space-y-3">
                <p role="alert" className="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700">
                  {fichesError}
                </p>
                <ModuleButton variant="secondary" onClick={() => void loadFiches()}>
                  Réessayer
                </ModuleButton>
              </div>
            ) : fiches.length === 0 ? (
              <ModuleEmptyState>
                Aucune fiche pour l&apos;instant. C&apos;est ici que les signalements apparaissent une fois
                déposés.
              </ModuleEmptyState>
            ) : (
              <ul className="divide-y divide-slate-100">
                {fiches.map((f) => (
                  <li key={f.id} className="flex flex-wrap items-center justify-between gap-2 py-2.5">
                    <div className="min-w-0">
                      <p className="text-sm font-bold text-slate-900">
                        Fiche n°{f.numero} — {f.declarantName || "Anonyme"}
                      </p>
                      <p className="truncate text-xs text-slate-500">
                        {f.observedDate}
                        {f.lieu ? ` · ${f.lieu}` : ""} — {f.observationsPreview}
                      </p>
                    </div>
                    <div className="flex items-center gap-2">
                      <span
                        className={`rounded-full px-2 py-0.5 text-[10px] font-bold uppercase ring-1 ${statusBadgeClass(f.status)}`}
                      >
                        {SST_FICHE_STATUS_LABELS[f.status]}
                      </span>
                      <button
                        type="button"
                        onClick={() => void openFiche(f.id)}
                        aria-label={`Ouvrir la fiche n°${f.numero}`}
                        className={`text-xs font-bold underline ${dash.linkBold}`}
                      >
                        Ouvrir
                      </button>
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </ModuleCard>
        </div>
      ) : null}

      {view === "fiche" ? (
        <ModuleCard bodyClassName="p-5">
          <h3 className="font-black text-slate-900">Nouvelle fiche du registre</h3>
          <p className="mt-1 text-sm text-slate-600">
            Signalez un risque, un incident, un dysfonctionnement ou une suggestion d&apos;amélioration.
          </p>
          <InlineFeedback error={ficheFormError} success={ficheFormSuccess} />
          <div className="mt-4 grid gap-3 sm:grid-cols-2">
            <label className="block text-sm" htmlFor="sst-fiche-date">
              <span className="font-bold text-slate-700">Date</span>
              <input
                id="sst-fiche-date"
                type="date"
                value={ficheForm.observedDate}
                onChange={(e) => setFicheForm((f) => ({ ...f, observedDate: e.target.value }))}
                className={`mt-1 ${dash.field}`}
              />
            </label>
            <label className="block text-sm" htmlFor="sst-fiche-time">
              <span className="font-bold text-slate-700">Heure</span>
              <input
                id="sst-fiche-time"
                type="time"
                value={ficheForm.observedTime}
                onChange={(e) => setFicheForm((f) => ({ ...f, observedTime: e.target.value }))}
                className={`mt-1 ${dash.field}`}
              />
            </label>
            <label className="block text-sm sm:col-span-2" htmlFor="sst-fiche-lieu">
              <span className="font-bold text-slate-700">Lieu (service, poste…)</span>
              <input
                id="sst-fiche-lieu"
                value={ficheForm.lieu}
                onChange={(e) => setFicheForm((f) => ({ ...f, lieu: e.target.value }))}
                className={`mt-1 ${dash.field}`}
                maxLength={200}
              />
            </label>
            <label className="block text-sm sm:col-span-2" htmlFor="sst-fiche-obs">
              <span className="font-bold text-slate-700">Observations</span>
              <textarea
                id="sst-fiche-obs"
                value={ficheForm.observations}
                onChange={(e) => setFicheForm((f) => ({ ...f, observations: e.target.value }))}
                rows={5}
                className={`mt-1 ${dash.field}`}
                maxLength={8000}
              />
            </label>
            <label className="block text-sm sm:col-span-2" htmlFor="sst-fiche-sugg">
              <span className="font-bold text-slate-700">Suggestions (facultatif)</span>
              <textarea
                id="sst-fiche-sugg"
                value={ficheForm.suggestions}
                onChange={(e) => setFicheForm((f) => ({ ...f, suggestions: e.target.value }))}
                rows={3}
                className={`mt-1 ${dash.field}`}
                maxLength={4000}
              />
            </label>
          </div>
          <div className="mt-4">
            <p id={ficheSigLabelId} className="mb-1 text-xs font-bold text-slate-600">
              Signature du déclarant (recommandée)
            </p>
            <p id={ficheSigHintId} className="mb-2 text-xs text-slate-500">
              Signez dans le cadre ci-dessous (souris, doigt ou stylet).
            </p>
            <PartenariatSignaturePad
              labelId={ficheSigLabelId}
              describedById={ficheSigHintId}
              onChange={setFicheSignature}
            />
          </div>
          <div className="mt-4 flex flex-wrap gap-2">
            <ModuleButton
              variant="primary"
              className="text-xs"
              disabled={savingFiche}
              onClick={() => void submitFiche()}
            >
              {savingFiche ? "Enregistrement…" : "Enregistrer la fiche"}
            </ModuleButton>
            <ModuleButton variant="secondary" className="text-xs" onClick={() => setView("accueil")}>
              Annuler
            </ModuleButton>
          </div>
        </ModuleCard>
      ) : null}

      {view === "suivi" && (me.canManage || mode === "pilotage") ? (
        <div className="space-y-4">
          <ModuleCard bodyClassName="p-5">
            <h3 className="font-black text-slate-900">Émargements — {me.campagne.anneeLabel}</h3>
            {suiviLoading && !suivi ? (
              <ModuleTabFallback />
            ) : suiviError ? (
              <div className="mt-3 space-y-3">
                <p role="alert" className="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700">
                  {suiviError}
                </p>
                <ModuleButton variant="secondary" onClick={() => void loadSuivi()}>
                  Réessayer
                </ModuleButton>
              </div>
            ) : suivi ? (
              <>
                <p className="mt-2 text-sm text-slate-600">
                  <strong className="text-slate-900">{suivi.signedCount}</strong> / {suivi.total}{" "}
                  collaborateurs ont signé —{" "}
                  <strong className="text-amber-800">{suivi.pendingCount}</strong> en attente.
                </p>
                <div className="mt-3 max-h-80 overflow-auto rounded-xl border border-slate-100">
                  <table className="w-full text-left text-sm">
                    <thead className="sticky top-0 bg-slate-50 text-xs uppercase text-slate-500">
                      <tr>
                        <th className="px-3 py-2">Nom</th>
                        <th className="px-3 py-2">Fonction</th>
                        <th className="px-3 py-2">Statut</th>
                      </tr>
                    </thead>
                    <tbody>
                      {(suivi.people ?? []).map((p) => (
                        <tr key={p.userId} className="border-t border-slate-50">
                          <td className="px-3 py-2 font-medium text-slate-900">
                            {p.lastName} {p.firstName}
                            <span className="block text-xs font-normal text-slate-400">{p.email}</span>
                          </td>
                          <td className="px-3 py-2 text-slate-600">{p.fonction}</td>
                          <td className="px-3 py-2">
                            {p.signed ? (
                              <span className="font-bold text-emerald-700">
                                Signé
                                {p.signedAt
                                  ? ` · ${new Date(p.signedAt).toLocaleDateString("fr-FR")}`
                                  : ""}
                              </span>
                            ) : (
                              <span className="font-bold text-amber-700">En attente</span>
                            )}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
                {pendingPeople.length > 0 ? (
                  <p className="mt-3 text-xs text-slate-500">
                    Les retardataires voient un rappel sur le tableau de bord tant qu&apos;ils n&apos;ont pas
                    signé.
                  </p>
                ) : null}
              </>
            ) : null}
          </ModuleCard>

          <ModuleCard bodyClassName="p-5">
            <h3 className="font-black text-slate-900">Visa de consultation</h3>
            <p className="mt-1 text-sm text-slate-600">
              Direction, référent sécurité, CSE / OGEC — attestez avoir consulté le registre.
            </p>
            <label className="mt-3 block text-sm" htmlFor="sst-consult-comments">
              <span className="font-bold text-slate-700">Commentaires</span>
              <textarea
                id="sst-consult-comments"
                value={consultComments}
                onChange={(e) => setConsultComments(e.target.value)}
                rows={2}
                className={`mt-1 ${dash.field}`}
              />
            </label>
            <p id={consultSigLabelId} className="mt-3 text-xs font-bold text-slate-600">
              Signature (optionnel)
            </p>
            <p id={consultSigHintId} className="text-xs text-slate-500">
              Signature facultative (souris, doigt ou stylet).
            </p>
            <PartenariatSignaturePad
              labelId={consultSigLabelId}
              describedById={consultSigHintId}
              onChange={setConsultSignature}
            />
            <ModuleButton
              variant="primary"
              className="mt-3 text-xs"
              disabled={savingConsult}
              onClick={() => void submitConsultation()}
            >
              {savingConsult ? "Enregistrement…" : "Enregistrer mon visa de consultation"}
            </ModuleButton>
            {suivi && suivi.consultations.length > 0 ? (
              <ul className="mt-4 space-y-2">
                {suivi.consultations.map((c: SstConsultationDto) => (
                  <li key={c.id} className="rounded-xl border border-slate-100 px-3 py-2 text-sm">
                    <span className="font-bold text-slate-900">
                      {c.lastName} {c.firstName}
                    </span>{" "}
                    <span className="text-slate-500">({c.fonction})</span>
                    <span className="block text-xs text-slate-400">
                      {new Date(c.consultedAt).toLocaleString("fr-FR")}
                      {c.comments ? ` — ${c.comments}` : ""}
                    </span>
                  </li>
                ))}
              </ul>
            ) : null}
          </ModuleCard>

          <ModuleCard bodyClassName="p-5">
            <h3 className="font-black text-slate-900">Tableau de suivi des fiches</h3>
            {fichesLoading ? (
              <ModuleTabFallback />
            ) : fichesError ? (
              <p role="alert" className="mt-2 rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700">
                {fichesError}
              </p>
            ) : fiches.length === 0 ? (
              <ModuleEmptyState>Aucune fiche.</ModuleEmptyState>
            ) : (
              <ul className="mt-3 divide-y divide-slate-100">
                {fiches.map((f) => (
                  <li key={f.id} className="flex flex-wrap items-center justify-between gap-2 py-2.5">
                    <div>
                      <p className="text-sm font-bold text-slate-900">
                        n°{f.numero} — {f.observationsPreview}
                      </p>
                      <p className="text-xs text-slate-500">
                        {f.observedDate} · {f.declarantName}
                      </p>
                    </div>
                    <div className="flex items-center gap-2">
                      <span
                        className={`rounded-full px-2 py-0.5 text-[10px] font-bold uppercase ring-1 ${statusBadgeClass(f.status)}`}
                      >
                        {SST_FICHE_STATUS_LABELS[f.status]}
                      </span>
                      <button
                        type="button"
                        onClick={() => void openFiche(f.id)}
                        aria-label={`Traiter la fiche n°${f.numero}`}
                        className={`text-xs font-bold underline ${dash.linkBold}`}
                      >
                        Traiter
                      </button>
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </ModuleCard>
        </div>
      ) : null}

      {selectedFiche ? (
        <SstFicheDetailModal
          fiche={selectedFiche}
          advancing={advancing}
          advanceNotes={advanceNotes}
          error={actionError}
          success={actionSuccess}
          onAdvanceNotesChange={setAdvanceNotes}
          onAdvanceSignatureChange={setAdvanceSignature}
          onAdvance={() => void advanceFiche()}
          onClose={() => {
            setSelectedFiche(null);
            setActionError(null);
            setActionSuccess(null);
          }}
        />
      ) : null}
    </div>
  );
}
