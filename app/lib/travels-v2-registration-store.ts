import "server-only";

import { randomUUID } from "node:crypto";
import { loadAppConfig } from "@/app/lib/app-config";
import { collectEleveParentEmails, normalizeParentEmail } from "@/app/lib/eleves-parent-emails";
import { loadElevesActifsRegistry } from "@/app/lib/eleves-registry";
import { maskEmail } from "@/app/lib/mask-email";
import {
  deleteJson,
  deleteObject,
  getJson,
  getObjectBytes,
  getSignedReadUrl,
  putJson,
  putObject,
} from "@/app/lib/s3-storage";
import {
  listJsonRecordsInDir,
  listJsonRecordsInDirByStringAttr,
  mutateJsonRecordLocked,
  putJsonIfAbsentToPostgres,
  withNamedLock,
} from "@/app/lib/ent-json-postgres";
import { sanitizeS3FileName } from "@/app/lib/s3-path";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
  sendMailWithTimeout,
} from "@/app/lib/tenant-mail";
import { buildFamilyPagePreviewModel, listPicnicDays } from "@/app/lib/travels-family-page";
import { FAMILY_PAGE_TEMPLATE } from "@/app/lib/travels-family-page-template";
import { getTravelTrip, listTravelsIndex } from "@/app/lib/travels-storage";
import type { TravelsTrip } from "@/app/lib/travels-types";
import { isWorkflowV2, todayIsoParis } from "@/app/lib/travels-v2";
import { selectInvitationsToSend } from "@/app/lib/travels-v2-lot4";
import {
  buildEngagement,
  buildFamilyIcs,
  buildShownSnapshot,
  buildInviteToken,
  buildPublicFamilyView,
  generateInviteSecret,
  hashInviteSecret,
  parseInviteToken,
  siblingDecision,
  tripDatesForIcs,
  verifyInviteSecret,
  type PublicFamilyView,
} from "@/app/lib/travels-v2-registration";
import {
  computeInvitationStatus,
  decodeSignaturePng,
  DOC_MAX_BYTES,
  INVITATION_STATUS_LABELS,
  isLocked,
  registrationWindow,
  signInputSchema,
  signingProblems,
  sniffAllowedDocType,
  type FamilyInvitation,
  type InvitationStatus,
} from "@/app/lib/travels-v2-registration-types";
import { buildSignedCircularPdf } from "@/app/lib/travels-v2-signed-pdf";
import { canRenderInPdf, PDF_NAME_RESTRICTION_MESSAGE } from "@/app/lib/travels-v2-pdf-text";
import {
  caseId,
  canObject,
  canUploadAfterSignature,
  evaluateClaim,
  studentState,
  type EngagementClaim,
  type StudentCase,
  type StudentState,
} from "@/app/lib/travels-v2-lot3";
import { getTenantAppUrl } from "@/app/lib/tenant-context";
import {
  mutAddTokenHash,
  mutAnswers,
  mutApplySignature,
  mutConsumeSigningCode,
  mutRemoveDocument,
  mutReserveSigningCode,
  mutRollbackSigningCode,
  mutSetDocument,
  mutTokenSendFailed,
  mutTokenSendOk,
  mutTouchSeen,
  windowOfInvitation,
  type ClosedReason,
  type Mut,
  type WindowInput,
} from "@/app/lib/travels-v2-invitation-mutations";

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

/**
 * Écriture complète d'une invitation (création, purge). Pour TOUTE modification d'une invitation existante
 * (réponses, code, signature, pièces, liens, relances…) utiliser `mutateInvitation` : relecture + écriture dans un
 * verrou, sinon deux requêtes concurrentes s'écrasent.
 */
export async function saveInvitation(inv: FamilyInvitation): Promise<void> {
  await putJson(invPath(inv.tripId, inv.id), inv);
}

/**
 * Mutation sérialisée d'une invitation : verrou advisory transactionnel par invitation
 * (`pg_advisory_xact_lock(hashtext(...))`), relecture DANS le verrou, `fn` (synchrone, sans entrée-sortie)
 * décide et modifie, écriture dans la même transaction. Retourne l'invitation à jour (`inv`) et la valeur de `fn`,
 * ou null si l'invitation n'existe pas.
 */
export async function mutateInvitation<R>(
  tripId: string,
  invId: string,
  fn: (inv: FamilyInvitation) => Mut<R>,
): Promise<{ value: R; inv: FamilyInvitation; saved: boolean } | null> {
  if (!TRIP_ID_RE.test(tripId) || !TRIP_ID_RE.test(invId)) return null;
  const box: { snap: FamilyInvitation | null } = { snap: null };
  const res = await mutateJsonRecordLocked<Record<string, unknown>, R>(invPath(tripId, invId), (current) => {
    const inv = current as unknown as FamilyInvitation;
    if (inv.tripId !== tripId || inv.id !== invId) return { save: false, value: undefined as unknown as R };
    const out = fn(inv);
    box.snap = structuredClone(inv);
    return out;
  });
  if (!res || !box.snap) return null;
  return { value: res.value, inv: box.snap, saved: res.saved };
}

/** Invitations d'UN élève (requête ciblée : pas besoin de charger tout le voyage). */
export async function listInvitationsForIne(tripId: string, ine: string): Promise<FamilyInvitation[]> {
  if (!TRIP_ID_RE.test(tripId) || !ine) return [];
  const rows = await listJsonRecordsInDirByStringAttr<Record<string, unknown>>(invDir(tripId), "ine", ine);
  return rows
    .filter((r) => r && typeof r === "object" && r.tripId === tripId && r.ine === ine && typeof r.id === "string")
    .map((r) => r as unknown as FamilyInvitation);
}

function windowInputOf(trip: Pick<TravelsTrip, "status" | "data">): WindowInput {
  return {
    tripStatus: trip.status,
    openedAt: trip.data?.familyPage?.openedAt,
    deadline: trip.data?.familyPage?.deadline,
    todayIso: todayIsoParis(),
  };
}

/* ─── Création des invitations à l'ouverture de la page ─── */

/** Durée de vie du verrou « par voyage » des envois d'e-mails et créations d'invitations (reprise si le serveur tombe). */
export const TRIP_LOCK_TTL_SECONDS = 20 * 60;
export const TRIP_INVITES_LOCK = "trip-invites";

export type CreateInvitationsResult = {
  /** Vrai si une autre création ou un autre envoi tourne déjà pour ce voyage : rien n'a été fait. */
  busy?: boolean;
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
  // Verrou par voyage : deux créations simultanées (clic + ouverture, deux onglets) ne doublent pas les invitations.
  const locked = await withNamedLock(TRIP_INVITES_LOCK, trip.id, TRIP_LOCK_TTL_SECONDS, () => createInvitationsUnlocked(trip));
  return locked.busy ? { busy: true, created: [], existing: 0, studentsWithoutEmail: [] } : locked.value;
}

async function createInvitationsUnlocked(trip: Pick<TravelsTrip, "id" | "data">): Promise<CreateInvitationsResult> {
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

export function escapeHtml(s: string): string {
  return String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]!);
}

export type MailAttachment = { filename: string; content: Buffer | string; contentType: string };

export async function mailContext() {
  const smtp = await getTenantSmtpConfig();
  const transporter = smtp ? await createTenantTransporter() : null;
  if (!smtp || !transporter) return null;
  const bundle = await loadAppConfig().catch(() => null);
  const school = bundle?.identity?.shortName || bundle?.identity?.name || "L'établissement";
  return { smtp, transporter, school };
}

/**
 * E-mail aux familles : pas d'adresse de réponse (l'expéditeur mailer@ ne lit pas les réponses) et mention
 * fixe en fin de message renvoyant vers la messagerie de l'établissement ou le carnet de correspondance.
 */
export function familyMail<T extends { text: string; html: string; replyTo?: string }>(msg: T): Omit<T, "replyTo"> {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const { replyTo: _ignored, ...rest } = msg;
  const note = FAMILY_PAGE_TEMPLATE.mailNoReply;
  return {
    ...rest,
    text: `${msg.text}\n\n—\n${note}`,
    html: `${msg.html}<p style="font-family: Arial, sans-serif; font-size:12px; color:#64748b; border-top:1px solid #e2e8f0; padding-top:12px; margin-top:24px; max-width:560px;">${escapeHtml(note)}</p>`,
  };
}

export function validReplyTo(email: string | undefined): string | undefined {
  const e = String(email || "").trim();
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(e) ? e : undefined;
}

export function firstName(studentName: string): string {
  return studentName.split(" ")[0] || studentName;
}

/**
 * Émet un NOUVEAU lien personnel : le secret n'est jamais stocké, seule son empreinte l'est, écrite sous verrou
 * AVANT l'e-mail (le lien ne fonctionne qu'une fois l'empreinte enregistrée). `aux` = relances et copies
 * automatiques : emplacements séparés (5), ils n'évincent jamais un lien principal (10 emplacements : envoi initial
 * et renvois demandés par la famille ou l'organisateur).
 * Retourne le lien et l'empreinte (pour défaire en cas d'échec d'envoi), ou null si l'invitation a disparu.
 */
export async function issueInviteLink(
  inv: FamilyInvitation,
  baseUrl: string,
  opts?: { aux?: boolean },
): Promise<{ link: string; hash: string } | null> {
  const secret = generateInviteSecret();
  const hash = hashInviteSecret(secret);
  const res = await mutateInvitation(inv.tripId, inv.id, (cur) => mutAddTokenHash(cur, hash, opts?.aux === true));
  if (!res) return null;
  Object.assign(inv, res.inv);
  const link = `${baseUrl}/voyages/famille/${encodeURIComponent(
    buildInviteToken({ invitationId: inv.id, tripId: inv.tripId, secret }),
  )}`;
  return { link, hash };
}

/** L'e-mail du lien n'est pas parti : on retire l'empreinte (pas de lien orphelin). */
export async function discardInviteLink(inv: FamilyInvitation, hash: string, error?: string): Promise<void> {
  const res = await mutateInvitation(inv.tripId, inv.id, (cur) => mutTokenSendFailed(cur, hash, error));
  if (res) Object.assign(inv, res.inv);
}

/** Envoie (ou renvoie) le lien personnel principal. Retourne true si l'e-mail est parti. */
export async function sendInvitationLink(opts: {
  trip: Pick<TravelsTrip, "id" | "data" | "ownerEmail" | "ownerName">;
  inv: FamilyInvitation;
  baseUrl: string;
  mail: NonNullable<Awaited<ReturnType<typeof mailContext>>>;
}): Promise<boolean> {
  const { trip, inv, baseUrl, mail } = opts;
  const issued = await issueInviteLink(inv, baseUrl);
  if (!issued) return false;
  const { link, hash } = issued;
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
      familyMail({
        from: `"Sorties scolaires" <${mail.smtp.user}>`,
        to: inv.email,
        subject: `Voyage scolaire « ${title} » — inscription de ${kid}`,
        text,
        html,
      }),
      60_000,
    );
    const res = await mutateInvitation(inv.tripId, inv.id, (cur) => mutTokenSendOk(cur, new Date().toISOString(), true));
    if (res) Object.assign(inv, res.inv);
    return true;
  } catch (e) {
    await discardInviteLink(inv, hash, e instanceof Error ? e.message.slice(0, 200) : "Envoi impossible");
    return false;
  }
}

type MailCtx = NonNullable<Awaited<ReturnType<typeof mailContext>>>;
type SendTrip = Pick<TravelsTrip, "id" | "data" | "ownerEmail" | "ownerName">;

/**
 * Envoi des liens SANS prendre le verrou (l'appelant le tient). `pick` choisit les invitations dans la liste lue ;
 * `take` (optionnel) est un plafond d'e-mails : à `false`, on s'arrête.
 */
async function sendLinksUnlocked(opts: {
  trip: SendTrip;
  baseUrl: string;
  mail: MailCtx;
  pick: (all: FamilyInvitation[]) => FamilyInvitation[];
  take?: () => boolean;
}): Promise<{ sent: number; failed: number; capped: boolean }> {
  const all = await listInvitations(opts.trip.id);
  let sent = 0;
  let failed = 0;
  let capped = false;
  for (const inv of opts.pick(all)) {
    if (opts.take && !opts.take()) {
      capped = true;
      break;
    }
    if (await sendInvitationLink({ trip: opts.trip, inv, baseUrl: opts.baseUrl, mail: opts.mail })) sent += 1;
    else failed += 1;
  }
  return { sent, failed, capped };
}

/** Envoie les liens des invitations demandées (ou de toutes celles jamais envoyées). */
export async function sendPendingInvitations(opts: {
  trip: SendTrip;
  baseUrl: string;
  only?: string[];
}): Promise<{ sent: number; failed: number; smtp: boolean; busy?: boolean }> {
  const mail = await mailContext();
  if (!mail) return { sent: 0, failed: 0, smtp: false };
  // Verrou par voyage : un seul envoi de liens à la fois (pas d'e-mails en double si deux envois se croisent).
  const locked = await withNamedLock(TRIP_INVITES_LOCK, opts.trip.id, TRIP_LOCK_TTL_SECONDS, async () => {
    const { sent, failed } = await sendLinksUnlocked({
      trip: opts.trip,
      baseUrl: opts.baseUrl,
      mail,
      pick: (all) => all.filter((i) => (opts.only ? opts.only.includes(i.id) : !i.firstSentAt && !i.signature)),
    });
    return { sent, failed };
  });
  if (locked.busy) return { sent: 0, failed: 0, smtp: true, busy: true };
  return { ...locked.value, smtp: true };
}

/**
 * Filet de sécurité (job horaire) : envoie les liens jamais envoyés d'un voyage ouvert, avec un nombre d'essais limité
 * pour ceux déjà en échec, dans le plafond d'e-mails du passage. Ne crée aucune invitation.
 */
export async function sendUnsentInvitationsSafetyNet(opts: {
  trip: SendTrip;
  baseUrl: string;
  take: () => boolean;
}): Promise<{ sent: number; failed: number; abandoned: number; capped: boolean; busy?: boolean; smtp: boolean }> {
  const preview = selectInvitationsToSend(await listInvitations(opts.trip.id));
  if (preview.send.length + preview.retry.length === 0) {
    return { sent: 0, failed: 0, abandoned: preview.abandoned.length, capped: false, smtp: true };
  }
  const mail = await mailContext();
  if (!mail) return { sent: 0, failed: 0, abandoned: preview.abandoned.length, capped: false, smtp: false };
  const locked = await withNamedLock(TRIP_INVITES_LOCK, opts.trip.id, TRIP_LOCK_TTL_SECONDS, () =>
    sendLinksUnlocked({
      trip: opts.trip,
      baseUrl: opts.baseUrl,
      mail,
      take: opts.take,
      // Relecture sous verrou : une invitation envoyée entre-temps (autre envoi) n'est pas renvoyée.
      pick: (all) => {
        const sel = selectInvitationsToSend(all);
        const ids = new Set([...sel.send, ...sel.retry]);
        return all.filter((i) => ids.has(i.id));
      },
    }),
  );
  if (locked.busy) return { sent: 0, failed: 0, abandoned: preview.abandoned.length, capped: false, busy: true, smtp: true };
  return { ...locked.value, abandoned: preview.abandoned.length, smtp: true };
}

export type OpenRegistrationsResult = CreateInvitationsResult & { sent: number; failed: number; smtp: boolean };

/**
 * Ouverture de la page (validation finale) : crée les invitations puis envoie les liens, sous UN SEUL verrou
 * (pas de fenêtre où un autre envoi s'intercale entre la création et l'envoi). Journalise le résultat sans donnée
 * personnelle. Si le verrou est occupé ou si l'SMTP est absent, le job horaire reprend les envois.
 */
export async function openRegistrationsForTrip(opts: {
  trip: SendTrip;
  baseUrl: string;
}): Promise<OpenRegistrationsResult> {
  const tag = `[voyages-v2-ouverture] voyage ${opts.trip.id}`;
  const mail = await mailContext();
  const locked = await withNamedLock(TRIP_INVITES_LOCK, opts.trip.id, TRIP_LOCK_TTL_SECONDS, async () => {
    const created = await createInvitationsUnlocked(opts.trip);
    if (!mail) return { ...created, sent: 0, failed: 0, smtp: false };
    const { sent, failed } = await sendLinksUnlocked({
      trip: opts.trip,
      baseUrl: opts.baseUrl,
      mail,
      pick: (all) => all.filter((i) => !i.firstSentAt && !i.signature),
    });
    return { ...created, sent, failed, smtp: true };
  });
  if (locked.busy) {
    console.error(`${tag}: verrou occupé, rien créé ni envoyé (le job horaire reprendra les envois)`);
    return { busy: true, created: [], existing: 0, studentsWithoutEmail: [], sent: 0, failed: 0, smtp: Boolean(mail) };
  }
  const r = locked.value;
  if (!r.smtp) {
    console.error(`${tag}: ${r.created.length} invitation(s) créée(s), SMTP absent : aucun e-mail envoyé (le job horaire reprendra les envois)`);
  } else if (r.failed > 0) {
    console.error(`${tag}: ${r.created.length} créée(s), ${r.sent} envoyée(s), ${r.failed} échec(s) d'envoi`);
  } else {
    console.info(`${tag}: ${r.created.length} créée(s), ${r.existing} déjà existante(s), ${r.sent} envoyée(s), ${r.studentsWithoutEmail.length} sans e-mail`);
  }
  return r;
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
    // Une invitation déjà signée n'a plus rien à faire : pas de nouveau lien (et pas d'emplacement de jeton consommé).
    const invs = (await listInvitations(trip.id)).filter((i) => i.email === email && !i.signature);
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
  // Liens principaux et liens des relances / copies (emplacements séparés).
  const secretOk = inv
    ? verifyInviteSecret(inv.tokenHashes, parsed.secret) || verifyInviteSecret(inv.auxTokenHashes, parsed.secret)
    : false;
  if (!inv || !secretOk) return null;
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
  // Marque l'ouverture (et une présence récente, au plus toutes les 5 minutes) — sans écrire si inutile.
  const nowMs = Date.now();
  const last = ctx.inv.lastSeenAt ? new Date(ctx.inv.lastSeenAt).getTime() : 0;
  if (!ctx.inv.firstOpenedAt || nowMs - last > 5 * 60 * 1000) {
    const touched = await mutateInvitation(ctx.trip.id, ctx.inv.id, (cur) => mutTouchSeen(cur, nowMs)).catch((e) => {
      console.error("[voyages-v2] lastSeen", e);
      return null;
    });
    if (touched) ctx.inv = touched.inv;
  }
  // Requête ciblée : seules les invitations de CET élève (l'autre responsable) sont nécessaires.
  const [sibs, caseRec] = await Promise.all([
    listInvitationsForIne(ctx.trip.id, ctx.inv.ine),
    getStudentCase(ctx.trip.id, ctx.inv.ine),
  ]);
  const mine = sibs.filter((i) => i.ine === ctx.inv.ine);
  return buildPublicFamilyView({
    trip: ctx.trip,
    inv: ctx.inv,
    todayIso: todayIsoParis(),
    maskedEmail: maskEmail(ctx.inv.email),
    siblingDecision: siblingDecision(sibs, ctx.inv),
    studentState: studentState(mine, caseRec),
    canObject: canObject({ inv: ctx.inv, siblings: sibs, caseRec }).ok,
    objectionDecision: caseRec?.decision?.kind ?? null,
  });
}

/* ─── Réponses (brouillon) ─── */

/**
 * Fusion champ par champ SOUS VERROU : la requête est appliquée sur la dernière version de l'invitation, jamais
 * sur la copie lue au début de la requête (deux enregistrements rapprochés ne s'écrasent donc pas).
 */
export async function saveAnswers(
  ctx: PublicContext,
  input: unknown,
): Promise<{ ok: true } | { ok: false; status: number; error: string }> {
  const res = await mutateInvitation(ctx.trip.id, ctx.inv.id, (inv) =>
    mutAnswers(inv, { input, picnicDayKeys: picnicKeys(ctx.trip), nowIso: new Date().toISOString(), window: windowInputOf(ctx.trip) }),
  );
  if (!res) return { ok: false, status: 404, error: "Lien invalide ou expiré." };
  ctx.inv = res.inv;
  const v = res.value;
  if (v.ok) return { ok: true };
  return v.status === 409 ? { ok: false, status: 409, error: closedMessage(v.reason) } : { ok: false, status: 400, error: v.error };
}

export async function objectionCheck(ctx: PublicContext) {
  // Un désaccord n'a de sens que pendant les inscriptions : après la clôture la liste officielle est figée.
  if (ctx.trip.status !== "INSCRIPTIONS_OUVERTES") return { ok: false as const, error: "Les inscriptions de ce voyage sont closes." };
  const [sibs, caseRec] = await Promise.all([listInvitationsForIne(ctx.trip.id, ctx.inv.ine), getStudentCase(ctx.trip.id, ctx.inv.ine)]);
  return canObject({ inv: ctx.inv, siblings: sibs, caseRec });
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

const DEAD_STATUSES = new Set(["ANNULE", "REJETE", "SEANCE_ANNULEE"]);

/** Pièce « à fournir avant le départ » : dépôt encore possible après la signature, jusqu'à sa date propre. */
export function afterSignUploadOkFor(trip: Pick<TravelsTrip, "status" | "data">, inv: Pick<FamilyInvitation, "signature">, docId: string): boolean {
  if (DEAD_STATUSES.has(String(trip.status))) return false;
  const doc = (trip.data?.familyPage?.documents || []).find((d) => d.id === docId);
  return canUploadAfterSignature(doc, inv, todayIsoParis());
}

export function afterSignUploadOk(ctx: PublicContext, docId: string): boolean {
  return afterSignUploadOkFor(ctx.trip, ctx.inv, docId);
}

/** Dépôt autorisé pour CETTE version de l'invitation (page ouverte, ou pièce « avant départ » après signature). */
function documentWriteAllowed(trip: Pick<TravelsTrip, "status" | "data">, docId: string) {
  const wi = windowInputOf(trip);
  return (inv: FamilyInvitation) => windowOfInvitation(inv, wi).open || afterSignUploadOkFor(trip, inv, docId);
}

function closedReasonFor(trip: Pick<TravelsTrip, "status" | "data">, inv: FamilyInvitation): ClosedReason {
  const w = windowOfInvitation(inv, windowInputOf(trip));
  return w.open ? "locked" : w.reason;
}

export async function storeFamilyDocument(
  ctx: PublicContext,
  docId: string,
  fileName: string,
  bytes: Buffer,
): Promise<{ ok: true } | { ok: false; status: number; error: string }> {
  const allowed = documentWriteAllowed(ctx.trip, docId);
  const request = (ctx.trip.data?.familyPage?.documents || []).find((d) => d.id === docId);
  if (!allowed(ctx.inv)) return { ok: false, status: 409, error: closedMessage(closedReasonFor(ctx.trip, ctx.inv)) };
  if (!request) return { ok: false, status: 400, error: "Pièce inconnue." };
  if (bytes.length === 0 || bytes.length > DOC_MAX_BYTES) {
    return { ok: false, status: 413, error: "Fichier vide ou trop volumineux (8 Mo maximum)." };
  }
  const type = sniffAllowedDocType(bytes);
  if (!type) return { ok: false, status: 415, error: "Format non accepté : PDF ou image (JPEG, PNG, WebP) uniquement." };
  const key = `travels-v2/${ctx.trip.id}/docs/${ctx.inv.id}/${docId}-${randomUUID()}.${type.ext}`;
  await putObject(key, bytes, type.contentType);
  const doc = {
    fileName: sanitizeS3FileName(fileName).slice(0, 120),
    key,
    size: bytes.length,
    contentType: type.contentType,
    uploadedAt: new Date().toISOString(),
  };
  // L'enregistrement de la référence se fait sous verrou, sur la dernière version (réponses et autres pièces intactes).
  const res = await mutateInvitation(ctx.trip.id, ctx.inv.id, (inv) => mutSetDocument(inv, docId, doc, allowed));
  if (!res || !res.value.ok) {
    await deleteObject(key).catch(() => undefined);
    return res
      ? { ok: false, status: 409, error: closedMessage(closedReasonFor(ctx.trip, res.inv)) }
      : { ok: false, status: 404, error: "Lien invalide ou expiré." };
  }
  ctx.inv = res.inv;
  if (res.value.previousKey) await deleteObject(res.value.previousKey).catch(() => undefined);
  return { ok: true };
}

export async function removeFamilyDocument(
  ctx: PublicContext,
  docId: string,
): Promise<{ ok: true } | { ok: false; status: number; error: string }> {
  const allowed = documentWriteAllowed(ctx.trip, docId);
  if (!allowed(ctx.inv)) return { ok: false, status: 409, error: closedMessage(closedReasonFor(ctx.trip, ctx.inv)) };
  const res = await mutateInvitation(ctx.trip.id, ctx.inv.id, (inv) => mutRemoveDocument(inv, docId, allowed));
  if (!res) return { ok: false, status: 404, error: "Lien invalide ou expiré." };
  ctx.inv = res.inv;
  if (!res.value.ok) return { ok: false, status: 409, error: closedMessage(closedReasonFor(ctx.trip, res.inv)) };
  if (res.value.previousKey) await deleteObject(res.value.previousKey).catch(() => undefined);
  return { ok: true };
}

/* ─── Code de signature ─── */

export async function sendSigningCode(
  ctx: PublicContext,
  opts?: { objection?: boolean },
): Promise<{ ok: true; maskedEmail: string } | { ok: false; status: number; error: string; retryAfterSec?: number }> {
  if (opts?.objection) {
    const chk = await objectionCheck(ctx);
    if (!chk.ok) return { ok: false, status: 409, error: chk.error };
  }
  const now = new Date();
  // Limites d'envoi (30 s entre deux codes, 5 par heure) contrôlées ET réservées sous verrou : des requêtes
  // parallèles ne peuvent pas les contourner. L'e-mail ne part qu'une fois la réservation enregistrée.
  const reserved = await mutateInvitation(ctx.trip.id, ctx.inv.id, (inv) =>
    mutReserveSigningCode(inv, { now, objection: opts?.objection === true, window: windowInputOf(ctx.trip) }),
  );
  if (!reserved) return { ok: false, status: 404, error: "Lien invalide ou expiré." };
  ctx.inv = reserved.inv;
  const v = reserved.value;
  if (!v.ok) {
    if (v.status === 409) return { ok: false, status: 409, error: closedMessage(v.reason) };
    if (v.status === 400) return { ok: false, status: 400, error: v.error };
    return {
      ok: false,
      status: 429,
      error: "Trop de demandes de code. Réessayez dans quelques instants.",
      retryAfterSec: v.retryAfterSec,
    };
  }
  const { reservation } = v;
  const undo = async () => {
    const r = await mutateInvitation(ctx.trip.id, ctx.inv.id, (inv) => mutRollbackSigningCode(inv, reservation)).catch(() => null);
    if (r) ctx.inv = r.inv;
  };
  const mail = await mailContext();
  if (!mail) {
    await undo();
    return { ok: false, status: 503, error: "L'envoi d'e-mails n'est pas disponible pour le moment." };
  }
  const title = String(ctx.trip.data?.title || "Voyage scolaire");
  const code = reservation.code;
  try {
    await sendMailWithTimeout(
      mail.transporter,
      familyMail({
        from: `"Sorties scolaires" <${mail.smtp.user}>`,
        to: ctx.inv.email,
        subject: `Votre code de signature : ${code}`,
        text: `Votre code de signature pour le voyage « ${title} » : ${code}\nIl est valable 10 minutes. Ne le communiquez à personne.`,
        html: `<p>Votre code de signature pour le voyage <strong>« ${escapeHtml(title)} »</strong> :</p>
               <p style="font-size:28px;font-weight:bold;letter-spacing:6px;">${code}</p>
               <p style="color:#64748b;font-size:13px;">Il est valable 10 minutes. Ne le communiquez à personne.</p>`,
      }),
      30_000,
    );
  } catch {
    await undo();
    return { ok: false, status: 502, error: "L'e-mail n'a pas pu être envoyé. Réessayez dans un instant." };
  }
  return { ok: true, maskedEmail: maskEmail(ctx.inv.email) };
}

/* ─── Signature, accusé, engagement ─── */

export type SignResult =
  | { ok: true; decision: "oui" | "non" }
  | { ok: false; status: number; error: string; attemptsLeft?: number };

/** Empreinte stable des réponses (indépendante de l'ordre des clés et de la date de mise à jour). */
export function answersFingerprint(a: FamilyInvitation["answers"] | undefined): string {
  const meals = a?.meals || {};
  return JSON.stringify({
    p: a?.participates ?? "",
    ph: a?.phone ?? "",
    m: Object.keys(meals)
      .sort()
      .map((k) => [k, meals[k]?.wants ?? "", meals[k]?.type ?? ""]),
  });
}

/**
 * Signature en trois temps, chacun sous verrou sur la dernière version de l'invitation :
 *  1. vérification du code ET persistance du résultat (compteur d'essais, usage unique) — garde : réponses
 *     inchangées depuis l'affichage, page ouverte, pas déjà signée (aucun essai consommé si la garde refuse) ;
 *  2. jeton d'engagement unique par élève, PDF et tracé écrits dans le stockage objet ;
 *  3. pose de la signature (refusée si une signature existe déjà).
 */
export async function submitSignature(ctx: PublicContext, rawInput: unknown): Promise<SignResult> {
  const { trip } = ctx;
  const wi = windowInputOf(trip);
  const win = windowOfInvitation(ctx.inv, wi);
  if (!win.open) return { ok: false, status: 409, error: closedMessage(win.reason) };
  const parsed = signInputSchema.safeParse(rawInput);
  if (!parsed.success) return { ok: false, status: 400, error: "Saisie invalide : nom, code à 6 chiffres et signature." };
  const input = parsed.data;
  const answers = ctx.inv.answers || {};
  const decision = answers.participates;
  const docs = trip.data?.familyPage?.documents || [];
  const problems = signingProblems({ answers, picnicDayKeys: picnicKeys(trip), docs, inv: ctx.inv });
  if (problems.length > 0 || (decision !== "oui" && decision !== "non")) {
    return { ok: false, status: 409, error: problems[0] || "Réponse incomplète." };
  }
  if (decision === "oui" && !canRenderInPdf(input.signerName)) {
    return { ok: false, status: 400, error: PDF_NAME_RESTRICTION_MESSAGE };
  }
  let signatureBytes: Buffer | undefined;
  if (decision === "oui") {
    const png = decodeSignaturePng(input.signaturePng);
    if (!png.ok) return { ok: false, status: 400, error: png.error };
    signatureBytes = png.bytes;
  }
  const siblings = await listInvitationsForIne(trip.id, ctx.inv.ine);
  const other = siblingDecision(siblings, ctx.inv);
  if (other === "oui") {
    return { ok: false, status: 409, error: "L'autre responsable a déjà inscrit votre enfant : aucune action n'est nécessaire." };
  }
  if (other === "non" && decision === "non") {
    return { ok: false, status: 409, error: "L'autre responsable a déjà répondu que votre enfant ne participe pas." };
  }

  // 1. Code : vérifié et persisté sous verrou, en dernier, pour ne consommer un essai que si tout le reste est prêt.
  const now = new Date();
  const fingerprint = answersFingerprint(answers);
  const consumed = await mutateInvitation(trip.id, ctx.inv.id, (cur) =>
    mutConsumeSigningCode(cur, {
      input: input.code,
      now,
      guard: (c) => {
        if (c.signature) return "Votre réponse est déjà signée.";
        const w = windowOfInvitation(c, wi);
        if (!w.open) return closedMessage(w.reason);
        if (answersFingerprint(c.answers) !== fingerprint) {
          return "Vos réponses viennent de changer : vérifiez-les puis signez à nouveau.";
        }
        return null;
      },
    }),
  );
  if (!consumed) return { ok: false, status: 404, error: "Lien invalide ou expiré." };
  ctx.inv = consumed.inv;
  const outcome = consumed.value;
  if (outcome.kind === "guard") return { ok: false, status: 409, error: outcome.error };
  const check = outcome.check;
  if (check.result !== "ok") {
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
  const inv = ctx.inv;

  // 2. Un seul engagement par élève, même si les deux responsables signent « oui » au même instant.
  let claimCreated = false;
  if (decision === "oui") {
    const claim = await claimEngagement(trip.id, inv);
    if (!claim.ok) {
      return { ok: false, status: 409, error: "L'autre responsable vient d'inscrire votre enfant : aucune action n'est nécessaire." };
    }
    claimCreated = claim.created;
  }

  const signedAt = now.toISOString();
  const signerName = input.signerName.replace(/\s+/g, " ").trim();
  const model = buildFamilyPagePreviewModel(trip, {
    ...(win.deadline ? { familyPage: { deadline: win.deadline } } : {}),
  });

  if (decision === "non") {
    const res = await mutateInvitation(trip.id, inv.id, (cur) =>
      mutApplySignature(cur, { signature: { decision: "non", signedAt, signerName } }, wi),
    );
    if (!res || !res.value.ok) {
      return { ok: false, status: 409, error: res && !res.value.ok ? res.value.error : "Lien invalide ou expiré." };
    }
    ctx.inv = res.inv;
    return { ok: true, decision: "non" };
  }

  const reference = `V2-${trip.id.slice(0, 8)}-${inv.id.slice(0, 8)}`.toUpperCase();
  const sigKey = `travels-v2/${trip.id}/signatures/${inv.id}.png`;
  const pdfKey = `travels-v2/${trip.id}/engagements/${inv.id}.pdf`;
  let pdf: Buffer;
  try {
    pdf = await buildSignedCircularPdf({
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
    // 3. Pose de la signature (jamais deux fois) + instantané de ce qui était affiché (prix, mois, pièces, date limite).
    const engagement = buildEngagement({
      inv,
      priceBlock: trip.data?.priceBlock,
      signerName,
      signedAt,
      pdfKey,
      shown: buildShownSnapshot(trip.data, win.deadline, signedAt),
    });
    const res = await mutateInvitation(trip.id, inv.id, (cur) =>
      mutApplySignature(cur, { signature: { decision: "oui", signedAt, signerName, signatureKey: sigKey, pdfKey }, engagement }, wi),
    );
    if (!res || !res.value.ok) {
      if (claimCreated) await releaseEngagementClaim(trip.id, inv);
      return { ok: false, status: 409, error: res && !res.value.ok ? res.value.error : "Lien invalide ou expiré." };
    }
    ctx.inv = res.inv;
  } catch (e) {
    // Signature non enregistrée : on libère le jeton pour que l'élève ne reste pas bloqué.
    if (claimCreated) await releaseEngagementClaim(trip.id, inv);
    throw e;
  }

  // Accusé par e-mail : un échec n'annule pas la signature (le PDF reste téléchargeable depuis la page).
  void sendConfirmationMail(ctx, pdf, model.title).catch((e) => console.error("[voyages-v2] accusé", e));
  // Copie au(x) autre(s) responsable(s) avec le lien « Je ne suis pas d'accord ».
  void getTenantAppUrl()
    .then((baseUrl) => sendSiblingCopies({ trip, signed: ctx.inv, baseUrl }))
    .catch((e) => console.error("[voyages-v2] copie second responsable", e));
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
  const note = async (patch: { confirmationMailSentAt?: string; confirmationMailError?: string | undefined }) => {
    await mutateInvitation(inv.tripId, inv.id, (cur) => {
      cur.confirmationMailSentAt = patch.confirmationMailSentAt ?? cur.confirmationMailSentAt;
      cur.confirmationMailError = patch.confirmationMailError;
      return { save: true, value: undefined };
    });
  };
  const mail = await mailContext();
  if (!mail) {
    await note({ confirmationMailError: "SMTP non configuré" });
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
      familyMail({
        from: `"Sorties scolaires" <${mail.smtp.user}>`,
        to: inv.email,
        subject: `Inscription confirmée — ${title} (${firstName(inv.studentName)})`,
        text: `Bonjour,\n\nVotre signature est enregistrée : ${inv.studentName} est inscrit(e) au voyage « ${title} ».\nVous trouverez en pièces jointes la circulaire signée (PDF) et le fichier agenda (.ics : séjour et rendez-vous de départ et de retour).\n\n${mail.school}`,
        html: `<p>Bonjour,</p><p>Votre signature est enregistrée : <strong>${escapeHtml(inv.studentName)}</strong> est inscrit(e) au voyage <strong>« ${escapeHtml(title)} »</strong>.</p>
               <p>Vous trouverez en pièces jointes la circulaire signée (PDF) et le fichier agenda (.ics : séjour et rendez-vous de départ et de retour).</p>
               <p>Cordialement,<br/>${escapeHtml(mail.school)}</p>`,
        attachments,
      }),
      60_000,
    );
    await note({ confirmationMailSentAt: new Date().toISOString() });
  } catch (e) {
    await note({ confirmationMailError: e instanceof Error ? e.message.slice(0, 200) : "Envoi impossible" });
  }
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
  /** Au moins une pièce dispensée pour cette famille. */
  exempted: boolean;
  /** Réponse enregistrée à partir d'un coupon papier. */
  paper: boolean;
  /** Un AUTRE responsable de cet élève a déjà signé (oui ou non) : cette famille n'a plus rien à faire. */
  otherSigned?: boolean;
  /** Élève compté dans la liste officielle (hors suspens, retirés et refus). */
  official: boolean;
  signed: boolean;
  /** Raison du désaccord : direction seulement. */
  objectionReason?: string;
  objectionBy?: string;
};

export function buildRegistrationRows(opts: {
  trip: Pick<TravelsTrip, "data">;
  invitations: FamilyInvitation[];
  includeAmount: boolean;
  cases?: StudentCase[];
  includeReason?: boolean;
}): RegistrationRow[] {
  const docs = opts.trip.data?.familyPage?.documents || [];
  const keys = picnicKeys(opts.trip);
  const caseByIne = new Map((opts.cases || []).map((c) => [caseId(c.ine), c]));
  const byIne = new Map<string, FamilyInvitation[]>();
  for (const i of opts.invitations) byIne.set(i.ine, [...(byIne.get(i.ine) || []), i]);
  const rows: RegistrationRow[] = opts.invitations.map((inv) => {
    const own = computeInvitationStatus({ inv, docs, picnicDayKeys: keys });
    const state = studentState(byIne.get(inv.ine) || [], caseByIne.get(caseId(inv.ine)));
    let status: InvitationStatus = own;
    if (state === "retire") status = "retire";
    else if (state === "desiste") status = "desiste";
    else if (state === "suspens") status = inv.objection ? "desaccord" : "suspens";
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
      exempted: Object.keys(inv.exemptions || {}).length > 0,
      paper: Boolean(inv.signature?.paper),
      otherSigned: !inv.signature && (byIne.get(inv.ine) || []).some((o) => o.id !== inv.id && Boolean(o.signature)),
      official: state === "inscrit" && inv.signature?.decision === "oui",
      signed: Boolean(inv.signature),
      ...(opts.includeReason && inv.objection
        ? { objectionReason: inv.objection.reason, objectionBy: inv.objection.signerName }
        : {}),
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
      exempted: false,
      paper: false,
      official: false,
      signed: false,
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
    return getSignedReadUrl(inv.signature.pdfKey, 120, { downloadFileName: inv.signature.paper ? inv.signature.paper.scanFileName : `engagement-${inv.ine}.pdf` });
  }
  const d = inv.documents?.[what.docId];
  if (!d?.key) return null;
  return getSignedReadUrl(d.key, 120, { downloadFileName: d.fileName });
}


/* ─── Lot 3 : dossier élève (désaccord / décision) ─── */

function casePath(tripId: string, ine: string): string {
  return `travels-v2/${tripId}/cases/${caseId(ine)}.json`;
}

export async function getStudentCase(tripId: string, ine: string): Promise<StudentCase | null> {
  if (!TRIP_ID_RE.test(tripId) || !caseId(ine)) return null;
  const hit = await getJson<StudentCase>(casePath(tripId, ine));
  const c = hit?.data;
  return c && c.tripId === tripId ? c : null;
}

export async function saveStudentCase(rec: StudentCase): Promise<void> {
  await putJson(casePath(rec.tripId, rec.ine), rec);
}

export async function deleteStudentCase(tripId: string, ine: string): Promise<void> {
  if (!TRIP_ID_RE.test(tripId) || !caseId(ine)) return;
  await deleteJson(casePath(tripId, ine));
}

export async function deleteInvitation(tripId: string, invId: string): Promise<void> {
  if (!TRIP_ID_RE.test(tripId) || !TRIP_ID_RE.test(invId)) return;
  await deleteJson(invPath(tripId, invId));
}

export async function listStudentCases(tripId: string): Promise<StudentCase[]> {
  if (!TRIP_ID_RE.test(tripId)) return [];
  const rows = await listJsonRecordsInDir<Record<string, unknown>>(`travels-v2/${tripId}/cases`);
  return rows.filter((r) => r && r.tripId === tripId).map((r) => r as unknown as StudentCase);
}

/** État de l'élève de cette invitation (toutes les invitations de l'élève + décision de la direction). */
export async function studentStateFor(tripId: string, inv: Pick<FamilyInvitation, "ine">): Promise<StudentState> {
  const [all, c] = await Promise.all([listInvitations(tripId), getStudentCase(tripId, inv.ine)]);
  return studentState(all.filter((i) => i.ine === inv.ine), c);
}

/* ─── Lot 3 : unicité de l'engagement ─── */

/**
 * Un seul engagement « oui » par élève, même si les deux responsables signent au même instant.
 * Jeton `travels-v2/{voyage}/engagements/{INE}.json` créé par INSERT ... ON CONFLICT DO NOTHING (atomique en base) :
 * un seul appelant le crée. Un jeton resté d'une signature interrompue (le détenteur n'a pas signé « oui ») est repris.
 */
export async function claimEngagement(
  tripId: string,
  inv: Pick<FamilyInvitation, "id" | "ine">,
): Promise<{ ok: true; created: boolean } | { ok: false }> {
  const path = `travels-v2/${tripId}/engagements/${caseId(inv.ine)}.json`;
  const rec: EngagementClaim = {
    id: caseId(inv.ine),
    tripId,
    ine: inv.ine,
    invitationId: inv.id,
    claimedAt: new Date().toISOString(),
  };
  for (let attempt = 0; attempt < 2; attempt++) {
    if (await putJsonIfAbsentToPostgres(path, rec as unknown as Record<string, unknown>)) {
      return { ok: true, created: true };
    }
    const hit = await getJson<EngagementClaim>(path);
    const holder = hit?.data ? await getInvitation(tripId, hit.data.invitationId) : null;
    const outcome = evaluateClaim(hit?.data, inv.id, holder?.signature?.decision === "oui");
    if (outcome === "mine") return { ok: true, created: false };
    if (outcome === "taken") return { ok: false };
    if (outcome === "stale") await deleteJson(path).catch(() => undefined);
  }
  return { ok: false };
}

export async function releaseEngagementClaim(tripId: string, inv: Pick<FamilyInvitation, "ine">): Promise<void> {
  await deleteJson(`travels-v2/${tripId}/engagements/${caseId(inv.ine)}.json`).catch(() => undefined);
}

/* ─── Lot 3 : lien neuf et copie au second responsable ─── */

/**
 * Après un « oui » : les AUTRES responsables de l'élève reçoivent une copie avec le lien « Je ne suis pas d'accord »
 * (leur propre jeton, émis dans les emplacements AUXILIAIRES : il n'évince jamais leur lien principal). Un échec d'envoi
 * n'annule rien. Jamais l'e-mail du signataire dans le message.
 */
export async function sendSiblingCopies(opts: {
  trip: Pick<TravelsTrip, "id" | "data" | "ownerEmail">;
  signed: FamilyInvitation;
  baseUrl: string;
}): Promise<number> {
  const { trip, signed } = opts;
  const mail = await mailContext();
  if (!mail) return 0;
  const siblings = (await listInvitationsForIne(trip.id, signed.ine)).filter((i) => i.id !== signed.id);
  const title = String(trip.data?.title || trip.data?.destination || "Voyage scolaire");
  const kid = firstName(signed.studentName);
  let sent = 0;
  for (const s of siblings) {
    if (s.objection) continue;
    const issued = await issueInviteLink(s, opts.baseUrl, { aux: true });
    if (!issued) continue;
    const { link, hash } = issued;
    try {
      await sendMailWithTimeout(
        mail.transporter,
        familyMail({
          from: `"Sorties scolaires" <${mail.smtp.user}>`,
          to: s.email,
          subject: `Voyage « ${title} » — ${kid} a été inscrit(e)`,
          text: `Bonjour,\n\nUn responsable de ${kid} a inscrit votre enfant au voyage scolaire « ${title} ».\nSi vous n'êtes pas d'accord, vous pouvez le signaler à la direction depuis cette page (un code vous sera envoyé par e-mail pour confirmer) : ${link}\nSans action de votre part, l'inscription est maintenue.\n\n${mail.school}`,
          html: `<div style="font-family: Arial, sans-serif; max-width: 560px; color: #1e293b;">
            <p>Bonjour,</p>
            <p>Un responsable de <strong>${escapeHtml(kid)}</strong> a inscrit votre enfant au voyage scolaire <strong>« ${escapeHtml(title)} »</strong>.</p>
            <p>Sans action de votre part, l'inscription est maintenue. Si vous n'êtes pas d'accord, vous pouvez le signaler à la direction :</p>
            <p style="margin:24px 0;"><a href="${escapeHtml(link)}" style="background:#b91c1c;color:#fff;padding:12px 20px;border-radius:10px;text-decoration:none;font-weight:bold;">Je ne suis pas d'accord</a></p>
            <p style="font-size:13px;color:#64748b;">Ce lien est personnel. Un code à 6 chiffres sera envoyé à cette adresse pour confirmer votre désaccord.</p>
            <p>Cordialement,<br/>${escapeHtml(mail.school)}</p></div>`,
        }),
        60_000,
      );
      sent += 1;
    } catch (e) {
      console.error("[voyages-v2] copie second responsable", e instanceof Error ? e.message : e);
      await discardInviteLink(s, hash);
    }
  }
  return sent;
}
