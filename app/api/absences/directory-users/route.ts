import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { canDeclareAbsenceOnBehalf } from "@/app/lib/absences-types";
import { listDirectoryMembers } from "@/app/lib/directory-members";

/** Annuaire pour déclaration d’absence pour un collègue (administratif, compta, direction). */
export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  if (!canDeclareAbsenceOnBehalf(roles)) {
    return NextResponse.json({ error: "Action non autorisée." }, { status: 403 });
  }

  try {
    const users = await listDirectoryMembers();
    return NextResponse.json({
      users: users
        .filter((u) => u.externalUserId && !u.pending)
        .map((u) => ({
          externalUserId: u.externalUserId,
          email: u.email,
          firstName: u.firstName,
          lastName: u.lastName,
          displayName: u.displayName,
          roles: u.roles,
          pending: u.pending,
        })),
    });
  } catch (err: unknown) {
    console.error("GET /absences/directory-users:", err);
    return NextResponse.json(
      { error: err instanceof Error ? err.message : "Impossible de charger l’annuaire." },
      { status: 500 },
    );
  }
}
