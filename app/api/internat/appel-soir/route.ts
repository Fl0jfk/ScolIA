import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  getOrOpenAppelSoir,
  setAppelSoirMarque,
  validerAppelSoir,
} from "@/app/lib/internat-appel-db";

export async function GET(req: Request) {
  const gate = await requireModule("internat");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const url = new URL(req.url);
  const date = url.searchParams.get("date")?.trim() || undefined;
  const batimentId = url.searchParams.get("batimentId")?.trim() || undefined;

  try {
    const payload = await getOrOpenAppelSoir(etabId, { date, batimentId });
    return NextResponse.json(payload);
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Chargement impossible." },
      { status: 400 },
    );
  }
}

export async function POST(req: Request) {
  const gate = await requireModule("internat");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const body = (await req.json().catch(() => ({}))) as {
    action?: string;
    appelId?: string;
    eleveId?: string;
    marque?: string;
    note?: string;
    date?: string;
    batimentId?: string | null;
  };

  try {
    if (body.action === "open" || body.action === "get") {
      const payload = await getOrOpenAppelSoir(etabId, {
        date: body.date,
        batimentId: body.batimentId,
      });
      return NextResponse.json(payload);
    }

    if (body.action === "setMarque") {
      const ligne = await setAppelSoirMarque(etabId, {
        appelId: String(body.appelId || ""),
        eleveId: String(body.eleveId || ""),
        marque: String(body.marque || ""),
        note: body.note,
      });
      const payload = await getOrOpenAppelSoir(etabId, {
        date: body.date,
        batimentId: body.batimentId,
      });
      return NextResponse.json({ ligne, ...payload });
    }

    if (body.action === "valider") {
      const payload = await validerAppelSoir(etabId, String(body.appelId || ""));
      return NextResponse.json(payload);
    }

    return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Opération impossible." },
      { status: 400 },
    );
  }
}
