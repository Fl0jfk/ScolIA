import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { findAccueilEnSortieHits } from "@/app/lib/accueil-absences-db";
import {
  formatAccueilEnSortieWarning,
  type AccueilEnSortieHit,
} from "@/app/lib/accueil-absence-occupancy";

/**
 * Lecture occupancy pour l’accueil — bandeau avant déclaration.
 * GET ?eleveId=&startDate=YYYY-MM-DD&endDate=YYYY-MM-DD
 */
export async function GET(req: Request) {
  const gate = await requireModule("accueil-absences");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const url = new URL(req.url);
  const eleveId = url.searchParams.get("eleveId")?.trim() || "";
  const startDate = url.searchParams.get("startDate")?.trim() || "";
  const endDate = url.searchParams.get("endDate")?.trim() || startDate;
  if (!eleveId || !/^\d{4}-\d{2}-\d{2}$/.test(startDate)) {
    return NextResponse.json({ error: "eleveId et startDate requis." }, { status: 400 });
  }
  if (endDate && !/^\d{4}-\d{2}-\d{2}$/.test(endDate)) {
    return NextResponse.json({ error: "endDate invalide." }, { status: 400 });
  }

  try {
    const enSortie: AccueilEnSortieHit[] = await findAccueilEnSortieHits({
      etablissementId: etabId,
      eleveId,
      startDate,
      endDate: endDate || startDate,
    });
    return NextResponse.json({
      eleveId,
      startDate,
      endDate: endDate || startDate,
      enSortie,
      warning: enSortie.length > 0 ? formatAccueilEnSortieWarning(enSortie) : null,
    });
  } catch (err) {
    const message = err instanceof Error ? err.message : "Lecture présence impossible.";
    return NextResponse.json({ error: message }, { status: 400 });
  }
}
