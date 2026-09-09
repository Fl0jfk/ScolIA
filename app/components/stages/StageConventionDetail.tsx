"use client";

import type { ReactNode } from "react";
import type { StageConvention } from "@/app/lib/stage-types";
import { STAGE_CONVENTION_STATUS_LABELS } from "@/app/lib/stage-types";
import StagePreconventionForm from "@/app/components/stages/StagePreconventionForm";
import StageSignatureProgress from "@/app/components/stages/StageSignatureProgress";
import { buildSignatureSummary } from "@/app/lib/stage-signature-summary";
import type { StagesHubPermissions } from "@/app/components/stages/stages-hub-types";
import type { OneDriveUserProfile } from "@/app/lib/onedrive-user-profiles";

export type StageConventionDetailData = {
  convention: StageConvention;
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
};

type OneDriveUi = {
  oneDriveEnabled: boolean;
  connected: boolean;
  msalReady: boolean;
  checking: boolean;
  accountLabel?: string | null;
  error?: string | null;
  login: () => void | Promise<void>;
};

export default function StageConventionDetail({
  detail,
  permissions,
  busy,
  adminReviewNote,
  adminEditing,
  sessionUser,
  oneDriveProfile,
  od,
  onClose,
  onAdminReviewNote,
  onAdminEditing,
  onConventionChange,
  onAdminReview,
  onAdminSave,
  onResendSignatures,
  onResendSignature,
  onMarkSignatureManual,
  onRemoveSignatory,
  onAddSignatory,
  onFileToEleveDossier,
  onFileToOneDrive,
}: {
  detail: StageConventionDetailData;
  permissions: StagesHubPermissions | undefined;
  busy: boolean;
  adminReviewNote: string;
  adminEditing: boolean;
  sessionUser: { id?: string } | null | undefined;
  oneDriveProfile: OneDriveUserProfile | null;
  od: OneDriveUi;
  onClose: () => void;
  onAdminReviewNote: (v: string) => void;
  onAdminEditing: (v: boolean) => void;
  onConventionChange: (next: StageConvention) => void;
  onAdminReview: (approved: boolean) => void;
  onAdminSave: () => void;
  onResendSignatures: () => void;
  onResendSignature: (signatureId: string) => void;
  onMarkSignatureManual: (signatureId: string) => void;
  onRemoveSignatory: (signatureId: string) => void;
  onAddSignatory: () => void;
  onFileToEleveDossier: () => void;
  onFileToOneDrive: () => void;
}) {
  const c = detail.convention;
  const canShowOneDriveFiling = permissions?.canFileToOneDrive && c.status === "signed";
  const canShowEleveDossierFiling = permissions?.canReviewPreconvention && c.status === "signed";

  return (
    <div className="mt-3 space-y-4 rounded-xl border border-[#2F6B4A]/25 bg-[#f7faf8] p-4">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div>
          <p className="text-sm font-bold text-[#1F3D2B]">
            {c.stageLabel ? `${c.stageLabel} · ` : ""}
            {c.company.name}
          </p>
          <p className="text-xs text-stone-600">
            {STAGE_CONVENTION_STATUS_LABELS[c.status]}
          </p>
        </div>
        <button
          type="button"
          onClick={onClose}
          className="rounded-lg border border-stone-300 px-3 py-1.5 text-xs font-semibold text-stone-700 hover:bg-white"
        >
          Fermer
        </button>
      </div>

      <div className="max-w-md">
        <StageSignatureProgress summary={buildSignatureSummary(c)} />
      </div>

      <div className="flex flex-wrap gap-2">
        {c.uploadedPdf && (
          <a
            href={`/api/stages/conventions/${c.id}/uploaded-pdf`}
            target="_blank"
            rel="noopener noreferrer"
            className="rounded-lg border border-emerald-300 bg-emerald-50 px-3 py-1.5 text-xs font-semibold text-emerald-900 hover:bg-emerald-100"
          >
            Télécharger PDF signé
          </a>
        )}
        <a
          href={`/api/stages/conventions/${c.id}/pdf`}
          className="rounded-lg border border-stone-300 bg-white px-3 py-1.5 text-xs font-semibold text-stone-800 hover:bg-stone-50"
        >
          Télécharger PDF
        </a>
        {permissions?.canReviewPreconvention && c.status === "signatures_pending" && (
          <button
            type="button"
            onClick={onResendSignatures}
            disabled={busy}
            className="rounded-lg border border-[#2F6B4A] px-3 py-1.5 text-xs font-semibold text-[#2F6B4A] disabled:opacity-50"
          >
            Renvoyer les e-mails
          </button>
        )}
        {permissions?.canReviewPreconvention &&
          (c.status === "signatures_pending" || c.status === "signed") && (
            <button
              type="button"
              onClick={onAddSignatory}
              disabled={busy}
              className="rounded-lg border border-indigo-300 bg-indigo-50 px-3 py-1.5 text-xs font-semibold text-indigo-900 disabled:opacity-50"
            >
              Ajouter un signataire
            </button>
          )}
      </div>

      {canShowEleveDossierFiling && (
        <FilingBlock
          title="Dossier élève (intranet)"
          hint="La convention signée est déposée dans le tiroir Scolaire du dossier élève."
        >
          {c.eleveDossierFilingPending && !c.eleveDossierFiling && (
            <div className="rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-900">
              <p className="font-semibold">Dépôt en attente</p>
              {c.eleveDossierFilingError && <p className="mt-1">{c.eleveDossierFilingError}</p>}
              <button
                type="button"
                disabled={busy}
                onClick={onFileToEleveDossier}
                className="mt-2 rounded-lg bg-[#2F6B4A] px-3 py-1.5 text-xs font-semibold text-white disabled:opacity-50"
              >
                Réessayer
              </button>
            </div>
          )}
          {c.eleveDossierFiling ? (
            <div className="rounded-lg border border-emerald-200 bg-emerald-50 px-3 py-2 text-xs text-emerald-900">
              <p className="font-semibold">{c.eleveDossierFiling.title}</p>
              <a
                href={`/eleves/dossier/${c.eleveDossierFiling.eleveId}`}
                className="mt-1 inline-block font-semibold text-[#2F6B4A] underline"
              >
                Ouvrir le dossier →
              </a>
            </div>
          ) : (
            !c.eleveDossierFilingPending && (
              <button
                type="button"
                disabled={busy}
                onClick={onFileToEleveDossier}
                className="rounded-lg border border-indigo-300 bg-white px-3 py-1.5 text-xs font-semibold text-indigo-900 disabled:opacity-50"
              >
                Déposer dans le dossier élève
              </button>
            )
          )}
        </FilingBlock>
      )}

      {canShowOneDriveFiling && (
        <FilingBlock
          title="OneDrive"
          hint="Dépôt dans le dossier OneDrive de l'élève (si configuré)."
        >
          {detail.eleveMatch?.targetOneDriveLabel && (
            <p className="text-xs text-stone-600">
              Cible : <strong>{detail.eleveMatch.targetOneDriveLabel}</strong>
              {detail.eleveMatch.folderPath ? (
                <>
                  {" "}
                  — <span className="font-mono">{detail.eleveMatch.folderPath}</span>
                </>
              ) : null}
            </p>
          )}
          {c.oneDriveFiling ? (
            <div className="rounded-lg border border-emerald-200 bg-emerald-50 px-3 py-2 text-xs text-emerald-900">
              {c.oneDriveFiling.folderPath}/{c.oneDriveFiling.fileName}
            </div>
          ) : (
            <div className="flex flex-wrap items-center gap-2">
              {sessionUser && oneDriveProfile && detail.eleveMatch?.secteur && oneDriveProfile.secteur !== detail.eleveMatch.secteur && (
                <p className="w-full rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-900">
                  Secteur « {detail.eleveMatch.targetOneDriveLabel ?? detail.eleveMatch.secteur} » —
                  connectez le compte Microsoft correspondant.
                </p>
              )}
              {od.oneDriveEnabled && (
                <>
                  {od.connected ? (
                    <span className="rounded-lg border border-emerald-200 bg-emerald-50 px-2 py-1 text-xs font-semibold text-emerald-800">
                      OneDrive connecté{od.accountLabel ? ` (${od.accountLabel})` : ""}
                    </span>
                  ) : (
                    <button
                      type="button"
                      onClick={() => void od.login()}
                      disabled={!od.msalReady || od.checking}
                      className="rounded-lg bg-blue-600 px-3 py-1.5 text-xs font-semibold text-white disabled:opacity-50"
                    >
                      Se connecter à OneDrive
                    </button>
                  )}
                  <button
                    type="button"
                    onClick={onFileToOneDrive}
                    disabled={busy || od.checking || !od.msalReady || !od.oneDriveEnabled}
                    className="rounded-lg bg-[#2F6B4A] px-3 py-1.5 text-xs font-semibold text-white disabled:opacity-50"
                  >
                    {busy || od.checking ? "Envoi…" : "Envoyer vers OneDrive"}
                  </button>
                </>
              )}
              {od.error && <p className="w-full text-xs text-rose-700">{od.error}</p>}
            </div>
          )}
        </FilingBlock>
      )}

      {permissions?.canReviewPreconvention && c.status === "convention_deposited" && (
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            onClick={() => onAdminReview(true)}
            className="rounded-lg bg-emerald-600 px-3 py-1.5 text-xs font-semibold text-white"
          >
            Valider → lancer les signatures
          </button>
          <button
            type="button"
            onClick={() => onAdminReview(false)}
            className="rounded-lg bg-amber-600 px-3 py-1.5 text-xs font-semibold text-white"
          >
            Refuser
          </button>
        </div>
      )}

      {permissions?.canReviewPreconvention && c.status === "admin_review" && (
        <div className="space-y-3">
          <button
            type="button"
            onClick={() => onAdminEditing(!adminEditing)}
            className="rounded-lg border border-stone-300 bg-white px-3 py-1.5 text-xs font-semibold text-stone-800"
          >
            {adminEditing ? "Masquer l'éditeur" : "Modifier la préconvention"}
          </button>
          {adminEditing && (
            <StagePreconventionForm
              convention={c}
              onChange={onConventionChange}
              onSave={onAdminSave}
              onSubmit={onAdminSave}
              busy={busy}
              identityLocked={Boolean(c.ocrMeta?.matchedEleveIne)}
              showAdminHint
            />
          )}
          <label className="block text-xs max-w-xl">
            Motif (si renvoi)
            <textarea
              className="mt-1 w-full rounded-lg border border-stone-300 px-3 py-2 text-sm min-h-[64px]"
              value={adminReviewNote}
              onChange={(e) => onAdminReviewNote(e.target.value)}
            />
          </label>
          <div className="flex flex-wrap gap-2">
            <button
              type="button"
              onClick={() => onAdminReview(true)}
              className="rounded-lg bg-emerald-600 px-3 py-1.5 text-xs font-semibold text-white"
            >
              Valider → lancer signatures
            </button>
            <button
              type="button"
              onClick={() => onAdminReview(false)}
              className="rounded-lg bg-amber-600 px-3 py-1.5 text-xs font-semibold text-white"
            >
              Renvoyer pour correction
            </button>
          </div>
        </div>
      )}

      {detail.signLinks?.length > 0 && (
        <div>
          <h3 className="text-xs font-bold uppercase tracking-wide text-stone-600">Signataires</h3>
          <ul className="mt-2 space-y-2">
            {detail.signLinks.map((s) => {
              const pending = c.signatures.find(
                (sig) => sig.role === s.role && sig.status === "en_attente" && sig.signToken,
              );
              return (
                <li
                  key={s.link}
                  className="flex flex-wrap items-center gap-2 rounded-lg border border-stone-200 bg-white px-3 py-2 text-sm"
                >
                  <span className="font-medium">{s.label}</span>
                  {s.email ? <span className="text-xs text-stone-500">({s.email})</span> : null}
                  {pending && permissions?.canReviewPreconvention && (
                    <>
                      <button
                        type="button"
                        disabled={busy}
                        onClick={() => onResendSignature(pending.id)}
                        className="rounded-lg border border-[#2F6B4A] px-2 py-1 text-xs font-semibold text-[#2F6B4A] disabled:opacity-50"
                      >
                        Relancer
                      </button>
                      <button
                        type="button"
                        disabled={busy}
                        onClick={() => onMarkSignatureManual(pending.id)}
                        className="rounded-lg border border-stone-400 px-2 py-1 text-xs font-semibold text-stone-700 disabled:opacity-50"
                      >
                        Valider manuellement
                      </button>
                      <button
                        type="button"
                        disabled={busy}
                        onClick={() => onRemoveSignatory(pending.id)}
                        className="rounded-lg border border-rose-300 px-2 py-1 text-xs font-semibold text-rose-700 disabled:opacity-50"
                      >
                        Retirer
                      </button>
                    </>
                  )}
                </li>
              );
            })}
          </ul>
        </div>
      )}
    </div>
  );
}

function FilingBlock({
  title,
  hint,
  children,
}: {
  title: string;
  hint: string;
  children: ReactNode;
}) {
  return (
    <div className="rounded-xl border border-stone-200 bg-white p-3 space-y-2">
      <div>
        <h3 className="text-xs font-bold text-[#1F3D2B]">{title}</h3>
        <p className="text-[11px] text-stone-500">{hint}</p>
      </div>
      {children}
    </div>
  );
}
