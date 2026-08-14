import { NextResponse } from "next/server";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canManagePersonnel } from "@/app/lib/personnel-types";
import { clearRhDriveLink } from "@/app/lib/rh/graph-rh-drive";


/** Déconnecte le OneDrive RH (supprime le refresh token). */
export async function POST() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  if (!user || !canManagePersonnel(rolesFromUserLike(user))) {
    return NextResponse.json({ error: "Réservé à la RH / comptabilité." }, { status: 403 });
  }

  try {
    await clearRhDriveLink();
    return NextResponse.json({ ok: true });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Déconnexion impossible." },
      { status: 500 },
    );
  }
}
