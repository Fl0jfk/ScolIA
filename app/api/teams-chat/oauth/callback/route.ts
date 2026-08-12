import { NextRequest, NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";
import { canUseTeamsChatOverlay } from "@/app/lib/teams-chat/access";
import { isTeamsChatOverlayEnabled } from "@/app/lib/teams-chat/flag";
import { fetchTeamsGraphMe } from "@/app/lib/teams-chat/graph";
import {
  TEAMS_CHAT_OAUTH_STATE_COOKIE,
  exchangeTeamsChatOAuthCode,
} from "@/app/lib/teams-chat/oauth";
import { saveTeamsChatLink } from "@/app/lib/teams-chat/tokens";

async function redirectDash(query: string) {
  const url = await tenantAbsolutePath(`/dashboard?${query}`);
  return NextResponse.redirect(url);
}

/** Callback OAuth — stocke le refresh token Teams de l’utilisateur et redirige. */
export async function GET(req: NextRequest) {
  if (!isTeamsChatOverlayEnabled()) {
    return redirectDash("teamsChat=disabled");
  }

  const gate = await requireAuth();
  if (!gate.ok) {
    return NextResponse.redirect(await tenantAbsolutePath("/sign-in"));
  }

  const user = await safeCurrentUser();
  const roles = intranetRolesFromMetadata(user?.publicMetadata);
  if (!user || !canUseTeamsChatOverlay(roles)) {
    return redirectDash("teamsChat=forbidden");
  }

  const sp = req.nextUrl.searchParams;
  const err = sp.get("error");
  if (err) {
    const desc = (sp.get("error_description") || err).slice(0, 400);
    return redirectDash(`teamsChat=error&detail=${encodeURIComponent(desc)}`);
  }

  const code = sp.get("code")?.trim();
  const state = sp.get("state")?.trim();
  const expected = req.cookies.get(TEAMS_CHAT_OAUTH_STATE_COOKIE)?.value;
  if (!code || !state || !expected || state !== expected) {
    return redirectDash("teamsChat=error&detail=state_invalid");
  }

  try {
    const tokens = await exchangeTeamsChatOAuthCode(code);
    const me = await fetchTeamsGraphMe(tokens.accessToken);
    if (!me.id) throw new Error("Profil Microsoft sans id.");

    await saveTeamsChatLink({
      clerkUserId: gate.ctx.userId,
      refreshToken: tokens.refreshToken,
      microsoftUserId: me.id,
      upn: me.userPrincipalName || me.mail,
      displayName: me.displayName,
      linkedAt: new Date().toISOString(),
    });

    const res = await redirectDash("teamsChat=linked");
    res.cookies.set(TEAMS_CHAT_OAUTH_STATE_COOKIE, "", { path: "/", maxAge: 0 });
    return res;
  } catch (e) {
    const raw = e instanceof Error ? e.message : "oauth_failed";
    return redirectDash(`teamsChat=error&detail=${encodeURIComponent(raw.slice(0, 400))}`);
  }
}
