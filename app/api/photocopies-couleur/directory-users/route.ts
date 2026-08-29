import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { canDeclarePhotocopiesOnBehalf } from "@/app/lib/photocopies-couleur-access";
import { listDirectoryMembers } from "@/app/lib/directory-members";

/** Annuaire enseignants pour dépôt d'une demande photocopies pour un collègue. */
export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  if (!canDeclarePhotocopiesOnBehalf(roles)) {
    return NextResponse.json({ error: "Action non autorisée." }, { status: 403 });
  }

  try {
    const users = await listDirectoryMembers();
    return NextResponse.json({
      users: users
        .filter((u) => u.externalUserId && !u.pending)
        .filter((u) => Array.isArray(u.roles) && u.roles.includes("professeur"))
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
    console.error("GET /photocopies-couleur/directory-users:", err);
    return NextResponse.json(
      { error: err instanceof Error ? err.message : "Impossible de charger l'annuaire." },
      { status: 500 },
    );
  }
}
