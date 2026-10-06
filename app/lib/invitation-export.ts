/** Export invitations cérémonies — CSV Excel FR (séparateur `;`, BOM UTF-8). */

import {
  diplomaLabel,
  formatInvitationFirstName,
  formatInvitationLastName,
  situationStatusLabel,
  type InvitationRsvpRecord,
} from "@/app/lib/invitation-types";

function csvCell(value: string | number | boolean | null | undefined): string {
  const raw = value == null ? "" : String(value);
  if (/[;"\n\r]/.test(raw)) {
    return `"${raw.replace(/"/g, '""')}"`;
  }
  return raw;
}

const HEADERS = [
  "prenom_eleve",
  "nom_eleve",
  "date_naissance",
  "reponse",
  "nb_personnes",
  "diplome",
  "email",
  "situation",
  "situation_detail",
  "situation_etablissement",
  "doublon_groupe",
  "date_reponse",
] as const;

function formatParisDate(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString("fr-FR", {
    timeZone: "Europe/Paris",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function buildInvitationRsvpExportCsv(rsvps: InvitationRsvpRecord[]): string {
  const lines = [HEADERS.join(";")];
  for (const r of rsvps) {
    lines.push(
      [
        csvCell(formatInvitationFirstName(r.eleveFirstName)),
        csvCell(formatInvitationLastName(r.eleveLastName)),
        csvCell(r.birthDate || ""),
        csvCell(r.response === "oui" ? "Oui" : "Non"),
        csvCell(r.response === "oui" ? r.presentCount : 0),
        csvCell(diplomaLabel(r.diploma) || ""),
        csvCell(r.parentEmail),
        csvCell(situationStatusLabel(r.situationStatus) || ""),
        csvCell(r.situationDetail || ""),
        csvCell(r.situationEstablishment || ""),
        csvCell(r.duplicateGroupId || ""),
        csvCell(formatParisDate(r.createdAt)),
      ].join(";"),
    );
  }
  return `\uFEFF${lines.join("\r\n")}\r\n`;
}

export function invitationExportFilename(slug: string, now = new Date()): string {
  const stamp = now
    .toLocaleDateString("fr-CA", { timeZone: "Europe/Paris" })
    .replace(/-/g, "");
  const safe = slug.replace(/[^a-z0-9-_]+/gi, "-").replace(/^-+|-+$/g, "") || "invitation";
  return `invitation-${safe}-${stamp}.csv`;
}
