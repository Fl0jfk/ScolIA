import { NextResponse } from "next/server";
import { listDirectoryMembers } from "@/app/lib/directory-members";
import { requireProfRoomModuleAdmin } from "@/app/lib/prof-room-auth";

/** Liste des utilisateurs du directory pour le paramétrage des administrateurs du module. */
export async function GET() {
  const gate = await requireProfRoomModuleAdmin();
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
    console.error("GET /reservation-rooms/directory-users:", err);
    return NextResponse.json(
      { error: err instanceof Error ? err.message : "Impossible de charger les utilisateurs du directory." },
      { status: 500 },
    );
  }
}
