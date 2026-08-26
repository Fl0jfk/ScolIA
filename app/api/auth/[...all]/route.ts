import { toNextJsHandler } from "better-auth/next-js";
import { getBetterAuth } from "@/app/lib/auth-server";
import { isBetterAuthConfigured } from "@/app/lib/auth-config";
import { isDatabaseConfigured } from "@/db/index";
import { ensureUserInvitationSentAtColumn } from "@/app/lib/user-invitation-sent";

async function prepareAuthSchema() {
  if (!isDatabaseConfigured()) return;
  try {
    await ensureUserInvitationSentAtColumn();
  } catch (e) {
    console.error("[auth] ensure invitation_sent_at", e);
  }
}

const baseHandler = isBetterAuthConfigured()
  ? toNextJsHandler(getBetterAuth())
  : null;

export async function GET(req: Request) {
  if (!baseHandler) {
    return new Response(JSON.stringify({ error: "Better-Auth non configuré." }), {
      status: 503,
      headers: { "content-type": "application/json" },
    });
  }
  await prepareAuthSchema();
  return baseHandler.GET(req);
}

export async function POST(req: Request) {
  if (!baseHandler) {
    return new Response(JSON.stringify({ error: "Better-Auth non configuré." }), {
      status: 503,
      headers: { "content-type": "application/json" },
    });
  }
  await prepareAuthSchema();
  return baseHandler.POST(req);
}
