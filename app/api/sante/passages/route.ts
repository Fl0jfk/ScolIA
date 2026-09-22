import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { requireAppUser } from "@/app/lib/app-session";
import {
  closeInfirmeriePassage,
  listInfirmeriePassagesOuverts,
  openInfirmeriePassage,
  searchElevesForInfirmerie,
} from "@/app/lib/infirmerie-passages-db";

export async function GET(req: Request) {
  const gate = await requireModule("sante");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const url = new URL(req.url);
  const q = url.searchParams.get("q")?.trim();
  if (q) {
    const eleves = await searchElevesForInfirmerie(etabId, q);
    return NextResponse.json({ eleves });
  }

  const passages = await listInfirmeriePassagesOuverts(etabId);
  return NextResponse.json({ passages });
}

export async function POST(req: Request) {
  const gate = await requireModule("sante");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const appUser = await requireAppUser();
  const body = (await req.json().catch(() => ({}))) as {
    eleveId?: string;
    motifCourt?: string;
  };

  try {
    const auteurNom = appUser.ok
      ? [appUser.user.firstName, appUser.user.lastName].filter(Boolean).join(" ") ||
        appUser.user.name ||
        null
      : null;
    const passage = await openInfirmeriePassage(etabId, {
      eleveId: String(body.eleveId || ""),
      motifCourt: body.motifCourt,
      auteurUserId: appUser.ok ? appUser.user.id : null,
      auteurNom,
    });
    return NextResponse.json({ passage });
  } catch (e) {
    const code = (e as { code?: string })?.code;
    const status = code === "ALREADY_OPEN" ? 409 : 400;
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Ouverture impossible.", code },
      { status },
    );
  }
}

export async function PATCH(req: Request) {
  const gate = await requireModule("sante");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const body = (await req.json().catch(() => ({}))) as {
    id?: string;
    action?: string;
  };
  if (body.action !== "close") {
    return NextResponse.json({ error: "Action inconnue (close)." }, { status: 400 });
  }

  try {
    const passage = await closeInfirmeriePassage(etabId, String(body.id || ""));
    return NextResponse.json({ passage });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Clôture impossible." },
      { status: 400 },
    );
  }
}
