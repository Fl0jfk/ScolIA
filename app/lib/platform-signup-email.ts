import "server-only";

import { MARKETING } from "@/app/lib/marketing-site";
import { PLATFORM_ASSISTANCE_EMAIL } from "@/app/lib/platform-assistance-email";
import { platformAppOrigin } from "@/app/lib/platform-portal-url";
import type { TenantSignupRequest } from "@/app/lib/platform-signup-request";
import { SIGNUP_STATUS_LABELS } from "@/app/lib/platform-signup-types";
import { createPlatformTransporter, getPlatformSmtpConfig } from "@/app/lib/tenant-mail";

async function sendMail(opts: { to: string; subject: string; html: string; text: string }) {
  const smtp = getPlatformSmtpConfig();
  const transporter = createPlatformTransporter();
  if (!smtp || !transporter) {
    console.warn("[platform-signup-email] SMTP non configuré — e-mail non envoyé:", opts.subject);
    return false;
  }
  await transporter.sendMail({
    from: `"${MARKETING.productName}" <${smtp.user}>`,
    to: opts.to,
    subject: opts.subject,
    html: opts.html,
    text: opts.text,
  });
  return true;
}

function statusUrl(token: string): string {
  return `${platformAppOrigin()}/souscrire/statut?token=${encodeURIComponent(token)}`;
}

function paymentUrl(token: string): string {
  return `${platformAppOrigin()}/souscrire/paiement?token=${encodeURIComponent(token)}`;
}

function contactName(req: TenantSignupRequest): string {
  return `${req.adminContact.firstName} ${req.adminContact.lastName}`.trim();
}

function microsoftMgmtLabel(req: TenantSignupRequest): string {
  const current =
    req.establishment.microsoftCurrentManagement === "external_provider"
      ? "Prestataire externe"
      : req.establishment.microsoftCurrentManagement === "internal_establishment"
        ? "Interne établissement"
        : "Aucun tenant Microsoft existant";
  const target =
    req.establishment.microsoftTargetMode === "scola_takeover"
      ? "Reprise d'administration par Scola"
      : "Déploiement Microsoft Scola indépendant";
  return `${current} → ${target}`;
}

export async function emailSignupReceivedProspect(req: TenantSignupRequest): Promise<void> {
  const url = statusUrl(req.accessToken);
  await sendMail({
    to: req.adminContact.email,
    subject: `[Scola] Votre dossier est en cours d'étude`,
    text: `Bonjour ${contactName(req)},\n\nNous avons bien reçu votre demande pour ${req.establishment.legalName} (RNE ${req.establishment.rne}).\n\nVotre dossier est en cours d'étude pour l'éligibilité Microsoft Education (revendeur partenaire).\nCette étape peut prendre quelques jours ouvrés selon les vérifications administratives ; nous pouvons vous demander des éléments complémentaires.\n\nRappel forfait Microsoft Scola :\n- jusqu'à 10 licences A3 incluses (postes direction / secrétariat / comptabilité / référents)\n- licences A1 illimitées pour les enseignants\n- au-delà de 10 A3, extension possible sur devis\n\nSuivez l'avancement : ${url}\n\n${MARKETING.contactEmail}`,
    html: `<p>Bonjour ${contactName(req)},</p>
<p>Nous avons bien reçu votre demande pour <strong>${req.establishment.legalName}</strong> (RNE ${req.establishment.rne}).</p>
<p>Votre dossier est <strong>en cours d'étude</strong> pour l'éligibilité Microsoft Education (revendeur partenaire).</p>
<p>Cette étape peut prendre quelques jours ouvrés selon les vérifications administratives ; nous pouvons vous demander des éléments complémentaires.</p>
<p><strong>Rappel forfait Microsoft Scola</strong> :</p>
<ul>
<li>jusqu'à 10 licences A3 incluses (postes direction / secrétariat / comptabilité / référents)</li>
<li>licences A1 illimitées pour les enseignants</li>
<li>au-delà de 10 A3, extension possible sur devis</li>
</ul>
<p><a href="${url}">Suivre l'avancement de votre dossier</a></p>`,
  });
}

export async function emailSignupReceivedMaster(req: TenantSignupRequest): Promise<void> {
  const masterUrl = `${platformAppOrigin()}/plateforme/demandes/${req.id}`;
  await sendMail({
    to: PLATFORM_ASSISTANCE_EMAIL,
    subject: `[Scola] Nouveau dossier — ${req.establishment.legalName}`,
    text: `Nouveau dossier d'inscription.\n\nÉtablissement : ${req.establishment.legalName}\nRNE : ${req.establishment.rne}\nRéférent : ${contactName(req)} <${req.adminContact.email}>\nEffectif : ${req.establishment.estimatedStudentCount}\nMicrosoft : ${req.establishment.wantsMicrosoftLicenses ? "oui" : "non"}\nPolitique Microsoft : ${microsoftMgmtLabel(req)}\nDécisionnaire Microsoft : ${req.establishment.microsoftDecisionContact?.fullName || "—"} <${req.establishment.microsoftDecisionContact?.email || "—"}>\n\nConsole : ${masterUrl}`,
    html: `<p><strong>Nouveau dossier d'inscription</strong></p>
<ul>
<li>Établissement : ${req.establishment.legalName}</li>
<li>RNE : ${req.establishment.rne}</li>
<li>Référent : ${contactName(req)} &lt;${req.adminContact.email}&gt;</li>
<li>Effectif : ${req.establishment.estimatedStudentCount}</li>
<li>Licences Microsoft : ${req.establishment.wantsMicrosoftLicenses ? "oui" : "non"}</li>
<li>Politique Microsoft : ${microsoftMgmtLabel(req)}</li>
<li>Décisionnaire Microsoft : ${req.establishment.microsoftDecisionContact?.fullName || "—"} &lt;${req.establishment.microsoftDecisionContact?.email || "—"}&gt;</li>
</ul>
<p><a href="${masterUrl}">Ouvrir dans la console Master</a></p>`,
  });
}

export async function emailMicrosoftApproved(req: TenantSignupRequest): Promise<void> {
  const payUrl = paymentUrl(req.accessToken);
  await sendMail({
    to: req.adminContact.email,
    subject: `[Scola] Établissement validé — passez à l'abonnement`,
    text: `Bonjour ${contactName(req)},\n\nVotre établissement ${req.establishment.legalName} a été reconnu comme éligible.\n\nVous pouvez maintenant souscrire à l'abonnement Scola :\n${payUrl}\n\n${MARKETING.contactEmail}`,
    html: `<p>Bonjour ${contactName(req)},</p>
<p>Votre établissement <strong>${req.establishment.legalName}</strong> a été validé.</p>
<p>Vous pouvez maintenant <strong>souscrire à l'abonnement Scola</strong> :</p>
<p><a href="${payUrl}">Choisir mon abonnement et payer</a></p>`,
  });
}

export async function emailSignupRejected(req: TenantSignupRequest): Promise<void> {
  await sendMail({
    to: req.adminContact.email,
    subject: `[Scola] Dossier non retenu`,
    text: `Bonjour ${contactName(req)},\n\nVotre dossier pour ${req.establishment.legalName} n'a pas pu être retenu.\n${req.rejectedReason ? `Motif : ${req.rejectedReason}\n` : ""}\nContact : ${MARKETING.contactEmail}`,
    html: `<p>Bonjour ${contactName(req)},</p>
<p>Votre dossier pour <strong>${req.establishment.legalName}</strong> n'a pas pu être retenu.</p>
${req.rejectedReason ? `<p><strong>Motif :</strong> ${req.rejectedReason}</p>` : ""}
<p>Contact : ${MARKETING.contactEmail}</p>`,
  });
}

export async function emailPaymentCompleted(req: TenantSignupRequest): Promise<void> {
  const status = statusUrl(req.accessToken);
  await sendMail({
    to: req.adminContact.email,
    subject: `[Scola] Paiement confirmé`,
    text: `Bonjour ${contactName(req)},\n\nVotre paiement a bien été enregistré. Nous finalisons la mise en service de votre espace.\n\nSuivi : ${status}\n\n${MARKETING.contactEmail}`,
    html: `<p>Bonjour ${contactName(req)},</p>
<p>Votre <strong>paiement est confirmé</strong>. Nous finalisons la mise en service de votre espace Scola.</p>
<p><a href="${status}">Suivre l'avancement</a></p>`,
  });
  await sendMail({
    to: PLATFORM_ASSISTANCE_EMAIL,
    subject: `[Scola] Paiement reçu — ${req.establishment.legalName}`,
    text: `Paiement confirmé pour ${req.establishment.legalName}.\nStatut : ${SIGNUP_STATUS_LABELS[req.status]}\nProvisioning : ${platformAppOrigin()}/plateforme/demandes/${req.id}`,
    html: `<p>Paiement confirmé pour <strong>${req.establishment.legalName}</strong>.</p>
<p><a href="${platformAppOrigin()}/plateforme/demandes/${req.id}">Provisionner le tenant</a></p>`,
  });
}

export async function emailTenantProvisioned(
  req: TenantSignupRequest,
  signInUrl: string,
): Promise<void> {
  await emailTenantSpaceReady({
    to: req.adminContact.email,
    firstName: req.adminContact.firstName,
    lastName: req.adminContact.lastName,
    establishmentLabel: req.establishment.legalName,
    signInUrl,
  });
}

/** E-mail « espace prêt » (création Master ou provision dossier). */
export async function emailTenantSpaceReady(opts: {
  to: string;
  firstName?: string;
  lastName?: string;
  establishmentLabel: string;
  signInUrl: string;
}): Promise<void> {
  const name = `${opts.firstName ?? ""} ${opts.lastName ?? ""}`.trim() || "Madame, Monsieur";
  await sendMail({
    to: opts.to,
    subject: `[Scola] Votre espace est prêt`,
    text: `Bonjour ${name},\n\nVotre espace Scola est prêt pour ${opts.establishmentLabel}.\nConnectez-vous avec cette adresse e-mail : ${opts.signInUrl}\n\nComplétez ensuite l'assistant de configuration.\n\n${MARKETING.contactEmail}`,
    html: `<p>Bonjour ${name},</p>
<p>Votre espace <strong>Scola</strong> est prêt pour <strong>${opts.establishmentLabel}</strong>.</p>
<p>Connectez-vous avec l&apos;adresse e-mail à laquelle ce message a été envoyé.</p>
<p><a href="${opts.signInUrl}">Se connecter et démarrer la configuration</a></p>`,
  });
}

async function emailMicrosoftLicenseRequest(
  tenantLabel: string,
  tenantSlug: string,
  people: { firstName: string; lastName: string; email: string; role: string; licenseType: string }[],
): Promise<void> {
  const lines = people
    .map(
      (p) =>
        `- ${p.firstName} ${p.lastName} <${p.email}> — ${p.role} — licence ${p.licenseType}`,
    )
    .join("\n");
  const htmlRows = people
    .map(
      (p) =>
        `<tr><td>${p.lastName}</td><td>${p.firstName}</td><td>${p.email}</td><td>${p.role}</td><td>${p.licenseType}</td></tr>`,
    )
    .join("");
  await sendMail({
    to: PLATFORM_ASSISTANCE_EMAIL,
    subject: `[Scola] Demande licences Microsoft — ${tenantLabel}`,
    text: `Demande de licences Microsoft pour ${tenantLabel} (${tenantSlug}).\n\n${lines}`,
    html: `<p>Demande de licences Microsoft pour <strong>${tenantLabel}</strong> (${tenantSlug}).</p>
<table border="1" cellpadding="6" cellspacing="0"><thead><tr><th>Nom</th><th>Prénom</th><th>E-mail</th><th>Rôle</th><th>Licence</th></tr></thead><tbody>${htmlRows}</tbody></table>`,
  });
}
