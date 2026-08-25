import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { requireAppUser } from "@/app/lib/app-session";
import {
  CARNET_CATEGORIES,
  createCarnetEntree,
  listCarnetEntrees,
  searchElevesForCarnet,
} from "@/app/lib/vs-carnet-db";

export async function GET(req: Request) {
  const gate = await requireModule("vs-carnet");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const url = new URL(req.url);
  const q = url.searchParams.get("q")?.trim();
  if (q) {
    const eleves = await searchElevesForCarnet(etabId, q);
    return NextResponse.json({ eleves });
  }

  const categorie = url.searchParams.get("categorie")?.trim() || undefined;
  const nonSignees = url.searchParams.get("nonSignees") === "1";
  const entrees = await listCarnetEntrees(etabId, {
    categorie,
    nonSignees,
    limit: 100,
  });
  return NextResponse.json({ categories: CARNET_CATEGORIES, entrees });
}

export async function POST(req: Request) {
  const gate = await requireModule("vs-carnet");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const appUser = await requireAppUser();
  const body = (await req.json().catch(() => ({}))) as {
    eleveId?: string;
    dateEntree?: string;
    categorie?: string;
    titre?: string;
    corps?: string;
    visibleFamille?: boolean;
  };

  try {
    const createdByNom = appUser.ok
      ? [appUser.user.firstName, appUser.user.lastName].filter(Boolean).join(" ") ||
        appUser.user.name ||
        null
      : null;
    const row = await createCarnetEntree(etabId, {
      eleveId: String(body.eleveId || ""),
      dateEntree: String(body.dateEntree || ""),
      categorie: String(body.categorie || "correspondance"),
      titre: String(body.titre || ""),
      corps: String(body.corps || ""),
      visibleFamille: body.visibleFamille !== false,
      createdByUserId: appUser.ok ? appUser.user.id : null,
      createdByNom,
    });
    return NextResponse.json({ entree: row });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Création impossible." },
      { status: 400 },
    );
  }
}
