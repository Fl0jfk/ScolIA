import { NextResponse } from "next/server";
import { and, eq } from "drizzle-orm";
import { requireAnyModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { getDb } from "@/db/index";
import { eleve } from "@/db/schema";
import { getPresenceJour } from "@/app/lib/occupancy";
import { parisDateKey } from "@/app/lib/paris-time";
import { listElevesForClasse } from "@/app/lib/vs-absences-db";
import {
  buildFeuilleDuJourRow,
  sortFeuilleDuJourRows,
  summarizeFeuilleDuJour,
  type FeuilleDuJourRow,
} from "@/app/lib/feuille-du-jour";
import { sqlPersonNameMatches } from "@/app/lib/person-name-search";

/**
 * Feuille du jour — lecture occupancy (« où est X ? »).
 * GET ?date=&classe= | ?eleveId= | ?q=
 * Zéro écriture.
 */
export async function GET(req: Request) {
  const gate = await requireAnyModule(["vs-appels", "accueil-absences"]);
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const url = new URL(req.url);
  const date = url.searchParams.get("date")?.trim() || parisDateKey(new Date());
  if (!/^\d{4}-\d{2}-\d{2}$/.test(date)) {
    return NextResponse.json({ error: "date invalide (YYYY-MM-DD)." }, { status: 400 });
  }
  const classe = url.searchParams.get("classe")?.trim() || "";
  const eleveId = url.searchParams.get("eleveId")?.trim() || "";
  const q = url.searchParams.get("q")?.trim() || "";

  if (!classe && !eleveId && q.length < 2) {
    return NextResponse.json(
      {
        error: "Indiquez une classe, un eleveId, ou une recherche (q, 2 lettres min).",
        code: "MISSING_TARGET",
      },
      { status: 400 },
    );
  }

  try {
    const db = getDb();
    type EleveLite = { id: string; nom: string; prenom: string; classe: string | null };
    let eleves: EleveLite[] = [];

    if (eleveId) {
      const [row] = await db
        .select({
          id: eleve.id,
          nom: eleve.nom,
          prenom: eleve.prenom,
          classe: eleve.classe,
        })
        .from(eleve)
        .where(and(eq(eleve.etablissementId, etabId), eq(eleve.id, eleveId)))
        .limit(1);
      if (!row) return NextResponse.json({ error: "Élève introuvable." }, { status: 404 });
      eleves = [row];
    } else if (classe) {
      const listed = await listElevesForClasse(etabId, classe);
      eleves = listed.map((e) => ({
        id: e.id,
        nom: e.nom,
        prenom: e.prenom,
        classe: e.classe ?? classe,
      }));
    } else {
      const nameSql = sqlPersonNameMatches({
        nom: eleve.nom,
        prenom: eleve.prenom,
        extras: [eleve.classe],
        query: q,
      });
      const rows = await db
        .select({
          id: eleve.id,
          nom: eleve.nom,
          prenom: eleve.prenom,
          classe: eleve.classe,
        })
        .from(eleve)
        .where(and(eq(eleve.etablissementId, etabId), eq(eleve.status, "inscrit"), nameSql))
        .limit(40);
      eleves = rows;
    }

    if (eleves.length === 0) {
      return NextResponse.json({
        date,
        classe: classe || null,
        query: q || null,
        rows: [] as FeuilleDuJourRow[],
        summary: summarizeFeuilleDuJour([]),
        coverage: "complete" as const,
      });
    }

    const presence = await getPresenceJour({
      etablissementId: etabId,
      date,
      eleveIds: eleves.map((e) => e.id),
    });
    const factById = new Map(presence.facts.map((f) => [f.eleveId, f]));

    const rows = sortFeuilleDuJourRows(
      eleves.map((e) => {
        const fact = factById.get(e.id) ?? {
          eleveId: e.id,
          date,
          tag: "inconnu" as const,
          source: "none",
          coverage: "partial" as const,
          confidenceTag: "unknown" as const,
        };
        return buildFeuilleDuJourRow({
          eleveId: e.id,
          nom: e.nom,
          prenom: e.prenom,
          classe: e.classe,
          fact,
        });
      }),
    );

    return NextResponse.json({
      date,
      classe: classe || null,
      query: q || null,
      rows,
      summary: summarizeFeuilleDuJour(rows),
      coverage: presence.coverage,
      unmatchedParticipants: presence.unmatchedParticipants,
    });
  } catch (err) {
    console.error("[presence-jour]", err);
    const message = err instanceof Error ? err.message : "Lecture présence impossible.";
    return NextResponse.json({ error: message }, { status: 500 });
  }
}
