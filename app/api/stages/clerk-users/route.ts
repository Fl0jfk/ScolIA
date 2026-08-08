import { safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";

import { listClerkMembers } from "@/app/lib/clerk-users";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { canReviewPreconvention } from "@/app/lib/stage-access";

/** Utilisateurs Clerk éligibles comme professeur référent (rôle professeur). */
export async function GET() {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    if (!canReviewPreconvention(roles)) {
      return NextResponse.json({ error: "Réservé à l'administratif." }, { status: 403 });
    }

    const members = await listClerkMembers();
    const users = members
      .filter((m) => m.clerkUserId && !m.pending)
      .filter(
        (m) =>
          m.roles.includes("professeur") ||
          m.roles.includes("education") ||
          m.roles.includes("cpe"),
      )
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
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
