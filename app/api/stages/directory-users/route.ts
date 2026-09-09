import { safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";

import { listDirectoryMembers } from "@/app/lib/directory-members";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { canReviewPreconvention, canViewReferentConventions } from "@/app/lib/stage-access";

/** Utilisateurs éligibles comme professeur principal / référent stage. */
export async function GET() {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    if (!canReviewPreconvention(roles) && !canViewReferentConventions(roles)) {
      return NextResponse.json({ error: "Accès réservé." }, { status: 403 });
    }

    const members = await listDirectoryMembers();
    const users = members
      .filter((m) => m.externalUserId && !m.pending)
      .filter(
        (m) =>
          m.roles.includes("professeur") ||
          m.roles.includes("surveillant") ||
          m.roles.includes("cpe") ||
          m.roles.includes("accueil") ||
          m.roles.includes("administratif"),
      )
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
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
