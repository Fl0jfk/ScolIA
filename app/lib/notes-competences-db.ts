import "server-only";

import { and, asc, eq, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  eleve,
  groupePedagogiqueMembre,
  noteCompetenceDomaine,
  noteCompetenceItem,
  noteCompetenceValeur,
  noteMatiere,
} from "@/db/schema";

export const COMPETENCE_NIVEAUX = [
  { code: "1", label: "Insuffisant" },
  { code: "2", label: "Fragile" },
  { code: "3", label: "Satisfaisant" },
  { code: "4", label: "Très bon" },
] as const;

export type CompetenceNiveauCode = (typeof COMPETENCE_NIVEAUX)[number]["code"];

export function normalizeCompetenceNiveau(raw: string): CompetenceNiveauCode | null {
  const v = raw.trim();
  if (v === "1" || v === "2" || v === "3" || v === "4") return v;
  const lower = v.toLowerCase();
  if (lower.startsWith("insuff")) return "1";
  if (lower.startsWith("frag")) return "2";
  if (lower.startsWith("sat")) return "3";
  if (lower.includes("très") || lower.includes("tres") || lower.includes("bon")) return "4";
  return null;
}

export async function listCompetenceDomaines(etablissementId: string) {
  const db = getDb();
  return db
    .select()
    .from(noteCompetenceDomaine)
    .where(eq(noteCompetenceDomaine.etablissementId, etablissementId))
    .orderBy(asc(noteCompetenceDomaine.ordre), asc(noteCompetenceDomaine.code));
}

export async function listCompetenceItems(etablissementId: string, domaineId?: string) {
  const db = getDb();
  const clauses = [eq(noteCompetenceItem.etablissementId, etablissementId)];
  if (domaineId) clauses.push(eq(noteCompetenceItem.domaineId, domaineId));

  return db
    .select({
      id: noteCompetenceItem.id,
      domaineId: noteCompetenceItem.domaineId,
      code: noteCompetenceItem.code,
      libelle: noteCompetenceItem.libelle,
      ordre: noteCompetenceItem.ordre,
      actif: noteCompetenceItem.actif,
      matiereId: noteCompetenceItem.matiereId,
      matiereLibelle: noteMatiere.libelle,
    })
    .from(noteCompetenceItem)
    .leftJoin(noteMatiere, eq(noteCompetenceItem.matiereId, noteMatiere.id))
    .where(and(...clauses))
    .orderBy(asc(noteCompetenceItem.ordre), asc(noteCompetenceItem.code));
}

export async function upsertCompetenceDomaine(
  etablissementId: string,
  input: { code: string; libelle: string; cycle?: string; ordre?: number },
) {
  const db = getDb();
  const code = input.code.trim().toUpperCase();
  const libelle = input.libelle.trim();
  if (!code || !libelle) throw new Error("Code et libellé obligatoires.");

  const [row] = await db
    .insert(noteCompetenceDomaine)
    .values({
      etablissementId,
      code,
      libelle,
      cycle: input.cycle?.trim() || "college",
      ordre: input.ordre ?? 1,
    })
    .onConflictDoUpdate({
      target: [noteCompetenceDomaine.etablissementId, noteCompetenceDomaine.code],
      set: {
        libelle,
        cycle: input.cycle?.trim() || "college",
        ordre: input.ordre ?? 1,
        updatedAt: new Date(),
      },
    })
    .returning();
  return row;
}

export async function upsertCompetenceItem(
  etablissementId: string,
  input: {
    domaineId: string;
    code: string;
    libelle: string;
    matiereId?: string | null;
    ordre?: number;
  },
) {
  const db = getDb();
  const code = input.code.trim().toUpperCase();
  const libelle = input.libelle.trim();
  if (!input.domaineId || !code || !libelle) throw new Error("Domaine, code et libellé obligatoires.");

  const [row] = await db
    .insert(noteCompetenceItem)
    .values({
      etablissementId,
      domaineId: input.domaineId,
      code,
      libelle,
      matiereId: input.matiereId || null,
      ordre: input.ordre ?? 1,
    })
    .onConflictDoUpdate({
      target: [
        noteCompetenceItem.etablissementId,
        noteCompetenceItem.domaineId,
        noteCompetenceItem.code,
      ],
      set: {
        libelle,
        matiereId: input.matiereId || null,
        ordre: input.ordre ?? 1,
        updatedAt: new Date(),
      },
    })
    .returning();
  return row;
}

export async function seedCompetencesCollegeDefaults(etablissementId: string) {
  const existing = await listCompetenceDomaines(etablissementId);
  if (existing.length > 0) return { seeded: false, domaines: existing.length };

  const domaines = [
    {
      code: "FR-LIT",
      libelle: "Français — Compréhension et expression",
      items: [
        { code: "FR-LECT", libelle: "Lire et comprendre des textes variés" },
        { code: "FR-ECR", libelle: "Écrire avec clarté et correction" },
        { code: "FR-ORL", libelle: "Participer à un échange oral structuré" },
      ],
    },
    {
      code: "MATH-RES",
      libelle: "Mathématiques — Résolution de problèmes",
      items: [
        { code: "MATH-CALC", libelle: "Calculer avec des nombres relatifs et fractions" },
        { code: "MATH-MOD", libelle: "Modéliser une situation par un calcul ou un graphique" },
        { code: "MATH-RAIS", libelle: "Raisonner et justifier une démarche" },
      ],
    },
    {
      code: "HG-CT",
      libelle: "Histoire-Géo — Culture et citoyenneté",
      items: [
        { code: "HG-ANA", libelle: "Analyser un document historique ou géographique" },
        { code: "HG-ARG", libelle: "Construire un raisonnement argumenté" },
      ],
    },
  ];

  let itemCount = 0;
  for (let i = 0; i < domaines.length; i += 1) {
    const d = domaines[i];
    const domaine = await upsertCompetenceDomaine(etablissementId, {
      code: d.code,
      libelle: d.libelle,
      cycle: "college",
      ordre: i + 1,
    });
    for (let j = 0; j < d.items.length; j += 1) {
      await upsertCompetenceItem(etablissementId, {
        domaineId: domaine.id,
        code: d.items[j].code,
        libelle: d.items[j].libelle,
        ordre: j + 1,
      });
      itemCount += 1;
    }
  }

  return { seeded: true, domaines: domaines.length, items: itemCount };
}

export async function listCompetenceValeurs(
  etablissementId: string,
  opts: { domaineId: string; periodeId: string; classe?: string; groupeId?: string },
) {
  const db = getDb();
  const trimmedClasse = opts.classe?.trim() || "";
  const groupeId = opts.groupeId?.trim() || "";

  const eleveScope = groupeId
    ? sql`exists (
      select 1 from ${groupePedagogiqueMembre}
      where ${groupePedagogiqueMembre.eleveId} = ${eleve.id}
      and ${groupePedagogiqueMembre.etablissementId} = ${etablissementId}
      and ${groupePedagogiqueMembre.groupeId} = ${groupeId}
    )`
    : eq(eleve.classe, trimmedClasse);

  return db
    .select({
      itemId: noteCompetenceValeur.itemId,
      eleveId: noteCompetenceValeur.eleveId,
      periodeId: noteCompetenceValeur.periodeId,
      niveau: noteCompetenceValeur.niveau,
      appreciation: noteCompetenceValeur.appreciation,
      itemCode: noteCompetenceItem.code,
      itemLibelle: noteCompetenceItem.libelle,
      eleveNom: eleve.nom,
      elevePrenom: eleve.prenom,
    })
    .from(noteCompetenceValeur)
    .innerJoin(noteCompetenceItem, eq(noteCompetenceValeur.itemId, noteCompetenceItem.id))
    .innerJoin(eleve, eq(noteCompetenceValeur.eleveId, eleve.id))
    .where(
      and(
        eq(noteCompetenceValeur.etablissementId, etablissementId),
        eq(noteCompetenceValeur.periodeId, opts.periodeId),
        eq(noteCompetenceItem.domaineId, opts.domaineId),
        eleveScope,
      ),
    )
    .orderBy(asc(eleve.nom), asc(noteCompetenceItem.ordre));
}

export async function upsertCompetenceValeur(
  etablissementId: string,
  input: {
    itemId: string;
    eleveId: string;
    periodeId: string;
    niveau?: string | null;
    appreciation?: string | null;
  },
) {
  const db = getDb();
  const niveau = input.niveau ? normalizeCompetenceNiveau(input.niveau) : null;
  if (input.niveau && !niveau) {
    throw new Error("Niveau invalide — utilisez 1 à 4 (insuffisant → très bon).");
  }

  await db
    .insert(noteCompetenceValeur)
    .values({
      etablissementId,
      itemId: input.itemId,
      eleveId: input.eleveId,
      periodeId: input.periodeId,
      niveau,
      appreciation: input.appreciation?.trim() || null,
    })
    .onConflictDoUpdate({
      target: [
        noteCompetenceValeur.etablissementId,
        noteCompetenceValeur.itemId,
        noteCompetenceValeur.eleveId,
        noteCompetenceValeur.periodeId,
      ],
      set: {
        niveau,
        appreciation: input.appreciation?.trim() || null,
        updatedAt: new Date(),
      },
    });
}

export async function listCompetencesForEleve(
  etablissementId: string,
  eleveId: string,
  periodeId?: string,
) {
  const db = getDb();
  const clauses = [
    eq(noteCompetenceValeur.etablissementId, etablissementId),
    eq(noteCompetenceValeur.eleveId, eleveId),
  ];
  if (periodeId) clauses.push(eq(noteCompetenceValeur.periodeId, periodeId));

  return db
    .select({
      domaineCode: noteCompetenceDomaine.code,
      domaineLibelle: noteCompetenceDomaine.libelle,
      itemCode: noteCompetenceItem.code,
      itemLibelle: noteCompetenceItem.libelle,
      niveau: noteCompetenceValeur.niveau,
      periodeId: noteCompetenceValeur.periodeId,
    })
    .from(noteCompetenceValeur)
    .innerJoin(noteCompetenceItem, eq(noteCompetenceValeur.itemId, noteCompetenceItem.id))
    .innerJoin(noteCompetenceDomaine, eq(noteCompetenceItem.domaineId, noteCompetenceDomaine.id))
    .where(and(...clauses))
    .orderBy(asc(noteCompetenceDomaine.ordre), asc(noteCompetenceItem.ordre));
}

export type LsuExportRow = {
  ine: string | null;
  nom: string;
  prenom: string;
  classe: string | null;
  domaineCode: string;
  domaineLibelle: string;
  itemCode: string;
  itemLibelle: string;
  niveau: string | null;
  niveauLabel: string;
};

export async function buildLsuExportRows(
  etablissementId: string,
  opts: { classe?: string; groupeId?: string; periodeId: string },
): Promise<LsuExportRow[]> {
  const db = getDb();
  const trimmedClasse = opts.classe?.trim() || "";
  const groupeId = opts.groupeId?.trim() || "";

  const eleveScope = groupeId
    ? sql`exists (
      select 1 from ${groupePedagogiqueMembre}
      where ${groupePedagogiqueMembre.eleveId} = ${eleve.id}
      and ${groupePedagogiqueMembre.etablissementId} = ${etablissementId}
      and ${groupePedagogiqueMembre.groupeId} = ${groupeId}
    )`
    : eq(eleve.classe, trimmedClasse);

  const rows = await db
    .select({
      ine: eleve.ine,
      nom: eleve.nom,
      prenom: eleve.prenom,
      classe: eleve.classe,
      domaineCode: noteCompetenceDomaine.code,
      domaineLibelle: noteCompetenceDomaine.libelle,
      itemCode: noteCompetenceItem.code,
      itemLibelle: noteCompetenceItem.libelle,
      niveau: noteCompetenceValeur.niveau,
    })
    .from(noteCompetenceValeur)
    .innerJoin(noteCompetenceItem, eq(noteCompetenceValeur.itemId, noteCompetenceItem.id))
    .innerJoin(noteCompetenceDomaine, eq(noteCompetenceItem.domaineId, noteCompetenceDomaine.id))
    .innerJoin(eleve, eq(noteCompetenceValeur.eleveId, eleve.id))
    .where(
      and(
        eq(noteCompetenceValeur.etablissementId, etablissementId),
        eq(noteCompetenceValeur.periodeId, opts.periodeId),
        eleveScope,
      ),
    )
    .orderBy(asc(eleve.nom), asc(noteCompetenceDomaine.ordre), asc(noteCompetenceItem.ordre));

  return rows.map((r) => ({
    ...r,
    niveauLabel: COMPETENCE_NIVEAUX.find((n) => n.code === r.niveau)?.label || "—",
  }));
}
