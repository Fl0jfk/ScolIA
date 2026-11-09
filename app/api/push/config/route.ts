import { NextResponse } from "next/server";
import { requireMessagingContext } from "@/app/lib/messaging/access";
import { readWebPushConfig } from "@/app/lib/web-push/config";

export const dynamic = "force-dynamic";

export async function GET() {
  const auth = await requireMessagingContext();
  if (!auth.ok) return auth.response;
  const cfg = readWebPushConfig();
  return NextResponse.json(
    { configured: Boolean(cfg), publicKey: cfg?.publicKey ?? null },
    { headers: { "Cache-Control": "no-store" } },
  );
}
