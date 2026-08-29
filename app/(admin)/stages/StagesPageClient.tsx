"use client";

import { Suspense, useCallback, useEffect, useMemo, useState } from "react";
import dynamic from "next/dynamic";
import { useSearchParams } from "next/navigation";
import { useSessionUser } from "@/app/hooks/useAppUser";
import { useOneDriveConnection } from "@/app/hooks/useOneDriveConnection";
import type { OneDriveUserProfile } from "@/app/lib/onedrive-user-profiles";
import type { StageConvention } from "@/app/lib/stage-types";
import { STAGE_CONVENTION_STATUS_LABELS } from "@/app/lib/stage-types";
import StagePendingSignaturesPanel from "@/app/components/stages/StagePendingSignaturesPanel";
import StagePreconventionForm from "@/app/components/stages/StagePreconventionForm";
import StageSignatureProgress from "@/app/components/stages/StageSignatureProgress";
import { buildSignatureSummary } from "@/app/lib/stage-signature-summary";
import StagesBoardPanel from "@/app/components/stages/StagesBoardPanel";
import type {
  StageTab,
  StagesHubBoard,
} from "@/app/components/stages/stages-hub-types";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import ModuleTabFallback from "@/app/components/module-chrome/ModuleTabFallback";
import ModuleTabNav from "@/app/components/module-chrome/ModuleTabNav";
import { MODULE_TOUR_STEP_EVENT } from "@/app/lib/module-tour-actions";
import { resolveStagesTourTab } from "@/app/lib/module-tours";

const StagesClassePanel = dynamic(() => import("@/app/components/stages/StagesClassePanel"), {
  ssr: false,
  loading: () => <ModuleTabFallback />,
});
const StagesConventionsPanel = dynamic(
  () => import("@/app/components/stages/StagesConventionsPanel"),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);
const StagesSettingsPanel = dynamic(() => import("@/app/components/stages/StagesSettingsPanel"), {
  ssr: false,
  loading: () => <ModuleTabFallback />,
});

function StagesContent() {
  const searchParams = useSearchParams();
  const { user: sessionUser } = useSessionUser();
  const [oneDriveProfile, setOneDriveProfile] = useState<OneDriveUserProfile | null>(null);
  useEffect(() => {
    if (!sessionUser) {
      setOneDriveProfile(null);
      return;
    }
    let cancelled = false;
    fetch("/api/onedrive/profile")
      .then((r) => r.json())
      .then((j) => {
        if (!cancelled) setOneDriveProfile(j.profile || null);
      })
      .catch(() => {
        if (!cancelled) setOneDriveProfile(null);
      });
    return () => {
      cancelled = true;
    };
  }, [sessionUser]);
  const od = useOneDriveConnection();
  const [board, setBoard] = useState<StagesHubBoard | null>(null);
  const [conventions, setConventions] = useState<StageConvention[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(searchParams.get("convention"));
  const [detail, setDetail] = useState<{
    convention: StageConvention;
    studentLink: string | null;
    signLinks: Array<{ role: string; label: string; link: string; email?: string }>;
    eleveMatch?: {
      matchedEleve: {
        ine?: string;
        nom: string;
        prenom: string;
        folderName: string;
      } | null;
      folderPath: string | null;
      secteur: string | null;
      targetOneDriveLabel: string | null;
    };
  } | null>(null);
  const [attachIne, setAttachIne] = useState("");
  const [tab, setTab] = useState<StageTab>(
    (searchParams.get("tab") as StageTab) || "board",
  );
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const [hasStoredSignature, setHasStoredSignature] = useState<boolean | undefined>(undefined);
  const [filingConventionId, setFilingConventionId] = useState<string | null>(null);
  const [adminReviewNote, setAdminReviewNote] = useState("");
  const [adminEditing, setAdminEditing] = useState(false);

  const load = useCallback(async () => {
    setError(null);
    try {
      const [bRes, cRes] = await Promise.all([
        fetch("/api/stages", { cache: "no-store" }),
        fetch("/api/stages/conventions", { cache: "no-store" }),
      ]);
      const b = await bRes.json();
      const c = await cRes.json();
      if (!bRes.ok) throw new Error(b?.error || "Erreur");
      setBoard(b);
      setConventions(c.conventions || []);
      if ((b.myPendingSignatures?.length ?? 0) > 0) {
        try {
          const sigRes = await fetch("/api/stages/my-signature", { cache: "no-store" });
          const sigData = await sigRes.json();
          if (sigRes.ok) setHasStoredSignature(Boolean(sigData.hasSignature));
        } catch {
          /* ignore */
        }
      }
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    }
  }, []);

  const loadDetail = useCallback(async (id: string) => {
    const res = await fetch(`/api/stages/conventions/${id}`, { cache: "no-store" });
    const data = await res.json();
    if (!res.ok) throw new Error(data?.error || "Erreur");
    setDetail(data);
    setAttachIne(data.convention?.ocrMeta?.matchedEleveIne ?? "");
    setSelectedId(id);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (searchParams.get("tab") === "offers") {
      setTab("board");
    }
  }, [searchParams]);

  useEffect(() => {
    if (board?.permissions.referentOnly && tab === "board") {
      setTab("classe");
    }
  }, [board, tab]);

  useEffect(() => {
    const onStep = (e: Event) => {
      const target = (e as CustomEvent<{ target?: string }>).detail?.target;
      const nextTab = resolveStagesTourTab(target);
      if (nextTab) setTab(nextTab);
    };
    window.addEventListener(MODULE_TOUR_STEP_EVENT, onStep);
    return () => window.removeEventListener(MODULE_TOUR_STEP_EVENT, onStep);
  }, []);

  useEffect(() => {
    const id = searchParams.get("convention");
    if (id) void loadDetail(id).catch(() => undefined);
  }, [searchParams, loadDetail]);

  const permissions = board?.permissions;

  async function adminReview(approved: boolean) {
    if (!detail) return;
    setBusy(true);
    try {
      const res = await fetch(`/api/stages/conventions/${detail.convention.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: "admin_review",
          approved,
          note: adminReviewNote.trim() || undefined,
        }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setDetail(data);
      setAdminEditing(false);
      setAdminReviewNote("");
      setMsg(
        approved
          ? detail.convention.status === "convention_deposited"
            ? "Dépôt validé — e-mails de signature envoyés à l'élève, l'entreprise, le prof référent et la direction."
            : "Convention validée — e-mails de signature envoyés aux signataires (si SMTP configuré)."
          : detail.convention.status === "convention_deposited"
            ? "Dépôt refusé."
            : "Renvoyé pour correction — e-mails envoyés aux responsables légaux si possible.",
      );
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function adminSavePreconvention() {
    if (!detail) return;
    setBusy(true);
    setError(null);
    try {
      const res = await fetch(`/api/stages/conventions/${detail.convention.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "save", convention: detail.convention }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setDetail({ ...detail, convention: data.convention });
      setMsg("Préconvention enregistrée (modification administrative).");
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function resendSignature(signatureId: string) {
    if (!detail) return;
    setBusy(true);
    setError(null);
    try {
      const res = await fetch(`/api/stages/conventions/${detail.convention.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "resend_signature", signatureId }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setMsg(
        data.mail?.sent
          ? `Relance envoyée à ${data.email || "le signataire"}.`
          : `Relance non envoyée (${data.mail?.reason || "erreur"}).`,
      );
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function resendSignatures() {
    if (!detail) return;
    setBusy(true);
    try {
      const res = await fetch(`/api/stages/conventions/${detail.convention.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "resend_signatures" }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setMsg(
        `Relance envoyée : ${data.mail?.sentCount ?? 0} e-mail(s) sur ${data.mail?.total ?? 0} signataire(s).`,
      );
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function reviewSignature(signatureId: string, accepted: boolean, note?: string) {
    if (!detail) return;
    setBusy(true);
    setError(null);
    try {
      const res = await fetch(`/api/stages/conventions/${detail.convention.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: "review_signature",
          signatureId,
          accepted,
          note: note?.trim() || undefined,
        }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setDetail({ ...detail, convention: data.convention });
      setMsg(
        accepted
          ? "Signature acceptée."
          : "Signature refusée — un e-mail de relance a été envoyé au signataire.",
      );
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function attachEleveIne() {
    if (!detail) return;
    setBusy(true);
    setError(null);
    try {
      const res = await fetch(`/api/stages/conventions/${detail.convention.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "attach_eleve", matchedEleveIne: attachIne.trim() }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setMsg(attachIne.trim() ? "Élève rattaché par INE." : "Rattachement INE retiré.");
      await loadDetail(detail.convention.id);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function fileConventionToOneDrive(conventionId: string) {
    setFilingConventionId(conventionId);
    setBusy(true);
    setError(null);
    setMsg(null);
    try {
      const token = await od.ensureToken();
      if (!token) {
        setError(od.error || "Connectez-vous à OneDrive avant d'envoyer la convention.");
        return;
      }
      const res = await fetch(`/api/stages/conventions/${conventionId}/file-onedrive`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ accessToken: token }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur envoi OneDrive");
      if (detail?.convention.id === conventionId) {
        setDetail({ ...detail, convention: data.convention });
      }
      setMsg(
        `Convention déposée dans le dossier élève : ${data.oneDrive?.fullPath ?? data.oneDrive?.folderPath ?? "OneDrive"}.`,
      );
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
      setFilingConventionId(null);
    }
  }

  async function fileToOneDrive() {
    if (!detail) return;
    await fileConventionToOneDrive(detail.convention.id);
  }

  async function fileToEleveDossier() {
    if (!detail) return;
    setBusy(true);
    setError(null);
    try {
      const res = await fetch(`/api/stages/conventions/${detail.convention.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "file_eleve_dossier" }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setDetail({ ...detail, convention: data.convention });
      setMsg(
        `Convention enregistrée dans le dossier élève (tiroir scolaire).${
          data.eleveDossier?.dossierUrl ? ` Voir : ${data.eleveDossier.dossierUrl}` : ""
        }`,
      );
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  const canShowOneDriveFiling =
    permissions?.canFileToOneDrive && detail?.convention.status === "signed";

  const canShowEleveDossierFiling =
    permissions?.canReviewPreconvention && detail?.convention.status === "signed";

  return (
    <ModulePageShell maxWidthClass="max-w-[1400px]" tourModuleId="stages">
      <ModulePageHeader
        title="Stages & conventions"
        description="Les élèves remplissent leur préconvention en ligne (entreprise, horaires, contacts). Après validation, chaque signataire reçoit un code sécurisé par e-mail."
      />

      {error && (
        <p className="mb-4 rounded-lg border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-800">{error}</p>
      )}
      {msg && (
        <p className="mb-4 rounded-lg border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-800 break-all">{msg}</p>
      )}

      {board?.myPendingSignatures && board.myPendingSignatures.length > 0 && (
        <StagePendingSignaturesPanel
          items={board.myPendingSignatures}
          hasStoredSignature={hasStoredSignature}
        />
      )}

      {permissions?.referentOnly && (
        <div className="mb-6 rounded-xl border border-blue-200 bg-blue-50 px-4 py-3 text-sm text-blue-900">
          <p className="font-semibold">Vue professeur principal / référent</p>
          <p className="mt-1 text-blue-800">
            Consultez l&apos;onglet <strong>Suivi classe</strong>, enregistrez votre signature ci-dessous, puis
            signez les conventions reçues par e-mail — votre paraphe sera ajouté directement sur le PDF.
          </p>
        </div>
      )}


      <ModuleTabNav
        className="mb-6"
        tabs={[
          {
            id: "board",
            label: "Tableau de bord",
            hidden: Boolean(permissions?.referentOnly),
            dataAttrs: { "data-stages-tab": "board" },
          },
          {
            id: "classe",
            label: "Suivi classe",
            hidden: !permissions?.canViewClassRoster,
            dataAttrs: { "data-stages-tab": "classe" },
          },
          {
            id: "conventions",
            label: "Conventions",
            dataAttrs: { "data-stages-tab": "conventions" },
          },
          {
            id: "settings",
            label: "Réglages",
            hidden: !permissions?.canManageStageSettings,
            dataAttrs: { "data-stages-tab": "settings" },
          },
        ]}
        active={tab}
        onChange={setTab}
        badges={{ conventions: board?.counts?.myPendingSignatures }}
      />

      {tab === "classe" && permissions?.canViewClassRoster && (
        <StagesClassePanel
          onOpenConvention={(id) => {
            void loadDetail(id);
            setTab("conventions");
          }}
          canFileOneDrive={Boolean(permissions?.canFileToOneDrive && od.oneDriveEnabled)}
          oneDriveConnected={od.connected}
          onFileOneDrive={(id) => void fileConventionToOneDrive(id)}
          filingConventionId={filingConventionId}
        />
      )}

      {tab === "board" && board && (
        <StagesBoardPanel
          board={board}
          permissions={permissions}
          onLoadDetail={(id) => void loadDetail(id)}
        />
      )}

      {tab === "settings" && permissions?.canManageStageSettings && (
        <StagesSettingsPanel onSavedMsg={setMsg} />
      )}

      {tab === "conventions" && (
        <StagesConventionsPanel
          conventions={conventions}
          permissions={permissions}
          oneDriveEnabled={Boolean(od.oneDriveEnabled)}
          oneDriveConnected={od.connected}
          filingConventionId={filingConventionId}
          busy={busy}
          onLoadDetail={(id) => void loadDetail(id)}
          onFileOneDrive={(id) => void fileConventionToOneDrive(id)}
        />
      )}

      {detail && (
        <section className="mt-10 rounded-2xl border border-[#2F6B4A]/20 bg-white p-6 shadow-sm">
          <h2 className="text-xl font-bold text-[#1F3D2B]">Détail convention</h2>
          <p className="text-sm text-stone-600 mt-1">
            {detail.convention.stageLabel ? `${detail.convention.stageLabel} · ` : ""}
            {detail.convention.student.firstName} {detail.convention.student.lastName} →{" "}
            {detail.convention.company.name} ·{" "}
            {STAGE_CONVENTION_STATUS_LABELS[detail.convention.status]}
          </p>
          <div className="mt-4 max-w-md">
            <StageSignatureProgress summary={buildSignatureSummary(detail.convention)} />
          </div>
          {detail.studentLink && (
            <p className="mt-3 text-sm break-all">
              <span className="font-semibold">Lien élève :</span>{" "}
              <a className="text-[#2F6B4A] underline" href={detail.studentLink}>
                {typeof window !== "undefined" ? window.location.origin : ""}
                {detail.studentLink}
              </a>
            </p>
          )}
          <div className="mt-4 flex flex-wrap gap-2">
            {detail.convention.uploadedPdf && (
              <a
                href={`/api/stages/conventions/${detail.convention.id}/uploaded-pdf`}
                target="_blank"
                rel="noopener noreferrer"
                className="rounded-lg border border-emerald-300 bg-emerald-50 px-4 py-2 text-sm font-semibold text-emerald-900 hover:bg-emerald-100"
              >
                Voir le PDF déposé
              </a>
            )}
            <a
              href={`/api/stages/conventions/${detail.convention.id}/pdf`}
              className="rounded-lg border border-stone-300 px-4 py-2 text-sm font-semibold text-stone-800 hover:bg-stone-50"
            >
              Télécharger PDF généré
            </a>
            {permissions?.canReviewPreconvention && detail.convention.status === "signatures_pending" && (
              <button
                type="button"
                onClick={() => void resendSignatures()}
                disabled={busy}
                className="rounded-lg border border-[#2F6B4A] px-4 py-2 text-sm font-semibold text-[#2F6B4A] disabled:opacity-50"
              >
                Renvoyer les e-mails de signature
              </button>
            )}
          </div>

          {detail.convention.ocrMeta && permissions?.canReviewPreconvention && (
            <div className="mt-4 rounded-xl border border-amber-200 bg-amber-50/80 p-4">
              <h3 className="text-sm font-bold text-[#1F3D2B]">Rattachement élève (eleves.json)</h3>
              <p className="mt-1 text-xs text-stone-600">
                L&apos;élève n&apos;a pas besoin de connaître son INE — le nom/prénom suffit en général. Si le
                matching est ambigu, vous pouvez préciser l&apos;INE ici (lu sur le PDF ou dans Pronote).
              </p>
              {detail.eleveMatch?.matchedEleve ? (
                <p className="mt-2 text-sm text-emerald-900">
                  Correspondance :{" "}
                  <strong>
                    {detail.eleveMatch.matchedEleve.prenom} {detail.eleveMatch.matchedEleve.nom}
                  </strong>
                  {detail.eleveMatch.matchedEleve.ine
                    ? ` (INE ${detail.eleveMatch.matchedEleve.ine})`
                    : ""}
                  {detail.eleveMatch.matchedEleve.folderName
                    ? ` — dossier « ${detail.eleveMatch.matchedEleve.folderName} »`
                    : ""}
                </p>
              ) : (
                <p className="mt-2 text-sm text-amber-900">Aucune correspondance fiable pour l&apos;instant.</p>
              )}
              <div className="mt-3 flex flex-wrap items-end gap-2">
                <label className="text-xs font-semibold text-stone-700">
                  INE (optionnel)
                  <input
                    type="text"
                    value={attachIne}
                    onChange={(e) => setAttachIne(e.target.value)}
                    placeholder="ex. 180123456AB"
                    className="mt-1 block w-48 rounded-lg border border-stone-300 px-3 py-2 text-sm"
                  />
                </label>
                <button
                  type="button"
                  onClick={() => void attachEleveIne()}
                  disabled={busy}
                  className="rounded-lg border border-[#2F6B4A] px-4 py-2 text-sm font-semibold text-[#2F6B4A] disabled:opacity-50"
                >
                  Enregistrer le rattachement
                </button>
              </div>
            </div>
          )}

          {canShowEleveDossierFiling && (
            <div className="mt-6 rounded-xl border border-indigo-200 bg-indigo-50/40 p-4">
              <h3 className="text-sm font-bold text-[#1F3D2B]">Dossier élève (intranet)</h3>
              <p className="mt-1 text-xs text-stone-600">
                Une fois toutes les signatures recueillies, la convention signée est déposée
                automatiquement dans le tiroir <strong>Scolaire</strong> du dossier élève.
              </p>

              {detail.convention.eleveDossierFilingPending &&
                !detail.convention.eleveDossierFiling && (
                  <div className="mt-3 rounded-lg border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900">
                    <p className="font-semibold">Dépôt dossier élève en attente</p>
                    {detail.convention.eleveDossierFilingError && (
                      <p className="mt-1 text-xs">{detail.convention.eleveDossierFilingError}</p>
                    )}
                    <button
                      type="button"
                      disabled={busy}
                      onClick={() => void fileToEleveDossier()}
                      className="mt-3 rounded-lg bg-[#2F6B4A] px-4 py-2 text-xs font-semibold text-white disabled:opacity-50"
                    >
                      Réessayer le dépôt dossier élève
                    </button>
                  </div>
                )}

              {detail.convention.eleveDossierFiling ? (
                <div className="mt-3 rounded-lg border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-900">
                  <p className="font-semibold">Enregistrée dans le dossier élève</p>
                  <p className="mt-1">{detail.convention.eleveDossierFiling.title}</p>
                  <p className="mt-1 text-xs text-emerald-800">
                    Par {detail.convention.eleveDossierFiling.filedBy} le{" "}
                    {new Date(detail.convention.eleveDossierFiling.filedAt).toLocaleString("fr-FR")}
                  </p>
                  <a
                    href={`/eleves/dossier/${detail.convention.eleveDossierFiling.eleveId}`}
                    className="mt-2 inline-block text-xs font-semibold text-[#2F6B4A] underline"
                  >
                    Ouvrir le dossier élève →
                  </a>
                </div>
              ) : (
                !detail.convention.eleveDossierFilingPending && (
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => void fileToEleveDossier()}
                    className="mt-3 rounded-lg border border-indigo-300 bg-white px-4 py-2 text-sm font-semibold text-indigo-900 disabled:opacity-50"
                  >
                    Déposer dans le dossier élève
                  </button>
                )
              )}
            </div>
          )}

          {canShowOneDriveFiling && (
            <div className="mt-6 rounded-xl border border-slate-200 bg-slate-50 p-4">
              <h3 className="text-sm font-bold text-[#1F3D2B]">Dépôt dossier élève (OneDrive)</h3>
              <p className="mt-1 text-xs text-stone-600">
                Après signature complète, dépôt automatique dans le dossier OneDrive de l&apos;élève (si configuré).
                Le PDF de transition est retiré de S3 une fois le dépôt réussi.
              </p>

              {detail.convention.oneDriveFilingPending && !detail.convention.oneDriveFiling && (
                <div className="mt-3 rounded-lg border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900">
                  <p className="font-semibold">Dépôt automatique en attente</p>
                  {detail.convention.oneDriveFilingError && (
                    <p className="mt-1 text-xs">{detail.convention.oneDriveFilingError}</p>
                  )}
                </div>
              )}

              {detail.eleveMatch?.targetOneDriveLabel && (
                <p className="mt-2 text-xs text-stone-600">
                  Arborescence cible : <strong>{detail.eleveMatch.targetOneDriveLabel}</strong>
                  {detail.eleveMatch.folderPath ? (
                    <>
                      {" "}
                      — <span className="font-mono">{detail.eleveMatch.folderPath}</span>
                    </>
                  ) : null}
                </p>
              )}

              {detail.convention.oneDriveFiling ? (
                <div className="mt-3 rounded-lg border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-900">
                  <p className="font-semibold">Déjà déposée</p>
                  <p className="mt-1">
                    {detail.convention.oneDriveFiling.folderPath}/{detail.convention.oneDriveFiling.fileName}
                  </p>
                  <p className="mt-1 text-xs text-emerald-800">
                    Par {detail.convention.oneDriveFiling.filedBy} le{" "}
                    {new Date(detail.convention.oneDriveFiling.filedAt).toLocaleString("fr-FR")}
                    {detail.convention.oneDriveFiling.matchedFolderName
                      ? ` — dossier ${detail.convention.oneDriveFiling.matchedFolderName}`
                      : ""}
                  </p>
                </div>
              ) : (
                <>
                  {sessionUser && oneDriveProfile && detail.eleveMatch?.secteur && oneDriveProfile.secteur !== detail.eleveMatch.secteur && (
                    <p className="mt-3 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-900">
                      Cette convention concerne le secteur « {detail.eleveMatch.targetOneDriveLabel ?? detail.eleveMatch.secteur} » — connectez-vous avec le compte Microsoft correspondant.
                    </p>
                  )}
                  {sessionUser && !oneDriveProfile && (
                    <p className="mt-3 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-900">
                      Profil OneDrive non reconnu pour votre compte — le dépôt utilisera l&apos;arborescence du secteur élève si configurée.
                    </p>
                  )}
                  {oneDriveProfile && (
                    <p className="mt-2 text-xs text-stone-600">
                      Dossier configuré : <strong>{oneDriveProfile.label}</strong> —{" "}
                      <span className="font-mono">{oneDriveProfile.basePath}</span>
                    </p>
                  )}
                  {!od.oneDriveEnabled && od.msalReady && (
                    <p className="mt-3 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-900">
                      OneDrive n&apos;est pas activé pour cet établissement (Paramètres → Intégrations).
                    </p>
                  )}
                  {od.oneDriveEnabled && (
                    <div className="mt-3 flex flex-wrap items-center gap-2">
                      {od.connected ? (
                        <span className="rounded-lg border border-emerald-200 bg-emerald-50 px-3 py-1.5 text-xs font-semibold text-emerald-800">
                          OneDrive connecté{od.accountLabel ? ` (${od.accountLabel})` : ""}
                        </span>
                      ) : (
                        <button
                          type="button"
                          onClick={() => void od.login()}
                          disabled={!od.msalReady || od.checking}
                          className="rounded-lg bg-blue-600 px-4 py-2 text-sm font-semibold text-white hover:bg-blue-700 disabled:opacity-50"
                        >
                          Se connecter à OneDrive
                        </button>
                      )}
                      <button
                        type="button"
                        onClick={() => void fileToOneDrive()}
                        disabled={
                          busy ||
                          od.checking ||
                          !od.msalReady ||
                          !od.oneDriveEnabled
                        }
                        className="rounded-lg bg-[#2F6B4A] px-4 py-2 text-sm font-semibold text-white disabled:opacity-50"
                      >
                        {busy || od.checking ? "Envoi…" : "Envoyer vers dossier élève"}
                      </button>
                    </div>
                  )}
                  {od.error && (
                    <p className="mt-2 text-xs text-rose-700">{od.error}</p>
                  )}
                </>
              )}
            </div>
          )}
          {permissions?.canReviewPreconvention && detail.convention.status === "convention_deposited" && (
            <div className="mt-4 flex gap-2">
              <button
                type="button"
                onClick={() => void adminReview(true)}
                className="rounded-lg bg-emerald-600 px-4 py-2 text-sm font-semibold text-white"
              >
                Valider → lancer les signatures
              </button>
              <button
                type="button"
                onClick={() => void adminReview(false)}
                className="rounded-lg bg-amber-600 px-4 py-2 text-sm font-semibold text-white"
              >
                Refuser
              </button>
            </div>
          )}
          {permissions?.canReviewPreconvention && detail.convention.status === "admin_review" && (
            <div className="mt-6 space-y-4">
              <div className="flex flex-wrap items-center gap-2">
                <button
                  type="button"
                  onClick={() => setAdminEditing((v) => !v)}
                  className="rounded-lg border border-stone-300 px-4 py-2 text-sm font-semibold text-stone-800"
                >
                  {adminEditing ? "Masquer l'éditeur" : "Modifier la préconvention"}
                </button>
              </div>
              {adminEditing && (
                <StagePreconventionForm
                  convention={detail.convention}
                  onChange={(next) => setDetail({ ...detail, convention: next })}
                  onSave={() => void adminSavePreconvention()}
                  onSubmit={() => void adminSavePreconvention()}
                  busy={busy}
                  identityLocked={Boolean(detail.convention.ocrMeta?.matchedEleveIne)}
                  showAdminHint
                />
              )}
              <label className="block text-sm max-w-xl">
                Motif (si renvoi pour correction)
                <textarea
                  className="mt-1 w-full rounded-lg border border-stone-300 px-3 py-2 text-sm min-h-[72px]"
                  placeholder="Ex. Les dates de stage ne correspondent pas à la période officielle…"
                  value={adminReviewNote}
                  onChange={(e) => setAdminReviewNote(e.target.value)}
                />
              </label>
              <div className="flex gap-2">
                <button
                  type="button"
                  onClick={() => void adminReview(true)}
                  className="rounded-lg bg-emerald-600 px-4 py-2 text-sm font-semibold text-white"
                >
                  Valider → lancer signatures
                </button>
                <button
                  type="button"
                  onClick={() => void adminReview(false)}
                  className="rounded-lg bg-amber-600 px-4 py-2 text-sm font-semibold text-white"
                >
                  Renvoyer pour correction
                </button>
              </div>
            </div>
          )}
          {detail.convention.signatures.some((s) => s.reviewStatus === "pending") &&
            permissions?.canReviewPreconvention && (
              <div className="mt-6 rounded-xl border border-amber-200 bg-amber-50 p-4">
                <h3 className="text-sm font-bold text-[#1F3D2B]">Signatures à valider</h3>
                <p className="mt-1 text-xs text-stone-600">
                  Vérifiez les signatures déposées (doigt ou document papier) avant de clôturer la
                  convention.
                </p>
                <ul className="mt-3 space-y-3">
                  {detail.convention.signatures
                    .filter((s) => s.reviewStatus === "pending")
                    .map((s) => (
                      <li
                        key={s.id}
                        className="flex flex-wrap items-center gap-2 rounded-lg border border-amber-100 bg-white px-3 py-2 text-sm"
                      >
                        <span className="font-semibold">{s.label}</span>
                        <span className="text-xs text-stone-500">
                          {s.signMethod === "paper_upload"
                            ? "Document papier"
                            : s.signMethod === "touch"
                              ? "Signature au doigt"
                              : "Code e-mail"}
                          {s.signedBy ? ` · ${s.signedBy}` : ""}
                        </span>
                        <button
                          type="button"
                          disabled={busy}
                          onClick={() => void reviewSignature(s.id, true)}
                          className="rounded-lg bg-emerald-700 px-3 py-1 text-xs font-bold text-white disabled:opacity-50"
                        >
                          Accepter
                        </button>
                        <button
                          type="button"
                          disabled={busy}
                          onClick={() => {
                            const note = window.prompt(
                              "Motif du refus (envoyé au signataire) :",
                              "Signature illisible ou absente sur le document.",
                            );
                            if (note === null) return;
                            void reviewSignature(s.id, false, note);
                          }}
                          className="rounded-lg bg-amber-600 px-3 py-1 text-xs font-bold text-white disabled:opacity-50"
                        >
                          Refuser et redemander
                        </button>
                      </li>
                    ))}
                </ul>
              </div>
            )}
          {detail.signLinks?.length > 0 && (
            <div className="mt-6">
              <h3 className="text-sm font-bold text-stone-700">Liens de signature</h3>
              <ul className="mt-2 space-y-2 text-sm">
                {detail.signLinks.map((s) => {
                  const pending = detail.convention.signatures.find(
                    (sig) => sig.role === s.role && sig.status === "en_attente" && sig.signToken,
                  );
                  return (
                    <li
                      key={s.link}
                      className="flex flex-wrap items-center gap-2 rounded-lg border border-stone-100 bg-stone-50 px-3 py-2"
                    >
                      <span className="font-medium">{s.label}</span>
                      {s.email ? <span className="text-stone-500">({s.email})</span> : null}
                      <a href={s.link} className="text-[#2F6B4A] underline break-all text-xs">
                        Lien
                      </a>
                      {pending && permissions?.canReviewPreconvention && (
                        <button
                          type="button"
                          disabled={busy}
                          onClick={() => void resendSignature(pending.id)}
                          className="rounded-lg border border-[#2F6B4A] px-2 py-1 text-xs font-semibold text-[#2F6B4A] disabled:opacity-50"
                        >
                          Relancer
                        </button>
                      )}
                    </li>
                  );
                })}
              </ul>
            </div>
          )}
        </section>
      )}
    </ModulePageShell>
  );
}

export default function StagesPage() {
  return (
    <Suspense
      fallback={
        <ModulePageShell maxWidthClass="max-w-[1400px]">
          <p>Chargement…</p>
        </ModulePageShell>
      }
    >
      <StagesContent />
    </Suspense>
  );
}
