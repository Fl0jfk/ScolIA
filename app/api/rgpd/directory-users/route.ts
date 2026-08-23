import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canAccessRgpdModule } from "@/app/lib/rgpd-access";
import { listDirectoryMembers } from "@/app/lib/directory-members";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";

/** Membres de l'établissement (sélection DPD interne). */
export async function GET() {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = rolesFromUserLike(user);
    if (!canAccessRgpdModule(roles)) {
      return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
    }

    const members = await listDirectoryMembers();
    const users = members
      .filter((m) => m.externalUserId && !m.pending)
      .map((m) => ({
        externalUserId: m.externalUserId,
        email: m.email,
        firstName: m.firstName,
        lastName: m.lastName,
        displayName: m.displayName,
        roles: m.roles,
      }));

    return NextResponse.json({ users });
  } catch (error) {
    console.error("[rgpd/directory-users]", error);
    return NextResponse.json(
      { error: error instanceof Error ? error.message : "Erreur chargement utilisateurs" },
      { status: 500 },
    );
  }
}
