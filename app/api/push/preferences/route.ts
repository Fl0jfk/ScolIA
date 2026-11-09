import { NextResponse } from "next/server";
import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { webPushPreference } from "@/db/schema";
import { requireMessagingContext } from "@/app/lib/messaging/access";
import { WEB_PUSH_KINDS } from "@/app/lib/web-push/config";
import type { WebPushKind } from "@/app/lib/web-push/types";

export const dynamic = "force-dynamic";

function validMinute(v: unknown): v is number {
  return typeof v === "number" && Number.isInteger(v) && v >= 0 && v < 1440;
}

export async function GET() {
  const auth = await requireMessagingContext();
  if (!auth.ok) return auth.response;
  const [row] = await getDb()
    .select()
    .from(webPushPreference)
    .where(
      and(
        eq(webPushPreference.userId, auth.ctx.userId),
        eq(webPushPreference.etablissementId, auth.ctx.etablissementId),
      ),
    )
    .limit(1);
  return NextResponse.json({
    mutedKinds: (row?.mutedKinds ?? []) as WebPushKind[],
    quietStartMinute: row?.quietStartMinute ?? null,
    quietEndMinute: row?.quietEndMinute ?? null,
  });
}

export async function PUT(req: Request) {
  const auth = await requireMessagingContext();
  if (!auth.ok) return auth.response;
  const body = (await req.json().catch(() => null)) as {
    mutedKinds?: unknown;
    quietStartMinute?: unknown;
    quietEndMinute?: unknown;
  } | null;
  if (!body || !Array.isArray(body.mutedKinds)) {
    return NextResponse.json({ error: "Préférences invalides." }, { status: 400 });
  }
  const allowed = new Set<string>(WEB_PUSH_KINDS);
  const muted = [...new Set(body.mutedKinds)].filter(
    (k): k is WebPushKind => typeof k === "string" && allowed.has(k),
  );
  const start = body.quietStartMinute ?? null;
  const end = body.quietEndMinute ?? null;
  const quietOk =
    (start === null && end === null) || (validMinute(start) && validMinute(end));
  if (!quietOk) {
    return NextResponse.json({ error: "Heures calmes invalides." }, { status: 400 });
  }
  const values = {
    mutedKinds: muted,
    quietStartMinute: start as number | null,
    quietEndMinute: end as number | null,
    etablissementId: auth.ctx.etablissementId,
    updatedAt: new Date(),
  };
  await getDb()
    .insert(webPushPreference)
    .values({ userId: auth.ctx.userId, ...values })
    .onConflictDoUpdate({ target: webPushPreference.userId, set: values });
  return NextResponse.json({ ok: true });
}
