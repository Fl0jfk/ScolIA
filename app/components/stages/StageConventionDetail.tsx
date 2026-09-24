"use client";

import type { ReactNode } from "react";
import type { StageConvention } from "@/app/lib/stage-types";
import {
  STAGE_CONVENTION_STATUS_LABELS,
  STAGE_OFFER_KIND_LABELS,
  formatCompanyAddress,
  isOfflinePaperConvention,
  stageCompanyRhDisplayName,
} from "@/app/lib/stage-types";
import StagePreconventionForm from "@/app/components/stages/StagePreconventionForm";
import StageSchedulePanel from "@/app/components/stages/StageSchedulePanel";
import StageSignatureProgress from "@/app/components/stages/StageSignatureProgress";
import { buildSignatureSummary } from "@/app/lib/stage-signature-summary";
import { formatPeriodRangeFr } from "@/app/lib/stage-schedule";
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
  onRevokeSignature,
  onAddSignatory,
  onFileToEleveDossier,
  onFileToOneDrive,
  onReviewTutorEmailChange,
  onReviewScheduleChange,
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
  onRevokeSignature: (signatureId: string) => void;
  onAddSignatory: () => void;
  onFileToEleveDossier: () => void;
  onFileToOneDrive: () => void;
  onReviewTutorEmailChange: (approved: boolean) => void;
  onReviewScheduleChange: (approved: boolean) => void;
}) {
  const c = detail.convention;
  const offlinePaper = isOfflinePaperConvention(c);
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
            {offlinePaper ? " · hors plateforme" : ""}
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

      {offlinePaper && (
        <p className="rounded-lg border border-sky-200 bg-sky-50 px-3 py-2 text-xs text-sky-950">
          Convention enregistrée hors plateforme (PDF déjà signé sur papier). Aucun circuit de
          signatures électroniques n&apos;a été lancé.
        </p>
      )}

      <div className="max-w-md">
        <StageSignatureProgress summary={buildSignatureSummary(c)} />
      </div>

      {!adminEditing && <ConventionInfoSummary convention={c} />}

      {permissions?.canReviewPreconvention && c.tutorEmailChangeRequest && (
        <div className="rounded-xl border-2 border-amber-400 bg-amber-50 px-4 py-3 space-y-3">
          <p className="text-sm font-black text-amber-950">
            Demande élève — changer l&apos;e-mail du tuteur
          </p>
          <p className="text-xs text-amber-900 leading-relaxed">
            Actuel : <strong>{c.tutorEmailChangeRequest.previousEmail || "—"}</strong>
            <br />
            Demandé : <strong>{c.tutorEmailChangeRequest.requestedEmail}</strong>
            {c.tutorEmailChangeRequest.note ? (
              <>
                <br />
                Motif : {c.tutorEmailChangeRequest.note}
              </>
            ) : null}
          </p>
          <p className="text-[11px] text-amber-800">
            Validez uniquement si l&apos;adresse est correcte. Ensuite le tuteur sera relancé sur la
            nouvelle adresse (aucun changement tant que vous n&apos;avez pas validé).
          </p>
          <div className="flex flex-wrap gap-2">
            <button
              type="button"
              disabled={busy}
              onClick={() => onReviewTutorEmailChange(true)}
              className="rounded-lg bg-[#2F6B4A] px-3 py-1.5 text-xs font-bold text-white disabled:opacity-50"
            >
              Valider et relancer le tuteur
            </button>
            <button
              type="button"
              disabled={busy}
              onClick={() => onReviewTutorEmailChange(false)}
              className="rounded-lg border border-rose-300 bg-white px-3 py-1.5 text-xs font-semibold text-rose-800 disabled:opacity-50"
            >
              Refuser
            </button>
          </div>
        </div>
      )}

      {permissions?.canReviewPreconvention && c.scheduleChangeRequest && (
        <div className="rounded-xl border-2 border-orange-400 bg-orange-50 px-4 py-3 space-y-3">
          <p className="text-sm font-black text-orange-950">
            Demande tuteur — modifier période / jours / horaires
          </p>
          <p className="text-xs text-orange-900 leading-relaxed">
            Demandé par :{" "}
            <strong>{c.scheduleChangeRequest.requestedByLabel || "Tuteur entreprise"}</strong>
            {c.scheduleChangeRequest.note ? (
              <>
                <br />
                Motif : {c.scheduleChangeRequest.note}
              </>
            ) : null}
          </p>
          <div className="grid gap-3 lg:grid-cols-2">
            <div>
              <p className="mb-1 text-[11px] font-bold uppercase tracking-wide text-orange-800">
                Actuel
              </p>
              <StageSchedulePanel
                periodLabel={
                  formatPeriodRangeFr(
                    c.scheduleChangeRequest.previousSchedule.periodStart,
                    c.scheduleChangeRequest.previousSchedule.periodEnd,
                  ) ||
                  `${c.scheduleChangeRequest.previousSchedule.periodStart} → ${c.scheduleChangeRequest.previousSchedule.periodEnd}`
                }
                days={c.scheduleChangeRequest.previousSchedule.days || []}
                mode={c.scheduleChangeRequest.previousSchedule.mode}
              />
            </div>
            <div>
              <p className="mb-1 text-[11px] font-bold uppercase tracking-wide text-orange-800">
                Demandé
              </p>
              <StageSchedulePanel
                periodLabel={
                  formatPeriodRangeFr(
                    c.scheduleChangeRequest.requestedSchedule.periodStart,
                    c.scheduleChangeRequest.requestedSchedule.periodEnd,
                  ) ||
                  `${c.scheduleChangeRequest.requestedSchedule.periodStart} → ${c.scheduleChangeRequest.requestedSchedule.periodEnd}`
                }
                days={c.scheduleChangeRequest.requestedSchedule.days || []}
                mode={c.scheduleChangeRequest.requestedSchedule.mode}
              />
            </div>
          </div>
          <p className="text-[11px] text-orange-900 leading-relaxed">
            Si vous validez : le PDF est régénéré, <strong>toutes les signatures</strong> (déjà
            déposées ou en attente) sont annulées, et chaque signataire reçoit un nouveau lien.
          </p>
          <div className="flex flex-wrap gap-2">
            <button
              type="button"
              disabled={busy}
              onClick={() => onReviewScheduleChange(true)}
              className="rounded-lg bg-[#2F6B4A] px-3 py-1.5 text-xs font-bold text-white disabled:opacity-50"
            >
              Valider et relancer toutes les signatures
            </button>
            <button
              type="button"
              disabled={busy}
              onClick={() => onReviewScheduleChange(false)}
              className="rounded-lg border border-rose-300 bg-white px-3 py-1.5 text-xs font-semibold text-rose-800 disabled:opacity-50"
            >
              Refuser
            </button>
          </div>
        </div>
      )}

      {c.stageAbsenceIds && c.stageAbsenceIds.length > 0 && c.schedule.periodStart && c.schedule.periodEnd && (
        <p className="rounded-lg border border-sky-200 bg-sky-50 px-3 py-2 text-xs text-sky-950">
          Absence motif stage déclarée à l&apos;accueil{" "}
          <strong>
            {formatPeriodRangeFr(c.schedule.periodStart, c.schedule.periodEnd) ||
              `${c.schedule.periodStart} → ${c.schedule.periodEnd}`}
          </strong>
          .
        </p>
      )}

      <div className="flex flex-wrap gap-2">
        {c.uploadedPdf && (
          <a
            href={`/api/stages/conventions/${c.id}/uploaded-pdf?t=${encodeURIComponent(c.updatedAt || c.id)}`}
            target="_blank"
            rel="noopener noreferrer"
            className="rounded-lg border border-emerald-300 bg-emerald-50 px-3 py-1.5 text-xs font-semibold text-emerald-900 hover:bg-emerald-100"
          >
            Télécharger PDF signé
          </a>
        )}
        <a
          href={`/api/stages/conventions/${c.id}/pdf?t=${encodeURIComponent(c.updatedAt || c.id)}`}
          className="rounded-lg border border-stone-300 bg-white px-3 py-1.5 text-xs font-semibold text-stone-800 hover:bg-stone-50"
        >
          Télécharger PDF à jour
        </a>
        {permissions?.canReviewPreconvention &&
          !offlinePaper &&
          c.status === "signatures_pending" && (
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
          !offlinePaper &&
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
          hint="La convention signée est déposée automatiquement dans le tiroir Scolaire."
        >
          {c.eleveDossierFiling ? (
            <div className="rounded-lg border border-emerald-200 bg-emerald-50 px-3 py-2 text-xs text-emerald-900">
              <p className="font-semibold">Déjà déposée dans le dossier scolaire</p>
              <p className="mt-1">{c.eleveDossierFiling.title}</p>
              <a
                href={`/eleves/dossier/${c.eleveDossierFiling.eleveId}`}
                className="mt-1 inline-block font-semibold text-[#2F6B4A] underline"
              >
                Ouvrir le dossier →
              </a>
            </div>
          ) : c.eleveDossierFilingPending ? (
            <div className="rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-900">
              <p className="font-semibold">Dépôt automatique en attente</p>
              {c.eleveDossierFilingError && <p className="mt-1">{c.eleveDossierFilingError}</p>}
              <button
                type="button"
                disabled={busy}
                onClick={onFileToEleveDossier}
                className="mt-2 rounded-lg bg-[#2F6B4A] px-3 py-1.5 text-xs font-semibold text-white disabled:opacity-50"
              >
                Réessayer le dépôt
              </button>
            </div>
          ) : (
            <p className="rounded-lg border border-stone-200 bg-white px-3 py-2 text-xs text-stone-600">
              Dépôt automatique en cours ou prévu dès que l&apos;élève est bien rattaché.
            </p>
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

      {(c.signatures.length > 0 || detail.signLinks?.length > 0) && (
        <div>
          <h3 className="text-xs font-bold uppercase tracking-wide text-stone-600">Signataires</h3>
          <ul className="mt-2 space-y-2">
            {(() => {
              const summary = buildSignatureSummary(c);
              const nonBlockingById = new Map(
                summary.items.map((item) => [item.id, Boolean(item.nonBlocking)]),
              );
              return c.signatures.map((sig) => {
              const pending = sig.status === "en_attente" && Boolean(sig.signToken);
              const signed = sig.status === "signe";
              const nonBlocking = nonBlockingById.get(sig.id) === true;
              return (
                <li
                  key={sig.id}
                  className={`flex flex-wrap items-center gap-2 rounded-lg border px-3 py-2 text-sm ${
                    nonBlocking
                      ? "border-stone-200 bg-stone-50 text-stone-600"
                      : "border-stone-200 bg-white"
                  }`}
                >
                  <span className={`font-medium ${nonBlocking ? "line-through decoration-stone-400" : ""}`}>
                    {sig.label}
                  </span>
                  {sig.signEmail ? (
                    <span className="text-xs text-stone-500">({sig.signEmail})</span>
                  ) : null}
                  {signed ? (
                    <span className="rounded-full bg-emerald-50 px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide text-emerald-800">
                      {sig.signMethod === "paper_upload" ? "Signé (papier)" : "Signé"}
                    </span>
                  ) : nonBlocking ? (
                    <span className="inline-flex items-center gap-1 rounded-full border border-stone-300 bg-stone-100 px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide text-stone-600">
                      <span aria-hidden className="text-sm font-black leading-none">
                        ×
                      </span>
                      Non requis
                    </span>
                  ) : (
                    <span className="rounded-full bg-amber-50 px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide text-amber-800">
                      En attente
                    </span>
                  )}
                  {permissions?.canReviewPreconvention && pending && !nonBlocking && (
                    <>
                      <button
                        type="button"
                        disabled={busy}
                        onClick={() => onResendSignature(sig.id)}
                        className="rounded-lg border border-[#2F6B4A] px-2 py-1 text-xs font-semibold text-[#2F6B4A] disabled:opacity-50"
                      >
                        Relancer
                      </button>
                      <button
                        type="button"
                        disabled={busy}
                        onClick={() => onMarkSignatureManual(sig.id)}
                        className="rounded-lg border border-stone-400 px-2 py-1 text-xs font-semibold text-stone-700 disabled:opacity-50"
                      >
                        Valider manuellement
                      </button>
                      <button
                        type="button"
                        disabled={busy}
                        onClick={() => onRemoveSignatory(sig.id)}
                        className="rounded-lg border border-rose-300 px-2 py-1 text-xs font-semibold text-rose-700 disabled:opacity-50"
                      >
                        Retirer
                      </button>
                    </>
                  )}
                  {permissions?.canReviewPreconvention && signed && (
                    <button
                      type="button"
                      disabled={busy}
                      onClick={() => onRevokeSignature(sig.id)}
                      className="rounded-lg border border-amber-500 px-2 py-1 text-xs font-semibold text-amber-800 disabled:opacity-50"
                    >
                      Demander une nouvelle signature
                    </button>
                  )}
                </li>
              );
            });
            })()}
          </ul>
        </div>
      )}
    </div>
  );
}

function ConventionInfoSummary({ convention }: { convention: StageConvention }) {
  const company = convention.company;
  const student = convention.student;
  const schedule = convention.schedule;
  const parent1 =
    convention.parentSignerEmail || student.parent1Email || student.parentEmail || "";
  const parent2 = convention.parent2SignerEmail || student.parent2Email || "";
  const kindLabel = STAGE_OFFER_KIND_LABELS[convention.internshipKind] ?? convention.internshipKind;
  const periodLabel = formatPeriodRangeFr(schedule.periodStart, schedule.periodEnd);

  return (
    <div className="rounded-xl border border-stone-200 bg-white p-4 space-y-3 text-sm text-stone-800">
      <h3 className="text-xs font-bold uppercase tracking-wide text-stone-600">
        Informations de la préconvention
      </h3>

      <dl className="grid gap-2 sm:grid-cols-2">
        <InfoRow label="Élève">
          {student.lastName} {student.firstName}
          {student.className ? ` · ${student.className}` : ""}
        </InfoRow>
        <InfoRow label="Type">{kindLabel}</InfoRow>
        {convention.stageLabel ? <InfoRow label="Période / libellé">{convention.stageLabel}</InfoRow> : null}
        <InfoRow label="Entreprise">{company.name || "—"}</InfoRow>
        <InfoRow label="Adresse">{formatCompanyAddress(company) || "—"}</InfoRow>
        {company.siret ? <InfoRow label="SIRET">{company.siret}</InfoRow> : null}
        {company.activity ? <InfoRow label="Activité">{company.activity}</InfoRow> : null}
        <InfoRow label="Tuteur">
          {company.tutorName || "—"}
          {company.tutorEmail ? (
            <>
              <br />
              <a className="text-[#2F6B4A] underline" href={`mailto:${company.tutorEmail}`}>
                {company.tutorEmail}
              </a>
            </>
          ) : null}
          {company.tutorPhone ? (
            <>
              <br />
              <span className="text-stone-600">{company.tutorPhone}</span>
            </>
          ) : null}
        </InfoRow>
        {(stageCompanyRhDisplayName(company) || company.rhEmail) ? (
          <InfoRow label="RH entreprise">
            {stageCompanyRhDisplayName(company) || null}
            {company.rhEmail?.includes("@") ? (
              <>
                {stageCompanyRhDisplayName(company) ? <br /> : null}
                <a className="text-[#2F6B4A] underline" href={`mailto:${company.rhEmail}`}>
                  {company.rhEmail}
                </a>
              </>
            ) : company.rhEmail ? (
              <>
                {stageCompanyRhDisplayName(company) ? <br /> : null}
                <span className="text-amber-800 text-xs">
                  E-mail manquant ou invalide (« {company.rhEmail} ») — corriger pour envoyer le lien
                  de signature.
                </span>
              </>
            ) : null}
          </InfoRow>
        ) : null}
        <InfoRow label="Responsable légal 1">
          {parent1 ? (
            <a className="text-[#2F6B4A] underline" href={`mailto:${parent1}`}>
              {parent1}
            </a>
          ) : (
            "—"
          )}
        </InfoRow>
        {parent2 ? (
          <InfoRow label="Responsable légal 2">
            <a className="text-[#2F6B4A] underline" href={`mailto:${parent2}`}>
              {parent2}
            </a>
          </InfoRow>
        ) : null}
        <InfoRow label="Référent">
          {convention.teacherReferent.name || "—"}
          {convention.teacherReferent.email ? (
            <>
              <br />
              <span className="text-stone-600">{convention.teacherReferent.email}</span>
            </>
          ) : null}
        </InfoRow>
      </dl>

      <StageSchedulePanel
        periodLabel={periodLabel || formatPeriodRangeFr(schedule.periodStart, schedule.periodEnd)}
        days={schedule.days || []}
        mode={schedule.mode}
      />
    </div>
  );
}

function InfoRow({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div>
      <dt className="text-[11px] font-semibold uppercase tracking-wide text-stone-500">{label}</dt>
      <dd className="mt-0.5 text-sm text-stone-900">{children}</dd>
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
