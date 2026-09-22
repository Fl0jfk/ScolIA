import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  createPassage,
  getAlertesCantinePourEleve,
  listPassagesDuJour,
  searchElevesForPassage,
} from "@/app/lib/passages-db";
import {
  createBrouillonsCantineFromPassages,
  getCantineFacturationPeriode,
  getCantineFactuMode,
  setCantineFactuMode,
} from "@/app/lib/passages-facturation-db";
import { isCantineFactuMode } from "@/app/lib/passages-facturation-shared";
import { getPrevisionRepasDuJour } from "@/app/lib/passages-prevision-db";
import { isPassageLieu } from "@/app/lib/passages-shared";
import type { RepasService } from "@/app/lib/passages-prevision-shared";

function parseService(raw: string | null): RepasService | undefined {
  if (raw === "midi" || raw === "soir") return raw;
  return undefined;
}

export async function GET(req: Request) {
  const gate = await requireModule("passages");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const url = new URL(req.url);
  const q = url.searchParams.get("q")?.trim();
  if (q) {
    const eleves = await searchElevesForPassage(etabId, q);
    return NextResponse.json({ eleves });
  }

  const eleveId = url.searchParams.get("eleveId")?.trim();
  if (eleveId && url.searchParams.get("alertes") === "cantine") {
    const alertes = await getAlertesCantinePourEleve(etabId, eleveId);
    return NextResponse.json({ alertes });
  }

  if (url.searchParams.get("prevision") === "1") {
    const date = url.searchParams.get("date")?.trim() || undefined;
    const service = parseService(url.searchParams.get("service"));
    const payload = await getPrevisionRepasDuJour(etabId, { date, service });
    return NextResponse.json(payload);
  }

  if (url.searchParams.get("facturation") === "1") {
    const dateDebut = url.searchParams.get("debut")?.trim() || undefined;
    const dateFin = url.searchParams.get("fin")?.trim() || undefined;
    const modeRaw = url.searchParams.get("mode")?.trim();
    const mode = modeRaw && isCantineFactuMode(modeRaw) ? modeRaw : undefined;
    const savedMode = await getCantineFactuMode(etabId);
    const payload = await getCantineFacturationPeriode(etabId, {
      dateDebut,
      dateFin,
      mode: mode ?? savedMode,
    });
    return NextResponse.json({ ...payload, modeSauve: savedMode });
  }

  const lieuRaw = url.searchParams.get("lieu")?.trim();
  const lieu = lieuRaw && isPassageLieu(lieuRaw) ? lieuRaw : undefined;
  const date = url.searchParams.get("date")?.trim() || undefined;
  const passages = await listPassagesDuJour(etabId, { lieu, date });
  return NextResponse.json({ passages });
}

export async function POST(req: Request) {
  const gate = await requireModule("passages");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const body = (await req.json().catch(() => ({}))) as {
    action?: string;
    eleveId?: string;
    inviteNom?: string;
    sens?: string;
    lieu?: string;
    mode?: string;
    debut?: string;
    fin?: string;
  };

  try {
    if (body.action === "setCantineFactuMode") {
      if (!body.mode || !isCantineFactuMode(body.mode)) {
        return NextResponse.json({ error: "Mode invalide (forfait | reel)." }, { status: 400 });
      }
      const mode = await setCantineFactuMode(etabId, body.mode);
      return NextResponse.json({ mode });
    }

    if (body.action === "createCantineBrouillons") {
      const mode =
        body.mode && isCantineFactuMode(body.mode) ? body.mode : await getCantineFactuMode(etabId);
      const result = await createBrouillonsCantineFromPassages(etabId, {
        dateDebut: body.debut,
        dateFin: body.fin,
        mode,
      });
      return NextResponse.json(result);
    }

    const result = await createPassage(etabId, {
      eleveId: body.eleveId,
      inviteNom: body.inviteNom,
      sens: String(body.sens || ""),
      lieu: String(body.lieu || ""),
      source: "manuel",
    });
    return NextResponse.json({ passage: result, alertesCantine: result.alertesCantine ?? [] });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Enregistrement impossible." },
      { status: 400 },
    );
  }
}
