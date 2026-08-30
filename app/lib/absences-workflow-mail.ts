import "server-only";

import { loadAppConfig } from "@/app/lib/app-config";
import { matchEstablishment } from "@/app/lib/establishment-catalog";
import { inferEstablishmentKind } from "@/app/lib/establishment-visual";
import { formatAbsencePeriod } from "@/app/lib/absence-period";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
} from "@/app/lib/tenant-mail";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";
import type { AbsenceRecord, AbsenceScope, Etablissement } from "@/app/lib/absences-types";
import { isEducationSurveillanceStaff } from "@/app/lib/absences-types";

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
  const n = bundle.notifications;
  const emails = new Set<string>();

  if (record.data.scope === "ogec") {
    for (const e of n.absencesNotifyOgecCompta) {
      if (e) emails.add(e.trim().toLowerCase());
    }
    if (isEducationSurveillanceStaff(record.createdBy.roles)) {
      for (const e of n.absencesNotifySurveillanceResponsables || []) {
        if (e) emails.add(e.trim().toLowerCase());
      }
    }
    return [...emails];
  }

  const est = matchEstablishment(bundle.establishments, record.data.etablissement);
  const kind = est
    ? inferEstablishmentKind(est)
    : inferEstablishmentKind({ label: record.data.etablissement || "" });
  if (kind === "ecole") {
    if (n.absencesNotifyProfEcole?.email) emails.add(n.absencesNotifyProfEcole.email.trim().toLowerCase());
  } else if (kind === "college") {
    const email = n.absencesNotifyProfCollege?.email || n.absencesNotifyProfCollegeLycee?.email;
    if (email) emails.add(email.trim().toLowerCase());
  } else if (kind === "lycee") {
    const email = n.absencesNotifyProfLycee?.email || n.absencesNotifyProfCollegeLycee?.email;
    if (email) emails.add(email.trim().toLowerCase());
  } else if (n.absencesNotifyProfCollegeLycee?.email) {
    emails.add(n.absencesNotifyProfCollegeLycee.email.trim().toLowerCase());
  }
  return [...emails];
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
