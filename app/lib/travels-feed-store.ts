import "server-only";

import { randomUUID } from "node:crypto";
import { GetObjectCommand } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
import type { SessionLikeUser } from "@/app/lib/app-actor-types";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { withNamedLock } from "@/app/lib/ent-json-postgres";
import { s3Key } from "@/app/lib/s3-path";
import { deleteObject, getBucketName, getS3Client, putObject } from "@/app/lib/s3-storage";
import { sendMailWithTimeout } from "@/app/lib/tenant-mail";
import { canSignTravelsDirectionForEtab } from "@/app/lib/establishments";
import {
  addFeedTokenHash,
  bumpPendingForTrip,
  claimFeedNotification,
  deleteFeedData,
  deleteFeedPost,
  getFeedAccess,
  getFeedPost,
  getFeedSettings,
  insertFeedAudit,
  insertFeedPhotos,
  insertFeedPost,
  listFeedAccess,
  listFeedAudit,
  listFeedPhotoKeys,
  listFeedPosts,
  listFeedPostsBetween,
  listFeedSettings,
  markFeedLinkSent,
  openFeedSettings,
  releaseTripEmail,
  removeFeedTokenHash,
  reserveTripEmail,
  restoreFeedNotification,
  setFeedAllowDownload,
  setFeedPostHidden,
  setFeedReaction,
  setFeedReturnDate,
  upsertFeedAccess,
  type FeedAccessRow,
  type FeedSettingsRow,
  type FeedPostWithMedia,
} from "@/app/lib/travels-feed-db";
import { storePhotosPost } from "@/app/lib/travels-feed-publish";
import { sanitizeFeedImage, FeedImageError } from "@/app/lib/travels-feed-image";
import {
  canOpenFeedOnStatus,
  checkPhotoPublication,
  familyKeyOf,
  FEED_EMAIL_CAP_PER_TRIP,
  buildFeedUpdateEmail,
  type FeedEmailItem,
  feedParticipants,
  feedParticipantsForImageRights,
  feedPurgeDecision,
  feedPhase,
  feedPostText,
  feedReturnIso,
  isReactionEmoji,
  pendingDelta,
  planFeedEmail,
  resolveFeedStaffRights,
  SIGNED_PHOTO_TTL_SECONDS,
  stillParticipantInes,
  studentsWithoutImageRight,
  summarizeReactions,
  validateFeedPost,
  type FeedKind,
  type FeedPhase,
  type FeedStaffRights,
  type ImageRightListed,
} from "@/app/lib/travels-feed-logic";
import { evaluateFamilyAccess } from "@/app/lib/travels-feed-access";
import { getTravelFromDb } from "@/app/lib/travel-db";
import { getTravelTrip } from "@/app/lib/travels-storage";
import type { TravelsParticipantEleve, TravelsTrip } from "@/app/lib/travels-types";
import { todayIsoParis } from "@/app/lib/travels-v2";
import {
  buildInviteToken,
  generateInviteSecret,
  hashInviteSecret,
  parseInviteToken,
} from "@/app/lib/travels-v2-registration";
import { resolveTripRecipients, keyOfIne } from "@/app/lib/travels-v2-recipients-store";
import { escapeHtml, familyMail, mailContext } from "@/app/lib/travels-v2-registration-store";

/**
 * « Fil du voyage » — orchestration (droits, publication, vue des familles, e-mails regroupés, purge).
 * Jamais public : les familles passent par un lien personnel (jeton haché par adresse) ; les profs par leur compte.
 */

const PHOTO_PREFIX = "travels-feed";

/** Une seule ligne (jamais de CR/LF dans un objet d'e-mail). */
function singleLine(v: string): string {
  return v.replace(/[\r\n]+/g, " ").trim().slice(0, 90);
}

function tripTitle(trip: Pick<TravelsTrip, "data">): string {
  return String(trip.data?.title || trip.data?.destination || "Voyage scolaire");
}

/* ─── URL pré-signées courtes (sans HEAD, jamais stockées) ─── */

export async function signFeedPhotoUrl(storageKey: string, opts?: { download?: boolean }): Promise<string | null> {
  try {
    const client = await getS3Client();
    const bucket = await getBucketName();
    return await getSignedUrl(client, new GetObjectCommand({ Bucket: bucket, Key: s3Key(storageKey), ResponseContentDisposition: opts?.download ? `attachment; filename="photo-voyage.jpg"` : "inline" }), {
      expiresIn: SIGNED_PHOTO_TTL_SECONDS,
    });
  } catch {
    return null;
  }
}

/* ─── Liens personnels ─── */

/** Nouveau lien personnel : seule l'empreinte est stockée (avant l'e-mail). Retourne le jeton, jamais stocké en clair. */
async function issueFeedToken(etabId: string, tripId: string, accessId: string): Promise<{ token: string; hash: string } | null> {
  const secret = generateInviteSecret();
  const hash = hashInviteSecret(secret);
  if (!(await addFeedTokenHash(etabId, tripId, accessId, hash))) return null;
  return { token: buildInviteToken({ invitationId: accessId, tripId, secret }), hash };
}

export function feedLinkFor(baseUrl: string, token: string): string {
  return `${baseUrl}/voyages/famille/fil/${encodeURIComponent(token)}`;
}

/* ─── Contexte famille ─── */

export type FeedFamilyContext = {
  etabId: string;
  trip: TravelsTrip;
  access: FeedAccessRow;
  settings: FeedSettingsRow;
  /** INE de la famille encore participants. */
  ines: string[];
  phase: FeedPhase;
  familyKey: string;
};

/**
 * Jeton → contexte. Retourne null pour TOUT cas refusé, sans distinction (jeton invalide, autre voyage, autre
 * établissement, élève désisté ou retiré, fil non ouvert, voyage annulé, purge échue) : réponse uniforme.
 */
export async function resolveFeedFamilyContext(token: string | null | undefined, now: Date = new Date()): Promise<FeedFamilyContext | null> {
  const parsed = parseInviteToken(token);
  if (!parsed) return null;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return null;
  const access = await getFeedAccess(etabId, parsed.tripId, parsed.invitationId);
  if (!access) return null;
  const [trip, settings] = await Promise.all([getTravelTrip(parsed.tripId), getFeedSettings(etabId, parsed.tripId)]);
  const decision = evaluateFamilyAccess({ token, access, trip, feedOpen: Boolean(settings), now });
  if (!decision.ok || !trip || !settings) return null;
  return { etabId, trip, access, settings, ines: decision.ines, phase: decision.phase, familyKey: decision.familyKey };
}

export type FamilyFeedView = {
  tripTitle: string;
  destination: string;
  readOnly: boolean;
  /** Dernier jour de consultation (suppression de tout le fil 7 jours après le retour). */
  deletedOn: string | null;
  allowDownload: boolean;
  reactionEmojis: string[];
  posts: {
    id: string;
    kind: string;
    text: string;
    createdAt: string;
    photos: { id: string; url: string; downloadUrl?: string; width: number; height: number }[];
    reactions: Record<string, number>;
    mine: string[];
  }[];
};

export async function buildFamilyFeedView(ctx: FeedFamilyContext, reactionEmojis: readonly string[]): Promise<FamilyFeedView> {
  const posts = await listFeedPosts(ctx.etabId, ctx.trip.id, { includeHidden: false });
  const shown = posts.slice(0, 80);
  const views = await Promise.all(shown.map((p) => toPostView(p, ctx.familyKey, ctx.settings.allowDownload)));
  const ret = feedReturnIso(ctx.trip);
  return {
    tripTitle: tripTitle(ctx.trip),
    destination: String(ctx.trip.data?.destination || ""),
    readOnly: ctx.phase === "readonly",
    deletedOn: ret ? addDays(ret, 7) : null,
    allowDownload: ctx.settings.allowDownload,
    reactionEmojis: [...reactionEmojis],
    posts: views,
  };
}

function addDays(iso: string, n: number): string {
  return new Date(new Date(`${iso}T00:00:00Z`).getTime() + n * 86_400_000).toISOString().slice(0, 10);
}

async function toPostView(p: FeedPostWithMedia, myFamilyKey: string, withDownload = false) {
  const photos = await Promise.all(
    p.photos.map(async (ph) => {
      const url = await signFeedPhotoUrl(ph.storageKey);
      if (!url) return null;
      // Lien de téléchargement SEULEMENT si l'option du voyage est activée : URL signée « attachment » (un attribut download ne marche pas entre origines).
      const downloadUrl = withDownload ? await signFeedPhotoUrl(ph.storageKey, { download: true }) : null;
      return { id: ph.id, url, ...(downloadUrl ? { downloadUrl } : {}), width: ph.width, height: ph.height };
    }),
  );
  const { counts, mine } = summarizeReactions(p.reactions, myFamilyKey);
  return {
    id: p.id,
    kind: p.kind,
    text: feedPostText({ kind: p.kind, body: p.body, delayMinutes: p.delayMinutes }),
    createdAt: p.createdAt.toISOString(),
    photos: photos.filter((x): x is NonNullable<typeof x> => Boolean(x)),
    reactions: counts,
    mine,
  };
}

/** Réaction d'une famille : une par type, retirable. Seulement tant que le fil est « live » (lecture seule après le retour). */
export async function reactAsFamily(
  ctx: FeedFamilyContext,
  input: { postId: unknown; emoji: unknown; on: unknown },
): Promise<{ ok: true } | { ok: false; status: number; error: string }> {
  if (ctx.phase !== "live") return { ok: false, status: 409, error: "Le fil est en lecture seule." };
  if (!isReactionEmoji(input.emoji)) return { ok: false, status: 400, error: "Réaction inconnue." };
  const postId = String(input.postId || "");
  if (!/^[0-9a-f-]{36}$/i.test(postId)) return { ok: false, status: 404, error: "Publication introuvable." };
  const post = await getFeedPost(ctx.etabId, ctx.trip.id, postId);
  if (!post || post.hiddenAt) return { ok: false, status: 404, error: "Publication introuvable." };
  await setFeedReaction(ctx.etabId, ctx.trip.id, postId, ctx.familyKey, input.emoji, input.on === true);
  return { ok: true };
}

/* ─── Contexte profs ─── */

export type FeedStaffContext = {
  etabId: string;
  trip: TravelsTrip;
  me: SessionLikeUser;
  rights: FeedStaffRights;
  settings: FeedSettingsRow | null;
  phase: FeedPhase;
};

export async function resolveFeedStaffContext(
  tripId: string,
  me: SessionLikeUser | null | undefined,
  now: Date = new Date(),
): Promise<FeedStaffContext | null> {
  if (!me) return null;
  const trip = await getTravelTrip(tripId); // déjà cloisonné par établissement
  if (!trip) return null;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return null;
  const isDirection = await canSignTravelsDirectionForEtab(me, trip.data?.etablissement).catch(() => false);
  const rights = resolveFeedStaffRights(trip, me, isDirection);
  if (!rights.canPublish && !rights.canModerate) return null;
  const settings = await getFeedSettings(etabId, trip.id);
  const phase = feedPhase({ status: trip.status, returnIso: feedReturnIso(trip), todayIso: todayIsoParis(now) });
  return { etabId, trip, me, rights, settings, phase };
}

function actorName(me: SessionLikeUser): string | null {
  const n = String(me.fullName ?? "").trim();
  return n ? n.slice(0, 120) : null;
}

export type StaffFeedView = {
  opened: boolean;
  phase: FeedPhase;
  canPublish: boolean;
  canModerate: boolean;
  tripTitle: string;
  allowDownload: boolean;
  /** Élèves dont le droit à l'image n'est PAS « oui » (refusé ou non renseigné). */
  imageRightList: ImageRightListed[];
  participantCount: number;
  families: { total: number; linkSent: number; failures: number };
  emailsSent: number;
  emailCap: number;
  deletedOn: string | null;
  posts: (Omit<Awaited<ReturnType<typeof toPostView>>, "mine"> & { hidden: boolean; authorName: string | null })[];
  audit: { action: string; actorName: string | null; at: string; detail: string | null }[];
};

export async function buildStaffFeedView(ctx: FeedStaffContext): Promise<StaffFeedView> {
  const participants = feedParticipants(ctx.trip);
  const ret = feedReturnIso(ctx.trip);
  const base = {
    phase: ctx.phase,
    canPublish: ctx.rights.canPublish && ctx.phase === "live",
    canModerate: ctx.rights.canModerate,
    tripTitle: tripTitle(ctx.trip),
    allowDownload: ctx.settings?.allowDownload ?? false,
    imageRightList: studentsWithoutImageRight(feedParticipantsForImageRights(ctx.trip)),
    participantCount: participants.length,
    emailCap: FEED_EMAIL_CAP_PER_TRIP,
    deletedOn: ret ? addDays(ret, 7) : null,
  };
  if (!ctx.settings) {
    return { ...base, opened: false, families: { total: 0, linkSent: 0, failures: 0 }, emailsSent: 0, posts: [], audit: [] };
  }
  const [posts, access, audit] = await Promise.all([
    listFeedPosts(ctx.etabId, ctx.trip.id, { includeHidden: true }),
    listFeedAccess(ctx.etabId, ctx.trip.id),
    ctx.rights.canModerate ? listFeedAudit(ctx.etabId, ctx.trip.id, 30) : Promise.resolve([]),
  ]);
  const views = await Promise.all(
    posts.slice(0, 80).map(async (p) => {
      const v = await toPostView(p, "");
      const { mine: _m, ...rest } = v; // eslint-disable-line @typescript-eslint/no-unused-vars
      return { ...rest, hidden: Boolean(p.hiddenAt), authorName: p.authorName };
    }),
  );
  return {
    ...base,
    opened: true,
    families: {
      total: access.length,
      linkSent: access.filter((a) => a.firstSentAt).length,
      failures: access.filter((a) => a.sendFailures > 0 && !a.firstSentAt).length,
    },
    emailsSent: ctx.settings.emailsSent,
    posts: views,
    audit: audit.map((a) => ({ action: a.action, actorName: a.actorName, at: a.createdAt.toISOString(), detail: a.detail })),
  };
}

/* ─── Ouverture du fil et envoi des liens personnels ─── */

const OPEN_LOCK = "travel-feed-open";
const WELCOME_LOCK = "travel-feed-welcome";
const NOTIFY_LOCK = "travel-feed-notify";
const LOCK_TTL = 15 * 60;

type MailCtx = NonNullable<Awaited<ReturnType<typeof mailContext>>>;

async function sendFeedLinkMail(opts: {
  etabId: string;
  trip: TravelsTrip;
  access: FeedAccessRow;
  baseUrl: string;
  mail: MailCtx;
  /** Absent = e-mail de bienvenue (ouverture du fil) ; présent = e-mail explicite de nouveautés (texte des publications, jamais les photos). */
  update?: { items: FeedEmailItem[] };
}): Promise<boolean | "empty"> {
  const title = tripTitle(opts.trip);
  const school = opts.mail.school;
  const issued = await issueFeedToken(opts.etabId, opts.trip.id, opts.access.id);
  if (!issued) return false;
  const link = feedLinkFor(opts.baseUrl, issued.token);
  let subject: string;
  let html: string;
  let text: string;
  if (opts.update) {
    const built = buildFeedUpdateEmail({
      destination: String(opts.trip.data?.destination || ""),
      tripTitle: title,
      school,
      link,
      items: opts.update.items,
    });
    if (!built) {
      await removeFeedTokenHash(opts.etabId, opts.trip.id, opts.access.id, issued.hash).catch(() => undefined);
      return "empty";
    }
    ({ subject, html, text } = built);
  } else {
    subject = `Le fil du voyage est ouvert — ${singleLine(title)}`;
    html = `
    <div style="font-family: Arial, sans-serif; max-width: 560px; color: #1e293b;">
      <p>Bonjour,</p>
      <p>Le fil du voyage <strong>« ${escapeHtml(title)} »</strong> est ouvert : vous y trouverez les nouvelles du groupe et quelques photos pendant le séjour.</p>
      <p style="margin: 24px 0;">
        <a href="${escapeHtml(link)}" style="background:#4338ca;color:#fff;padding:12px 20px;border-radius:10px;text-decoration:none;font-weight:bold;">Ouvrir le fil du voyage</a>
      </p>
      <p style="font-size: 13px; color: #64748b;">Ce lien est personnel et privé : ne le transmettez pas. Le fil est réservé aux familles des élèves participants et sera supprimé 7 jours après le retour.</p>
      <p>Cordialement,<br/>${escapeHtml(school)}</p>
    </div>`;
    text = `Bonjour,

Le fil du voyage « ${title} » est ouvert : vous y trouverez les nouvelles du groupe et quelques photos pendant le séjour.
Ouvrir le fil : ${link}

Ce lien est personnel et privé : ne le transmettez pas. Le fil est supprimé 7 jours après le retour.

${school}`;
  }
  try {
    await sendMailWithTimeout(
      opts.mail.transporter,
      familyMail({ from: `"Sorties scolaires" <${opts.mail.smtp.user}>`, to: opts.access.email, subject, text, html }),
      60_000,
    );
    return true;
  } catch {
    await removeFeedTokenHash(opts.etabId, opts.trip.id, opts.access.id, issued.hash).catch(() => undefined);
    return false;
  }
}

export type OpenFeedResult =
  | { ok: true; families: number; withoutAddress: number; toSend: number; busy?: boolean }
  | { ok: false; status: number; error: string };

/**
 * Ouvre le fil (ou prépare le renvoi des liens aux familles qui n'en ont pas encore reçu — idempotent, rejouable).
 * Cette étape est RAPIDE (base seulement) : l'envoi des e-mails de bienvenue est fait ensuite, hors de la requête HTTP,
 * par `sendFeedWelcomeLinks`.
 * Une adresse = un accès personnel, quel que soit le voyage (v1 ou v2) : indépendant des invitations d'inscription,
 * qui sont purgées dès le lendemain du retour alors que le fil vit jusqu'à J+7.
 */
export async function openFeedAndSendLinks(ctx: FeedStaffContext): Promise<OpenFeedResult> {
  if (!ctx.rights.canModerate) return { ok: false, status: 403, error: "Seuls l'organisateur et la direction ouvrent le fil." };
  if (!canOpenFeedOnStatus(ctx.trip.status)) return { ok: false, status: 409, error: "Le fil s'ouvre une fois le voyage finalisé." };
  if (ctx.phase !== "live") return { ok: false, status: 409, error: "Le fil n'est plus ouvrable (voyage terminé ou sans date de retour)." };
  const participants = feedParticipants(ctx.trip);
  if (participants.length === 0) return { ok: false, status: 409, error: "Aucun élève participant dans la liste officielle." };

  const locked = await withNamedLock(OPEN_LOCK, ctx.trip.id, LOCK_TTL, async () => {
    const wasOpen = Boolean(ctx.settings);
    await openFeedSettings(ctx.etabId, ctx.trip.id, ctx.me.id ?? null, feedReturnIso(ctx.trip));
    const asPreList = { id: ctx.trip.id, data: { ...ctx.trip.data, preListeEleves: participants as TravelsParticipantEleve[] } } as unknown as Pick<TravelsTrip, "id" | "data">;
    const { byIne, withoutAddress } = await resolveTripRecipients(asPreList);
    const byEmail = new Map<string, string[]>();
    for (const p of participants) {
      for (const r of byIne.get(keyOfIne(p.ine)) || []) {
        byEmail.set(r.email, [...(byEmail.get(r.email) || []), p.ine]);
      }
    }
    for (const [email, ines] of byEmail) {
      await upsertFeedAccess(ctx.etabId, ctx.trip.id, { email, ines, familyKey: familyKeyOf(ines) });
    }
    await insertFeedAudit(ctx.etabId, { tripId: ctx.trip.id, actorUserId: ctx.me.id ?? null, actorName: actorName(ctx.me), action: wasOpen ? "feed_links_resent" : "feed_opened" });
    const toSend = (await listFeedAccess(ctx.etabId, ctx.trip.id)).filter((a) => !a.firstSentAt && a.sendFailures < 3).length;
    return { families: byEmail.size, withoutAddress: withoutAddress.length, toSend };
  });
  if (locked.busy) return { ok: true, families: 0, withoutAddress: 0, toSend: 0, busy: true };
  return { ok: true, ...locked.value };
}

/** Envoi des e-mails de bienvenue (lien personnel) aux familles qui n'en ont pas reçu. À appeler HORS de la requête (after/job). */
export async function sendFeedWelcomeLinks(etabId: string, trip: TravelsTrip, baseUrl: string): Promise<{ sent: number; failed: number; smtp: boolean; busy?: boolean }> {
  const locked = await withNamedLock(WELCOME_LOCK, trip.id, LOCK_TTL, async () => {
    const mail = await mailContext();
    if (!mail) return { sent: 0, failed: 0, smtp: false };
    let sent = 0;
    let failed = 0;
    for (const a of await listFeedAccess(etabId, trip.id)) {
      if (a.firstSentAt) continue;
      if (a.sendFailures >= 3) continue; // pas d'insistance sur une adresse en échec répété
      if (await sendFeedLinkMail({ etabId, trip, access: a, baseUrl, mail })) {
        await markFeedLinkSent(etabId, trip.id, a.id);
        sent += 1;
      } else failed += 1;
    }
    return { sent, failed, smtp: true };
  });
  if (locked.busy) return { sent: 0, failed: 0, smtp: true, busy: true };
  console.info(`[voyages-fil] liens de bienvenue du voyage ${trip.id}: ${locked.value.sent} envoyé(s), ${locked.value.failed} échec(s)${locked.value.smtp ? "" : ", SMTP absent"}`);
  return locked.value;
}

/* ─── Publication ─── */

type PublishResult = { ok: true; postId: string } | { ok: false; status: number; error: string };

function publishGate(ctx: FeedStaffContext): PublishResult | null {
  if (!ctx.rights.canPublish) return { ok: false, status: 403, error: "Accès refusé." };
  if (!ctx.settings) return { ok: false, status: 409, error: "Le fil n'est pas encore ouvert aux familles." };
  if (ctx.phase !== "live") return { ok: false, status: 409, error: "Le fil est en lecture seule (voyage terminé)." };
  return null;
}

export async function publishFeedText(ctx: FeedStaffContext, raw: { kind?: unknown; body?: unknown; delayMinutes?: unknown }): Promise<PublishResult> {
  const gate = publishGate(ctx);
  if (gate) return gate;
  const v = validateFeedPost(raw);
  if (!v.ok) return { ok: false, status: 400, error: v.error };
  const post = await insertFeedPost(ctx.etabId, {
    tripId: ctx.trip.id,
    kind: v.kind,
    body: v.body,
    delayMinutes: v.delayMinutes,
    authorUserId: ctx.me.id ?? null,
    authorName: actorName(ctx.me),
  });
  await bumpPendingForTrip(ctx.etabId, ctx.trip.id, pendingDelta(v.kind));
  return { ok: true, postId: post.id };
}

export async function publishFeedPhotos(
  ctx: FeedStaffContext,
  input: { files: Buffer[]; acknowledged: boolean; caption?: unknown },
): Promise<PublishResult> {
  const gate = publishGate(ctx);
  if (gate) return gate;
  const listed = studentsWithoutImageRight(feedParticipantsForImageRights(ctx.trip));
  const check = checkPhotoPublication({ listed, acknowledged: input.acknowledged, photoCount: input.files.length });
  if (!check.ok) return { ok: false, status: 400, error: check.error };

  // Nettoyage AVANT tout stockage : si une seule photo est refusée, rien n'est publié.
  const cleaned = [];
  for (const f of input.files) {
    try {
      cleaned.push(await sanitizeFeedImage(f));
    } catch (e) {
      return { ok: false, status: 400, error: e instanceof FeedImageError ? e.message : "Photo illisible." };
    }
  }
  const caption = String(input.caption ?? "").replace(/\s+/g, " ").trim().slice(0, 500) || null;
  const folder = randomUUID();
  try {
    const { postId, stored } = await storePhotosPost(
      {
        putObject: (path, bytes) => putObject(path, bytes, "image/jpeg"),
        insertPost: () =>
          insertFeedPost(ctx.etabId, {
            tripId: ctx.trip.id,
            kind: "photos",
            body: caption,
            delayMinutes: null,
            authorUserId: ctx.me.id ?? null,
            authorName: actorName(ctx.me),
          }),
        insertPhotos: (postId, photos) => insertFeedPhotos(ctx.etabId, ctx.trip.id, postId, photos),
        deletePost: (postId) => deleteFeedPost(ctx.etabId, ctx.trip.id, postId),
        deleteObject,
        newId: randomUUID,
      },
      { pathPrefix: `${PHOTO_PREFIX}/${ctx.trip.id}/${folder}`, cleaned },
    );
    // La publication est en ligne : un échec de journal ou de compteur ne doit pas la défaire.
    await insertFeedAudit(ctx.etabId, {
      tripId: ctx.trip.id,
      actorUserId: ctx.me.id ?? null,
      actorName: actorName(ctx.me),
      action: "photos_published",
      postId,
      detail: `${stored.length} photo(s) ; case « non reconnaissables » ${listed.length > 0 ? "cochée" : "non requise"} (${listed.length} élève(s) listé(s))`,
    }).catch(() => undefined);
    await bumpPendingForTrip(ctx.etabId, ctx.trip.id, pendingDelta("photos")).catch(() => undefined);
    return { ok: true, postId };
  } catch (e) {
    console.error("[travels-feed] publication photos", e instanceof Error ? e.message : "erreur");
    return { ok: false, status: 500, error: "Publication impossible." };
  }
}

/* ─── Modération et réglages ─── */

export async function moderateFeedPost(ctx: FeedStaffContext, postId: string, action: "hide" | "unhide" | "delete"): Promise<{ ok: true } | { ok: false; status: number; error: string }> {
  if (!ctx.rights.canModerate) return { ok: false, status: 403, error: "Accès refusé." };
  if (!/^[0-9a-f-]{36}$/i.test(postId)) return { ok: false, status: 404, error: "Publication introuvable." };
  const post = await getFeedPost(ctx.etabId, ctx.trip.id, postId);
  if (!post) return { ok: false, status: 404, error: "Publication introuvable." };
  const meta = { tripId: ctx.trip.id, actorUserId: ctx.me.id ?? null, actorName: actorName(ctx.me), postId, detail: `type ${post.kind}` };
  if (action === "delete") {
    // Retrait définitif : photos supprimées du stockage, lignes (photos, réactions) en cascade ; le journal garde la trace sans contenu.
    const keys = (await listFeedPosts(ctx.etabId, ctx.trip.id, { includeHidden: true })).find((p) => p.id === postId)?.photos.map((p) => p.storageKey) ?? [];
    await insertFeedAudit(ctx.etabId, { ...meta, action: "post_deleted" });
    await deleteFeedPost(ctx.etabId, ctx.trip.id, postId);
    await Promise.all(keys.map((k) => deleteObject(k).catch(() => undefined)));
    return { ok: true };
  }
  const changed = await setFeedPostHidden(ctx.etabId, ctx.trip.id, postId, ctx.me.id ?? null, action === "hide");
  if (!changed) return { ok: false, status: 404, error: "Publication introuvable." };
  await insertFeedAudit(ctx.etabId, { ...meta, action: action === "hide" ? "post_hidden" : "post_unhidden" });
  return { ok: true };
}

export async function setDownloadAllowed(ctx: FeedStaffContext, allow: boolean): Promise<{ ok: true } | { ok: false; status: number; error: string }> {
  if (!ctx.rights.canModerate) return { ok: false, status: 403, error: "Accès refusé." };
  if (!ctx.settings) return { ok: false, status: 409, error: "Le fil n'est pas ouvert." };
  await setFeedAllowDownload(ctx.etabId, ctx.trip.id, allow);
  await insertFeedAudit(ctx.etabId, { tripId: ctx.trip.id, actorUserId: ctx.me.id ?? null, actorName: actorName(ctx.me), action: allow ? "download_enabled" : "download_disabled" });
  return { ok: true };
}

/* ─── E-mails regroupés ─── */

/**
 * Envoie, pour UN voyage, l'e-mail « Nouveau message / Nouvelles photos / N nouveautés » aux familles dont le dernier
 * e-mail date de plus d'une heure. Les autres gardent leurs nouveautés en attente (reprise au passage suivant du job).
 * Aucun texte, aucune photo, aucun nom d'élève : seulement le lien personnel.
 */
export async function flushFeedNotificationsForTrip(opts: { etabId: string; trip: TravelsTrip; baseUrl: string; now?: Date }): Promise<{ sent: number; deferred: number; failed: number }> {
  const now = opts.now ?? new Date();
  const out = { sent: 0, deferred: 0, failed: 0 };
  const phase = feedPhase({ status: opts.trip.status, returnIso: feedReturnIso(opts.trip), todayIso: todayIsoParis(now) });
  if (phase !== "live") return out;
  const settings = await getFeedSettings(opts.etabId, opts.trip.id);
  if (!settings) return out;
  const locked = await withNamedLock(NOTIFY_LOCK, opts.trip.id, LOCK_TTL, async () => {
    const access = await listFeedAccess(opts.etabId, opts.trip.id);
    const due = access.filter((a) => a.pendingMessages + a.pendingPhotos > 0);
    if (due.length === 0) return;
    const mail = await mailContext();
    if (!mail) return;
    const participants = feedParticipants(opts.trip);
    for (const a of due) {
      if (stillParticipantInes(a.ines, participants).length === 0) continue; // élève retiré : plus d'e-mail
      const decision = planFeedEmail({
        state: { lastNotifiedAt: a.lastNotifiedAt, pendingMessages: a.pendingMessages, pendingPhotos: a.pendingPhotos, notifyCount: a.notifyCount },
        now,
        tripEmailsSent: settings.emailsSent + out.sent,
      });
      if (!decision.send) {
        if (decision.reason === "too_soon") out.deferred += 1;
        continue;
      }
      if (!(await reserveTripEmail(opts.etabId, opts.trip.id, FEED_EMAIL_CAP_PER_TRIP))) continue;
      const claim = await claimFeedNotification(opts.etabId, opts.trip.id, a.id);
      if (!claim) {
        await releaseTripEmail(opts.etabId, opts.trip.id);
        out.deferred += 1;
        continue;
      }
      // Contenu de l'e-mail : publications visibles (non masquées) depuis le dernier e-mail, texte seulement.
      const since = claim.previousNotifiedAt ?? settings.openedAt;
      const fresh = (await listFeedPostsBetween(opts.etabId, opts.trip.id, since, claim.claimedAt)).map((p) => ({
        kind: p.kind,
        body: p.body,
        delayMinutes: p.delayMinutes,
        authorName: p.authorName,
        createdAt: p.createdAt,
        photoCount: p.photos.length,
      }));
      const ok = await sendFeedLinkMail({ etabId: opts.etabId, trip: opts.trip, access: a, baseUrl: opts.baseUrl, mail, update: { items: fresh } });
      if (ok === "empty") {
        // Tout a été masqué ou retiré entre-temps : rien à dire, pas d'e-mail ; le compteur du voyage est rendu.
        await releaseTripEmail(opts.etabId, opts.trip.id);
        continue;
      }
      if (ok) {
        out.sent += 1;
      } else {
        out.failed += 1;
        await restoreFeedNotification(opts.etabId, opts.trip.id, a.id, claim);
        await releaseTripEmail(opts.etabId, opts.trip.id);
      }
    }
  });
  void locked;
  return out;
}

/* ─── Purge ─── */

/** Supprime tout le fil d'un voyage : objets photo, textes, réactions, accès (jetons), réglages. Rejouable. */
export async function purgeFeedForTrip(etabId: string, tripId: string, reason: string): Promise<{ photos: number }> {
  const keys = await listFeedPhotoKeys(etabId, tripId);
  // Objets d'abord : si une suppression échoue, les lignes restent et la purge sera rejouée.
  for (const k of keys) await deleteObject(k);
  await deleteFeedData(etabId, tripId);
  await insertFeedAudit(etabId, { tripId, actorUserId: null, actorName: "système", action: "feed_purged", detail: `${reason} ; ${keys.length} photo(s) supprimée(s)` });
  return { photos: keys.length };
}

/**
 * Passage du job cyclique pour l'établissement courant : purge des fils échus (retour + 7 jours, ou voyage annulé),
 * puis envoi des e-mails regroupés des fils ouverts. Indépendant des invitations d'inscription (v1 et v2).
 */
export async function runFeedCycleForTenant(baseUrl: string, opts?: { inWindow?: boolean; now?: Date }): Promise<{ purged: number; emailed: number; failed: number }> {
  const now = opts?.now ?? new Date();
  const etabId = await resolveCurrentEtablissementId();
  const total = { purged: 0, emailed: 0, failed: 0 };
  if (!etabId) return total;
  const rows = await listFeedSettings(etabId);
  for (const s of rows) {
    try {
      // Jamais de purge parce qu'une lecture renvoie « rien » (base indisponible, cache) : absence CONFIRMÉE par lecture directe,
      // sinon date de retour mémorisée dans les réglages du fil.
      let trip = await getTravelTrip(s.tripId);
      let tripState: "found" | "absent" | "unknown" = trip ? "found" : "unknown";
      if (!trip) {
        try {
          trip = await getTravelFromDb(etabId, s.tripId);
          tripState = trip ? "found" : "absent";
        } catch {
          tripState = "unknown";
        }
      }
      const returnIso = (trip ? feedReturnIso(trip) : null) ?? s.returnDate;
      if (feedPurgeDecision({ tripState, status: trip?.status, returnIso, openedAt: s.openedAt, todayIso: todayIsoParis(now) })) {
        await purgeFeedForTrip(etabId, s.tripId, tripState === "absent" ? "voyage supprimé" : returnIso ? "retour + 7 jours" : "30 jours sans date de retour");
        total.purged += 1;
        continue;
      }
      if (!trip) continue; // voyage momentanément illisible : ni purge ni e-mail, nouvel essai au prochain passage
      if (feedReturnIso(trip) && s.returnDate !== feedReturnIso(trip)) await setFeedReturnDate(etabId, s.tripId, feedReturnIso(trip));
      const r = await flushFeedNotificationsForTrip({ etabId, trip, baseUrl, now });
      total.emailed += r.sent;
      total.failed += r.failed;
    } catch (e) {
      console.error("[voyages-fil] voyage ignoré", e instanceof Error ? e.message : "erreur");
    }
  }
  return total;
}

/** Envoi immédiat (best effort) après une publication : les familles servies il y a moins d'une heure attendent le job. */
export async function notifyAfterPublish(etabId: string, tripId: string, baseUrl: string): Promise<void> {
  const trip = await getTravelTrip(tripId);
  if (!trip) return;
  await flushFeedNotificationsForTrip({ etabId, trip, baseUrl });
}
