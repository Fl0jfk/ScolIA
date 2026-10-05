import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { requireAppUser } from "@/app/lib/app-session";
import { searchElevesForInfirmerie } from "@/app/lib/infirmerie-passages-db";
import {
  createSanteInaptitudeEps,
  desactiverSanteInaptitudeEps,
  listSanteInaptitudesEps,
} from "@/app/lib/sante-inaptitudes-db";

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

  const eleveId = url.searchParams.get("eleveId")?.trim() || undefined;
  const inaptitudes = await listSanteInaptitudesEps(etabId, { eleveId });
  return NextResponse.json({ inaptitudes });
}

export async function POST(req: Request) {
  const gate = await requireModule("sante");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const appUser = await requireAppUser();
  const body = (await req.json().catch(() => ({}))) as {
    eleveId?: string;
    dateDebut?: string;
    dateFin?: string | null;
    motif?: string;
    libelleExtrait?: string;
  };

  try {
    const auteurNom = appUser.ok
      ? [appUser.user.firstName, appUser.user.lastName].filter(Boolean).join(" ") ||
        appUser.user.name ||
        null
      : null;
    const inaptitude = await createSanteInaptitudeEps(etabId, {
      eleveId: String(body.eleveId || ""),
      dateDebut: String(body.dateDebut || ""),
      dateFin: body.dateFin,
      motif: body.motif,
      libelleExtrait: String(body.libelleExtrait || ""),
      auteurUserId: appUser.ok ? appUser.user.id : null,
      auteurNom,
    });
    return NextResponse.json({ inaptitude });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Création impossible." },
      { status: 400 },
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
  if (body.action !== "desactiver") {
    return NextResponse.json({ error: "Action inconnue (desactiver)." }, { status: 400 });
  }

  try {
    await desactiverSanteInaptitudeEps(etabId, String(body.id || ""));
    return NextResponse.json({ ok: true });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Désactivation impossible." },
      { status: 400 },
    );
  }
}
