"use client";

import type { ReactNode } from "react";
import type { StageConvention, StageDaySlot } from "@/app/lib/stage-types";
import {
  STAGE_CONVENTION_STATUS_LABELS,
  STAGE_OFFER_KIND_LABELS,
} from "@/app/lib/stage-types";
import StagePreconventionForm from "@/app/components/stages/StagePreconventionForm";
import StageSignatureProgress from "@/app/components/stages/StageSignatureProgress";
import { buildSignatureSummary } from "@/app/lib/stage-signature-summary";
import {
  formatDaySlotTimeParts,
  formatPeriodRangeFr,
  groupStageDaysByCalendarWeek,
  STAGE_WEEKDAY_LABELS,
} from "@/app/lib/stage-schedule";
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

      {!adminEditing && <ConventionInfoSummary convention={c} />}

      {c.stageAbsenceIds && c.stageAbsenceIds.length > 0 && c.schedule.periodStart && c.schedule.periodEnd && (
        <p className="rounded-lg border border-sky-200 bg-sky-50 px-3 py-2 text-xs text-sky-950">
          Absence motif stage déclarée à l&apos;accueil du{" "}
          <strong>{c.schedule.periodStart}</strong> au <strong>{c.schedule.periodEnd}</strong>.
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
        <InfoRow label="Adresse">{company.address || "—"}</InfoRow>
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
        {company.rhEmail ? (
          <InfoRow label="RH entreprise">
            <a className="text-[#2F6B4A] underline" href={`mailto:${company.rhEmail}`}>
              {company.rhEmail}
            </a>
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

      <ScheduleHoursPanel
        periodLabel={periodLabel || `${schedule.periodStart} → ${schedule.periodEnd}`}
        days={schedule.days || []}
        mode={schedule.mode}
      />
    </div>
  );
}

function ScheduleHoursPanel({
  periodLabel,
  days,
  mode,
}: {
  periodLabel: string;
  days: StageDaySlot[];
  mode: StageConvention["schedule"]["mode"];
}) {
  const weekGroups = groupStageDaysByCalendarWeek(days);

  return (
    <div className="overflow-hidden rounded-xl border border-[#2F6B4A]/20 bg-gradient-to-br from-[#f3f8f5] via-white to-[#eef5f1]">
      <div className="flex flex-wrap items-end justify-between gap-2 border-b border-[#2F6B4A]/15 px-4 py-3">
        <div>
          <p className="text-[11px] font-bold uppercase tracking-[0.08em] text-[#2F6B4A]">
            Horaires du stage
          </p>
          <p className="mt-0.5 text-sm font-semibold capitalize text-[#1F3D2B]">{periodLabel}</p>
        </div>
        <span className="rounded-full bg-white/80 px-2.5 py-1 text-[10px] font-semibold uppercase tracking-wide text-[#2F6B4A] ring-1 ring-[#2F6B4A]/20">
          {mode === "per_day" ? "Jour par jour" : "Semaine type"}
        </span>
      </div>

      {days.length === 0 ? (
        <p className="px-4 py-4 text-xs text-stone-500">Aucun horaire renseigné.</p>
      ) : (
        <div className="space-y-4 p-3">
          {weekGroups.map((group) => (
            <div key={group.weekKey} className="space-y-2">
              {weekGroups.length > 1 && (
                <div className="flex flex-wrap items-center gap-2 px-1">
                  <span className="rounded-md bg-[#2F6B4A] px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide text-white">
                    Semaine {group.weekIndex + 1}
                  </span>
                  <span className="text-xs font-medium text-[#1F3D2B]/80">
                    {group.rangeLabel}
                  </span>
                  {group.weekIndex > 0 && (
                    <span className="ml-auto hidden h-px flex-1 bg-[#2F6B4A]/15 sm:block" aria-hidden />
                  )}
                </div>
              )}
              <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
                {group.days.map((day, index) => (
                  <DayHoursCard key={dayCardKey(day, index)} day={day} />
                ))}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function dayCardKey(day: StageDaySlot, index: number) {
  return day.date || (day.weekday != null ? `w-${day.weekday}` : `d-${index}`);
}

function dayCardTitle(day: StageDaySlot) {
  if (day.date) {
    return new Date(`${day.date}T12:00:00`).toLocaleDateString("fr-FR", {
      weekday: "long",
      day: "numeric",
      month: "short",
    });
  }
  if (day.weekday != null && STAGE_WEEKDAY_LABELS[day.weekday]) {
    return STAGE_WEEKDAY_LABELS[day.weekday];
  }
  return "Jour";
}

function DayHoursCard({ day }: { day: StageDaySlot }) {
  const times = formatDaySlotTimeParts(day);

  return (
    <div className="rounded-lg border border-white/80 bg-white/90 p-3 shadow-sm shadow-[#1F3D2B]/5 ring-1 ring-stone-200/70">
      <p className="text-xs font-bold capitalize text-[#1F3D2B]">{dayCardTitle(day)}</p>
      <div className="mt-2 space-y-1.5">
        {times.continuous ? (
          <TimeChip
            label="Journée"
            start={(day.fullDayStart || day.morningStart || "").toString()}
            end={(day.fullDayEnd || day.morningEnd || "").toString()}
            accent="full"
          />
        ) : (
          <>
            <TimeChip
              label="Matin"
              start={day.morningStart || ""}
              end={day.morningEnd || ""}
              accent="morning"
            />
            <TimeChip
              label="Après-midi"
              start={day.afternoonStart || ""}
              end={day.afternoonEnd || ""}
              accent="afternoon"
            />
          </>
        )}
      </div>
    </div>
  );
}

function TimeChip({
  label,
  start,
  end,
  accent,
}: {
  label: string;
  start: string;
  end: string;
  accent: "morning" | "afternoon" | "full";
}) {
  if (!start && !end) {
    return (
      <div className="flex items-center justify-between gap-2 rounded-md bg-stone-50 px-2 py-1.5 text-[11px] text-stone-400">
        <span>{label}</span>
        <span>—</span>
      </div>
    );
  }

  const tone =
    accent === "morning"
      ? "bg-amber-50 text-amber-950 ring-amber-200/80"
      : accent === "afternoon"
        ? "bg-sky-50 text-sky-950 ring-sky-200/80"
        : "bg-emerald-50 text-emerald-950 ring-emerald-200/80";

  return (
    <div className={`flex items-center justify-between gap-2 rounded-md px-2 py-1.5 text-[11px] ring-1 ${tone}`}>
      <span className="font-semibold opacity-80">{label}</span>
      <span className="font-mono text-xs font-bold tracking-tight">
        {start || "—"}
        <span className="mx-1 opacity-40">→</span>
        {end || "—"}
      </span>
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
