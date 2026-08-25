import { NextResponse } from "next/server";
import { requireAppUser } from "@/app/lib/app-session";
import { requireFamilleAccess } from "@/app/lib/famille-auth";
import { listFamilleCarnet, signerCarnetEntree } from "@/app/lib/vs-carnet-db";

export async function GET(req: Request) {
  const gate = await requireFamilleAccess();
  if (!gate.ok) return gate.response;

  const url = new URL(req.url);
  const eleveId = url.searchParams.get("eleveId")?.trim();
  const eleveIds = eleveId
    ? gate.ctx.enfants.filter((e) => e.id === eleveId).map((e) => e.id)
    : gate.ctx.enfants.map((e) => e.id);

  if (eleveId && !eleveIds.length) {
    return NextResponse.json(
      { error: "Accès refusé à cet élève.", code: "FAMILLE_ELEVE_FORBIDDEN" },
      { status: 403 },
    );
  }

  const entrees = await listFamilleCarnet(gate.ctx.etablissementId, eleveIds);
  return NextResponse.json({ enfants: gate.ctx.enfants, entrees });
}

export async function POST(req: Request) {
  const gate = await requireFamilleAccess();
  if (!gate.ok) return gate.response;

  const body = (await req.json().catch(() => ({}))) as { id?: string; action?: string };
  if (body.action !== "signer" || !body.id?.trim()) {
    return NextResponse.json({ error: "Action invalide." }, { status: 400 });
  }

  try {
    const appUser = await requireAppUser();
    const userNom = appUser.ok
      ? [appUser.user.firstName, appUser.user.lastName].filter(Boolean).join(" ") ||
        appUser.user.name ||
        appUser.user.email ||
        null
      : gate.ctx.email;
    const row = await signerCarnetEntree(gate.ctx.etablissementId, body.id.trim(), {
      eleveIds: gate.ctx.enfants.map((e) => e.id),
      userId: gate.ctx.authUserId,
      userNom,
    });
    return NextResponse.json({ entree: row });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Signature impossible." },
      { status: 400 },
    );
  }
}
