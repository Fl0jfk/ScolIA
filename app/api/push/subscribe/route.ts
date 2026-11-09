import { NextResponse } from "next/server";
import { getDb } from "@/db/index";
import { webPushSubscription } from "@/db/schema";
import { requireMessagingContext } from "@/app/lib/messaging/access";
import { readWebPushConfig } from "@/app/lib/web-push/config";
import { isAllowedPushEndpoint } from "@/app/lib/web-push/endpoint";
import { createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";
import { and, desc, eq, inArray } from "drizzle-orm";

export const dynamic = "force-dynamic";

const MAX_SUBSCRIPTIONS_PER_USER = 10;
const subscribeLimiter = createMemoryRateLimiter({ windowMs: 60_000, max: 10, name: "push-subscribe" });

type Body = { endpoint?: unknown; keys?: { p256dh?: unknown; auth?: unknown } };

function str(v: unknown, max: number): string | null {
  return typeof v === "string" && v.length > 0 && v.length <= max ? v : null;
}

export async function POST(req: Request) {
  const auth = await requireMessagingContext();
  if (!auth.ok) return auth.response;
  if (!(await subscribeLimiter.allow(auth.ctx.userId))) {
    return NextResponse.json({ error: "Trop de demandes.", code: "RATE_LIMITED" }, { status: 429 });
  }
  if (!readWebPushConfig()) {
    return NextResponse.json({ error: "Notifications non configurées.", code: "PUSH_NOT_CONFIGURED" }, { status: 503 });
  }
  const body = (await req.json().catch(() => null)) as Body | null;
  const endpoint = str(body?.endpoint, 2048);
  const p256dh = str(body?.keys?.p256dh, 256);
  const authKey = str(body?.keys?.auth, 128);
  if (!endpoint || !p256dh || !authKey) {
    return NextResponse.json({ error: "Abonnement invalide." }, { status: 400 });
  }
  if (!isAllowedPushEndpoint(endpoint)) {
    return NextResponse.json(
      { error: "Service de notification du navigateur non reconnu.", code: "PUSH_ENDPOINT_REFUSED" },
      { status: 400 },
    );
  }
  const userAgent = (req.headers.get("user-agent") ?? "").slice(0, 300) || null;
  const { ctx } = auth;
  const db = getDb();
  await db
    .insert(webPushSubscription)
    .values({
      id: crypto.randomUUID(),
      etablissementId: ctx.etablissementId,
      userId: ctx.userId,
      endpoint,
      p256dh,
      auth: authKey,
      userAgent,
    })
    .onConflictDoUpdate({
      target: webPushSubscription.endpoint,
      set: {
        etablissementId: ctx.etablissementId,
        userId: ctx.userId,
        p256dh,
        auth: authKey,
        userAgent,
        failureCount: 0,
      },
    });

  // Plafond d'appareils par personne : on garde les plus récents.
  const mine = await db
    .select({ id: webPushSubscription.id })
    .from(webPushSubscription)
    .where(
      and(
        eq(webPushSubscription.etablissementId, ctx.etablissementId),
        eq(webPushSubscription.userId, ctx.userId),
      ),
    )
    .orderBy(desc(webPushSubscription.createdAt));
  const excess = mine.slice(MAX_SUBSCRIPTIONS_PER_USER).map((r) => r.id);
  if (excess.length > 0) {
    await db.delete(webPushSubscription).where(inArray(webPushSubscription.id, excess));
  }
  return NextResponse.json({ ok: true });
}
