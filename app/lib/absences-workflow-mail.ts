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
  formatHoursTreatmentCreatorMailLine,
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

async function absenceAppLink(view: "a-traiter" | "traitement" | "se-declarer") {
  return tenantAbsolutePath(`/rh?tab=dashboard&section=absences&view=${view}`);
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
  const absencesLink = await absenceAppLink("a-traiter");
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
      `Action attendue : Valider / Refuser dans l’application.`,
      `Après votre accord, le calendrier est mis à jour et le dossier passe à la personne qui traite (rectorat / RH) dans l’intranet.`,
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
  let recipients: string[] = [];
  try {
    recipients = await resolveAbsenceValidationRecipients(record);
  } catch (err) {
    console.error("Absences validation recipients error:", err);
    return { sent: false, recipients: [] };
  }
  const scope = resolveAbsenceScope(record);
  if (recipients.length === 0) {
    if (scope === "professeur") {
      console.warn(
        "Absences — aucun destinataire rectorat / ONISE configuré (Absences → Paramétrage).",
      );
    }
    return { sent: false, recipients };
  }

  let mail: Awaited<ReturnType<typeof getMailer>> = null;
  try {
    mail = await getMailer();
  } catch (err) {
    console.error("Absences validation mailer error:", err);
    return { sent: false, recipients };
  }
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
      ? "Une absence personnel OGEC a été validée par la direction. Elle figure au calendrier. Merci de la traiter dans l’application (pièces, déclaration, clôture)."
      : fromAccueil
        ? "Une absence professeur déclarée à l’accueil a été validée par la direction. Elle figure au calendrier. Traitez le dossier dans l’application (déclaration rectorat / instance, pièces si besoin)."
        : "Une absence professeur a été validée par la direction. Elle figure au calendrier. Traitez le dossier dans l’application.";

  const treatLink = await absenceAppLink("traitement");

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
        ``,
        `Traiter l’absence dans l’application :`,
        treatLink,
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

export async function notifyAbsenceCreatorValidated(record: AbsenceRecord): Promise<void> {
  const email = record.createdBy.email?.trim();
  if (!email) return;
  const mail = await getMailer();
  if (!mail) return;
  const link = await absenceAppLink("se-declarer");
  const scope = resolveAbsenceScope(record);
  const treatment = record.hoursTreatment
    ? formatHoursTreatmentCreatorMailLine(record.hoursTreatment, scope)
    : "";
  await mail.transporter.sendMail({
    from: `"Absences" <${mail.smtp.user}>`,
    to: email,
    subject: "La direction a validé votre absence",
    text: [
      `Bonjour ${record.createdBy.name},`,
      ``,
      `La direction a validé votre absence.`,
      ``,
      `Période : ${formatAbsencePeriod(record.data)}`,
      `Motif : ${record.data.reason}`,
      record.data.details ? `Détails : ${record.data.details}` : "",
      treatment,
      ``,
      `Le dossier est maintenant chez la personne qui assure le traitement administratif (rectorat / RH). Vous pouvez suivre l’avancement et déposer une pièce si elle vous est demandée :`,
      link,
      ``,
      `Cordialement,`,
      `L'établissement`,
    ]
      .filter(Boolean)
      .join("\n"),
  });
}

export async function notifyAbsenceJustificatifRequested(input: {
  record: AbsenceRecord;
  fromProcessor: boolean;
  note?: string;
}): Promise<void> {
  const email = input.record.createdBy.email?.trim();
  if (!email) return;
  const mail = await getMailer();
  if (!mail) return;
  const link = await absenceAppLink("se-declarer");
  const who = input.fromProcessor
    ? "La personne en charge du traitement administratif"
    : "La direction";
  const hasFile = Boolean(input.record.justification?.fileName);
  await mail.transporter.sendMail({
    from: `"Absences" <${mail.smtp.user}>`,
    to: email,
    subject: hasFile ? "Complément de justificatif demandé" : "Pièce justificative demandée",
    text: [
      `Bonjour ${input.record.createdBy.name},`,
      ``,
      hasFile
        ? `${who} a besoin d’un complément ou d’un autre document (${input.record.justification?.fileName}).`
        : `${who} vous demande une pièce justificative pour finaliser le dossier (par exemple un arrêt maladie, selon le cas).`,
      input.fromProcessor
        ? `La direction a déjà validé l’absence : cette demande ne repasse pas par elle.`
        : "",
      ``,
      `Période : ${formatAbsencePeriod(input.record.data)}`,
      `Motif : ${input.record.data.reason}`,
      input.note ? `Message : ${input.note}` : "",
      ``,
      `Déposez le document dans l’application :`,
      link,
      ``,
      `Cordialement,`,
      `L'établissement`,
    ]
      .filter(Boolean)
      .join("\n"),
  });
}

export async function notifyAbsenceJustificatifDeposited(record: AbsenceRecord): Promise<void> {
  const recipients = await resolveAbsenceValidationRecipients(record);
  if (recipients.length === 0) return;
  const mail = await getMailer();
  if (!mail) return;
  const link = await absenceAppLink("traitement");
  const mailAttachments = await loadAbsenceValidationAttachments(record);
  await mail.transporter.sendMail({
    from: `"Absences" <${mail.smtp.user}>`,
    to: recipients.join(","),
    subject: `Pièce déposée — ${record.displayName || record.createdBy.name}`,
    text: [
      `Bonjour,`,
      ``,
      `Une pièce justificative a été déposée pour une absence déjà validée par la direction.`,
      `Personne : ${record.displayName || record.createdBy.name}`,
      `Période : ${formatAbsencePeriod(record.data)}`,
      `Motif : ${record.data.reason}`,
      formatJustificatifMailLine(record, mailAttachments),
      ``,
      `Traiter le dossier dans l’application :`,
      link,
    ]
      .filter(Boolean)
      .join("\n"),
    attachments: mailAttachments.map((a) => ({
      filename: a.filename,
      content: a.content,
      contentType: a.contentType,
    })),
  });
}

export async function notifyAbsenceAdminTreated(record: AbsenceRecord): Promise<void> {
  const email = record.createdBy.email?.trim();
  if (!email) return;
  const mail = await getMailer();
  if (!mail) return;
  const link = await absenceAppLink("se-declarer");
  const scope = resolveAbsenceScope(record);
  await mail.transporter.sendMail({
    from: `"Absences" <${mail.smtp.user}>`,
    to: email,
    subject: "Votre absence a été traitée",
    text: [
      `Bonjour ${record.createdBy.name},`,
      ``,
      scope === "ogec"
        ? `La RH a clôturé le traitement administratif de votre absence.`
        : `Le traitement administratif de votre absence est terminé (déclaration rectorat / instance si nécessaire).`,
      ``,
      `Période : ${formatAbsencePeriod(record.data)}`,
      `Motif : ${record.data.reason}`,
      record.adminNote ? `Note : ${record.adminNote}` : "",
      ``,
      `Suivi dans l’application :`,
      link,
      ``,
      `Cordialement,`,
      `L'établissement`,
    ]
      .filter(Boolean)
      .join("\n"),
  });
}
