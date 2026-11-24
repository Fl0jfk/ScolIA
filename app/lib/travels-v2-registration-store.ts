import "server-only";

import { randomUUID } from "node:crypto";
import { loadAppConfig } from "@/app/lib/app-config";
import { collectEleveParentEmails, normalizeParentEmail } from "@/app/lib/eleves-parent-emails";
import { loadElevesActifsRegistry } from "@/app/lib/eleves-registry";
import { maskEmail } from "@/app/lib/mask-email";
import {
  deleteObject,
  getJson,
  getObjectBytes,
  getSignedReadUrl,
  putJson,
  putObject,
} from "@/app/lib/s3-storage";
import { listJsonRecordsInDir } from "@/app/lib/ent-json-postgres";
import { sanitizeS3FileName } from "@/app/lib/s3-path";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
  sendMailWithTimeout,
} from "@/app/lib/tenant-mail";
import { buildFamilyPagePreviewModel, listPicnicDays } from "@/app/lib/travels-family-page";
import { getTravelTrip, listTravelsIndex } from "@/app/lib/travels-storage";
import type { TravelsTrip } from "@/app/lib/travels-types";
import { isWorkflowV2, todayIsoParis } from "@/app/lib/travels-v2";
import {
  addTokenHash,
  buildEngagement,
  buildFamilyIcs,
  buildInviteToken,
  buildPublicFamilyView,
  checkSigningCode,
  generateInviteSecret,
  hashInviteSecret,
  issueSigningCode,
  parseInviteToken,
  siblingDecision,
  tripDatesForIcs,
  verifyInviteSecret,
  type PublicFamilyView,
} from "@/app/lib/travels-v2-registration";
import {
  canSendCode,
  computeInvitationStatus,
  decodeSignaturePng,
  DOC_MAX_BYTES,
  INVITATION_STATUS_LABELS,
  isLocked,
  mergeAnswers,
  pruneCodeSends,
  registrationWindow,
  signInputSchema,
  signingProblems,
  sniffAllowedDocType,
  type FamilyInvitation,
  type InvitationStatus,
} from "@/app/lib/travels-v2-registration-types";
import { buildSignedCircularPdf } from "@/app/lib/travels-v2-signed-pdf";

/* ─── Stockage : un enregistrement JSON par invitation (pas de verrou global) ─── */

const TRIP_ID_RE = /^[A-Za-z0-9_-]{8,64}$/;

function invDir(tripId: string): string {
  return `travels-v2/${tripId}/invitations`;
}
function invPath(tripId: string, invId: string): string {
  return `${invDir(tripId)}/${invId}.json`;
}

export async function listInvitations(tripId: string): Promise<FamilyInvitation[]> {
  if (!TRIP_ID_RE.test(tripId)) return [];
  const rows = await listJsonRecordsInDir<Record<string, unknown>>(invDir(tripId));
  return rows
    .filter((r) => r && typeof r === "object" && r.tripId === tripId && typeof r.id === "string")
    .map((r) => r as unknown as FamilyInvitation);
}

export async function getInvitation(tripId: string, invId: string): Promise<FamilyInvitation | null> {
  if (!TRIP_ID_RE.test(tripId) || !TRIP_ID_RE.test(invId)) return null;
  const hit = await getJson<FamilyInvitation>(invPath(tripId, invId));
  const inv = hit?.data;
  if (!inv || inv.tripId !== tripId || inv.id !== invId) return null;
  return inv;
}

export async function saveInvitation(inv: FamilyInvitation): Promise<void> {
  await putJson(invPath(inv.tripId, inv.id), inv);
}

/* ─── Création des invitations à l'ouverture de la page ─── */

export type CreateInvitationsResult = {
  created: FamilyInvitation[];
  existing: number;
  /** Élèves de la pré-liste sans aucun e-mail de responsable connu. */
  studentsWithoutEmail: { ine: string; name: string }[];
};

function studentDisplayName(prenom: string, nom: string): string {
  return `${String(prenom || "").trim()} ${String(nom || "").trim().toUpperCase()}`.trim();
}

/**
 * Source des e-mails : fiche élève du registre de l'établissement (`parentEmail`, `parent1Email`, `parent2Email`),
 * comme pour le blog parents. Idempotent : une invitation par (élève, e-mail).
 */
export async function createInvitationsForTrip(trip: Pick<TravelsTrip, "id" | "data">): Promise<CreateInvitationsResult> {
  const preListe = trip.data?.preListeEleves || [];
  const eleves = await loadElevesActifsRegistry().catch(() => []);
  const byIne = new Map(eleves.map((e) => [String(e.ine || "").trim().toUpperCase(), e]));
  const current = await listInvitations(trip.id);
  const known = new Set(current.map((i) => `${i.ine}|${i.email}`));
  const created: FamilyInvitation[] = [];
  const studentsWithoutEmail: { ine: string; name: string }[] = [];
  let existing = 0;
  const seenIne = new Set<string>();

  for (const p of preListe) {
    const ine = String(p.ine || "").trim();
    if (!ine || seenIne.has(ine.toUpperCase())) continue;
    seenIne.add(ine.toUpperCase());
    const fiche = byIne.get(ine.toUpperCase());
    const emails = fiche ? collectEleveParentEmails(fiche) : [];
    const name = studentDisplayName(fiche?.prenom ?? p.prenom, fiche?.nom ?? p.nom);
    if (emails.length === 0) {
      studentsWithoutEmail.push({ ine, name });
      continue;
    }
    for (const email of emails) {
      if (known.has(`${ine}|${email}`)) {
        existing += 1;
        continue;
      }
      const inv: FamilyInvitation = {
        id: randomUUID(),
        tripId: trip.id,
        ine,
        studentName: name,
        classe: fiche?.classe || p.classe || undefined,
        email,
        createdAt: new Date().toISOString(),
        tokenHashes: [],
        sendCount: 0,
        answers: {},
        documents: {},
        exemptions: {},
        codeSends: [],
      };
      await saveInvitation(inv);
      known.add(`${ine}|${email}`);
      created.push(inv);
    }
  }
  return { created, existing, studentsWithoutEmail };
}

/* ─── E-mails ─── */

function escapeHtml(s: string): string {
  return String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]!);
}

type MailAttachment = { filename: string; content: Buffer | string; contentType: string };

async function mailContext() {
  const smtp = await getTenantSmtpConfig();
  const transporter = smtp ? await createTenantTransporter() : null;
  if (!smtp || !transporter) return null;
  const bundle = await loadAppConfig().catch(() => null);
  const school = bundle?.identity?.shortName || bundle?.identity?.name || "L'établissement";
  return { smtp, transporter, school };
}

function validReplyTo(email: string | undefined): string | undefined {
  const e = String(email || "").trim();
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(e) ? e : undefined;
}

function firstName(studentName: string): string {
  return studentName.split(" ")[0] || studentName;
}

/**
 * Envoie (ou renvoie) le lien personnel. Un NOUVEAU secret est généré à chaque envoi : seule son empreinte est
 * stockée, les liens précédents (5 derniers) restent valables. Retourne true si l'e-mail est parti.
 */
export async function sendInvitationLink(opts: {
  trip: Pick<TravelsTrip, "id" | "data" | "ownerEmail" | "ownerName">;
  inv: FamilyInvitation;
  baseUrl: string;
  mail: NonNullable<Awaited<ReturnType<typeof mailContext>>>;
}): Promise<boolean> {
  const { trip, inv, baseUrl, mail } = opts;
  const secret = generateInviteSecret();
  inv.tokenHashes = addTokenHash(inv.tokenHashes, hashInviteSecret(secret));
  const link = `${baseUrl}/voyages/famille/${encodeURIComponent(
    buildInviteToken({ invitationId: inv.id, tripId: inv.tripId, secret }),
  )}`;
  const title = String(trip.data?.title || trip.data?.destination || "Voyage scolaire");
  const deadline = trip.data?.familyPage?.deadline;
  const deadlineFr = deadline ? deadline.split("-").reverse().join("/") : "";
  const kid = firstName(inv.studentName);
  const html = `
    <div style="font-family: Arial, sans-serif; max-width: 560px; color: #1e293b;">
      <p>Bonjour,</p>
      <p>Le voyage scolaire <strong>« ${escapeHtml(title)} »</strong> est ouvert aux inscriptions pour <strong>${escapeHtml(kid)}</strong>.</p>
      <p>Merci de répondre sur la page dédiée${deadlineFr ? ` <strong>avant le ${escapeHtml(deadlineFr)}</strong>` : ""}. Sans votre signature, votre enfant ne sera pas inscrit.</p>
      <p style="margin: 24px 0;">
        <a href="${escapeHtml(link)}" style="background:#4338ca;color:#fff;padding:12px 20px;border-radius:10px;text-decoration:none;font-weight:bold;">Ouvrir la page du voyage</a>
      </p>
      <p style="font-size: 13px; color: #64748b;">Ce lien est personnel : ne le transmettez pas. Pour signer, un code à 6 chiffres vous sera envoyé à cette adresse. Si vous avez perdu ce message, vous pourrez demander un nouveau lien depuis la page.</p>
      <p>Cordialement,<br/>${escapeHtml(mail.school)}</p>
    </div>`;
  const text = `Bonjour,\n\nLe voyage scolaire « ${title} » est ouvert aux inscriptions pour ${kid}.\nRépondez sur cette page${deadlineFr ? ` avant le ${deadlineFr}` : ""} : ${link}\nSans votre signature, votre enfant ne sera pas inscrit.\n\nCe lien est personnel : ne le transmettez pas.\n\n${mail.school}`;
  try {
    await sendMailWithTimeout(
      mail.transporter,
      {
        from: `"Sorties scolaires" <${mail.smtp.user}>`,
        to: inv.email,
        replyTo: validReplyTo(trip.ownerEmail),
        subject: `Voyage scolaire « ${title} » — inscription de ${kid}`,
        text,
        html,
      },
      60_000,
    );
    const now = new Date().toISOString();
    inv.sendCount = (inv.sendCount || 0) + 1;
    inv.firstSentAt = inv.firstSentAt || now;
    inv.lastSentAt = now;
    inv.lastSendError = undefined;
    await saveInvitation(inv);
    return true;
  } catch (e) {
    inv.lastSendError = e instanceof Error ? e.message.slice(0, 200) : "Envoi impossible";
    // Le nouveau lien n'a pas été reçu : on retire son empreinte pour ne pas garder un lien orphelin.
    inv.tokenHashes = inv.tokenHashes.filter((h) => h !== hashInviteSecret(secret));
    await saveInvitation(inv);
    return false;
  }
}

/** Envoie les liens des invitations demandées (ou de toutes celles jamais envoyées). */
export async function sendPendingInvitations(opts: {
  trip: Pick<TravelsTrip, "id" | "data" | "ownerEmail" | "ownerName">;
  baseUrl: string;
  only?: string[];
}): Promise<{ sent: number; failed: number; smtp: boolean }> {
  const mail = await mailContext();
  if (!mail) return { sent: 0, failed: 0, smtp: false };
  const all = await listInvitations(opts.trip.id);
  const targets = all.filter((i) => (opts.only ? opts.only.includes(i.id) : !i.firstSentAt && !i.signature));
  let sent = 0;
  let failed = 0;
  for (const inv of targets) {
    if (await sendInvitationLink({ trip: opts.trip, inv, baseUrl: opts.baseUrl, mail })) sent += 1;
    else failed += 1;
  }
  return { sent, failed, smtp: true };
}

/** Ouverture de la page (validation finale) : crée les invitations puis envoie les liens. */
export async function openRegistrationsForTrip(opts: {
  trip: Pick<TravelsTrip, "id" | "data" | "ownerEmail" | "ownerName">;
  baseUrl: string;
}) {
  const created = await createInvitationsForTrip(opts.trip);
  const sent = await sendPendingInvitations({ trip: opts.trip, baseUrl: opts.baseUrl });
  return { ...created, ...sent };
}

/**
 * Renvoi du lien : l'appelant ne doit JAMAIS savoir si l'e-mail est connu. Cette fonction retrouve les invitations
 * de cette adresse sur les voyages v2 ouverts et renvoie un nouveau lien à chacune (limites appliquées par l'appelant).
 */
export async function resendLinksForEmail(emailRaw: string, baseUrl: string): Promise<number> {
  const email = normalizeParentEmail(emailRaw);
  if (!email) return 0;
  const mail = await mailContext();
  if (!mail) return 0;
  const index = await listTravelsIndex().catch(() => []);
  let n = 0;
  for (const summary of index) {
    if (summary.status !== "INSCRIPTIONS_OUVERTES") continue;
    const trip = await getTravelTrip(summary.id);
    if (!trip || !isWorkflowV2(trip) || trip.status !== "INSCRIPTIONS_OUVERTES") continue;
    const invs = (await listInvitations(trip.id)).filter((i) => i.email === email);
    for (const inv of invs) {
      if (await sendInvitationLink({ trip, inv, baseUrl, mail })) n += 1;
    }
  }
  return n;
}

/* ─── Contexte public : un jeton ne donne accès qu'à SON invitation ─── */

export type PublicContext = { trip: TravelsTrip; inv: FamilyInvitation };

/** Retourne null pour tout jeton invalide, inconnu, d'un autre tenant, ou d'un voyage non v2 (réponse uniforme). */
export async function resolvePublicContext(token: string | null | undefined): Promise<PublicContext | null> {
  const parsed = parseInviteToken(token);
  if (!parsed) return null;
  const inv = await getInvitation(parsed.tripId, parsed.invitationId);
  if (!inv || !verifyInviteSecret(inv.tokenHashes, parsed.secret)) return null;
  const trip = await getTravelTrip(parsed.tripId);
  if (!trip || !isWorkflowV2(trip) || !trip.data?.familyPage?.openedAt) return null;
  return { trip, inv };
}

export function windowFor(ctx: PublicContext) {
  return registrationWindow({
    tripStatus: ctx.trip.status,
    openedAt: ctx.trip.data?.familyPage?.openedAt,
    deadline: ctx.trip.data?.familyPage?.deadline,
    extendedDeadline: ctx.inv.extendedDeadline,
    signed: isLocked(ctx.inv),
    todayIso: todayIsoParis(),
  });
}

function picnicKeys(trip: Pick<TravelsTrip, "data">): string[] {
  return listPicnicDays(trip.data?.piqueNiqueDetails).map((d) => d.key);
}

export async function publicViewFor(ctx: PublicContext): Promise<PublicFamilyView> {
  // Marque l'ouverture (et une présence récente, au plus toutes les 5 minutes).
  const now = Date.now();
  const last = ctx.inv.lastSeenAt ? new Date(ctx.inv.lastSeenAt).getTime() : 0;
  if (!ctx.inv.firstOpenedAt || now - last > 5 * 60 * 1000) {
    ctx.inv.firstOpenedAt = ctx.inv.firstOpenedAt || new Date(now).toISOString();
    ctx.inv.lastSeenAt = new Date(now).toISOString();
    await saveInvitation(ctx.inv).catch((e) => console.error("[voyages-v2] lastSeen", e));
  }
  const sibs = await listInvitations(ctx.trip.id);
  return buildPublicFamilyView({
    trip: ctx.trip,
    inv: ctx.inv,
    todayIso: todayIsoParis(),
    maskedEmail: maskEmail(ctx.inv.email),
    siblingDecision: siblingDecision(sibs, ctx.inv),
  });
}

/* ─── Réponses (brouillon) ─── */

export async function saveAnswers(
  ctx: PublicContext,
  input: unknown,
): Promise<{ ok: true } | { ok: false; status: number; error: string }> {
  const win = windowFor(ctx);
  if (!win.open) return { ok: false, status: 409, error: closedMessage(win.reason) };
  const merged = mergeAnswers({
    current: ctx.inv.answers || {},
    input,
    picnicDayKeys: picnicKeys(ctx.trip),
    nowIso: new Date().toISOString(),
  });
  if (!merged.ok) return { ok: false, status: 400, error: merged.error };
  ctx.inv.answers = merged.answers;
  await saveInvitation(ctx.inv);
  return { ok: true };
}

export function closedMessage(reason: "status" | "deadline" | "locked" | "not_opened"): string {
  switch (reason) {
    case "deadline":
      return "La date limite de réponse est dépassée : la page est fermée.";
    case "locked":
      return "Votre réponse est signée : elle ne peut plus être modifiée.";
    case "status":
      return "Les inscriptions de ce voyage sont closes.";
    default:
      return "Cette page n'est pas ouverte.";
  }
}

/* ─── Pièces déposées (stockage objet PRIVÉ) ─── */

export async function storeFamilyDocument(
  ctx: PublicContext,
  docId: string,
  fileName: string,
  bytes: Buffer,
): Promise<{ ok: true } | { ok: false; status: number; error: string }> {
  const win = windowFor(ctx);
  if (!win.open) return { ok: false, status: 409, error: closedMessage(win.reason) };
  const request = (ctx.trip.data?.familyPage?.documents || []).find((d) => d.id === docId);
  if (!request) return { ok: false, status: 400, error: "Pièce inconnue." };
  if (bytes.length === 0 || bytes.length > DOC_MAX_BYTES) {
    return { ok: false, status: 413, error: "Fichier vide ou trop volumineux (8 Mo maximum)." };
  }
  const type = sniffAllowedDocType(bytes);
  if (!type) return { ok: false, status: 415, error: "Format non accepté : PDF ou image (JPEG, PNG, WebP) uniquement." };
  const key = `travels-v2/${ctx.trip.id}/docs/${ctx.inv.id}/${docId}-${randomUUID()}.${type.ext}`;
  await putObject(key, bytes, type.contentType);
  const previous = ctx.inv.documents?.[docId];
  ctx.inv.documents = {
    ...(ctx.inv.documents || {}),
    [docId]: {
      fileName: sanitizeS3FileName(fileName).slice(0, 120),
      key,
      size: bytes.length,
      contentType: type.contentType,
      uploadedAt: new Date().toISOString(),
    },
  };
  await saveInvitation(ctx.inv);
  if (previous?.key) await deleteObject(previous.key).catch(() => undefined);
  return { ok: true };
}

export async function removeFamilyDocument(
  ctx: PublicContext,
  docId: string,
): Promise<{ ok: true } | { ok: false; status: number; error: string }> {
  const win = windowFor(ctx);
  if (!win.open) return { ok: false, status: 409, error: closedMessage(win.reason) };
  const previous = ctx.inv.documents?.[docId];
  if (!previous) return { ok: true };
  const rest = { ...ctx.inv.documents };
  delete rest[docId];
  ctx.inv.documents = rest;
  await saveInvitation(ctx.inv);
  await deleteObject(previous.key).catch(() => undefined);
  return { ok: true };
}

/* ─── Code de signature ─── */

export async function sendSigningCode(
  ctx: PublicContext,
): Promise<{ ok: true; maskedEmail: string } | { ok: false; status: number; error: string; retryAfterSec?: number }> {
  const win = windowFor(ctx);
  if (!win.open) return { ok: false, status: 409, error: closedMessage(win.reason) };
  if (ctx.inv.answers?.participates !== "oui" && ctx.inv.answers?.participates !== "non") {
    return { ok: false, status: 400, error: "Indiquez d'abord si votre enfant participe au voyage." };
  }
  const now = new Date();
  const allowed = canSendCode(ctx.inv, now);
  if (!allowed.ok) {
    return {
      ok: false,
      status: 429,
      error: "Trop de demandes de code. Réessayez dans quelques instants.",
      retryAfterSec: allowed.retryAfterSec,
    };
  }
  const mail = await mailContext();
  if (!mail) return { ok: false, status: 503, error: "L'envoi d'e-mails n'est pas disponible pour le moment." };
  const { code, state } = issueSigningCode(ctx.inv.id, now);
  const title = String(ctx.trip.data?.title || "Voyage scolaire");
  try {
    await sendMailWithTimeout(
      mail.transporter,
      {
        from: `"Sorties scolaires" <${mail.smtp.user}>`,
        to: ctx.inv.email,
        subject: `Votre code de signature : ${code}`,
        text: `Votre code de signature pour le voyage « ${title} » : ${code}\nIl est valable 10 minutes. Ne le communiquez à personne.`,
        html: `<p>Votre code de signature pour le voyage <strong>« ${escapeHtml(title)} »</strong> :</p>
               <p style="font-size:28px;font-weight:bold;letter-spacing:6px;">${code}</p>
               <p style="color:#64748b;font-size:13px;">Il est valable 10 minutes. Ne le communiquez à personne.</p>`,
      },
      30_000,
    );
  } catch {
    return { ok: false, status: 502, error: "L'e-mail n'a pas pu être envoyé. Réessayez dans un instant." };
  }
  ctx.inv.code = state;
  ctx.inv.codeSends = [...pruneCodeSends(ctx.inv.codeSends, now), now.toISOString()];
  await saveInvitation(ctx.inv);
  return { ok: true, maskedEmail: maskEmail(ctx.inv.email) };
}

/* ─── Signature, accusé, engagement ─── */

export type SignResult =
  | { ok: true; decision: "oui" | "non" }
  | { ok: false; status: number; error: string; attemptsLeft?: number };

export async function submitSignature(ctx: PublicContext, rawInput: unknown): Promise<SignResult> {
  const { trip, inv } = ctx;
  const win = windowFor(ctx);
  if (!win.open) return { ok: false, status: 409, error: closedMessage(win.reason) };
  const parsed = signInputSchema.safeParse(rawInput);
  if (!parsed.success) return { ok: false, status: 400, error: "Saisie invalide : nom, code à 6 chiffres et signature." };
  const input = parsed.data;
  const answers = inv.answers || {};
  const decision = answers.participates;
  const docs = trip.data?.familyPage?.documents || [];
  const problems = signingProblems({ answers, picnicDayKeys: picnicKeys(trip), docs, inv });
  if (problems.length > 0 || (decision !== "oui" && decision !== "non")) {
    return { ok: false, status: 409, error: problems[0] || "Réponse incomplète." };
  }
  let signatureBytes: Buffer | undefined;
  if (decision === "oui") {
    const png = decodeSignaturePng(input.signaturePng);
    if (!png.ok) return { ok: false, status: 400, error: png.error };
    signatureBytes = png.bytes;
  }
  const siblings = await listInvitations(trip.id);
  const other = siblingDecision(siblings, inv);
  if (other === "oui") {
    return { ok: false, status: 409, error: "L'autre responsable a déjà inscrit votre enfant : aucune action n'est nécessaire." };
  }
  if (other === "non" && decision === "non") {
    return { ok: false, status: 409, error: "L'autre responsable a déjà répondu que votre enfant ne participe pas." };
  }

  // Code : vérifié en dernier, pour ne consommer un essai que si tout le reste est prêt.
  const now = new Date();
  const check = checkSigningCode(inv.id, inv.code, input.code, now);
  if (check.result !== "ok") {
    inv.code = check.result === "wrong" ? check.next : undefined;
    await saveInvitation(inv);
    if (check.result === "wrong") {
      return { ok: false, status: 403, error: "Code incorrect.", attemptsLeft: check.attemptsLeft };
    }
    const msg =
      check.result === "expired"
        ? "Ce code a expiré. Demandez un nouveau code."
        : check.result === "locked"
          ? "Trop d'essais. Demandez un nouveau code."
          : "Demandez d'abord un code par e-mail.";
    return { ok: false, status: 403, error: msg };
  }
  inv.code = undefined;

  const signedAt = now.toISOString();
  const signerName = input.signerName.replace(/\s+/g, " ").trim();
  const model = buildFamilyPagePreviewModel(trip, {
    ...(win.deadline ? { familyPage: { deadline: win.deadline } } : {}),
  });

  if (decision === "non") {
    inv.signature = { decision: "non", signedAt, signerName };
    await saveInvitation(inv);
    return { ok: true, decision: "non" };
  }

  const reference = `V2-${trip.id.slice(0, 8)}-${inv.id.slice(0, 8)}`.toUpperCase();
  const sigKey = `travels-v2/${trip.id}/signatures/${inv.id}.png`;
  const pdfKey = `travels-v2/${trip.id}/engagements/${inv.id}.pdf`;
  const pdf = await buildSignedCircularPdf({
    model,
    studentName: inv.studentName,
    answers,
    decision: "oui",
    signerName,
    signedAt,
    maskedEmail: maskEmail(inv.email),
    documents: docs.map((d) => ({
      label: d.label,
      status: inv.documents?.[d.id]
        ? `déposé (${inv.documents[d.id].fileName})`
        : inv.exemptions?.[d.id]
          ? "dispense accordée par l'organisateur"
          : d.kind === "avant_depart"
            ? "à fournir avant le départ"
            : "non fournie",
    })),
    signaturePng: signatureBytes,
    reference,
  });
  await putObject(sigKey, signatureBytes!, "image/png");
  await putObject(pdfKey, pdf, "application/pdf");

  inv.signature = { decision: "oui", signedAt, signerName, signatureKey: sigKey, pdfKey };
  inv.engagement = buildEngagement({ inv, priceBlock: trip.data?.priceBlock, signerName, signedAt, pdfKey });
  await saveInvitation(inv);

  // Accusé par e-mail : un échec n'annule pas la signature (le PDF reste téléchargeable depuis la page).
  void sendConfirmationMail(ctx, pdf, model.title).catch((e) => console.error("[voyages-v2] accusé", e));
  return { ok: true, decision: "oui" };
}

export function buildIcsForInvitation(trip: Pick<TravelsTrip, "id" | "data">, inv: FamilyInvitation): string {
  const dates = tripDatesForIcs(trip);
  return buildFamilyIcs({
    tripId: trip.id,
    invitationId: inv.id,
    title: String(trip.data?.title || "Voyage scolaire"),
    destination: trip.data?.destination,
    ...dates,
    parentCalendar: trip.data?.parentCalendar,
    studentName: inv.studentName,
  });
}

async function sendConfirmationMail(ctx: PublicContext, pdf: Buffer, title: string): Promise<void> {
  const { trip, inv } = ctx;
  const mail = await mailContext();
  if (!mail) {
    inv.confirmationMailError = "SMTP non configuré";
    await saveInvitation(inv);
    return;
  }
  const ics = buildIcsForInvitation(trip, inv);
  const attachments: MailAttachment[] = [
    { filename: "circulaire-signee.pdf", content: pdf, contentType: "application/pdf" },
    { filename: "voyage.ics", content: ics, contentType: "text/calendar" },
  ];
  try {
    await sendMailWithTimeout(
      mail.transporter,
      {
        from: `"Sorties scolaires" <${mail.smtp.user}>`,
        to: inv.email,
        replyTo: validReplyTo(trip.ownerEmail),
        subject: `Inscription confirmée — ${title} (${firstName(inv.studentName)})`,
        text: `Bonjour,\n\nVotre signature est enregistrée : ${inv.studentName} est inscrit(e) au voyage « ${title} ».\nVous trouverez en pièces jointes la circulaire signée (PDF) et le fichier agenda (.ics : séjour et rendez-vous de départ et de retour).\n\n${mail.school}`,
        html: `<p>Bonjour,</p><p>Votre signature est enregistrée : <strong>${escapeHtml(inv.studentName)}</strong> est inscrit(e) au voyage <strong>« ${escapeHtml(title)} »</strong>.</p>
               <p>Vous trouverez en pièces jointes la circulaire signée (PDF) et le fichier agenda (.ics : séjour et rendez-vous de départ et de retour).</p>
               <p>Cordialement,<br/>${escapeHtml(mail.school)}</p>`,
        attachments,
      },
      60_000,
    );
    inv.confirmationMailSentAt = new Date().toISOString();
    inv.confirmationMailError = undefined;
  } catch (e) {
    inv.confirmationMailError = e instanceof Error ? e.message.slice(0, 200) : "Envoi impossible";
  }
  await saveInvitation(inv);
}

/* ─── Lecture des fichiers par la famille (via le jeton) ─── */

export async function readFamilyFile(
  ctx: PublicContext,
  kind: "pdf" | "ics",
): Promise<{ bytes: Buffer | string; contentType: string; fileName: string } | null> {
  const { trip, inv } = ctx;
  if (inv.signature?.decision !== "oui") return null;
  if (kind === "ics") {
    return { bytes: buildIcsForInvitation(trip, inv), contentType: "text/calendar; charset=utf-8", fileName: "voyage.ics" };
  }
  if (!inv.signature.pdfKey) return null;
  const bytes = await getObjectBytes(inv.signature.pdfKey);
  return bytes ? { bytes, contentType: "application/pdf", fileName: "circulaire-signee.pdf" } : null;
}

/* ─── Vue établissement (hub du voyage) ─── */

export type RegistrationRow = {
  invitationId: string | null;
  ine: string;
  studentName: string;
  classe?: string;
  email: string | null;
  status: InvitationStatus | "sans_email";
  statusLabel: string;
  sentAt?: string;
  lastSendError?: string;
  firstOpenedAt?: string;
  signedAt?: string;
  signerName?: string;
  /** Montant : renvoyé seulement aux profils qui y ont droit (compta, direction, admin). */
  amount?: number;
  documents: { id: string; label: string; kind: string; uploaded: boolean; exempt: boolean }[];
  extendedDeadline?: string;
};

export function buildRegistrationRows(opts: {
  trip: Pick<TravelsTrip, "data">;
  invitations: FamilyInvitation[];
  includeAmount: boolean;
}): RegistrationRow[] {
  const docs = opts.trip.data?.familyPage?.documents || [];
  const keys = picnicKeys(opts.trip);
  const rows: RegistrationRow[] = opts.invitations.map((inv) => {
    const status = computeInvitationStatus({ inv, docs, picnicDayKeys: keys });
    return {
      invitationId: inv.id,
      ine: inv.ine,
      studentName: inv.studentName,
      classe: inv.classe,
      email: inv.email,
      status,
      statusLabel: INVITATION_STATUS_LABELS[status],
      sentAt: inv.lastSentAt,
      lastSendError: inv.lastSendError,
      firstOpenedAt: inv.firstOpenedAt,
      signedAt: inv.signature?.signedAt,
      signerName: inv.signature?.signerName,
      ...(opts.includeAmount && inv.engagement ? { amount: inv.engagement.amount } : {}),
      documents: docs.map((d) => ({
        id: d.id,
        label: d.label,
        kind: d.kind,
        uploaded: Boolean(inv.documents?.[d.id]),
        exempt: Boolean(inv.exemptions?.[d.id]),
      })),
      extendedDeadline: inv.extendedDeadline,
    };
  });
  const withInv = new Set(opts.invitations.map((i) => i.ine));
  for (const p of opts.trip.data?.preListeEleves || []) {
    const ine = String(p.ine || "").trim();
    if (!ine || withInv.has(ine)) continue;
    rows.push({
      invitationId: null,
      ine,
      studentName: studentDisplayName(p.prenom, p.nom),
      classe: p.classe,
      email: null,
      status: "sans_email",
      statusLabel: "Aucun e-mail de responsable connu",
      documents: [],
    });
  }
  return rows.sort((a, b) => a.studentName.localeCompare(b.studentName, "fr", { sensitivity: "base" }));
}

/** URL pré-signée courte (2 min), à n'émettre qu'après contrôle des droits côté appelant. */
export async function signedUrlForInvitationFile(
  inv: FamilyInvitation,
  what: { kind: "document"; docId: string } | { kind: "engagement" },
): Promise<string | null> {
  if (what.kind === "engagement") {
    if (!inv.signature?.pdfKey) return null;
    return getSignedReadUrl(inv.signature.pdfKey, 120, { downloadFileName: `engagement-${inv.ine}.pdf` });
  }
  const d = inv.documents?.[what.docId];
  if (!d?.key) return null;
  return getSignedReadUrl(d.key, 120, { downloadFileName: d.fileName });
}

