import { NextResponse } from "next/server";
import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { webPushSubscription } from "@/db/schema";
import { requireMessagingContext } from "@/app/lib/messaging/access";

export const dynamic = "force-dynamic";

export async function POST(req: Request) {
  const auth = await requireMessagingContext();
  if (!auth.ok) return auth.response;
  const body = (await req.json().catch(() => null)) as { endpoint?: unknown } | null;
  const endpoint = typeof body?.endpoint === "string" ? body.endpoint : "";
  if (!endpoint || endpoint.length > 2048) {
    return NextResponse.json({ error: "Requête invalide." }, { status: 400 });
  }
  await getDb()
    .delete(webPushSubscription)
    .where(
      and(
        eq(webPushSubscription.endpoint, endpoint),
        eq(webPushSubscription.userId, auth.ctx.userId),
        eq(webPushSubscription.etablissementId, auth.ctx.etablissementId),
      ),
    );
  return NextResponse.json({ ok: true });
}
