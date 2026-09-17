import { randomBytes } from "crypto";
import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canManageRdvInscription } from "@/app/lib/rdv-inscription-access";
import {
  RDV_INSCRIPTION_OAUTH_STATE_COOKIE,
  buildRdvInscriptionOAuthAuthorizeUrl,
} from "@/app/lib/rdv-inscription-oauth";

/** Démarre le flux OAuth Google Calendar (compte technique). */
export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  if (!user || !canManageRdvInscription(rolesFromUserLike(user))) {
    return NextResponse.json({ error: "Réservé à la direction / administratif." }, { status: 403 });
  }

  try {
    const state = randomBytes(24).toString("hex");
    const url = await buildRdvInscriptionOAuthAuthorizeUrl(state);
    const res = NextResponse.redirect(url);
    res.cookies.set(RDV_INSCRIPTION_OAUTH_STATE_COOKIE, state, {
      httpOnly: true,
      sameSite: "lax",
      secure: process.env.NODE_ENV === "production",
      path: "/",
      maxAge: 600,
    });
    return res;
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Impossible de démarrer OAuth Google." },
      { status: 500 },
    );
  }
}
