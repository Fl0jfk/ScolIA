import "server-only";

import { randomBytes } from "crypto";
import { and, asc, desc, eq, lt, isNull } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  travel,
  travelParentBlog,
  travelParentBlogPhoto,
  travelParentBlogPost,
} from "@/db/schema";
import { canSignTravelsDirectionForEtab } from "@/app/lib/establishments";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { deleteObject, putObject, getSignedReadUrl } from "@/app/lib/s3-storage";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";
import {
  buildParentsCalendarMailCopy,
  buildTravelsParentsTripIcs,
  defaultParentCalendarFromTrip,
} from "@/app/lib/travels-parent-calendar";
import { isTripOwnerOrCreator } from "@/app/lib/travels-direction-permissions";
import type {
  TravelsParentBlogDelegate,
  TravelsParentBlogMeta,
  TravelsParentCalendar,
  TravelsParentComLog,
  TravelsTrip,
} from "@/app/lib/travels-types";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
  sendMailWithTimeout,
} from "@/app/lib/tenant-mail";
import { collectEleveParentEmails } from "@/app/lib/eleves-parent-emails";
import { loadElevesRegistry } from "@/app/lib/eleves-registry";
import { getJson, putJson } from "@/app/lib/s3-storage";

export const PARENT_BLOG_RETENTION_DAYS = 15;
export const PARENT_BLOG_MAX_PHOTOS = 12;
export const PARENT_BLOG_MAX_PHOTO_BYTES = 900_000;
const PARENT_BATCH = 40;

export function parentBlogPublicPath(token: string): string {
  return `/voyages/suivi/${encodeURIComponent(token)}`;
}

/** Dernier jour du séjour en YYYY-MM-DD. */
export function parentBlogEndDateYmd(trip: Pick<TravelsTrip, "type" | "data">): string {
  const d = trip.data;
  if (!d) return "";
  const raw =
    trip.type === "COMPLEX"
      ? d.endDate || d.startDate || d.date
      : d.date || d.startDate || d.endDate;
  return String(raw || "").slice(0, 10);
}

export function computeParentBlogExpiresAt(trip: Pick<TravelsTrip, "type" | "data">): Date {
  const endYmd = parentBlogEndDateYmd(trip);
  if (!/^\d{4}-\d{2}-\d{2}$/.test(endYmd)) {
    const fallback = new Date();
    fallback.setDate(fallback.getDate() + PARENT_BLOG_RETENTION_DAYS);
    return fallback;
  }
  // Fin du séjour + 15 jours (fin de journée Europe/Paris, approx. +02:00).
  const expires = new Date(`${endYmd}T23:59:59+02:00`);
  expires.setUTCDate(expires.getUTCDate() + PARENT_BLOG_RETENTION_DAYS);
  return expires;
}

export function isParentBlogPublicExpired(expiresAt: Date | string, now = new Date()): boolean {
  const exp = typeof expiresAt === "string" ? new Date(expiresAt) : expiresAt;
  return Number.isNaN(exp.getTime()) || now.getTime() > exp.getTime();
}

export function metaFromBlogRow(
  row: typeof travelParentBlog.$inferSelect,
): TravelsParentBlogMeta {
  return {
    enabled: row.enabled,
    token: row.token,
    activatedAt: row.activatedAt.toISOString(),
    expiresAt: row.expiresAt.toISOString(),
    publicPath: parentBlogPublicPath(row.token),
    parentsNotifiedAt: row.parentsNotifiedAt?.toISOString() ?? null,
  };
}

function newToken(): string {
  return randomBytes(24).toString("base64url");
}

function newId(prefix: string): string {
  return `${prefix}_${Date.now().toString(36)}_${randomBytes(4).toString("hex")}`;
}

export function isParentBlogDelegate(
  trip: TravelsTrip,
  userId: string | null | undefined,
): boolean {
  if (!userId) return false;
  const list = trip.data.parentBlogDelegates || [];
  return list.some((d) => d.userId === userId);
}

export async function assertParentBlogEditAccess(
  trip: TravelsTrip,
): Promise<
  | { ok: true; user: NonNullable<NonNullable<Awaited<ReturnType<typeof safeCurrentUser>>>> }
  | { ok: false; status: number; error: string }
> {
  const user = await safeCurrentUser();
  if (!user) return { ok: false, status: 401, error: "Non autorisé" };

  const isOwner = isTripOwnerOrCreator(trip, user);
  const canSign = await canSignTravelsDirectionForEtab(user, trip.data?.etablissement);
  const isDelegate = isParentBlogDelegate(trip, user.id);

  if (!isOwner && !canSign && !isDelegate) {
    return {
      ok: false,
      status: 403,
      error: "Réservé à l'organisateur, la direction ou un délégué du blog.",
    };
  }
  return { ok: true, user };
}

export async function getParentBlogForTrip(
  etablissementId: string,
  travelId: string,
): Promise<(typeof travelParentBlog.$inferSelect) | null> {
  const db = getDb();
  const [row] = await db
    .select()
    .from(travelParentBlog)
    .where(
      and(
        eq(travelParentBlog.etablissementId, etablissementId),
        eq(travelParentBlog.travelId, travelId),
      ),
    )
    .limit(1);
  return row ?? null;
}

export async function getParentBlogByToken(
  token: string,
): Promise<(typeof travelParentBlog.$inferSelect) | null> {
  const db = getDb();
  const [row] = await db
    .select()
    .from(travelParentBlog)
    .where(eq(travelParentBlog.token, token))
    .limit(1);
  return row ?? null;
}

export async function syncParentBlogMetaOnTrip(
  trip: TravelsTrip,
  etablissementId: string,
): Promise<TravelsTrip> {
  const row = await getParentBlogForTrip(etablissementId, trip.id);
  if (!row) {
    if (!trip.data.parentBlog) return trip;
    const next = {
      ...trip,
      data: { ...trip.data, parentBlog: undefined },
    };
    await putJson(`travels/${trip.id}.json`, next);
    return next;
  }
  const meta = metaFromBlogRow(row);
  const next: TravelsTrip = {
    ...trip,
    data: { ...trip.data, parentBlog: meta },
  };
  await putJson(`travels/${trip.id}.json`, next);
  return next;
}

export type ActivateParentBlogResult = {
  trip: TravelsTrip;
  meta: TravelsParentBlogMeta;
  created: boolean;
  parentsNotified: number;
  parentsSkippedReason?: string;
};

export async function activateParentBlog(opts: {
  etablissementId: string;
  trip: TravelsTrip;
  activatedByUserId: string;
  activatedByName: string;
  /** Si true, envoie le mail parents (ICS + lien) sauf déjà notifié. */
  notifyParents: boolean;
  /** Inclure l’ICS dans le mail (défaut true). */
  attachIcs?: boolean;
  parentCalendar?: TravelsParentCalendar;
}): Promise<ActivateParentBlogResult> {
  const { etablissementId, trip, activatedByUserId, activatedByName } = opts;
  const attachIcs = opts.attachIcs !== false;
  const db = getDb();
  const existing = await getParentBlogForTrip(etablissementId, trip.id);
  const now = new Date();
  const expiresAt = computeParentBlogExpiresAt(trip);

  if (existing && isParentBlogPublicExpired(existing.expiresAt, now) && existing.purgedAt) {
    throw new Error("La fenêtre du blog parents est terminée pour ce dossier.");
  }
  if (existing && isParentBlogPublicExpired(existing.expiresAt, now)) {
    throw new Error("La fenêtre du blog parents est terminée pour ce dossier.");
  }

  let row = existing;
  let created = false;
  if (!row) {
    const token = newToken();
    const [inserted] = await db
      .insert(travelParentBlog)
      .values({
        etablissementId,
        travelId: trip.id,
        token,
        enabled: true,
        activatedAt: now,
        expiresAt,
        activatedByUserId,
      })
      .returning();
    row = inserted;
    created = true;
  } else if (!row.enabled) {
    const [updated] = await db
      .update(travelParentBlog)
      .set({
        enabled: true,
        activatedAt: now,
        expiresAt,
        activatedByUserId,
        purgedAt: null,
      })
      .where(
        and(
          eq(travelParentBlog.etablissementId, etablissementId),
          eq(travelParentBlog.travelId, trip.id),
        ),
      )
      .returning();
    row = updated;
  } else {
    // Recalcule l’expiration si les dates voyage ont bougé.
    const [updated] = await db
      .update(travelParentBlog)
      .set({ expiresAt })
      .where(
        and(
          eq(travelParentBlog.etablissementId, etablissementId),
          eq(travelParentBlog.travelId, trip.id),
        ),
      )
      .returning();
    row = updated ?? row;
  }

  const meta = metaFromBlogRow(row);
  let parentsNotified = 0;
  let parentsSkippedReason: string | undefined;

  const calendar =
    opts.parentCalendar && Array.isArray(opts.parentCalendar.points)
      ? opts.parentCalendar
      : trip.data.parentCalendar || defaultParentCalendarFromTrip(trip.data);

  let updatedTrip: TravelsTrip = {
    ...trip,
    updatedAt: now.toISOString(),
    data: {
      ...trip.data,
      parentCalendar: calendar,
      parentBlog: meta,
    },
    history: [
      ...(Array.isArray(trip.history) ? trip.history : []),
      {
        date: now.toISOString(),
        user: activatedByName,
        action: created
          ? "Blog parents activé (page de suivi publique)"
          : "Blog parents réactivé / mis à jour",
        note: meta.publicPath,
      },
    ],
  };

  const alreadyNotified = Boolean(row.parentsNotifiedAt);
  if (opts.notifyParents && !alreadyNotified) {
    const notify = await sendParentBlogActivationMail({
      trip: updatedTrip,
      token: row.token,
      calendar,
      attachIcs,
      senderName: activatedByName,
    });
    parentsNotified = notify.recipients;
    parentsSkippedReason = notify.skippedReason;
    if (notify.recipients > 0) {
      await db
        .update(travelParentBlog)
        .set({ parentsNotifiedAt: now })
        .where(eq(travelParentBlog.id, row.id));
      meta.parentsNotifiedAt = now.toISOString();
      const log: TravelsParentComLog = {
        id: newId("pc"),
        sentAt: now.toISOString(),
        sentBy: { userId: activatedByUserId, name: activatedByName },
        subject: `Calendrier & suivi — ${trip.data.title || trip.data.destination || trip.id}`,
        body: "Activation blog parents + calendrier",
        photoCount: 0,
        recipientCount: notify.recipients,
        icsAttached: attachIcs && notify.icsAttached,
      };
      updatedTrip = {
        ...updatedTrip,
        data: {
          ...updatedTrip.data,
          parentBlog: meta,
          parentComLogs: [...(updatedTrip.data.parentComLogs || []), log],
        },
      };
    }
  }

  await putJson(`travels/${trip.id}.json`, updatedTrip);
  return {
    trip: updatedTrip,
    meta,
    created,
    parentsNotified,
    parentsSkippedReason,
  };
}

function escapeHtml(s: string) {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

async function sendParentBlogActivationMail(opts: {
  trip: TravelsTrip;
  token: string;
  calendar: TravelsParentCalendar;
  attachIcs: boolean;
  senderName: string;
}): Promise<{ recipients: number; skippedReason?: string; icsAttached: boolean }> {
  const { trip, token, calendar, attachIcs, senderName } = opts;
  const participants = trip.data.participantEleves || [];
  if (participants.length === 0) {
    return { recipients: 0, skippedReason: "Aucun élève sur la liste.", icsAttached: false };
  }

  const eleves = await loadElevesRegistry().catch(() => []);
  const byIne = new Map(eleves.map((e) => [e.ine, e]));
  const emailSet = new Set<string>();
  for (const p of participants) {
    const full = byIne.get(p.ine);
    if (!full) continue;
    for (const mail of collectEleveParentEmails(full)) emailSet.add(mail);
  }
  const recipients = [...emailSet];
  if (recipients.length === 0) {
    return {
      recipients: 0,
      skippedReason: "Aucun e-mail parent trouvé pour les élèves de la liste.",
      icsAttached: false,
    };
  }

  const smtp = await getTenantSmtpConfig();
  const transporter = smtp ? await createTenantTransporter() : null;
  if (!smtp || !transporter) {
    return { recipients: 0, skippedReason: "SMTP non configuré.", icsAttached: false };
  }

  const tripTitle = String(trip.data.title || trip.data.destination || "Sortie scolaire");
  const publicUrl = await tenantAbsolutePath(parentBlogPublicPath(token));
  const mailCopy = buildParentsCalendarMailCopy({
    tripTitle,
    data: trip.data,
    calendar,
  });

  const attachments: Array<{ filename: string; content: Buffer; contentType: string }> = [];
  let icsAttached = false;
  if (attachIcs) {
    const ics = buildTravelsParentsTripIcs({
      tripId: trip.id,
      tripTitle,
      destination: trip.data.destination ? String(trip.data.destination) : undefined,
      data: { ...trip.data, parentCalendar: calendar },
      calendar,
    });
    attachments.push({
      filename: "calendrier-sortie.ics",
      content: Buffer.from(ics, "utf8"),
      contentType: "text/calendar; charset=utf-8",
    });
    icsAttached = true;
  }

  const subject = `Calendrier & suivi — ${tripTitle}`;
  const text = [
    "Bonjour,",
    "",
    mailCopy.intro,
    "",
    icsAttached
      ? "Un fichier calendrier (.ics) est joint : ouvrez-le pour ajouter les créneaux (dépôt / récupération) à votre agenda."
      : "",
    "",
    mailCopy.pointsBlock,
    "",
    "Page de suivi de la sortie (lecture seule) :",
    publicUrl,
    "",
    "Vous pourrez y consulter les messages et photos publiés par l’équipe pendant le séjour.",
    "Cette page se ferme automatiquement 15 jours après le retour.",
    "",
    "Cordialement,",
    senderName || "L'établissement",
  ]
    .filter(Boolean)
    .join("\n");

  const html = `
    <div style="font-family: sans-serif; line-height: 1.55; color: #334155; max-width: 560px;">
      <p>Bonjour,</p>
      <p>${escapeHtml(mailCopy.intro)}</p>
      ${
        icsAttached
          ? `<p>Un fichier calendrier (<strong>.ics</strong>) est joint : ouvrez-le pour ajouter les créneaux à votre agenda.</p>`
          : ""
      }
      ${
        mailCopy.pointsBlock
          ? `<pre style="white-space: pre-wrap; background:#eff6ff; border:1px solid #bfdbfe; border-radius:8px; padding:12px; font-size:13px;">${escapeHtml(mailCopy.pointsBlock)}</pre>`
          : ""
      }
      <p style="margin: 18px 0 8px;">Suivez le séjour sur la page dédiée (lecture seule) :</p>
      <p style="margin: 0 0 16px;">
        <a href="${escapeHtml(publicUrl)}" style="display:inline-block;background:#0ea5e9;color:#fff;text-decoration:none;padding:10px 16px;border-radius:8px;font-weight:600;">
          Ouvrir la page de suivi
        </a>
      </p>
      <p style="font-size: 13px; color: #64748b;">
        Messages et photos publiés par l’équipe. Pas de réponse attendue sur ce canal.
        La page se ferme automatiquement <strong>15 jours après le retour</strong>.
      </p>
      <p>Cordialement,<br/>${escapeHtml(senderName || "L'établissement")}</p>
    </div>
  `;

  for (let i = 0; i < recipients.length; i += PARENT_BATCH) {
    const batch = recipients.slice(i, i + PARENT_BATCH);
    await sendMailWithTimeout(
      transporter,
      {
        from: `"Sorties scolaires" <${smtp.user}>`,
        bcc: batch,
        subject,
        text,
        html,
        attachments: attachments.length ? attachments : undefined,
      },
      120_000,
    );
  }

  return { recipients: recipients.length, icsAttached };
}

export type ParentBlogPostView = {
  id: string;
  authorName: string;
  body: string;
  createdAt: string;
  photos: Array<{ id: string; url: string; contentType: string }>;
};

export async function listParentBlogPosts(
  etablissementId: string,
  travelId: string,
  opts?: { includeSignedUrls?: boolean },
): Promise<ParentBlogPostView[]> {
  const db = getDb();
  const posts = await db
    .select()
    .from(travelParentBlogPost)
    .where(
      and(
        eq(travelParentBlogPost.etablissementId, etablissementId),
        eq(travelParentBlogPost.travelId, travelId),
      ),
    )
    .orderBy(desc(travelParentBlogPost.createdAt));

  if (posts.length === 0) return [];

  const photos = await db
    .select()
    .from(travelParentBlogPhoto)
    .where(
      and(
        eq(travelParentBlogPhoto.etablissementId, etablissementId),
        eq(travelParentBlogPhoto.travelId, travelId),
      ),
    )
    .orderBy(asc(travelParentBlogPhoto.sortOrder));

  const byPost = new Map<string, typeof photos>();
  for (const ph of photos) {
    const list = byPost.get(ph.postId) || [];
    list.push(ph);
    byPost.set(ph.postId, list);
  }

  const includeUrls = opts?.includeSignedUrls !== false;
  const result: ParentBlogPostView[] = [];
  for (const p of posts) {
    const phs = byPost.get(p.id) || [];
    const photoViews: ParentBlogPostView["photos"] = [];
    for (const ph of phs) {
      let url = "";
      if (includeUrls) {
        url = (await getSignedReadUrl(ph.s3Key, 3600)) || "";
      }
      photoViews.push({ id: ph.id, url, contentType: ph.contentType });
    }
    result.push({
      id: p.id,
      authorName: p.authorName,
      body: p.body,
      createdAt: p.createdAt.toISOString(),
      photos: photoViews,
    });
  }
  return result;
}

export type PhotoUploadPayload = {
  filename: string;
  contentType: string;
  contentBase64: string;
};

export async function createParentBlogPost(opts: {
  etablissementId: string;
  trip: TravelsTrip;
  authorUserId: string;
  authorName: string;
  body: string;
  photos: PhotoUploadPayload[];
}): Promise<ParentBlogPostView> {
  const { etablissementId, trip, authorUserId, authorName } = opts;
  const body = String(opts.body || "").trim();
  if (!body) throw new Error("Message requis.");
  if (opts.photos.length > PARENT_BLOG_MAX_PHOTOS) {
    throw new Error(`Maximum ${PARENT_BLOG_MAX_PHOTOS} photos par publication.`);
  }

  const blog = await getParentBlogForTrip(etablissementId, trip.id);
  if (!blog?.enabled) throw new Error("Le blog parents n’est pas activé.");
  if (isParentBlogPublicExpired(blog.expiresAt)) {
    throw new Error("La page publique est fermée (délai de 15 jours après le retour écoulé).");
  }

  const db = getDb();
  const postId = newId("pbp");
  const now = new Date();

  await db.insert(travelParentBlogPost).values({
    id: postId,
    etablissementId,
    travelId: trip.id,
    authorUserId,
    authorName,
    body,
    createdAt: now,
  });

  const photoViews: ParentBlogPostView["photos"] = [];
  for (let i = 0; i < opts.photos.length; i++) {
    const ph = opts.photos[i];
    const raw = String(ph.contentBase64 || "").replace(/\s/g, "");
    if (!raw) continue;
    const buf = Buffer.from(raw, "base64");
    if (buf.length > PARENT_BLOG_MAX_PHOTO_BYTES) {
      throw new Error(`Photo « ${ph.filename || i + 1} » trop volumineuse (max ~900 Ko).`);
    }
    const ct = String(ph.contentType || "image/jpeg").toLowerCase();
    if (!ct.startsWith("image/")) throw new Error("Seules les images sont acceptées.");

    const safeName = String(ph.filename || `photo_${i + 1}.jpg`).replace(/[^\w.\-]+/g, "_");
    const relative = `travels/${trip.id}/parent-blog/${postId}/${i}_${safeName}`;
    const key = await putObject(relative, buf, ct);
    const photoId = newId("pbph");
    await db.insert(travelParentBlogPhoto).values({
      id: photoId,
      etablissementId,
      postId,
      travelId: trip.id,
      s3Key: key,
      contentType: ct,
      sortOrder: i,
    });
    const url = (await getSignedReadUrl(key, 3600)) || "";
    photoViews.push({ id: photoId, url, contentType: ct });
  }

  const updatedTrip: TravelsTrip = {
    ...trip,
    updatedAt: now.toISOString(),
    history: [
      ...(Array.isArray(trip.history) ? trip.history : []),
      {
        date: now.toISOString(),
        user: authorName,
        action: "Publication blog parents",
        note: body.slice(0, 120),
      },
    ],
  };
  await putJson(`travels/${trip.id}.json`, updatedTrip);

  return {
    id: postId,
    authorName,
    body,
    createdAt: now.toISOString(),
    photos: photoViews,
  };
}

export async function deleteParentBlogPost(opts: {
  etablissementId: string;
  trip: TravelsTrip;
  postId: string;
  actorName: string;
}): Promise<void> {
  const { etablissementId, trip, postId, actorName } = opts;
  const db = getDb();
  const photos = await db
    .select()
    .from(travelParentBlogPhoto)
    .where(
      and(
        eq(travelParentBlogPhoto.etablissementId, etablissementId),
        eq(travelParentBlogPhoto.postId, postId),
      ),
    );

  for (const ph of photos) {
    try {
      await deleteObject(ph.s3Key);
    } catch {
      /* ignore missing */
    }
  }

  await db
    .delete(travelParentBlogPost)
    .where(
      and(
        eq(travelParentBlogPost.etablissementId, etablissementId),
        eq(travelParentBlogPost.id, postId),
        eq(travelParentBlogPost.travelId, trip.id),
      ),
    );

  const now = new Date().toISOString();
  const updatedTrip: TravelsTrip = {
    ...trip,
    updatedAt: now,
    history: [
      ...(Array.isArray(trip.history) ? trip.history : []),
      {
        date: now,
        user: actorName,
        action: "Suppression publication blog parents",
        note: postId,
      },
    ],
  };
  await putJson(`travels/${trip.id}.json`, updatedTrip);
}

export async function setParentBlogDelegates(opts: {
  trip: TravelsTrip;
  delegates: TravelsParentBlogDelegate[];
  actorName: string;
}): Promise<TravelsTrip> {
  const cleaned = opts.delegates
    .filter((d) => d.userId && d.name)
    .map((d) => ({
      userId: String(d.userId).trim(),
      name: String(d.name).trim(),
      ...(d.email ? { email: String(d.email).trim() } : {}),
    }));

  const now = new Date().toISOString();
  const updatedTrip: TravelsTrip = {
    ...opts.trip,
    updatedAt: now,
    data: {
      ...opts.trip.data,
      parentBlogDelegates: cleaned,
    },
    history: [
      ...(Array.isArray(opts.trip.history) ? opts.trip.history : []),
      {
        date: now,
        user: opts.actorName,
        action: "Délégués blog parents mis à jour",
        note: `${cleaned.length} personne(s)`,
      },
    ],
  };
  await putJson(`travels/${opts.trip.id}.json`, updatedTrip);
  return updatedTrip;
}

/** Purge posts + photos S3 pour un blog expiré (idempotent). */
export async function purgeParentBlogContent(
  etablissementId: string,
  travelId: string,
): Promise<{ purgedPosts: number; purgedPhotos: number }> {
  const db = getDb();
  const blog = await getParentBlogForTrip(etablissementId, travelId);
  if (!blog) return { purgedPosts: 0, purgedPhotos: 0 };
  if (blog.purgedAt) return { purgedPosts: 0, purgedPhotos: 0 };

  const photos = await db
    .select()
    .from(travelParentBlogPhoto)
    .where(
      and(
        eq(travelParentBlogPhoto.etablissementId, etablissementId),
        eq(travelParentBlogPhoto.travelId, travelId),
      ),
    );

  for (const ph of photos) {
    try {
      await deleteObject(ph.s3Key);
    } catch {
      /* ignore */
    }
  }

  const deletedPosts = await db
    .delete(travelParentBlogPost)
    .where(
      and(
        eq(travelParentBlogPost.etablissementId, etablissementId),
        eq(travelParentBlogPost.travelId, travelId),
      ),
    )
    .returning({ id: travelParentBlogPost.id });

  await db
    .update(travelParentBlog)
    .set({ purgedAt: new Date(), enabled: false })
    .where(eq(travelParentBlog.id, blog.id));

  const hit = await getJson<TravelsTrip>(`travels/${travelId}.json`);
  if (hit?.data) {
    const trip = hit.data;
    await putJson(`travels/${travelId}.json`, {
      ...trip,
      data: {
        ...trip.data,
        parentBlog: trip.data.parentBlog
          ? { ...trip.data.parentBlog, enabled: false }
          : undefined,
      },
    });
  }

  return { purgedPosts: deletedPosts.length, purgedPhotos: photos.length };
}

/** Filet de sécurité : purge des blogs déjà expirés du tenant. */
export async function purgeExpiredParentBlogsForEtablissement(
  etablissementId: string,
): Promise<number> {
  const db = getDb();
  const now = new Date();
  const rows = await db
    .select()
    .from(travelParentBlog)
    .where(
      and(
        eq(travelParentBlog.etablissementId, etablissementId),
        lt(travelParentBlog.expiresAt, now),
        isNull(travelParentBlog.purgedAt),
      ),
    );
  let n = 0;
  for (const row of rows) {
    await purgeParentBlogContent(etablissementId, row.travelId);
    n += 1;
  }
  return n;
}

export async function loadTripTitleForPublic(
  etablissementId: string,
  travelId: string,
): Promise<{ title: string; destination: string | null; startDate: string | null; endDate: string | null } | null> {
  const db = getDb();
  const [m] = await db
    .select({
      title: travel.title,
      destination: travel.destination,
      startDate: travel.startDate,
      endDate: travel.endDate,
    })
    .from(travel)
    .where(and(eq(travel.etablissementId, etablissementId), eq(travel.id, travelId)))
    .limit(1);
  if (!m) return null;
  return {
    title: m.title || "Sortie scolaire",
    destination: m.destination,
    startDate: m.startDate,
    endDate: m.endDate,
  };
}
