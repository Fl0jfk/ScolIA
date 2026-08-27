import { NextResponse } from "next/server";
import { listDirectoryMembers } from "@/app/lib/directory-members";
import { requireModule } from "@/app/lib/intranet-auth";

/**
 * Annuaire pour choisir les accompagnateurs d’une sortie / séjour.
 * Accessible à tout utilisateur du module Voyages (pas seulement admin).
 */
export async function GET() {
  const gate = await requireModule("travels");
  if (!gate.ok) return gate.response;

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
        })),
    });
  } catch (err: unknown) {
    console.error("[travels/directory-escorts]", err);
    return NextResponse.json(
      { error: err instanceof Error ? err.message : "Impossible de charger l’annuaire." },
      { status: 500 },
    );
  }
}
