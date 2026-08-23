import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import {
  canSignAwardAsDirectionWithRoles,
  canViewAward,
  requiredDirectionRoleForAward,
} from "@/app/lib/certificates-auth";
import { getDirectoryUserRoles } from "@/app/lib/directory-members";
import { loadAward, loadProgram, saveAward } from "@/app/lib/certificates-storage";
import { signAwardAsDirection } from "@/app/lib/certificates-workflow";
import { safeCurrentUser } from "@/app/lib/intranet-session";

type Params = { params: Promise<{ id: string }> };

export async function POST(_req: Request, { params }: Params) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const user = await safeCurrentUser();
  const { id } = await params;
  const award = await loadAward(id);
  if (!award) return NextResponse.json({ error: "Fiche introuvable." }, { status: 404 });
  const program = await loadProgram(award.programId);
  if (!program) return NextResponse.json({ error: "Parcours introuvable." }, { status: 404 });
  if (!canViewAward(award, program, gate.ctx.userId, user)) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }

  const roles = await getDirectoryUserRoles(gate.ctx.userId);
  const requiredRole = requiredDirectionRoleForAward(award);
  if (!roles.includes(requiredRole)) {
    return NextResponse.json(
      {
        error: `Réservé aux comptes avec le rôle « ${requiredRole} » dans le directory.`,
        code: "DIRECTION_ROLE_REQUIRED",
        requiredRole,
      },
      { status: 403 },
    );
  }

  if (!canSignAwardAsDirectionWithRoles(roles, award)) {
    return NextResponse.json(
      { error: "Cette fiche n'est pas prête pour la signature direction." },
      { status: 403 },
    );
  }

  const result = await signAwardAsDirection(award, {
    id: user?.id,
    fullName: user?.fullName,
    firstName: user?.firstName,
    lastName: user?.lastName,
    publicMetadata: user?.publicMetadata as Record<string, unknown> | null,
  });
  if ("error" in result) return NextResponse.json({ error: result.error }, { status: 403 });
  await saveAward(result);
  return NextResponse.json({ award: result });
}
