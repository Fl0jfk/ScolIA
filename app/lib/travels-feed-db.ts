import "server-only";

import { and, asc, desc, eq, gt, inArray, isNull, lt, lte, or, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  travelFeedAccess,
  travelFeedAudit,
  travelFeedPhoto,
  travelFeedPost,
  travelFeedReaction,
  travelFeedSettings,
} from "@/db/schema";
import { FEED_EMAIL_CAP_PER_FAMILY, FEED_EMAIL_MIN_INTERVAL_MS, FEED_MAX_TOKEN_HASHES } from "@/app/lib/travels-feed-logic";

/**
 * Accès base du « fil du voyage ». Chaque requête est filtrée par `etablissement_id` ET `trip_id`.
 * Mutations ciblées : jamais de réécriture du JSON du voyage (pas de course avec les autres modules).
 */

export type FeedSettingsRow = typeof travelFeedSettings.$inferSelect;
export type FeedAccessRow = typeof travelFeedAccess.$inferSelect;
export type FeedPostRow = typeof travelFeedPost.$inferSelect;
export type FeedPhotoRow = typeof travelFeedPhoto.$inferSelect;

const norm = (s: string) => String(s || "").trim().toLowerCase();

/* ─── Réglages ─── */

export async function getFeedSettings(etabId: string, tripId: string): Promise<FeedSettingsRow | null> {
  const [row] = await getDb()
    .select()
    .from(travelFeedSettings)
    .where(and(eq(travelFeedSettings.etablissementId, etabId), eq(travelFeedSettings.tripId, tripId)))
    .limit(1);
  return row ?? null;
}

export async function listFeedSettings(etabId: string): Promise<FeedSettingsRow[]> {
  return getDb().select().from(travelFeedSettings).where(eq(travelFeedSettings.etablissementId, etabId));
}

export async function openFeedSettings(etabId: string, tripId: string, openedBy: string | null, returnDate: string | null): Promise<void> {
  await getDb()
    .insert(travelFeedSettings)
    .values({ etablissementId: etabId, tripId, openedBy, returnDate })
    .onConflictDoUpdate({
      target: [travelFeedSettings.etablissementId, travelFeedSettings.tripId],
      set: { returnDate },
    });
}

export async function setFeedReturnDate(etabId: string, tripId: string, returnDate: string | null): Promise<void> {
  await getDb()
    .update(travelFeedSettings)
    .set({ returnDate })
    .where(and(eq(travelFeedSettings.etablissementId, etabId), eq(travelFeedSettings.tripId, tripId)));
}

export async function setFeedAllowDownload(etabId: string, tripId: string, allow: boolean): Promise<void> {
  await getDb()
    .update(travelFeedSettings)
    .set({ allowDownload: allow })
    .where(and(eq(travelFeedSettings.etablissementId, etabId), eq(travelFeedSettings.tripId, tripId)));
}

/** Réserve un e-mail sur le plafond du voyage (atomique). Retourne false si le plafond est atteint. */
export async function reserveTripEmail(etabId: string, tripId: string, cap: number): Promise<boolean> {
  const rows = await getDb()
    .update(travelFeedSettings)
    .set({ emailsSent: sql`${travelFeedSettings.emailsSent} + 1` })
    .where(
      and(
        eq(travelFeedSettings.etablissementId, etabId),
        eq(travelFeedSettings.tripId, tripId),
        lt(travelFeedSettings.emailsSent, cap),
      ),
    )
    .returning({ n: travelFeedSettings.emailsSent });
  return rows.length > 0;
}

export async function releaseTripEmail(etabId: string, tripId: string): Promise<void> {
  await getDb()
    .update(travelFeedSettings)
    .set({ emailsSent: sql`greatest(${travelFeedSettings.emailsSent} - 1, 0)` })
    .where(and(eq(travelFeedSettings.etablissementId, etabId), eq(travelFeedSettings.tripId, tripId)));
}

/* ─── Accès familiaux ─── */

export async function listFeedAccess(etabId: string, tripId: string): Promise<FeedAccessRow[]> {
  return getDb()
    .select()
    .from(travelFeedAccess)
    .where(and(eq(travelFeedAccess.etablissementId, etabId), eq(travelFeedAccess.tripId, tripId)))
    .orderBy(asc(travelFeedAccess.createdAt));
}

export async function getFeedAccess(etabId: string, tripId: string, accessId: string): Promise<FeedAccessRow | null> {
  const [row] = await getDb()
    .select()
    .from(travelFeedAccess)
    .where(
      and(
        eq(travelFeedAccess.etablissementId, etabId),
        eq(travelFeedAccess.tripId, tripId),
        eq(travelFeedAccess.id, accessId),
      ),
    )
    .limit(1);
  return row ?? null;
}

export async function upsertFeedAccess(
  etabId: string,
  tripId: string,
  input: { email: string; ines: string[]; familyKey: string },
): Promise<FeedAccessRow> {
  const [row] = await getDb()
    .insert(travelFeedAccess)
    .values({ etablissementId: etabId, tripId, email: norm(input.email), ines: input.ines, familyKey: input.familyKey })
    .onConflictDoUpdate({
      target: [travelFeedAccess.etablissementId, travelFeedAccess.tripId, travelFeedAccess.email],
      set: { ines: input.ines, familyKey: input.familyKey },
    })
    .returning();
  return row!;
}

/** Ajoute une empreinte de jeton (les plus récentes sont gardées : les anciens liens des e-mails restent valables). */
export async function addFeedTokenHash(etabId: string, tripId: string, accessId: string, hash: string): Promise<boolean> {
  return getDb().transaction(async (tx) => {
    const [row] = await tx
      .select({ hashes: travelFeedAccess.tokenHashes })
      .from(travelFeedAccess)
      .where(
        and(
          eq(travelFeedAccess.etablissementId, etabId),
          eq(travelFeedAccess.tripId, tripId),
          eq(travelFeedAccess.id, accessId),
        ),
      )
      .for("update");
    if (!row) return false;
    const next = [...row.hashes.filter((h) => h !== hash), hash].slice(-FEED_MAX_TOKEN_HASHES);
    await tx.update(travelFeedAccess).set({ tokenHashes: next }).where(eq(travelFeedAccess.id, accessId));
    return true;
  });
}

/** L'e-mail n'est pas parti : on retire l'empreinte (pas de lien orphelin). */
export async function removeFeedTokenHash(etabId: string, tripId: string, accessId: string, hash: string): Promise<void> {
  await getDb()
    .update(travelFeedAccess)
    .set({
      tokenHashes: sql`array_remove(${travelFeedAccess.tokenHashes}, ${hash})`,
      sendFailures: sql`${travelFeedAccess.sendFailures} + 1`,
    })
    .where(
      and(
        eq(travelFeedAccess.etablissementId, etabId),
        eq(travelFeedAccess.tripId, tripId),
        eq(travelFeedAccess.id, accessId),
      ),
    );
}

export async function markFeedLinkSent(etabId: string, tripId: string, accessId: string): Promise<void> {
  await getDb()
    .update(travelFeedAccess)
    .set({ firstSentAt: sql`coalesce(${travelFeedAccess.firstSentAt}, now())`, sendFailures: 0 })
    .where(
      and(
        eq(travelFeedAccess.etablissementId, etabId),
        eq(travelFeedAccess.tripId, tripId),
        eq(travelFeedAccess.id, accessId),
      ),
    );
}

/** Une publication de plus pour toutes les familles du voyage (résumé « N nouveautés »). */
export async function bumpPendingForTrip(etabId: string, tripId: string, delta: { messages: number; photos: number }): Promise<void> {
  await getDb()
    .update(travelFeedAccess)
    .set({
      pendingMessages: sql`${travelFeedAccess.pendingMessages} + ${delta.messages}`,
      pendingPhotos: sql`${travelFeedAccess.pendingPhotos} + ${delta.photos}`,
    })
    .where(and(eq(travelFeedAccess.etablissementId, etabId), eq(travelFeedAccess.tripId, tripId)));
}

/**
 * Prend la main sur l'e-mail de nouveautés d'une famille (atomique : deux passages simultanés n'envoient qu'un e-mail).
 * Conditions repérées EN BASE : des nouveautés en attente, dernier e-mail d'il y a plus d'une heure, plafond famille non atteint.
 * Les compteurs en attente sont remis à zéro et retournés pour construire l'e-mail.
 */
export async function claimFeedNotification(
  etabId: string,
  tripId: string,
  accessId: string,
): Promise<{ pendingMessages: number; pendingPhotos: number; previousNotifiedAt: Date | null; previousCount: number; email: string; claimedAt: Date } | null> {
  return getDb().transaction(async (tx) => {
    const [cur] = await tx
      .select()
      .from(travelFeedAccess)
      .where(
        and(
          eq(travelFeedAccess.etablissementId, etabId),
          eq(travelFeedAccess.tripId, tripId),
          eq(travelFeedAccess.id, accessId),
          or(
            isNull(travelFeedAccess.lastNotifiedAt),
            lt(travelFeedAccess.lastNotifiedAt, sql`now() - (${FEED_EMAIL_MIN_INTERVAL_MS}::double precision * interval '1 millisecond')`),
          ),
          lt(travelFeedAccess.notifyCount, FEED_EMAIL_CAP_PER_FAMILY),
          sql`${travelFeedAccess.pendingMessages} + ${travelFeedAccess.pendingPhotos} > 0`,
        ),
      )
      .for("update", { skipLocked: true });
    if (!cur) return null;
    const [upd] = await tx
      .update(travelFeedAccess)
      .set({ pendingMessages: 0, pendingPhotos: 0, lastNotifiedAt: sql`now()`, notifyCount: sql`${travelFeedAccess.notifyCount} + 1` })
      .where(eq(travelFeedAccess.id, accessId))
      .returning({ at: travelFeedAccess.lastNotifiedAt });
    return {
      pendingMessages: cur.pendingMessages,
      pendingPhotos: cur.pendingPhotos,
      previousNotifiedAt: cur.lastNotifiedAt,
      previousCount: cur.notifyCount,
      email: cur.email,
      claimedAt: upd?.at ?? new Date(),
    };
  });
}

/** L'e-mail n'est pas parti : on rend les nouveautés à la famille (elles seront reprises au prochain passage). */
export async function restoreFeedNotification(
  etabId: string,
  tripId: string,
  accessId: string,
  prev: { pendingMessages: number; pendingPhotos: number; previousNotifiedAt: Date | null; previousCount: number },
): Promise<void> {
  await getDb()
    .update(travelFeedAccess)
    .set({
      pendingMessages: sql`${travelFeedAccess.pendingMessages} + ${prev.pendingMessages}`,
      pendingPhotos: sql`${travelFeedAccess.pendingPhotos} + ${prev.pendingPhotos}`,
      lastNotifiedAt: prev.previousNotifiedAt,
      notifyCount: prev.previousCount,
    })
    .where(
      and(
        eq(travelFeedAccess.etablissementId, etabId),
        eq(travelFeedAccess.tripId, tripId),
        eq(travelFeedAccess.id, accessId),
      ),
    );
}

/* ─── Publications ─── */

export async function insertFeedPost(
  etabId: string,
  input: {
    tripId: string;
    kind: string;
    body: string | null;
    delayMinutes: number | null;
    authorUserId: string | null;
    authorName: string | null;
  },
): Promise<FeedPostRow> {
  const [row] = await getDb()
    .insert(travelFeedPost)
    .values({ etablissementId: etabId, ...input })
    .returning();
  return row!;
}

export async function insertFeedPhotos(
  etabId: string,
  tripId: string,
  postId: string,
  photos: { id: string; storageKey: string; width: number; height: number; bytes: number; position: number }[],
): Promise<void> {
  if (photos.length === 0) return;
  await getDb()
    .insert(travelFeedPhoto)
    .values(photos.map((p) => ({ etablissementId: etabId, tripId, postId, ...p })));
}

export async function deleteFeedPost(etabId: string, tripId: string, postId: string): Promise<void> {
  await getDb()
    .delete(travelFeedPost)
    .where(and(eq(travelFeedPost.etablissementId, etabId), eq(travelFeedPost.tripId, tripId), eq(travelFeedPost.id, postId)));
}

export type FeedPostWithMedia = FeedPostRow & { photos: FeedPhotoRow[]; reactions: { familyKey: string; emoji: string }[] };

/** Publications du voyage (récentes d'abord) ; les publications masquées sont exclues sauf pour la modération. */
export async function listFeedPosts(etabId: string, tripId: string, opts: { includeHidden: boolean }): Promise<FeedPostWithMedia[]> {
  const db = getDb();
  const posts = await db
    .select()
    .from(travelFeedPost)
    .where(
      and(
        eq(travelFeedPost.etablissementId, etabId),
        eq(travelFeedPost.tripId, tripId),
        opts.includeHidden ? undefined : isNull(travelFeedPost.hiddenAt),
      ),
    )
    .orderBy(desc(travelFeedPost.createdAt))
    .limit(300);
  if (posts.length === 0) return [];
  const ids = posts.map((p) => p.id);
  const [photos, reactions] = await Promise.all([
    db
      .select()
      .from(travelFeedPhoto)
      .where(and(eq(travelFeedPhoto.etablissementId, etabId), inArray(travelFeedPhoto.postId, ids)))
      .orderBy(asc(travelFeedPhoto.position)),
    db
      .select({ postId: travelFeedReaction.postId, familyKey: travelFeedReaction.familyKey, emoji: travelFeedReaction.emoji })
      .from(travelFeedReaction)
      .where(and(eq(travelFeedReaction.etablissementId, etabId), inArray(travelFeedReaction.postId, ids))),
  ]);
  return posts.map((p) => ({
    ...p,
    photos: photos.filter((x) => x.postId === p.id),
    reactions: reactions.filter((r) => r.postId === p.id).map((r) => ({ familyKey: r.familyKey, emoji: r.emoji })),
  }));
}

/** Publications VISIBLES (non masquées) créées dans ]since, until] : contenu de l'e-mail de nouveautés. */
export async function listFeedPostsBetween(etabId: string, tripId: string, since: Date, until: Date): Promise<FeedPostWithMedia[]> {
  const posts = await getDb()
    .select()
    .from(travelFeedPost)
    .where(
      and(
        eq(travelFeedPost.etablissementId, etabId),
        eq(travelFeedPost.tripId, tripId),
        isNull(travelFeedPost.hiddenAt),
        gt(travelFeedPost.createdAt, since),
        lte(travelFeedPost.createdAt, until),
      ),
    )
    .orderBy(asc(travelFeedPost.createdAt))
    .limit(100);
  if (posts.length === 0) return [];
  const photos = await getDb()
    .select()
    .from(travelFeedPhoto)
    .where(and(eq(travelFeedPhoto.etablissementId, etabId), inArray(travelFeedPhoto.postId, posts.map((p) => p.id))));
  return posts.map((p) => ({ ...p, photos: photos.filter((x) => x.postId === p.id), reactions: [] }));
}

export async function getFeedPost(etabId: string, tripId: string, postId: string): Promise<FeedPostRow | null> {
  const [row] = await getDb()
    .select()
    .from(travelFeedPost)
    .where(and(eq(travelFeedPost.etablissementId, etabId), eq(travelFeedPost.tripId, tripId), eq(travelFeedPost.id, postId)))
    .limit(1);
  return row ?? null;
}

export async function setFeedPostHidden(etabId: string, tripId: string, postId: string, hiddenBy: string | null, hidden: boolean): Promise<boolean> {
  const rows = await getDb()
    .update(travelFeedPost)
    .set({ hiddenAt: hidden ? sql`now()` : null, hiddenBy: hidden ? hiddenBy : null })
    .where(and(eq(travelFeedPost.etablissementId, etabId), eq(travelFeedPost.tripId, tripId), eq(travelFeedPost.id, postId)))
    .returning({ id: travelFeedPost.id });
  return rows.length > 0;
}

/* ─── Réactions ─── */

export async function setFeedReaction(
  etabId: string,
  tripId: string,
  postId: string,
  familyKey: string,
  emoji: string,
  on: boolean,
): Promise<void> {
  const db = getDb();
  if (on) {
    // Clé primaire (publication, famille, emoji) : une seule réaction de ce type par famille.
    await db.insert(travelFeedReaction).values({ etablissementId: etabId, tripId, postId, familyKey, emoji }).onConflictDoNothing();
  } else {
    await db
      .delete(travelFeedReaction)
      .where(
        and(
          eq(travelFeedReaction.etablissementId, etabId),
          eq(travelFeedReaction.tripId, tripId),
          eq(travelFeedReaction.postId, postId),
          eq(travelFeedReaction.familyKey, familyKey),
          eq(travelFeedReaction.emoji, emoji),
        ),
      );
  }
}

/* ─── Journal ─── */

export async function insertFeedAudit(
  etabId: string,
  input: { tripId: string; actorUserId: string | null; actorName: string | null; action: string; postId?: string | null; detail?: string | null },
): Promise<void> {
  await getDb()
    .insert(travelFeedAudit)
    .values({
      etablissementId: etabId,
      tripId: input.tripId,
      actorUserId: input.actorUserId,
      actorName: input.actorName,
      action: input.action,
      postId: input.postId ?? null,
      detail: input.detail ?? null,
    });
}

export async function listFeedAudit(etabId: string, tripId: string, limit = 50) {
  return getDb()
    .select()
    .from(travelFeedAudit)
    .where(and(eq(travelFeedAudit.etablissementId, etabId), eq(travelFeedAudit.tripId, tripId)))
    .orderBy(desc(travelFeedAudit.createdAt))
    .limit(limit);
}

/* ─── Purge ─── */

export async function listFeedPhotoKeys(etabId: string, tripId: string): Promise<string[]> {
  const rows = await getDb()
    .select({ key: travelFeedPhoto.storageKey })
    .from(travelFeedPhoto)
    .where(and(eq(travelFeedPhoto.etablissementId, etabId), eq(travelFeedPhoto.tripId, tripId)));
  return rows.map((r) => r.key);
}

/** Supprime textes, photos (lignes), réactions, accès (donc jetons) et réglages. Le journal (sans texte ni photo) est gardé. */
export async function deleteFeedData(etabId: string, tripId: string): Promise<void> {
  await getDb().transaction(async (tx) => {
    await tx.delete(travelFeedReaction).where(and(eq(travelFeedReaction.etablissementId, etabId), eq(travelFeedReaction.tripId, tripId)));
    await tx.delete(travelFeedPhoto).where(and(eq(travelFeedPhoto.etablissementId, etabId), eq(travelFeedPhoto.tripId, tripId)));
    await tx.delete(travelFeedPost).where(and(eq(travelFeedPost.etablissementId, etabId), eq(travelFeedPost.tripId, tripId)));
    await tx.delete(travelFeedAccess).where(and(eq(travelFeedAccess.etablissementId, etabId), eq(travelFeedAccess.tripId, tripId)));
    await tx.delete(travelFeedSettings).where(and(eq(travelFeedSettings.etablissementId, etabId), eq(travelFeedSettings.tripId, tripId)));
  });
}
