import "server-only";

import { loadAppConfig } from "@/app/lib/app-config";
import type {
  PartenariatInscriptionRecord,
  PartenariatOffreRecord,
} from "@/app/lib/partenariats-types";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
  sendMailWithTimeout,
} from "@/app/lib/tenant-mail";

export async function notifyPartenariatInscription(params: {
  offre: PartenariatOffreRecord;
  inscription: PartenariatInscriptionRecord;
}): Promise<void> {
  const { offre, inscription } = params;
  try {
    const smtp = await getTenantSmtpConfig();
    const transporter = await createTenantTransporter();
    if (!smtp || !transporter) return;
    const bundle = await loadAppConfig();
    const school = bundle.identity.shortName || bundle.identity.name || "Établissement";
    const from = `"${school}" <${smtp.user}>`;

    const parentHtml = `
      <p>Bonjour ${escapeHtml(inscription.parentFirstName)},</p>
      <p>Nous avons bien reçu l’inscription de <strong>${escapeHtml(inscription.eleveFirstName)} ${escapeHtml(inscription.eleveLastName)}</strong> à l’offre <strong>${escapeHtml(offre.title)}</strong>.</p>
      <p>L’établissement vous recontactera si des précisions sont nécessaires.</p>
      <p>Cordialement,<br/>${escapeHtml(school)}</p>
    `;
    await sendMailWithTimeout(transporter, {
      from,
      to: inscription.parentEmail,
      subject: `[${school}] Inscription reçue — ${offre.title}`,
      html: parentHtml,
    });

    if (offre.notifyEmail) {
      const adminHtml = `
        <p>Nouvelle inscription — <strong>${escapeHtml(offre.title)}</strong></p>
        <ul>
          <li>Élève : ${escapeHtml(inscription.eleveFirstName)} ${escapeHtml(inscription.eleveLastName)} (${escapeHtml(inscription.eleveNiveau || "—")} / ${escapeHtml(inscription.eleveClasse || "—")})</li>
          <li>Parent : ${escapeHtml(inscription.parentFirstName)} ${escapeHtml(inscription.parentLastName)}</li>
          <li>E-mail : ${escapeHtml(inscription.parentEmail)}</li>
          <li>Tél. : ${escapeHtml(inscription.parentPhone || "—")}</li>
        </ul>
      `;
      await sendMailWithTimeout(transporter, {
        from,
        to: offre.notifyEmail,
        subject: `[Partenariats] Inscription — ${offre.title}`,
        html: adminHtml,
      });
    }
  } catch (e) {
    console.error("[partenariats-mail]", e);
  }
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}
