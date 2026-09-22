import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { requireAppUser } from "@/app/lib/app-session";
import { searchElevesForInfirmerie } from "@/app/lib/infirmerie-passages-db";
import {
  createSanteMedicamentPrise,
  listSanteMedicamentPrises,
} from "@/app/lib/sante-medicaments-db";

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
  const prises = await listSanteMedicamentPrises(etabId, { eleveId });
  return NextResponse.json({ prises });
}

export async function POST(req: Request) {
  const gate = await requireModule("sante");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const appUser = await requireAppUser();
  const body = (await req.json().catch(() => ({}))) as {
    eleveId?: string;
    medicament?: string;
    dose?: string;
    prisAt?: string;
    notes?: string;
  };

  try {
    const auteurNom = appUser.ok
      ? [appUser.user.firstName, appUser.user.lastName].filter(Boolean).join(" ") ||
        appUser.user.name ||
        null
      : null;
    const prise = await createSanteMedicamentPrise(etabId, {
      eleveId: String(body.eleveId || ""),
      medicament: String(body.medicament || ""),
      dose: body.dose,
      prisAt: body.prisAt,
      notes: body.notes,
      auteurUserId: appUser.ok ? appUser.user.id : null,
      auteurNom,
    });
    return NextResponse.json({ prise });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Enregistrement impossible." },
      { status: 400 },
    );
  }
}
