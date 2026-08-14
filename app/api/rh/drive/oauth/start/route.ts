import { randomBytes } from "crypto";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canManagePersonnel } from "@/app/lib/personnel-types";
import {
  RH_OAUTH_STATE_COOKIE,
  buildRhOAuthAuthorizeUrl,
} from "@/app/lib/rh/oauth-rh-drive";


/** Démarre le flux OAuth pour lier le OneDrive de l'attachée RH. */
export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  if (!user || !canManagePersonnel(rolesFromUserLike(user))) {
    return NextResponse.json({ error: "Réservé à la RH / comptabilité." }, { status: 403 });
  }

  try {
    const state = randomBytes(24).toString("hex");
    const url = await buildRhOAuthAuthorizeUrl(state);
    const res = NextResponse.redirect(url);
    res.cookies.set(RH_OAUTH_STATE_COOKIE, state, {
      httpOnly: true,
      sameSite: "lax",
      secure: process.env.NODE_ENV === "production",
      path: "/",
      maxAge: 600,
    });
    return res;
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Impossible de démarrer OAuth RH." },
      { status: 500 },
    );
  }
}
