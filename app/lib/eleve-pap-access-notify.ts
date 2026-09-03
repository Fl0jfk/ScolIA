import "server-only";

import { loadAppConfig } from "@/app/lib/app-config";
import { createTenantTransporter, getTenantSmtpConfig } from "@/app/lib/tenant-mail";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";
import {
  resolveStagesDirectionEmail,
  stageCycleKindFromStudent,
  stageCycleLabel,
} from "@/app/lib/stage-config";
import { documentAccessDurationLabel } from "@/app/lib/eleve-document-access-duration";
import {
  accompagnementKindDef,
  detectAccompagnementKind,
  type AccompagnementKind,
} from "@/app/lib/eleve-pap";

async function mailer() {
  const smtp = await getTenantSmtpConfig();
  if (!smtp) return null;
  const transporter = await createTenantTransporter();
  if (!transporter) return null;
  return { smtp, transporter };
}

/** Notifie la direction du cycle d’une demande d’accès PAP / PAI / PPS. */
export async function notifyDirectionPapAccessRequest(input: {
  eleveNom: string;
  elevePrenom: string;
  classe?: string | null;
  level?: string | null;
  documentTitle: string;
  requesterName: string;
  requesterEmail?: string | null;
  durationDays: number;
  note?: string | null;
  /** Si omis, déduit du titre du document. */
  kind?: AccompagnementKind | null;
}): Promise<{ sent: boolean; to?: string; reason?: string }> {
  const levelHint = String(input.level || input.classe || "").trim() || "lycee";
  const to = await resolveStagesDirectionEmail(levelHint, input.classe || undefined);
  if (!to) return { sent: false, reason: "no_direction_email" };

  const m = await mailer();
  if (!m) return { sent: false, reason: "smtp" };

  const bundle = await loadAppConfig();
  const school = bundle.identity.shortName || bundle.identity.name;
  const cycle = stageCycleLabel(stageCycleKindFromStudent(levelHint, input.classe || undefined));
  const link = await tenantAbsolutePath("/eleves/dossiers?tab=acces");
  const eleve = `${input.elevePrenom} ${input.eleveNom}`.trim();
  const kind =
    input.kind ?? detectAccompagnementKind(input.documentTitle) ?? ("pap" as AccompagnementKind);
  const def = accompagnementKindDef(kind);
  const text = [
    `Bonjour,`,
    ``,
    `${input.requesterName}${input.requesterEmail ? ` (${input.requesterEmail})` : ""} souhaite consulter le ${def.code} (${def.fullLabel}) de :`,
    ``,
    `Élève : ${eleve}`,
    input.classe ? `Classe : ${input.classe}` : null,
    `Cycle : ${cycle}`,
    `Document : ${input.documentTitle}`,
    `Durée demandée : ${documentAccessDurationLabel(input.durationDays)}`,
    input.note?.trim() ? `Motif : ${input.note.trim()}` : null,
    ``,
    `Validez ou refusez dans l’intranet → Dossiers élèves → Demandes d’accès :`,
    link,
    ``,
    `Cordialement,`,
    school,
  ]
    .filter(Boolean)
    .join("\n");

  await m.transporter.sendMail({
    from: `"Dossiers élèves ${school}" <${m.smtp.user}>`,
    to,
    subject: `[${def.code}] Demande d’accès — ${eleve} (${cycle})`,
    text,
  });

  return { sent: true, to };
}
