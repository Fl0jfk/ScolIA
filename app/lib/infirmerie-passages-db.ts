import "server-only";

import { and, desc, eq, isNull } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve, infirmeriePassage } from "@/db/schema";
import { sqlPersonNameMatches } from "@/app/lib/person-name-search";
import {
  isInfirmerieSuite,
  type InfirmeriePassageRow,
  type InfirmerieSuite,
} from "@/app/lib/infirmerie-passages-shared";

export {
  INFIRMERIE_SUITES,
  INFIRMERIE_SUITE_LABELS,
  isInfirmerieSuite,
  type InfirmeriePassageRow,
  type InfirmerieSuite,
} from "@/app/lib/infirmerie-passages-shared";

function toIso(d: Date | string | null | undefined): string | null {
  if (!d) return null;
  if (typeof d === "string") return d;
  return d.toISOString();
}

function mapRow(r: {
  id: string;
  eleveId: string;
  eleveNom: string;
  elevePrenom: string;
  eleveClasse: string | null;
  arrivee: Date | string;
  sortie: Date | string | null;
  motifCourt: string;
  suite: string | null;
  soinsNotes: string | null;
  signalVieScolaire: boolean;
  auteurNom: string | null;
}): InfirmeriePassageRow {
  return {
    id: r.id,
    eleveId: r.eleveId,
    eleveNom: r.eleveNom,
    elevePrenom: r.elevePrenom,
    eleveClasse: r.eleveClasse,
    arrivee: toIso(r.arrivee) ?? "",
    sortie: toIso(r.sortie),
    motifCourt: r.motifCourt,
    suite: isInfirmerieSuite(r.suite) ? r.suite : null,
    soinsNotes: r.soinsNotes,
    signalVieScolaire: r.signalVieScolaire,
    auteurNom: r.auteurNom,
  };
}

export async function listInfirmeriePassagesOuverts(
  etablissementId: string,
): Promise<InfirmeriePassageRow[]> {
  const db = getDb();
  const rows = await db
    .select({
      id: infirmeriePassage.id,
      eleveId: infirmeriePassage.eleveId,
      eleveNom: eleve.nom,
      elevePrenom: eleve.prenom,
      eleveClasse: eleve.classe,
      arrivee: infirmeriePassage.arrivee,
      sortie: infirmeriePassage.sortie,
      motifCourt: infirmeriePassage.motifCourt,
      suite: infirmeriePassage.suite,
      soinsNotes: infirmeriePassage.soinsNotes,
      signalVieScolaire: infirmeriePassage.signalVieScolaire,
      auteurNom: infirmeriePassage.auteurNom,
    })
    .from(infirmeriePassage)
    .innerJoin(
      eleve,
      and(eq(eleve.id, infirmeriePassage.eleveId), eq(eleve.etablissementId, etablissementId)),
    )
    .where(
      and(
        eq(infirmeriePassage.etablissementId, etablissementId),
        isNull(infirmeriePassage.sortie),
      ),
    )
    .orderBy(desc(infirmeriePassage.arrivee));

  return rows.map(mapRow);
}

export async function searchElevesForInfirmerie(
  etablissementId: string,
  q: string,
): Promise<Array<{ id: string; nom: string; prenom: string; classe: string | null }>> {
  const db = getDb();
  const needle = q.trim();
  if (needle.length < 2) return [];
  return db
    .select({
      id: eleve.id,
      nom: eleve.nom,
      prenom: eleve.prenom,
      classe: eleve.classe,
    })
    .from(eleve)
    .where(
      and(
        eq(eleve.etablissementId, etablissementId),
        eq(eleve.status, "inscrit"),
        sqlPersonNameMatches({
          nom: eleve.nom,
          prenom: eleve.prenom,
          extras: [eleve.classe],
          query: needle,
        }),
      ),
    )
    .limit(20);
}

export async function openInfirmeriePassage(
  etablissementId: string,
  opts: {
    eleveId: string;
    motifCourt?: string;
    auteurUserId?: string | null;
    auteurNom?: string | null;
  },
): Promise<InfirmeriePassageRow> {
  const db = getDb();
  const eleveId = opts.eleveId.trim();
  if (!eleveId) throw new Error("Élève requis.");

  const [eleveRow] = await db
    .select({
      id: eleve.id,
      nom: eleve.nom,
      prenom: eleve.prenom,
      classe: eleve.classe,
    })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etablissementId), eq(eleve.id, eleveId)))
    .limit(1);
  if (!eleveRow) throw new Error("Élève introuvable.");

  const [open] = await db
    .select({ id: infirmeriePassage.id })
    .from(infirmeriePassage)
    .where(
      and(
        eq(infirmeriePassage.etablissementId, etablissementId),
        eq(infirmeriePassage.eleveId, eleveId),
        isNull(infirmeriePassage.sortie),
      ),
    )
    .limit(1);
  if (open) {
    const err = new Error("Cet élève est déjà à l’infirmerie.");
    (err as Error & { code?: string }).code = "ALREADY_OPEN";
    throw err;
  }

  const [inserted] = await db
    .insert(infirmeriePassage)
    .values({
      etablissementId,
      eleveId,
      arrivee: new Date(),
      motifCourt: (opts.motifCourt ?? "").trim(),
      signalVieScolaire: true,
      auteurUserId: opts.auteurUserId ?? null,
      auteurNom: opts.auteurNom ?? null,
    })
    .returning({
      id: infirmeriePassage.id,
      eleveId: infirmeriePassage.eleveId,
      arrivee: infirmeriePassage.arrivee,
      sortie: infirmeriePassage.sortie,
      motifCourt: infirmeriePassage.motifCourt,
      suite: infirmeriePassage.suite,
      soinsNotes: infirmeriePassage.soinsNotes,
      signalVieScolaire: infirmeriePassage.signalVieScolaire,
      auteurNom: infirmeriePassage.auteurNom,
    });

  return mapRow({
    ...inserted!,
    eleveNom: eleveRow.nom,
    elevePrenom: eleveRow.prenom,
    eleveClasse: eleveRow.classe,
  });
}

/** Clôture avec suite (soin). Upsert par id + etablissement_id. */
export async function closeInfirmeriePassage(
  etablissementId: string,
  passageId: string,
  opts?: { suite?: string; soinsNotes?: string },
): Promise<InfirmeriePassageRow> {
  const db = getDb();
  const id = passageId.trim();
  if (!id) throw new Error("Passage requis.");

  const suiteRaw = (opts?.suite ?? "").trim();
  if (!suiteRaw || !isInfirmerieSuite(suiteRaw)) {
    throw new Error("Suite requise (repos, retour en cours, renvoi famille, urgence…).");
  }

  const [existing] = await db
    .select({
      id: infirmeriePassage.id,
      eleveId: infirmeriePassage.eleveId,
      sortie: infirmeriePassage.sortie,
    })
    .from(infirmeriePassage)
    .where(
      and(eq(infirmeriePassage.etablissementId, etablissementId), eq(infirmeriePassage.id, id)),
    )
    .limit(1);
  if (!existing) throw new Error("Passage introuvable.");
  if (existing.sortie) throw new Error("Passage déjà clos.");

  const now = new Date();
  await db
    .update(infirmeriePassage)
    .set({
      sortie: now,
      suite: suiteRaw,
      soinsNotes: (opts?.soinsNotes ?? "").trim() || null,
      updatedAt: now,
    })
    .where(
      and(eq(infirmeriePassage.etablissementId, etablissementId), eq(infirmeriePassage.id, id)),
    );

  const [row] = await db
    .select({
      id: infirmeriePassage.id,
      eleveId: infirmeriePassage.eleveId,
      eleveNom: eleve.nom,
      elevePrenom: eleve.prenom,
      eleveClasse: eleve.classe,
      arrivee: infirmeriePassage.arrivee,
      sortie: infirmeriePassage.sortie,
      motifCourt: infirmeriePassage.motifCourt,
      suite: infirmeriePassage.suite,
      soinsNotes: infirmeriePassage.soinsNotes,
      signalVieScolaire: infirmeriePassage.signalVieScolaire,
      auteurNom: infirmeriePassage.auteurNom,
    })
    .from(infirmeriePassage)
    .innerJoin(
      eleve,
      and(eq(eleve.id, infirmeriePassage.eleveId), eq(eleve.etablissementId, etablissementId)),
    )
    .where(
      and(eq(infirmeriePassage.etablissementId, etablissementId), eq(infirmeriePassage.id, id)),
    )
    .limit(1);

  return mapRow(row!);
}
