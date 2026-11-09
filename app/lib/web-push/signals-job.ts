import "server-only";

import { createHash } from "node:crypto";
import postgres from "postgres";
import { and, eq, inArray, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { etablissement, webPushSubscription } from "@/db/schema";
import { webPushSignalState } from "@/db/schema-web-push";
import { readWebPushConfig } from "./config";
import { sendWebPushToUsers } from "./notify";
import { diffSignals, safeRelativeHref } from "./signals-diff";
import { notificationBody, notificationTitle } from "./signals-body";

/** Notifications des « nouveautés de l'ENT » envoyées par le serveur, sans onglet ouvert. */

const INTERVAL_MS = 5 * 60_000;
const FIRST_RUN_MS = 2 * 60_000;
const USER_TIMEOUT_MS = 20_000;
const CONCURRENCY = 2;
const MAX_USERS = 300;
const LOCK_KEY = 7_204_117_001; // clé constante du verrou advisory Postgres
const REFERENCE_ROW = "__ref"; // ligne témoin : référence initiale déjà prise, même si tous les compteurs sont à 0

type Target = { userId: string; etablissementId: string; slug: string };

export function isSignalsJobEnabled(): boolean {
  return Boolean(readWebPushConfig()) && process.env.WEB_PUSH_SIGNALS_DISABLED !== "1";
}

function withTimeout<T>(p: Promise<T>, ms: number): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const t = setTimeout(() => reject(new Error("timeout")), ms);
    p.then(
      (v) => {
        clearTimeout(t);
        resolve(v);
      },
      (e) => {
        clearTimeout(t);
        reject(e);
      },
    );
  });
}

async function processUser(target: Target): Promise<void> {
  const { resolveTenantBySlug } = await import("@/app/lib/tenant-registry");
  const tenant = await resolveTenantBySlug(target.slug);
  if (!tenant) return; // jamais de repli sur le tenant par défaut
  const { runWithTenant } = await import("@/app/lib/tenant-context");
  const { computeDashboardNotificationsForUser } = await import("./signals-compute");
  const current = await runWithTenant(tenant, () =>
    computeDashboardNotificationsForUser(target.userId, target.etablissementId),
  );
  if (!current) return;

  const db = getDb();
  const rows = await db
    .select({ signalId: webPushSignalState.signalId, count: webPushSignalState.count })
    .from(webPushSignalState)
    .where(eq(webPushSignalState.userId, target.userId));
  const stored =
    rows.length === 0 ? null : Object.fromEntries(rows.map((r) => [r.signalId, r.count]));
  if (stored) delete stored[REFERENCE_ROW];

  const { toNotify, nextCounts, toDelete } = diffSignals(stored, current);

  const upserts = [
    { id: REFERENCE_ROW, count: 0 },
    ...Object.entries(nextCounts).map(([id, count]) => ({ id, count })),
  ];
  for (const u of upserts) {
    await db
      .insert(webPushSignalState)
      .values({
        userId: target.userId,
        etablissementId: target.etablissementId,
        signalId: u.id,
        count: u.count,
      })
      .onConflictDoUpdate({
        target: [webPushSignalState.userId, webPushSignalState.signalId],
        set: { count: u.count, etablissementId: target.etablissementId, updatedAt: sql`now()` },
      });
  }
  if (toDelete.length > 0) {
    await db
      .delete(webPushSignalState)
      .where(
        and(
          eq(webPushSignalState.userId, target.userId),
          inArray(webPushSignalState.signalId, toDelete),
        ),
      );
  }

  for (const n of toNotify) {
    await sendWebPushToUsers({
      etablissementId: target.etablissementId,
      userIds: [target.userId],
      payload: {
        kind: "signal",
        title: notificationTitle(n),
        body: notificationBody(n),
        url: safeRelativeHref(n.href),
        // Tag court et unique par signal (les ids longs se confondraient une fois tronqués).
        tag: `s-${createHash("sha256").update(n.id).digest("base64url").slice(0, 24)}`,
      },
    });
  }
}

async function runPass(): Promise<void> {
  const url = process.env.DATABASE_URL?.trim();
  if (!url) return;
  // Connexion dédiée : le verrou advisory est lié à la session Postgres.
  // Pas d'idle_timeout : la connexion reste inactive pendant le passage, et sa fermeture
  // relâcherait le verrou (doublons possibles entre serveurs). Fermée explicitement à la fin.
  const lockClient = postgres(url, { max: 1, prepare: false, connect_timeout: 5, idle_timeout: 0 });
  try {
    const [lock] = await lockClient`select pg_try_advisory_lock(${LOCK_KEY}) as ok`;
    if (!lock?.ok) return; // un autre serveur fait déjà le passage
    try {
      const rows = await getDb()
        .selectDistinct({
          userId: webPushSubscription.userId,
          etablissementId: webPushSubscription.etablissementId,
          slug: etablissement.slug,
        })
        .from(webPushSubscription)
        .innerJoin(etablissement, eq(etablissement.id, webPushSubscription.etablissementId))
        .limit(MAX_USERS + 1);
      if (rows.length > MAX_USERS) {
        console.warn(`[web-push-signals] plafond de ${MAX_USERS} utilisateurs atteint`);
      }
      const targets = rows.slice(0, MAX_USERS);
      let i = 0;
      await Promise.all(
        Array.from({ length: Math.min(CONCURRENCY, targets.length) }, async () => {
          while (i < targets.length) {
            const target = targets[i++]!;
            try {
              await withTimeout(processUser(target), USER_TIMEOUT_MS);
            } catch (err) {
              console.error(
                "[web-push-signals] utilisateur ignoré",
                err instanceof Error ? err.message : "error",
              );
            }
          }
        }),
      );
    } finally {
      await lockClient`select pg_advisory_unlock(${LOCK_KEY})`.catch(() => undefined);
    }
  } catch (err) {
    console.error("[web-push-signals] passage en échec", err instanceof Error ? err.name : "error");
  } finally {
    await lockClient.end({ timeout: 2 }).catch(() => undefined);
  }
}

const g = globalThis as { __scoliaSignalsJob?: boolean };

/** Démarre le job périodique (idempotent). Appelé depuis instrumentation.ts (runtime nodejs). */
export function startSignalsJob(): void {
  if (g.__scoliaSignalsJob || !isSignalsJobEnabled()) return;
  g.__scoliaSignalsJob = true;
  let running = false;
  const tick = async () => {
    if (running) return; // pas de chevauchement
    running = true;
    try {
      await runPass();
    } catch {
      /* jamais de crash */
    } finally {
      running = false;
    }
  };
  setTimeout(() => {
    void tick();
    setInterval(() => void tick(), INTERVAL_MS).unref();
  }, FIRST_RUN_MS).unref();
}
