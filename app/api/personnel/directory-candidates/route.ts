import { safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { NextResponse } from "next/server";

import { requireAuth } from "@/app/lib/intranet-auth";
import { listRhDirectoryCandidates } from "@/app/lib/personnel-directory";
import { getPersonnelIndex } from "@/app/lib/personnel-storage";
import { canAccessPersonnelModule } from "@/app/lib/personnel-types";


export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  if (!canAccessPersonnelModule(roles)) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
  }

  try {
    const index = await getPersonnelIndex();
    const candidates = await listRhDirectoryCandidates(index);
    return NextResponse.json({ candidates });
  } catch (e) {
    console.error("[personnel/directory-candidates]", e);
    return NextResponse.json({ error: "Impossible de charger les utilisateurs du directory." }, { status: 500 });
  }
}
