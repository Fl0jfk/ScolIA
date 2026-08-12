import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canUseTeamsChatOverlay } from "@/app/lib/teams-chat/access";
import { isTeamsChatOverlayEnabled } from "@/app/lib/teams-chat/flag";
import { getTeamsChatAccessContext, TeamsChatUnlinkedError } from "@/app/lib/teams-chat/graph";
import { getTeamsChatOAuthRedirectUri } from "@/app/lib/teams-chat/oauth";
import { loadTeamsChatLink } from "@/app/lib/teams-chat/tokens";
import type { TeamsChatStatus } from "@/app/lib/teams-chat/types";

export async function GET() {
  if (!isTeamsChatOverlayEnabled()) {
    const body: TeamsChatStatus = { enabled: false, allowed: false, linked: false };
    return NextResponse.json(body);
  }

  const gate = await requireAuth();
  if (!gate.ok) {
    const body: TeamsChatStatus = { enabled: true, allowed: false, linked: false };
    return NextResponse.json(body);
  }

  const user = await safeCurrentUser();
  const roles = intranetRolesFromMetadata(user?.publicMetadata);
  if (!user || !canUseTeamsChatOverlay(roles)) {
    const body: TeamsChatStatus = { enabled: true, allowed: false, linked: false };
    return NextResponse.json(body);
  }

  let oauthRedirectUri: string | null = null;
  try {
    oauthRedirectUri = await getTeamsChatOAuthRedirectUri();
  } catch {
    oauthRedirectUri = null;
  }

  const link = await loadTeamsChatLink(gate.ctx.userId);
  if (!link) {
    const body: TeamsChatStatus = {
      enabled: true,
      allowed: true,
      linked: false,
      oauthRedirectUri,
    };
    return NextResponse.json(body);
  }

  try {
    const ctx = await getTeamsChatAccessContext(gate.ctx.userId);
    const body: TeamsChatStatus = {
      enabled: true,
      allowed: true,
      linked: true,
      me: { displayName: ctx.displayName || "Vous", upn: ctx.upn },
      oauthRedirectUri,
    };
    return NextResponse.json(body);
  } catch (e) {
    if (e instanceof TeamsChatUnlinkedError) {
      const body: TeamsChatStatus = {
        enabled: true,
        allowed: true,
        linked: false,
        oauthRedirectUri,
        error: e.message,
      };
      return NextResponse.json(body);
    }
    const body: TeamsChatStatus = {
      enabled: true,
      allowed: true,
      linked: true,
      me: { displayName: link.displayName || "Vous", upn: link.upn },
      oauthRedirectUri,
      error: e instanceof Error ? e.message : "Erreur Microsoft Graph.",
    };
    return NextResponse.json(body);
  }
}
