import { NextResponse } from "next/server";
import { and, desc, eq } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { vsAbsenceEleve } from "@/db/schema";
import { requireEleveAccess } from "@/app/lib/eleve-auth";
import { listMoyennesForEleve } from "@/app/lib/notes-saisie-db";

/** Notes + absences récentes — surface élève app. */
export async function GET() {
  const gate = await requireEleveAccess();
  if (!gate.ok) return gate.response;

  const { etablissementId, eleve } = gate.ctx;
  const notes = await listMoyennesForEleve(etablissementId, eleve.id).catch(() => []);

  let absences: Array<{
    id: string;
    type: string;
    statut: string;
    dateDebut: string;
    dateFin: string | null;
  }> = [];

  if (isDatabaseConfigured()) {
    const db = getDb();
    const rows = await db
      .select({
        id: vsAbsenceEleve.id,
        type: vsAbsenceEleve.type,
        statut: vsAbsenceEleve.statut,
        dateDebut: vsAbsenceEleve.dateDebut,
        dateFin: vsAbsenceEleve.dateFin,
      })
      .from(vsAbsenceEleve)
      .where(
        and(
          eq(vsAbsenceEleve.etablissementId, etablissementId),
          eq(vsAbsenceEleve.eleveId, eleve.id),
        ),
      )
      .orderBy(desc(vsAbsenceEleve.dateDebut))
      .limit(40);
    absences = rows.map((r) => ({
      id: r.id,
      type: r.type,
      statut: r.statut,
      dateDebut: String(r.dateDebut),
      dateFin: r.dateFin ? String(r.dateFin) : null,
    }));
  }

  return NextResponse.json({
    channel: "eleve",
    eleveId: eleve.id,
    notes,
    absences,
  });
}
