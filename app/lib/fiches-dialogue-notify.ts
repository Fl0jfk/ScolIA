import "server-only";

import {
  createTenantTransporter,
  getTenantSmtpConfig,
  sendMailWithTimeout,
} from "@/app/lib/tenant-mail";
import { loadAppConfig } from "@/app/lib/app-config";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";
import type { FdAppelConfig } from "@/db/schema-fiches-dialogue";

async function mailer() {
  const smtp = await getTenantSmtpConfig();
  if (!smtp) return null;
  const transporter = await createTenantTransporter();
  if (!transporter) return null;
  return { smtp, transporter };
}

async function schoolName(): Promise<string> {
  const bundle = await loadAppConfig();
  return bundle.identity.shortName || bundle.identity.name || "Établissement";
}

export async function notifyFdFamilleSaisie(params: {
  to: string[];
  elevePrenom: string;
  eleveNom: string;
  classe: string;
  campagneLabel: string;
  etapeLabel: string;
  token: string;
  secureCode: string;
  delaiJours: number;
  reminder?: boolean;
}): Promise<{ sent: boolean; reason?: string }> {
  const m = await mailer();
  if (!m) return { sent: false, reason: "smtp" };
  if (!params.to.length) return { sent: false, reason: "no_recipients" };

  const school = await schoolName();
  const link = await tenantAbsolutePath(
    `/fiches-dialogue/remplir?token=${encodeURIComponent(params.token)}`,
  );
  const codeEntry = await tenantAbsolutePath("/fiches-dialogue/remplir");
  const subjectPrefix = params.reminder ? "[Rappel] " : "";
  const text = [
    "Bonjour,",
    "",
    params.reminder
      ? `Nous n’avons pas encore reçu votre réponse pour la fiche de dialogue de ${params.elevePrenom} ${params.eleveNom}.`
      : `Une fiche de dialogue est disponible pour ${params.elevePrenom} ${params.eleveNom} (${params.classe || "classe non renseignée"}).`,
    "",
    `Campagne : ${params.campagneLabel}`,
    `Étape : ${params.etapeLabel}`,
    `Vous disposez de ${params.delaiJours} jour(s) pour répondre.`,
    "",
    `Lien unique : ${link}`,
    "",
    `Vous pouvez aussi ouvrir ${codeEntry} et saisir votre e-mail avec le code : ${params.secureCode}`,
    "",
    "Cordialement,",
    school,
  ].join("\n");

  for (const to of params.to) {
    await sendMailWithTimeout(m.transporter, {
      from: `"Fiches de dialogue — ${school}" <${m.smtp.user}>`,
      to,
      subject: `${subjectPrefix}Fiche de dialogue — ${params.elevePrenom} ${params.eleveNom}`,
      text,
    });
  }
  return { sent: true };
}

export async function notifyFdDecisionPdf(params: {
  to: string[];
  elevePrenom: string;
  eleveNom: string;
  etapeLabel: string;
  pdfBytes: Uint8Array;
  fileName: string;
  intro: string;
}): Promise<{ sent: boolean; reason?: string }> {
  const m = await mailer();
  if (!m) return { sent: false, reason: "smtp" };
  if (!params.to.length) return { sent: false, reason: "no_recipients" };

  const school = await schoolName();
  const text = [
    "Bonjour,",
    "",
    params.intro,
    "",
    `Élève : ${params.elevePrenom} ${params.eleveNom}`,
    `Étape : ${params.etapeLabel}`,
    "",
    "Le document PDF est joint à ce message.",
    "",
    "Cordialement,",
    school,
  ].join("\n");

  for (const to of params.to) {
    await sendMailWithTimeout(m.transporter, {
      from: `"Fiches de dialogue — ${school}" <${m.smtp.user}>`,
      to,
      subject: `Fiche de dialogue — ${params.etapeLabel} — ${params.elevePrenom} ${params.eleveNom}`,
      text,
      attachments: [
        {
          filename: params.fileName,
          content: Buffer.from(params.pdfBytes),
          contentType: "application/pdf",
        },
      ],
    });
  }
  return { sent: true };
}

export async function notifyFdAcceptationRequest(params: {
  to: string[];
  elevePrenom: string;
  eleveNom: string;
  token: string;
  secureCode: string;
  pdfBytes?: Uint8Array;
}): Promise<{ sent: boolean; reason?: string }> {
  const m = await mailer();
  if (!m) return { sent: false, reason: "smtp" };
  if (!params.to.length) return { sent: false, reason: "no_recipients" };

  const school = await schoolName();
  const link = await tenantAbsolutePath(
    `/fiches-dialogue/remplir?token=${encodeURIComponent(params.token)}`,
  );
  const text = [
    "Bonjour,",
    "",
    `La décision définitive du conseil de classe concernant ${params.elevePrenom} ${params.eleveNom} est disponible.`,
    "",
    "Merci d’indiquer si vous acceptez cette décision, ou si vous souhaitez engager une procédure d’appel.",
    "",
    `Lien : ${link}`,
    `Code : ${params.secureCode}`,
    "",
    "Cordialement,",
    school,
  ].join("\n");

  for (const to of params.to) {
    await sendMailWithTimeout(m.transporter, {
      from: `"Fiches de dialogue — ${school}" <${m.smtp.user}>`,
      to,
      subject: `Décision définitive — acceptation — ${params.elevePrenom} ${params.eleveNom}`,
      text,
      attachments: params.pdfBytes
        ? [
            {
              filename: `decision-finale-${params.eleveNom}.pdf`,
              content: Buffer.from(params.pdfBytes),
              contentType: "application/pdf",
            },
          ]
        : undefined,
    });
  }
  return { sent: true };
}

export async function notifyFdAppelProcedure(params: {
  to: string[];
  elevePrenom: string;
  eleveNom: string;
  appel: FdAppelConfig;
  pdfBytes: Uint8Array;
  procedureAttachments?: Array<{ filename: string; content: Buffer; contentType: string }>;
}): Promise<{ sent: boolean; reason?: string }> {
  const m = await mailer();
  if (!m) return { sent: false, reason: "smtp" };
  if (!params.to.length) return { sent: false, reason: "no_recipients" };

  const school = await schoolName();
  const docs =
    params.appel.documentsLabels?.length
      ? params.appel.documentsLabels.map((d) => `— ${d}`).join("\n")
      : "— les documents joints à ce message";

  const text = [
    "Bonjour,",
    "",
    `Vous avez indiqué ne pas être d’accord avec la décision définitive du conseil de classe concernant ${params.elevePrenom} ${params.eleveNom}.`,
    "",
    "Vous pouvez faire appel de cette décision. Voici la procédure :",
    "",
    params.appel.procedureHtml
      ? params.appel.procedureHtml.replace(/<[^>]+>/g, " ").replace(/\s+/g, " ").trim()
      : "Contactez le secrétariat de l’établissement pour constituer votre dossier d’appel dans les délais indiqués.",
    "",
    params.appel.dateLimite
      ? `Date limite pour déposer un appel : ${params.appel.dateLimite}`
      : "Respectez la date limite communiquée par l’établissement.",
    "",
    "Documents utiles :",
    docs,
    "",
    "La fiche de dialogue (décision finale) est jointe à ce message. Conservez-la précieusement.",
    "",
    "Nous restons à votre disposition pour toute question sur le déroulement de la procédure.",
    "",
    "Cordialement,",
    school,
  ].join("\n");

  const attachments = [
    {
      filename: `fiche-dialogue-decision-finale-${params.eleveNom}.pdf`,
      content: Buffer.from(params.pdfBytes),
      contentType: "application/pdf",
    },
    ...(params.procedureAttachments ?? []),
  ];

  for (const to of params.to) {
    await sendMailWithTimeout(m.transporter, {
      from: `"Fiches de dialogue — ${school}" <${m.smtp.user}>`,
      to,
      subject: `Procédure d’appel — ${params.elevePrenom} ${params.eleveNom}`,
      text,
      attachments,
    });
  }
  return { sent: true };
}

/** Alerte PP + direction + admin : refus de la décision du conseil. */
export async function notifyFdRefusStaff(params: {
  to: string[];
  elevePrenom: string;
  eleveNom: string;
  classe: string;
  campagneLabel: string;
  dateLimiteAppel?: string;
  contactPpLabel?: string;
}): Promise<{ sent: boolean; reason?: string }> {
  const m = await mailer();
  if (!m) return { sent: false, reason: "smtp" };
  if (!params.to.length) return { sent: false, reason: "no_recipients" };

  const school = await schoolName();
  const adminLink = await tenantAbsolutePath("/fiches-dialogue");
  const text = [
    "ALERTE — Refus de la décision du conseil de classe",
    "",
    `Élève : ${params.elevePrenom} ${params.eleveNom}`,
    `Classe : ${params.classe || "non renseignée"}`,
    `Campagne : ${params.campagneLabel}`,
    "",
    "La famille a refusé la décision définitive du conseil de classe.",
    "Une procédure d’appel doit être engagée sans délai.",
    params.dateLimiteAppel ? `Date limite d’appel : ${params.dateLimiteAppel}` : "",
    params.contactPpLabel
      ? `Canal indiqué à la famille : ${params.contactPpLabel}`
      : "",
    "",
    `Suivi : ${adminLink}`,
    "",
    "Cordialement,",
    school,
  ]
    .filter(Boolean)
    .join("\n");

  for (const to of params.to) {
    await sendMailWithTimeout(m.transporter, {
      from: `"Fiches de dialogue — ${school}" <${m.smtp.user}>`,
      to,
      subject: `[APPEL] Refus conseil — ${params.elevePrenom} ${params.eleveNom} (${params.classe})`,
      text,
    });
  }
  return { sent: true };
}

export async function notifyFdParentOtp(params: {
  to: string;
  elevePrenom: string;
  eleveNom: string;
  code: string;
  expiresMinutes: number;
}): Promise<{ sent: boolean; reason?: string }> {
  const m = await mailer();
  if (!m) return { sent: false, reason: "smtp" };

  const school = await schoolName();
  const entry = await tenantAbsolutePath("/fiches-dialogue/remplir");
  const text = [
    "Bonjour,",
    "",
    `Voici votre code d’accès pour la fiche de dialogue de ${params.elevePrenom} ${params.eleveNom} :`,
    "",
    `  ${params.code}`,
    "",
    `Ce code expire dans ${params.expiresMinutes} minutes.`,
    `Ouvrez ${entry} puis saisissez ce code.`,
    "",
    "Si vous n’êtes pas à l’origine de cette demande, ignorez ce message.",
    "",
    "Cordialement,",
    school,
  ].join("\n");

  await sendMailWithTimeout(m.transporter, {
    from: `"Fiches de dialogue — ${school}" <${m.smtp.user}>`,
    to: params.to,
    subject: `Code d’accès — fiche de dialogue ${params.elevePrenom} ${params.eleveNom}`,
    text,
  });
  return { sent: true };
}

export async function notifyFdParent2Accord(params: {
  to: string;
  elevePrenom: string;
  eleveNom: string;
  deposantLabel: string;
  resume: string;
  deadlineLabel: string;
  token: string;
}): Promise<{ sent: boolean; reason?: string }> {
  const m = await mailer();
  if (!m) return { sent: false, reason: "smtp" };

  const school = await schoolName();
  const link = await tenantAbsolutePath(
    `/fiches-dialogue/remplir?token=${encodeURIComponent(params.token)}`,
  );
  const text = [
    "Bonjour,",
    "",
    `Une réponse a été déposée pour ${params.elevePrenom} ${params.eleveNom} par ${params.deposantLabel}.`,
    "",
    "Résumé :",
    params.resume,
    "",
    `Vous pouvez confirmer (« j’accepte ») ou signaler un désaccord jusqu’au ${params.deadlineLabel}.`,
    "Sans action de votre part avant cette date, la réponse sera considérée comme acceptée.",
    "",
    `Lien : ${link}`,
    "",
    "Cordialement,",
    school,
  ].join("\n");

  await sendMailWithTimeout(m.transporter, {
    from: `"Fiches de dialogue — ${school}" <${m.smtp.user}>`,
    to: params.to,
    subject: `À confirmer — fiche de dialogue ${params.elevePrenom} ${params.eleveNom}`,
    text,
  });
  return { sent: true };
}
