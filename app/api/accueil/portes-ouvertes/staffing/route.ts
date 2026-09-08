import { NextResponse } from "next/server";
import { z } from "zod";
import { isOrgAdminFromAppUser } from "@/app/lib/auth-roles-db";
import { requireModule } from "@/app/lib/intranet-auth";
import { canManagePortesOuvertesParametrage } from "@/app/lib/portes-ouvertes-access";
import {
  addPortesOuvertesStaff,
  deletePortesOuvertesStaff,
  listPortesOuvertesStaff,
} from "@/app/lib/portes-ouvertes-db";

const AddSchema = z.object({
  slotId: z.string().min(1),
  role: z.enum(["ambassadeur", "enseignant", "personnel"]),
  refId: z.string().min(1).max(120),
  displayName: z.string().min(1).max(200),
  meta: z.record(z.string(), z.string()).optional(),
});

function requireParametrage(
  gate: Extract<Awaited<ReturnType<typeof requireModule>>, { ok: true }>,
): NextResponse | null {
  if (canManagePortesOuvertesParametrage(gate.ctx.user.roles, isOrgAdminFromAppUser(gate.ctx.user))) {
    return null;
  }
  return NextResponse.json(
    { error: "Paramétrage réservé à la direction et aux admins." },
    { status: 403 },
  );
}

export async function GET(req: Request) {
  const gate = await requireModule("accueil-portes-ouvertes");
  if (!gate.ok) return gate.response;
  const slotId = new URL(req.url).searchParams.get("slotId") || undefined;
  const staff = await listPortesOuvertesStaff(undefined, slotId || undefined);
  return NextResponse.json({ staff });
}

export async function POST(req: Request) {
  const gate = await requireModule("accueil-portes-ouvertes");
  if (!gate.ok) return gate.response;
  const denied = requireParametrage(gate);
  if (denied) return denied;
  const parsed = AddSchema.safeParse(await req.json().catch(() => null));
  if (!parsed.success) {
    return NextResponse.json({ error: "Données staffing invalides." }, { status: 400 });
  }
  const row = await addPortesOuvertesStaff(parsed.data);
  return NextResponse.json({ success: true, staff: row });
}

export async function DELETE(req: Request) {
  const gate = await requireModule("accueil-portes-ouvertes");
  if (!gate.ok) return gate.response;
  const denied = requireParametrage(gate);
  if (denied) return denied;
  const id = new URL(req.url).searchParams.get("id")?.trim();
  if (!id) return NextResponse.json({ error: "id requis." }, { status: 400 });
  const ok = await deletePortesOuvertesStaff(id);
  if (!ok) return NextResponse.json({ error: "Introuvable." }, { status: 404 });
  return NextResponse.json({ success: true });
}
