import { randomBytes } from "crypto";
import { NextResponse } from "next/server";
import { requireTeamsChatAccess } from "@/app/lib/teams-chat/access";
import {
  TEAMS_CHAT_OAUTH_STATE_COOKIE,
  buildTeamsChatOAuthAuthorizeUrl,
} from "@/app/lib/teams-chat/oauth";

/** Démarre le flux OAuth pour lier le compte Microsoft (A1/A3) de l’utilisateur. */
export async function GET() {
  const gate = await requireTeamsChatAccess();
  if (!gate.ok) return gate.response;

  try {
    const state = randomBytes(24).toString("hex");
    const url = await buildTeamsChatOAuthAuthorizeUrl(state);
    const res = NextResponse.redirect(url);
    res.cookies.set(TEAMS_CHAT_OAUTH_STATE_COOKIE, state, {
      httpOnly: true,
      sameSite: "lax",
      secure: process.env.NODE_ENV === "production",
      path: "/",
      maxAge: 600,
    });
    return res;
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Impossible de démarrer OAuth Teams." },
      { status: 500 },
    );
  }
}
