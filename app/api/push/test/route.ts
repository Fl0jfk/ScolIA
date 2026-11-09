import { NextResponse } from "next/server";
import { requireMessagingContext } from "@/app/lib/messaging/access";
import { readWebPushConfig } from "@/app/lib/web-push/config";
import { sendWebPushToUsers } from "@/app/lib/web-push/notify";
import { createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";

export const dynamic = "force-dynamic";

const testLimiter = createMemoryRateLimiter({ windowMs: 60_000, max: 3, name: "push-test" });

export async function POST() {
  const auth = await requireMessagingContext();
  if (!auth.ok) return auth.response;
  if (!(await testLimiter.allow(auth.ctx.userId))) {
    return NextResponse.json(
      { error: "Trop d’essais, réessayez dans une minute.", code: "RATE_LIMITED" },
      { status: 429 },
    );
  }
  if (!readWebPushConfig()) {
    return NextResponse.json({ error: "Notifications non configurées.", code: "PUSH_NOT_CONFIGURED" }, { status: 503 });
  }
  await sendWebPushToUsers({
    etablissementId: auth.ctx.etablissementId,
    userIds: [auth.ctx.userId],
    payload: {
      kind: "signal",
      title: "Notifications activées ✅",
      body: "Vous serez prévenu(e) des nouveautés de ScolIA.",
      url: "/",
      tag: "push-test",
    },
  });
  return NextResponse.json({ ok: true });
}
