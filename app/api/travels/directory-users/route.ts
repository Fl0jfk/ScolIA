import { NextResponse } from "next/server";
import { listDirectoryMembers } from "@/app/lib/directory-members";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canReassignTravelsOwner } from "@/app/lib/travels-roles";

/** Liste directory pour affecter un voyage à un enseignant (administratif uniquement). */
export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  if (!canReassignTravelsOwner(user)) {
    return NextResponse.json({ error: "Réservé à l'administratif." }, { status: 403 });
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
        })),
    });
  } catch (err: unknown) {
    console.error("[travels/directory-users]", err);
    return NextResponse.json(
      { error: err instanceof Error ? err.message : "Impossible de charger les utilisateurs." },
      { status: 500 },
    );
  }
}
