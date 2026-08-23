import { NextResponse } from "next/server";
import { listDirectoryMembers } from "@/app/lib/directory-members";
import { requireDomainPlanningDirectoryUsersList } from "@/app/lib/domain-planning-auth";

/** Liste des utilisateurs du directory (paramétrage + affectation de créneaux). */
export async function GET() {
  const gate = await requireDomainPlanningDirectoryUsersList();
  if (!gate.ok) return gate.response;
  try {
    const users = await listDirectoryMembers();
    return NextResponse.json({
      users: users.map((u) => ({
        externalUserId: u.externalUserId,
        email: u.email,
        firstName: u.firstName,
        lastName: u.lastName,
        displayName: u.displayName,
        pending: u.pending,
      })),
    });
  } catch (err: unknown) {
    console.error("GET /domain-planning/directory-users:", err);
    return NextResponse.json(
      { error: err instanceof Error ? err.message : "Impossible de charger les utilisateurs du directory." },
      { status: 500 },
    );
  }
}
