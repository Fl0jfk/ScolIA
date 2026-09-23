import "server-only";

import { and, asc, desc, eq, gte, lte, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  depense,
  encaissement,
  mouvementTresorerie,
  tresorerieCompte,
} from "@/db/schema";

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

export async function loadComptaHub(etablissementId: string, opts?: { from?: string; to?: string }) {
  const comptes = await listComptesTresorerie(etablissementId);
  const [depenses, mouvements, synthese, encaissementsOrphelins] = await Promise.all([
    listDepenses(etablissementId),
    listMouvements(etablissementId),
    synthesePeriode(etablissementId, opts),
    countEncaissementsSansMouvement(etablissementId),
  ]);
  return { comptes, depenses, mouvements, synthese, encaissementsOrphelins };
}

/** Compte cible pour un encaissement famille (virement → banque, sinon caisse). */
async function resolveComptePourEncaissement(
  etablissementId: string,
  mode: string,
  compteId?: string,
) {
  await ensureComptesDefaut(etablissementId);
  const comptes = await listComptesTresorerie(etablissementId);
  if (compteId) {
    const hit = comptes.find((c) => c.id === compteId);
    if (hit) return hit;
  }
  const m = mode.toLowerCase();
  if (m.includes("espec") || m.includes("espèc") || m.includes("cheque") || m.includes("chèque")) {
    return comptes.find((c) => c.nature === "caisse") || comptes[0]!;
  }
  return comptes.find((c) => c.nature === "banque") || comptes[0]!;
}

export async function countEncaissementsSansMouvement(etablissementId: string): Promise<number> {
  const db = getDb();
  const linked = await db
    .select({ id: mouvementTresorerie.encaissementId })
    .from(mouvementTresorerie)
    .where(
      and(
        eq(mouvementTresorerie.etablissementId, etablissementId),
        sql`${mouvementTresorerie.encaissementId} is not null`,
      ),
    );
  const linkedIds = new Set(linked.map((r) => r.id).filter(Boolean) as string[]);
  const all = await db
    .select({ id: encaissement.id })
    .from(encaissement)
    .where(eq(encaissement.etablissementId, etablissementId));
  return all.filter((e) => !linkedIds.has(e.id)).length;
}

/**
 * Importe les encaissements familles sans mouvement → entrées livre.
 * Idempotent (skip si déjà lié).
 */
export async function importerEncaissementsVersLivre(
  etablissementId: string,
  opts?: { compteId?: string },
) {
  const db = getDb();
  const linked = await db
    .select({ id: mouvementTresorerie.encaissementId })
    .from(mouvementTresorerie)
    .where(
      and(
        eq(mouvementTresorerie.etablissementId, etablissementId),
        sql`${mouvementTresorerie.encaissementId} is not null`,
      ),
    );
  const linkedIds = new Set(linked.map((r) => r.id).filter(Boolean) as string[]);

  const encs = await db
    .select()
    .from(encaissement)
    .where(eq(encaissement.etablissementId, etablissementId))
    .orderBy(asc(encaissement.dateEncaissement));

  const created = [];
  for (const enc of encs) {
    if (linkedIds.has(enc.id)) continue;
    const montant = Number(enc.montant);
    if (!Number.isFinite(montant) || montant <= 0) continue;

    const compte = await resolveComptePourEncaissement(etablissementId, enc.mode, opts?.compteId);
    const libelle = (enc.reference?.trim() || `Encaissement famille ${enc.mode}`).slice(0, 200);
    const [mov] = await db
      .insert(mouvementTresorerie)
      .values({
        etablissementId,
        compteId: compte.id,
        dateMouvement: String(enc.dateEncaissement),
        sens: "entree",
        montant: montant.toFixed(2),
        libelle,
        encaissementId: enc.id,
      })
      .returning();
    if (mov) created.push(mov);
  }

  return {
    imported: created.length,
    mouvements: created,
    comptes: await listComptesTresorerie(etablissementId),
  };
}

/** Crée l’entrée livre pour un encaissement unique (appel depuis facturation). */
export async function ensureMouvementPourEncaissement(
  etablissementId: string,
  enc: {
    id: string;
    montant: string | number;
    mode: string;
    dateEncaissement: string;
    reference?: string | null;
  },
) {
  const db = getDb();
  const [existing] = await db
    .select({ id: mouvementTresorerie.id })
    .from(mouvementTresorerie)
    .where(
      and(
        eq(mouvementTresorerie.etablissementId, etablissementId),
        eq(mouvementTresorerie.encaissementId, enc.id),
      ),
    )
    .limit(1);
  if (existing) return existing;

  const montant = Number(enc.montant);
  if (!Number.isFinite(montant) || montant <= 0) return null;

  const compte = await resolveComptePourEncaissement(etablissementId, enc.mode);
  const [mov] = await db
    .insert(mouvementTresorerie)
    .values({
      etablissementId,
      compteId: compte.id,
      dateMouvement: String(enc.dateEncaissement).slice(0, 10),
      sens: "entree",
      montant: montant.toFixed(2),
      libelle: (enc.reference?.trim() || `Encaissement famille ${enc.mode}`).slice(0, 200),
      encaissementId: enc.id,
    })
    .returning();
  return mov ?? null;
}

export type SynthesePeriode = {
  from: string;
  to: string;
  recettesLivre: string;
  depensesPayees: string;
  soldePeriode: string;
  encaissementsFamille: string;
  nbMouvements: number;
  nbDepensesPayees: number;
  soldesComptes: Array<{ id: string; libelle: string; nature: string; solde: string }>;
};

export async function synthesePeriode(
  etablissementId: string,
  opts?: { from?: string; to?: string },
): Promise<SynthesePeriode> {
  const db = getDb();
  const today = new Date().toISOString().slice(0, 10);
  const from = opts?.from?.trim().slice(0, 10) || `${today.slice(0, 8)}01`;
  const to = opts?.to?.trim().slice(0, 10) || today;

  const [recettesRow] = await db
    .select({
      n: sql<number>`coalesce(sum(${mouvementTresorerie.montant}::numeric), 0)::float`,
      c: sql<number>`count(*)::int`,
    })
    .from(mouvementTresorerie)
    .where(
      and(
        eq(mouvementTresorerie.etablissementId, etablissementId),
        eq(mouvementTresorerie.sens, "entree"),
        gte(mouvementTresorerie.dateMouvement, from),
        lte(mouvementTresorerie.dateMouvement, to),
      ),
    );

  const [sortiesRow] = await db
    .select({
      n: sql<number>`coalesce(sum(${mouvementTresorerie.montant}::numeric), 0)::float`,
      c: sql<number>`count(*)::int`,
    })
    .from(mouvementTresorerie)
    .where(
      and(
        eq(mouvementTresorerie.etablissementId, etablissementId),
        eq(mouvementTresorerie.sens, "sortie"),
        gte(mouvementTresorerie.dateMouvement, from),
        lte(mouvementTresorerie.dateMouvement, to),
      ),
    );

  const [depRow] = await db
    .select({
      n: sql<number>`coalesce(sum(${depense.montant}::numeric), 0)::float`,
      c: sql<number>`count(*)::int`,
    })
    .from(depense)
    .where(
      and(
        eq(depense.etablissementId, etablissementId),
        eq(depense.statut, "payee"),
        gte(depense.dateDepense, from),
        lte(depense.dateDepense, to),
      ),
    );

  const [encRow] = await db
    .select({
      n: sql<number>`coalesce(sum(${encaissement.montant}::numeric), 0)::float`,
    })
    .from(encaissement)
    .where(
      and(
        eq(encaissement.etablissementId, etablissementId),
        gte(encaissement.dateEncaissement, from),
        lte(encaissement.dateEncaissement, to),
      ),
    );

  const recettes = Number(recettesRow?.n ?? 0);
  const sorties = Number(sortiesRow?.n ?? 0);
  const comptes = await listComptesTresorerie(etablissementId);

  return {
    from,
    to,
    recettesLivre: recettes.toFixed(2),
    depensesPayees: Number(depRow?.n ?? 0).toFixed(2),
    soldePeriode: (recettes - sorties).toFixed(2),
    encaissementsFamille: Number(encRow?.n ?? 0).toFixed(2),
    nbMouvements: Number(recettesRow?.c ?? 0) + Number(sortiesRow?.c ?? 0),
    nbDepensesPayees: Number(depRow?.c ?? 0),
    soldesComptes: comptes.map((c) => ({
      id: c.id,
      libelle: c.libelle,
      nature: c.nature,
      solde: c.solde,
    })),
  };
}

export type ExportCabinetRow = {
  datePiece: string | null;
  typePiece: string;
  nature: string;
  numero: string;
  montant: string;
  statut: string;
  foyerId: string | null;
};

/** Sortie pour l’expert-comptable (vue export_comptable_famille + dépenses payées). */
export async function listExportCabinet(
  etablissementId: string,
  opts?: { from?: string; to?: string },
): Promise<ExportCabinetRow[]> {
  const db = getDb();
  const today = new Date().toISOString().slice(0, 10);
  const from = opts?.from?.trim().slice(0, 10) || "2000-01-01";
  const to = opts?.to?.trim().slice(0, 10) || today;

  const famille = await db.execute(sql`
    SELECT date_piece::text AS "datePiece",
           type_piece AS "typePiece",
           nature,
           numero,
           montant::text AS montant,
           statut,
           foyer_id::text AS "foyerId"
    FROM export_comptable_famille
    WHERE etablissement_id = ${etablissementId}::uuid
      AND date_piece >= ${from}::date
      AND date_piece <= ${to}::date
    ORDER BY date_piece, type_piece, numero
  `);

  const famRows = Array.from(famille as unknown as Array<Record<string, unknown>>);

  const deps = await db
    .select({
      datePiece: depense.dateDepense,
      libelle: depense.libelle,
      portee: depense.portee,
      montant: depense.montant,
      statut: depense.statut,
    })
    .from(depense)
    .where(
      and(
        eq(depense.etablissementId, etablissementId),
        eq(depense.statut, "payee"),
        gte(depense.dateDepense, from),
        lte(depense.dateDepense, to),
      ),
    )
    .orderBy(asc(depense.dateDepense));

  const rows: ExportCabinetRow[] = [];
  for (const r of famRows) {
    rows.push({
      datePiece: r.datePiece != null ? String(r.datePiece) : null,
      typePiece: String(r.typePiece ?? ""),
      nature: String(r.nature ?? ""),
      numero: String(r.numero ?? ""),
      montant: String(r.montant ?? "0"),
      statut: String(r.statut ?? ""),
      foyerId: r.foyerId != null ? String(r.foyerId) : null,
    });
  }
  for (const d of deps) {
    rows.push({
      datePiece: String(d.datePiece),
      typePiece: "depense",
      nature: d.portee,
      numero: d.libelle,
      montant: String(d.montant),
      statut: d.statut,
      foyerId: null,
    });
  }
  rows.sort((a, b) => String(a.datePiece || "").localeCompare(String(b.datePiece || "")));
  return rows;
}

export function exportCabinetToCsv(rows: ExportCabinetRow[]): string {
  const header = ["date_piece", "type_piece", "nature", "numero", "montant", "statut", "foyer_id"];
  const lines = [header.join(";")];
  for (const r of rows) {
    lines.push(
      [
        r.datePiece || "",
        r.typePiece,
        r.nature,
        `"${String(r.numero).replace(/"/g, '""')}"`,
        r.montant,
        r.statut,
        r.foyerId || "",
      ].join(";"),
    );
  }
  return lines.join("\n");
}
