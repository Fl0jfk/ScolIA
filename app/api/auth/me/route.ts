import { NextResponse } from "next/server";
import { getAppSession, getEffectiveViewUser } from "@/app/lib/app-session";
import { isBetterAuthConfigured } from "@/app/lib/auth-config";
import { resolveActiveSupervision } from "@/app/lib/supervision";

/** Profil client unifié (rôles inclus) — vue effective (supervision si active). */
export async function GET() {
  if (!isBetterAuthConfigured()) {
    return NextResponse.json({ user: null });
  }
  try {
    const actorSession = await getAppSession();
    if (!actorSession) {
      return NextResponse.json({ user: null });
    }
    const supervision = await resolveActiveSupervision({
      actorUserId: actorSession.user.id,
    });
    const user = await getEffectiveViewUser();
    if (!user) {
      return NextResponse.json({ user: null });
    }
    return NextResponse.json({
      user,
      supervisionActive: Boolean(supervision),
    });
  } catch (error) {
    console.error("[api/auth/me]", error);
    return NextResponse.json({ user: null }, { status: 500 });
  }
}
