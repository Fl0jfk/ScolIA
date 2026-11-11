import "server-only";

import { NextResponse } from "next/server";
import { isOrgAdminFromAppUser } from "@/app/lib/auth-roles-db";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveSession } from "@/app/lib/intranet-session";
import type { ChannelViewer } from "@/app/lib/channels-access";

/** Garde des routes /api/channels : module « channels » (personnel) + identité de session. */
export async function requireChannelsViewer(): Promise<
  { ok: true; viewer: ChannelViewer } | { ok: false; response: NextResponse }
> {
  const gate = await requireModule("channels");
  if (!gate.ok) return gate;
  const session = await resolveSession();
  if (!session?.userId) {
    return { ok: false, response: NextResponse.json({ error: "Non autorisé.", code: "AUTH_REQUIRED" }, { status: 401 }) };
  }
  const roles = gate.ctx.user.roles;
  const isAdmin = isOrgAdminFromAppUser(gate.ctx.user) || roles.includes("admin") || roles.includes("master");
  return { ok: true, viewer: { userId: session.userId, roles, isAdmin } };
}
