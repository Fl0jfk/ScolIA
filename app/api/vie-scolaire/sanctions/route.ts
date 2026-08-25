import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { requireAppUser } from "@/app/lib/app-session";
import {
  annulerSanction,
  createSanction,
  listSanctionTypes,
  listSanctions,
  searchElevesForSanction,
} from "@/app/lib/vs-sanctions-db";

export async function GET(req: Request) {
  const gate = await requireModule("vs-sanctions");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const url = new URL(req.url);
  const q = url.searchParams.get("q")?.trim();
  if (q) {
    const eleves = await searchElevesForSanction(etabId, q);
    return NextResponse.json({ eleves });
  }

  const [types, sanctions] = await Promise.all([
    listSanctionTypes(etabId),
    listSanctions(etabId, { statut: "active", limit: 100 }),
  ]);
  return NextResponse.json({ types, sanctions });
}

export async function POST(req: Request) {
  const gate = await requireModule("vs-sanctions");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const appUser = await requireAppUser();
  const body = (await req.json().catch(() => ({}))) as {
    eleveId?: string;
    typeId?: string;
    dateSanction?: string;
    motif?: string;
  };

  try {
    const createdByNom = appUser.ok
      ? [appUser.user.firstName, appUser.user.lastName].filter(Boolean).join(" ") ||
        appUser.user.name ||
        null
      : null;
    const row = await createSanction(etabId, {
      eleveId: String(body.eleveId || ""),
      typeId: String(body.typeId || ""),
      dateSanction: String(body.dateSanction || ""),
      motif: body.motif,
      createdByUserId: appUser.ok ? appUser.user.id : null,
      createdByNom,
    });
    return NextResponse.json({ sanction: row });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Création impossible." },
      { status: 400 },
    );
  }
}

export async function PATCH(req: Request) {
  const gate = await requireModule("vs-sanctions");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const body = (await req.json().catch(() => ({}))) as { id?: string; action?: string };
  if (body.action !== "annuler" || !body.id?.trim()) {
    return NextResponse.json({ error: "action=annuler et id requis." }, { status: 400 });
  }

  const row = await annulerSanction(etabId, body.id.trim());
  if (!row) return NextResponse.json({ error: "Sanction introuvable." }, { status: 404 });
  return NextResponse.json({ sanction: row });
}
