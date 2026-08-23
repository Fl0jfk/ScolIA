import { NextResponse } from "next/server";
import { getAppSession } from "@/app/lib/app-session";
import { isBetterAuthConfigured } from "@/app/lib/auth-config";

/** Profil client unifié (rôles inclus) — session Better-Auth. */
export async function GET() {
  if (!isBetterAuthConfigured()) {
    return NextResponse.json({ user: null });
  }
  try {
    const session = await getAppSession();
    if (!session) {
      return NextResponse.json({ user: null });
    }
    return NextResponse.json({ user: session.user });
  } catch (error) {
    console.error("[api/auth/me]", error);
    return NextResponse.json({ user: null }, { status: 500 });
  }
}
