import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canAccessRgpdModule } from "@/app/lib/rgpd-access";
import { listClerkMembers } from "@/app/lib/clerk-users";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";

/** Membres Clerk de l'établissement (sélection DPD interne). */
export async function GET() {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = rolesFromUserLike(user);
    if (!canAccessRgpdModule(roles)) {
      return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
    }

    const members = await listClerkMembers();
    const users = members
      .filter((m) => m.clerkUserId && !m.pending)
      .map((m) => ({
        clerkUserId: m.clerkUserId,
        email: m.email,
        firstName: m.firstName,
        lastName: m.lastName,
        displayName: m.displayName,
        roles: m.roles,
      }));

    return NextResponse.json({ users });
  } catch (error) {
    console.error("[rgpd/clerk-users]", error);
    return NextResponse.json(
      { error: error instanceof Error ? error.message : "Erreur chargement utilisateurs" },
      { status: 500 },
    );
  }
}
