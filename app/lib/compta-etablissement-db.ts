import "server-only";

import { and, asc, desc, eq, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { depense, mouvementTresorerie, tresorerieCompte } from "@/db/schema";

const PORTES = ["cantine", "internat", "voyage", "scolarite", "autre"] as const;
const NATURES = ["caisse", "banque"] as const;
const SENS = ["entree", "sortie"] as const;

export type DepensePortee = (typeof PORTES)[number];
export type CompteNature = (typeof NATURES)[number];
export type MouvementSens = (typeof SENS)[number];

function isPortee(v: string): v is DepensePortee {
  return (PORTES as readonly string[]).includes(v);
}
function isNature(v: string): v is CompteNature {
  return (NATURES as readonly string[]).includes(v);
}
function isSens(v: string): v is MouvementSens {
  return (SENS as readonly string[]).includes(v);
}

export async function listComptesTresorerie(etablissementId: string) {
  const db = getDb();
  const rows = await db
    .select()
    .from(tresorerieCompte)
    .where(eq(tresorerieCompte.etablissementId, etablissementId))
    .orderBy(
      sql`case when ${tresorerieCompte.nature} = 'caisse' then 0 else 1 end`,
      asc(tresorerieCompte.libelle),
    );

  const withSoldes = [];
  for (const c of rows) {
    const solde = await soldeCompte(etablissementId, c.id);
    withSoldes.push({ ...c, solde });
  }
  return withSoldes;
}

export async function upsertCompteTresorerie(
  etablissementId: string,
  input: { id?: string; libelle: string; nature: string; actif?: boolean },
) {
  const db = getDb();
  const libelle = input.libelle.trim();
  if (!libelle) throw new Error("Libellé obligatoire.");
  if (!isNature(input.nature)) throw new Error("Nature : caisse ou banque.");

  if (input.id) {
    const [row] = await db
      .update(tresorerieCompte)
      .set({
        libelle,
        nature: input.nature,
        actif: input.actif ?? true,
        updatedAt: new Date(),
      })
      .where(
        and(eq(tresorerieCompte.etablissementId, etablissementId), eq(tresorerieCompte.id, input.id)),
      )
      .returning();
    if (!row) throw new Error("Compte introuvable.");
    return row;
  }

  const [row] = await db
    .insert(tresorerieCompte)
    .values({
      etablissementId,
      libelle,
      nature: input.nature,
      actif: input.actif ?? true,
    })
    .returning();
  if (!row) throw new Error("Création compte impossible.");
  return row;
}

export async function ensureComptesDefaut(etablissementId: string) {
  const existing = await listComptesTresorerie(etablissementId);
  if (existing.length) return existing;
  await upsertCompteTresorerie(etablissementId, { libelle: "Caisse", nature: "caisse" });
  await upsertCompteTresorerie(etablissementId, { libelle: "Banque", nature: "banque" });
  return listComptesTresorerie(etablissementId);
}

export async function soldeCompte(etablissementId: string, compteId: string): Promise<string> {
  const db = getDb();
  const [row] = await db
    .select({
      n: sql<number>`coalesce(sum(case when ${mouvementTresorerie.sens} = 'entree' then ${mouvementTresorerie.montant}::numeric else -${mouvementTresorerie.montant}::numeric end), 0)::float`,
    })
    .from(mouvementTresorerie)
    .where(
      and(
        eq(mouvementTresorerie.etablissementId, etablissementId),
        eq(mouvementTresorerie.compteId, compteId),
      ),
    );
  return Number(row?.n ?? 0).toFixed(2);
}

export async function listDepenses(etablissementId: string) {
  const db = getDb();
  return db
    .select()
    .from(depense)
    .where(eq(depense.etablissementId, etablissementId))
    .orderBy(desc(depense.dateDepense), desc(depense.createdAt));
}

export async function createDepense(
  etablissementId: string,
  input: {
    dateDepense?: string;
    libelle: string;
    fournisseur?: string;
    portee?: string;
    montant: string | number;
    statut?: string;
    travelId?: string | null;
  },
) {
  const db = getDb();
  const libelle = input.libelle.trim();
  if (!libelle) throw new Error("Libellé obligatoire.");
  const montant = Number(input.montant);
  if (!Number.isFinite(montant) || montant <= 0) throw new Error("Montant invalide.");
  const portee = input.portee?.trim() || "autre";
  if (!isPortee(portee)) throw new Error("Portée invalide.");
  const statut = input.statut === "payee" ? "payee" : "prevue";
  const dateDepense =
    input.dateDepense?.trim().slice(0, 10) || new Date().toISOString().slice(0, 10);

  const [row] = await db
    .insert(depense)
    .values({
      etablissementId,
      dateDepense,
      libelle,
      fournisseur: input.fournisseur?.trim() || null,
      portee,
      montant: montant.toFixed(2),
      statut,
      travelId: input.travelId || null,
    })
    .returning();
  if (!row) throw new Error("Création dépense impossible.");
  return row;
}

/** Marque une dépense payée et crée la sortie de trésorerie liée. */
export async function payerDepense(
  etablissementId: string,
  depenseId: string,
  opts: { compteId: string; dateMouvement?: string },
) {
  const db = getDb();
  const [d] = await db
    .select()
    .from(depense)
    .where(and(eq(depense.etablissementId, etablissementId), eq(depense.id, depenseId)))
    .limit(1);
  if (!d) throw new Error("Dépense introuvable.");
  if (d.statut === "payee") throw new Error("Dépense déjà payée.");

  const [compte] = await db
    .select()
    .from(tresorerieCompte)
    .where(
      and(
        eq(tresorerieCompte.etablissementId, etablissementId),
        eq(tresorerieCompte.id, opts.compteId),
      ),
    )
    .limit(1);
  if (!compte) throw new Error("Compte de trésorerie introuvable.");

  const dateMouvement =
    opts.dateMouvement?.trim().slice(0, 10) ||
    String(d.dateDepense) ||
    new Date().toISOString().slice(0, 10);

  const [mov] = await db
    .insert(mouvementTresorerie)
    .values({
      etablissementId,
      compteId: compte.id,
      dateMouvement,
      sens: "sortie",
      montant: String(d.montant),
      libelle: `Paiement : ${d.libelle}`.slice(0, 200),
      depenseId: d.id,
    })
    .returning();

  const [updated] = await db
    .update(depense)
    .set({ statut: "payee", updatedAt: new Date() })
    .where(eq(depense.id, d.id))
    .returning();

  return { depense: updated, mouvement: mov, solde: await soldeCompte(etablissementId, compte.id) };
}

export async function listMouvements(etablissementId: string, opts?: { compteId?: string }) {
  const db = getDb();
  const rows = await db
    .select({
      id: mouvementTresorerie.id,
      compteId: mouvementTresorerie.compteId,
      dateMouvement: mouvementTresorerie.dateMouvement,
      sens: mouvementTresorerie.sens,
      montant: mouvementTresorerie.montant,
      libelle: mouvementTresorerie.libelle,
      depenseId: mouvementTresorerie.depenseId,
      encaissementId: mouvementTresorerie.encaissementId,
      factureId: mouvementTresorerie.factureId,
      createdAt: mouvementTresorerie.createdAt,
      compteLibelle: tresorerieCompte.libelle,
      compteNature: tresorerieCompte.nature,
    })
    .from(mouvementTresorerie)
    .innerJoin(tresorerieCompte, eq(tresorerieCompte.id, mouvementTresorerie.compteId))
    .where(
      opts?.compteId
        ? and(
            eq(mouvementTresorerie.etablissementId, etablissementId),
            eq(mouvementTresorerie.compteId, opts.compteId),
          )
        : eq(mouvementTresorerie.etablissementId, etablissementId),
    )
    .orderBy(desc(mouvementTresorerie.dateMouvement), desc(mouvementTresorerie.createdAt));
  return rows;
}

export async function createMouvement(
  etablissementId: string,
  input: {
    compteId: string;
    dateMouvement?: string;
    sens: string;
    montant: string | number;
    libelle: string;
    depenseId?: string | null;
    encaissementId?: string | null;
    factureId?: string | null;
  },
) {
  const db = getDb();
  if (!isSens(input.sens)) throw new Error("Sens : entree ou sortie.");
  const montant = Number(input.montant);
  if (!Number.isFinite(montant) || montant <= 0) throw new Error("Montant invalide.");
  const libelle = input.libelle.trim();
  if (!libelle) throw new Error("Libellé obligatoire.");

  const [compte] = await db
    .select()
    .from(tresorerieCompte)
    .where(
      and(
        eq(tresorerieCompte.etablissementId, etablissementId),
        eq(tresorerieCompte.id, input.compteId),
      ),
    )
    .limit(1);
  if (!compte) throw new Error("Compte introuvable.");

  const [row] = await db
    .insert(mouvementTresorerie)
    .values({
      etablissementId,
      compteId: input.compteId,
      dateMouvement:
        input.dateMouvement?.trim().slice(0, 10) || new Date().toISOString().slice(0, 10),
      sens: input.sens,
      montant: montant.toFixed(2),
      libelle: libelle.slice(0, 200),
      depenseId: input.depenseId || null,
      encaissementId: input.encaissementId || null,
      factureId: input.factureId || null,
    })
    .returning();
  if (!row) throw new Error("Création mouvement impossible.");
  return { mouvement: row, solde: await soldeCompte(etablissementId, input.compteId) };
}

export async function loadComptaHub(etablissementId: string) {
  const comptes = await listComptesTresorerie(etablissementId);
  const [depenses, mouvements] = await Promise.all([
    listDepenses(etablissementId),
    listMouvements(etablissementId),
  ]);
  return { comptes, depenses, mouvements };
}
