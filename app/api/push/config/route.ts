import { NextResponse } from "next/server";
import { requireMessagingContext } from "@/app/lib/messaging/access";
import { readWebPushConfig } from "@/app/lib/web-push/config";
import { isSignalsJobEnabled } from "@/app/lib/web-push/signals-job";

export const dynamic = "force-dynamic";

export async function GET() {
  const auth = await requireMessagingContext();
  if (!auth.ok) return auth.response;
  const cfg = readWebPushConfig();
  return NextResponse.json(
    {
      configured: Boolean(cfg),
      publicKey: cfg?.publicKey ?? null,
      signalsServer: isSignalsJobEnabled() && process.env.SCOLIA_TEST_MODE !== "1",
    },
    { headers: { "Cache-Control": "no-store" } },
  );
}
