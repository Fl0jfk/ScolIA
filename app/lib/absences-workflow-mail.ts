import "server-only";

import { loadAppConfig } from "@/app/lib/app-config";
import { matchEstablishment } from "@/app/lib/establishment-catalog";
import { formatAbsencePeriod } from "@/app/lib/absence-period";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
} from "@/app/lib/tenant-mail";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";
import type { AbsenceRecord, AbsenceScope, Etablissement } from "@/app/lib/absences-types";
import { resolveAbsenceScope } from "@/app/lib/absences-types";
import { collectAbsenceValidationEmails } from "@/app/lib/absences-validation-recipients";
import {
  formatHoursTreatmentMailLine,
} from "@/app/lib/absence-hours-treatment";
import {
  formatJustificatifMailLine,
  loadAbsenceValidationAttachments,
} from "@/app/lib/absences-notify";

export type AbsenceDecisionTarget = {
  roleLabel: string;
  name: string;
  email: string;
};

export async function resolveAbsenceDecisionTarget(
  scope: AbsenceScope,
  etablissement: Etablissement | null,
): Promise<AbsenceDecisionTarget> {
  const bundle = await loadAppConfig();
  if (scope === "ogec") {
    const dirs = bundle.establishments.filter((e) => e.active !== false);
    const fallback = dirs[dirs.length - 1];
    return {
      roleLabel: fallback ? `Direction ${fallback.label}` : "Direction",
      name: fallback?.directorName || bundle.identity.name,
      email: fallback?.directorEmail || "",
    };
  }
  const est = etablissement ? matchEstablishment(bundle.establishments, etablissement) : null;
  if (est) {
    return {
      roleLabel: `Direction ${est.label}`,
      name: est.directorName || est.label,
      email: est.directorEmail || "",
    };
  }
  return { roleLabel: "Direction", name: bundle.identity.name, email: "" };
}

/** Destinataires après validation : secrétariat cycle (déclaration rectorat / ONISE) ou compta OGEC. */
export async function resolveAbsenceValidationRecipients(record: AbsenceRecord): Promise<string[]> {
  const bundle = await loadAppConfig();
  return collectAbsenceValidationEmails(record, bundle.notifications, bundle.establishments);
}

async function getMailer() {
  const smtp = await getTenantSmtpConfig();
  if (!smtp) return null;
  const transporter = await createTenantTransporter();
  if (!transporter) return null;
  return { smtp, transporter };
}

/** Mail direction : nouvelle demande (saisie accueil ou auto-déclaration). */
export async function notifyAbsenceCreated(input: {
  record: AbsenceRecord;
  actorName: string;
  fromAccueil?: boolean;
}): Promise<void> {
  const scope = input.record.data.scope;
  const etablissement = input.record.data.etablissement;
  const target = await resolveAbsenceDecisionTarget(scope, scope === "ogec" ? null : etablissement);
  if (!target.email.trim()) return;
  const mail = await getMailer();
  if (!mail) return;
  const absencesLink = await tenantAbsolutePath("/rh?tab=absences");
  const origin = input.fromAccueil ? "saisie à l'accueil (standard)" : "demande d'autorisation";
  await mail.transporter.sendMail({
    from: `"Absences" <${mail.smtp.user}>`,
    to: target.email,
    subject: `Nouvelle ${origin} — ${scope === "ogec" ? "Personnel OGEC" : `Professeur ${etablissement || ""}`}`.trim(),
    text: [
      `Bonjour ${target.name},`,
      ``,
      input.fromAccueil
        ? `L’accueil a déclaré une absence qui nécessite votre validation.`
        : `Une nouvelle demande d'autorisation d'absence nécessite votre décision.`,
      ``,
      `Type : ${scope === "ogec" ? "Personnel OGEC" : "Professeur"}`,
      `Établissement : ${scope === "ogec" ? "OGEC" : etablissement || "—"}`,
      `Personne concernée : ${input.record.displayName}`,
      `Saisie par : ${input.actorName}`,
      `Période : ${formatAbsencePeriod(input.record.data)}`,
      `Motif : ${input.record.data.reason}`,
      input.record.data.details ? `Détails : ${input.record.data.details}` : "",
      ``,
      `Action attendue : Valider / Refuser`,
      `Après validation, le calendrier absences est mis à jour et le secrétariat (déclaration rectorat / instance) est notifié.`,
      ``,
      `Espace Absences: ${absencesLink}`,
    ]
      .filter(Boolean)
      .join("\n"),
  });
}

/**
 * Après validation direction : calendrier (géré par l’appelant via calendarVisible)
 * + mail à la personne qui déclare au rectorat / ONISE (profs) ou à la compta (OGEC).
 */
export async function notifyAbsenceValidated(
  record: AbsenceRecord,
): Promise<{ sent: boolean; recipients: string[] }> {
  const recipients = await resolveAbsenceValidationRecipients(record);
  const scope = resolveAbsenceScope(record);
  if (recipients.length === 0) {
    if (scope === "professeur") {
      console.warn(
        "Absences — aucun destinataire secrétariat/rectorat configuré (Réglages → Notifications).",
      );
    }
    return { sent: false, recipients };
  }

  const mail = await getMailer();
  if (!mail) return { sent: false, recipients };

  const fromAccueil = record.source === "accueil";
  const subjectPerson = record.displayName || record.createdBy.name;
  const subject =
    scope === "professeur"
      ? fromAccueil
        ? `Absence professeur à déclarer — ${subjectPerson}`
        : `Absence professeur validée — ${subjectPerson}`
      : `Absence validée — ${subjectPerson}`;

  const intro =
    scope === "ogec"
      ? "Une absence personnel OGEC a été validée par la direction. Elle figure désormais au calendrier."
      : fromAccueil
        ? "Une absence professeur déclarée à l’accueil a été validée par la direction. Elle figure désormais au calendrier des absences professeurs. Merci d’effectuer la déclaration auprès du rectorat (ou de l’instance) selon le traitement indiqué."
        : "Une absence professeur a été validée par la direction. Elle figure désormais au calendrier des absences professeurs. Merci d’effectuer la déclaration auprès du rectorat (ou de l’instance) si le traitement l’exige.";

  try {
    const mailAttachments = await loadAbsenceValidationAttachments(record);
    const justificatifLine = formatJustificatifMailLine(record, mailAttachments);
    const treatmentLine = record.hoursTreatment
      ? formatHoursTreatmentMailLine(record.hoursTreatment, scope)
      : "";
    await mail.transporter.sendMail({
      from: `"Absences" <${mail.smtp.user}>`,
      to: recipients.join(","),
      subject,
      text: [
        `Bonjour,`,
        ``,
        intro,
        ``,
        `Personne : ${subjectPerson}`,
        `Type : ${scope === "ogec" ? "Personnel OGEC" : "Professeur"}`,
        `Établissement : ${scope === "ogec" ? "OGEC" : record.data.etablissement || "—"}`,
        fromAccueil && record.submittedBy?.name
          ? `Déclarée à l’accueil par : ${record.submittedBy.name}`
          : "",
        `Période : ${formatAbsencePeriod(record.data)}`,
        `Motif : ${record.data.reason}`,
        record.data.details ? `Détails : ${record.data.details}` : "",
        justificatifLine,
        treatmentLine,
        mailAttachments.length > 0 ? `` : undefined,
        mailAttachments.length > 0
          ? `Les justificatifs et documents sont en pièce(s) jointe(s) à ce message.`
          : undefined,
      ]
        .filter(Boolean)
        .join("\n"),
      attachments: mailAttachments.map((a) => ({
        filename: a.filename,
        content: a.content,
        contentType: a.contentType,
      })),
    });
    return { sent: true, recipients };
  } catch (err) {
    console.error("Absences validation mail error:", err);
    return { sent: false, recipients };
  }
}
