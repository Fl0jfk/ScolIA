import "server-only";

import webpush from "web-push";
import { and, eq, inArray } from "drizzle-orm";
import { getDb } from "@/db/index";
import { webPushPreference, webPushSubscription } from "@/db/schema";
import { readWebPushConfig } from "./config";
import { isAllowedPushEndpoint } from "./endpoint";
import { isInQuietHours, parisMinuteOfDay } from "./quiet-hours";
import type { WebPushPayload, WebPushSendInput } from "./types";

const TTL_SECONDS = 12 * 60 * 60;
const MAX_FAILURES = 5;
const CONCURRENCY = 8;
const MAX_USERS = 500;
const TOPIC_RE = /^[A-Za-z0-9_-]{1,32}$/;

function safePayload(p: WebPushPayload): string {
  const url = p.url.startsWith("/") && !p.url.startsWith("//") ? p.url : "/";
  return JSON.stringify({
    kind: p.kind,
    title: p.title.slice(0, 120),
    body: p.body.slice(0, 240),
    url,
    ...(p.tag ? { tag: p.tag.slice(0, 120) } : {}),
  });
}

async function runBounded<T>(items: T[], worker: (item: T) => Promise<void>): Promise<void> {
  let i = 0;
  const runners = Array.from({ length: Math.min(CONCURRENCY, items.length) }, async () => {
    while (i < items.length) {
      const item = items[i++];
      try {
        await worker(item);
      } catch {
        /* best effort */
      }
    }
  });
  await Promise.all(runners);
}

/**
 * Envoie une notification navigateur aux abonnements des utilisateurs donnés.
 * Ne lève jamais d'erreur (best effort) ; sans clés VAPID configurées, ne fait rien.
 */
export async function sendWebPushToUsers(input: WebPushSendInput): Promise<void> {
  try {
    const config = readWebPushConfig();
    if (!config) return;
    const userIds = [...new Set(input.userIds)].slice(0, MAX_USERS);
    if (!input.etablissementId || userIds.length === 0) return;

    const db = getDb();
    const [prefs, subs] = await Promise.all([
      db
        .select()
        .from(webPushPreference)
        .where(
          and(
            eq(webPushPreference.etablissementId, input.etablissementId),
            inArray(webPushPreference.userId, userIds),
          ),
        ),
      db
        .select()
        .from(webPushSubscription)
        .where(
          and(
            eq(webPushSubscription.etablissementId, input.etablissementId),
            inArray(webPushSubscription.userId, userIds),
          ),
        ),
    ]);
    if (subs.length === 0) return;

    const prefByUser = new Map(prefs.map((p) => [p.userId, p]));
    const minute = parisMinuteOfDay();
    const eligible = subs.filter((s) => {
      // Défense en profondeur : jamais d'envoi vers un hôte hors liste blanche.
      if (!isAllowedPushEndpoint(s.endpoint)) return false;
      const pref = prefByUser.get(s.userId);
      if (!pref) return true;
      if (pref.mutedKinds.includes(input.payload.kind)) return false;
      return !isInQuietHours(minute, pref.quietStartMinute, pref.quietEndMinute);
    });
    if (eligible.length === 0) return;

    const body = safePayload(input.payload);
    const topic = input.payload.tag && TOPIC_RE.test(input.payload.tag) ? input.payload.tag : undefined;
    const options: webpush.RequestOptions = {
      TTL: TTL_SECONDS,
      urgency: "normal",
      ...(topic ? { topic } : {}),
      vapidDetails: {
        subject: config.subject,
        publicKey: config.publicKey,
        privateKey: config.privateKey,
      },
    };

    await runBounded(eligible, async (sub) => {
      try {
        await webpush.sendNotification(
          { endpoint: sub.endpoint, keys: { p256dh: sub.p256dh, auth: sub.auth } },
          body,
          options,
        );
        await db
          .update(webPushSubscription)
          .set({ lastSuccessAt: new Date(), failureCount: 0 })
          .where(eq(webPushSubscription.id, sub.id));
      } catch (err) {
        const status = (err as { statusCode?: number })?.statusCode;
        if (status === 404 || status === 410 || sub.failureCount + 1 >= MAX_FAILURES) {
          await db.delete(webPushSubscription).where(eq(webPushSubscription.id, sub.id));
        } else {
          await db
            .update(webPushSubscription)
            .set({ failureCount: sub.failureCount + 1 })
            .where(eq(webPushSubscription.id, sub.id));
        }
      }
    });
  } catch (err) {
    console.error("[web-push] envoi impossible", err instanceof Error ? err.name : "error");
  }
}
