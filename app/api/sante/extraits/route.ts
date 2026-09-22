import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  SANTE_EXTRAIT_PORTEES,
  SANTE_EXTRAIT_PORTEE_LABELS,
  createSanteExtrait,
  desactiverSanteExtrait,
  listSanteExtraits,
  searchElevesForSanteExtrait,
} from "@/app/lib/sante-extraits-db";

export async function GET(req: Request) {
  const gate = await requireModule("sante");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const url = new URL(req.url);
  const q = url.searchParams.get("q")?.trim();
  if (q) {
    const eleves = await searchElevesForSanteExtrait(etabId, q);
    return NextResponse.json({ eleves });
  }

  const portee = url.searchParams.get("portee")?.trim() || undefined;
  const extraits = await listSanteExtraits(etabId, { portee });
  return NextResponse.json({
    portees: SANTE_EXTRAIT_PORTEES.map((p) => ({
      id: p,
      label: SANTE_EXTRAIT_PORTEE_LABELS[p],
    })),
    extraits,
  });
}

export async function POST(req: Request) {
  const gate = await requireModule("sante");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const body = (await req.json().catch(() => ({}))) as {
    eleveId?: string;
    portee?: string;
    libelle?: string;
  };

  try {
    const extrait = await createSanteExtrait(etabId, {
      eleveId: String(body.eleveId || ""),
      portee: String(body.portee || ""),
      libelle: String(body.libelle || ""),
    });
    return NextResponse.json({ extrait });
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
    await desactiverSanteExtrait(etabId, String(body.id || ""));
    return NextResponse.json({ ok: true });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Désactivation impossible." },
      { status: 400 },
    );
  }
}
