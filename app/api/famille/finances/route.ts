import { NextResponse } from "next/server";
import { requireFamilleAccess } from "@/app/lib/famille-auth";
import { loadFinancesForEleve } from "@/app/lib/facturation-db";
import { parisDateKey } from "@/app/lib/paris-time";

const HIDDEN_STATUTS = new Set(["brouillon"]);

function isEnRetard(statut: string, dateEcheance: string | null, today: string): boolean {
  if (statut !== "emise" || !dateEcheance) return false;
  return dateEcheance <= today;
}

export async function GET(req: Request) {
  const gate = await requireFamilleAccess();
  if (!gate.ok) return gate.response;

  const url = new URL(req.url);
  const eleveId = url.searchParams.get("eleveId")?.trim();
  const enfants = eleveId
    ? gate.ctx.enfants.filter((e) => e.id === eleveId)
    : gate.ctx.enfants;

  if (eleveId && !enfants.length) {
    return NextResponse.json(
      { error: "Accès refusé à cet élève.", code: "FAMILLE_ELEVE_FORBIDDEN" },
      { status: 403 },
    );
  }

  const today = parisDateKey(new Date());
  const items = [];

  for (const enfant of enfants) {
    const raw = await loadFinancesForEleve(gate.ctx.etablissementId, enfant.id);
    items.push({
      eleve: {
        id: enfant.id,
        nom: enfant.nom,
        prenom: enfant.prenom,
        classe: enfant.classe,
      },
      finances: raw.map((block) => ({
        foyer: block.foyer,
        facturation: block.facturation
          ? {
              acceptePrelevement: Boolean(block.facturation.acceptePrelevement),
              iban: block.facturation.iban,
              rum: block.facturation.rum,
              mandatDate: block.facturation.mandatDate,
            }
          : null,
        factures: block.factures
          .filter((f) => !HIDDEN_STATUTS.has(f.statut))
          .map((f) => ({
            id: f.id,
            numero: f.numero,
            statut: f.statut,
            totalTtc: f.totalTtc,
            dateEmission: f.dateEmission,
            dateEcheance: f.dateEcheance ?? null,
            enRetard: isEnRetard(f.statut, f.dateEcheance ?? null, today),
            hasPdf: Boolean(f.pdfKey),
          })),
      })),
    });
  }

  return NextResponse.json({ enfants: gate.ctx.enfants, items });
}
