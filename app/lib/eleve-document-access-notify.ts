import "server-only";

import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { user, userRole } from "@/db/schema";
import { loadAppConfig } from "@/app/lib/app-config";
import { createTenantTransporter, getTenantSmtpConfig } from "@/app/lib/tenant-mail";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";
import { documentAccessDurationLabel } from "@/app/lib/eleve-document-access-duration";
import { TIROIR_LABELS } from "@/app/lib/eleve-doc-categories";

async function mailer() {
  const smtp = await getTenantSmtpConfig();
  if (!smtp) return null;
  const transporter = await createTenantTransporter();
  if (!transporter) return null;
  return { smtp, transporter };
}

/** E-mails des comptes ayant le rôle demandé (établissement courant). */
export async function listEmailsForIntranetRole(
  etablissementId: string,
  role: string,
): Promise<string[]> {
  const db = getDb();
  const rows = await db
    .select({ email: user.email })
    .from(userRole)
    .innerJoin(user, eq(user.id, userRole.userId))
    .where(
      and(
        eq(userRole.etablissementId, etablissementId),
        eq(user.etablissementId, etablissementId),
        eq(userRole.role, role),
      ),
    );
  const emails = new Set<string>();
  for (const row of rows) {
    const email = String(row.email || "").trim();
    if (email) emails.add(email);
  }
  return [...emails];
}

export type DocumentAccessOwnerKind = "psychologue" | "infirmerie" | "direction";

export function documentAccessOwnerKind(doc: {
  tiroir: string;
  title: string;
  confidentialite?: string;
}): DocumentAccessOwnerKind {
  if (doc.tiroir === "psychologue") return "psychologue";
  if (doc.tiroir === "sante") return "infirmerie";
  if (doc.confidentialite === "sante") return "infirmerie";
  return "direction";
}

export function documentAccessOwnerLabel(kind: DocumentAccessOwnerKind): string {
  if (kind === "psychologue") return "le psychologue";
  if (kind === "infirmerie") return "l’infirmerie";
  return "la direction";
}

/** Notifie le propriétaire du silo (psy / infirmerie) d’une demande d’accès. */
export async function notifyDocumentAccessOwner(input: {
  etablissementId: string;
  ownerKind: DocumentAccessOwnerKind;
  eleveNom: string;
  elevePrenom: string;
  classe?: string | null;
  documentTitle: string;
  documentTiroir: string;
  requesterName: string;
  requesterEmail?: string | null;
  durationDays: number;
  note?: string | null;
}): Promise<{ sent: boolean; to?: string[]; reason?: string }> {
  const role =
    input.ownerKind === "psychologue"
      ? "psychologue"
      : input.ownerKind === "infirmerie"
        ? "infirmerie"
        : null;
  if (!role) {
    return { sent: false, reason: "owner_is_direction_queue" };
  }

  const to = await listEmailsForIntranetRole(input.etablissementId, role);
  if (to.length === 0) return { sent: false, reason: "no_owner_email" };

  const m = await mailer();
  if (!m) return { sent: false, reason: "smtp" };

  const bundle = await loadAppConfig();
  const school = bundle.identity.shortName || bundle.identity.name;
  const link = await tenantAbsolutePath("/eleves/dossiers?tab=acces");
  const eleve = `${input.elevePrenom} ${input.eleveNom}`.trim();
  const ownerLabel = documentAccessOwnerLabel(input.ownerKind);
  const tiroirLabel = TIROIR_LABELS[input.documentTiroir] || input.documentTiroir;
  const text = [
    `Bonjour,`,
    ``,
    `${input.requesterName}${input.requesterEmail ? ` (${input.requesterEmail})` : ""} souhaite consulter un document réservé (${ownerLabel}) :`,
    ``,
    `Élève : ${eleve}`,
    input.classe ? `Classe : ${input.classe}` : null,
    `Document : ${input.documentTitle}`,
    `Tiroir : ${tiroirLabel}`,
    `Durée demandée : ${documentAccessDurationLabel(input.durationDays)}`,
    input.note?.trim() ? `Motif : ${input.note.trim()}` : null,
    ``,
    `Validez ou refusez dans l’intranet → Dossiers élèves → Accès documents :`,
    link,
    ``,
    `Cordialement,`,
    school,
  ]
    .filter(Boolean)
    .join("\n");

  await m.transporter.sendMail({
    from: `"Dossiers élèves ${school}" <${m.smtp.user}>`,
    to: to.join(", "),
    subject: `[Accès document] Demande — ${eleve}`,
    text,
  });

  return { sent: true, to };
}
