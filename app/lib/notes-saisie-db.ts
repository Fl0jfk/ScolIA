import "server-only";

import { and, asc, desc, eq, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  eleve,
  groupePedagogique,
  groupePedagogiqueMembre,
  noteDevoir,
  noteMatiere,
  noteMoyenneEleve,
  notePeriode,
  noteValeur,
} from "@/db/schema";

export type DevoirInput = {
  matiereId: string;
  periodeId: string;
  typeDevoirId?: string | null;
  classe: string;
  groupeId?: string | null;
  libelle: string;
  dateDevoir?: string | null;
  coefficient?: string | number;
  createdByUserId?: string | null;
};

export async function listElevesForGroupe(etablissementId: string, groupeId: string) {
  const db = getDb();
  const gid = groupeId.trim();
  const rows = await db
    .select({
      id: eleve.id,
      nom: eleve.nom,
      prenom: eleve.prenom,
      classe: eleve.classe,
    })
    .from(groupePedagogiqueMembre)
    .innerJoin(eleve, eq(groupePedagogiqueMembre.eleveId, eleve.id))
    .where(
      and(
        eq(groupePedagogiqueMembre.etablissementId, etablissementId),
        eq(groupePedagogiqueMembre.groupeId, gid),
        eq(eleve.etablissementId, etablissementId),
      ),
    )
    .orderBy(asc(eleve.nom), asc(eleve.prenom));
  return rows;
}

export async function listDevoirs(
  etablissementId: string,
  opts: { classe?: string; groupeId?: string; periodeId?: string; matiereId?: string },
) {
  const db = getDb();
  const clauses = [eq(noteDevoir.etablissementId, etablissementId)];
  if (opts.classe?.trim()) clauses.push(eq(noteDevoir.classe, opts.classe.trim()));
  if (opts.groupeId?.trim()) clauses.push(eq(noteDevoir.groupeId, opts.groupeId.trim()));
  if (opts.periodeId) clauses.push(eq(noteDevoir.periodeId, opts.periodeId));
  if (opts.matiereId) clauses.push(eq(noteDevoir.matiereId, opts.matiereId));

  return db
    .select({
      id: noteDevoir.id,
      libelle: noteDevoir.libelle,
      classe: noteDevoir.classe,
      groupeId: noteDevoir.groupeId,
      groupeCode: groupePedagogique.code,
      dateDevoir: noteDevoir.dateDevoir,
      coefficient: noteDevoir.coefficient,
      matiereId: noteDevoir.matiereId,
      periodeId: noteDevoir.periodeId,
      matiereLibelle: noteMatiere.libelle,
      matiereCode: noteMatiere.code,
    })
    .from(noteDevoir)
    .innerJoin(noteMatiere, eq(noteDevoir.matiereId, noteMatiere.id))
    .leftJoin(groupePedagogique, eq(noteDevoir.groupeId, groupePedagogique.id))
    .where(and(...clauses))
    .orderBy(desc(noteDevoir.dateDevoir), desc(noteDevoir.createdAt));
}

export async function createDevoir(etablissementId: string, input: DevoirInput) {
  const db = getDb();
  const libelle = input.libelle.trim();
  const groupeId = input.groupeId?.trim() || null;
  let classe = input.classe.trim();

  if (groupeId) {
    const [groupe] = await db
      .select({ code: groupePedagogique.code })
      .from(groupePedagogique)
      .where(
        and(eq(groupePedagogique.etablissementId, etablissementId), eq(groupePedagogique.id, groupeId)),
      )
      .limit(1);
    if (!groupe) throw new Error("Groupe pédagogique introuvable.");
    if (!classe) classe = groupe.code;
  }

  if (!libelle || !classe) throw new Error("Libellé et classe (ou groupe) obligatoires.");

  const [periode] = await db
    .select({ statut: notePeriode.statut })
    .from(notePeriode)
    .where(
      and(eq(notePeriode.etablissementId, etablissementId), eq(notePeriode.id, input.periodeId)),
    )
    .limit(1);
  if (!periode) throw new Error("Période introuvable.");
  if (periode.statut === "cloturee") {
    throw new Error("Période clôturée — saisie impossible.");
  }

  const [row] = await db
    .insert(noteDevoir)
    .values({
      etablissementId,
      matiereId: input.matiereId,
      periodeId: input.periodeId,
      typeDevoirId: input.typeDevoirId || null,
      classe,
      groupeId,
      libelle,
      dateDevoir: input.dateDevoir || null,
      coefficient: String(input.coefficient ?? "1"),
      createdByUserId: input.createdByUserId || null,
    })
    .returning();
  return row;
}

export async function listElevesForClasse(etablissementId: string, classe: string) {
  const db = getDb();
  const cls = classe.trim();
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
        sql`lower(trim(${eleve.classe})) = lower(${cls})`,
      ),
    )
    .orderBy(asc(eleve.nom), asc(eleve.prenom));
}

export async function listNotesForDevoir(etablissementId: string, devoirId: string) {
  const db = getDb();
  const notes = await db
    .select()
    .from(noteValeur)
    .where(
      and(eq(noteValeur.etablissementId, etablissementId), eq(noteValeur.devoirId, devoirId)),
    );
  return notes;
}

export async function upsertNoteValeur(
  etablissementId: string,
  input: {
    devoirId: string;
    eleveId: string;
    valeur?: string | number | null;
    absent?: boolean;
    dispense?: boolean;
    appreciation?: string | null;
  },
) {
  const db = getDb();
  const [devoir] = await db
    .select({
      periodeId: noteDevoir.periodeId,
      matiereId: noteDevoir.matiereId,
      classe: noteDevoir.classe,
      groupeId: noteDevoir.groupeId,
    })
    .from(noteDevoir)
    .where(
      and(eq(noteDevoir.etablissementId, etablissementId), eq(noteDevoir.id, input.devoirId)),
    )
    .limit(1);
  if (!devoir) throw new Error("Devoir introuvable.");

  const [periode] = await db
    .select({ statut: notePeriode.statut })
    .from(notePeriode)
    .where(
      and(eq(notePeriode.etablissementId, etablissementId), eq(notePeriode.id, devoir.periodeId)),
    )
    .limit(1);
  if (periode?.statut === "cloturee") throw new Error("Période clôturée.");

  const absent = Boolean(input.absent);
  const dispense = Boolean(input.dispense);
  const valeurRaw = input.valeur;
  const valeur =
    absent || dispense || valeurRaw == null || String(valeurRaw).trim() === ""
      ? null
      : String(valeurRaw);

  await db
    .insert(noteValeur)
    .values({
      etablissementId,
      devoirId: input.devoirId,
      eleveId: input.eleveId,
      valeur,
      absent,
      dispense,
      appreciation: input.appreciation?.trim() || null,
    })
    .onConflictDoUpdate({
      target: [noteValeur.etablissementId, noteValeur.devoirId, noteValeur.eleveId],
      set: {
        valeur,
        absent,
        dispense,
        appreciation: input.appreciation?.trim() || null,
        updatedAt: new Date(),
      },
    });

  await recomputeMoyennes(etablissementId, {
    classe: devoir.classe,
    groupeId: devoir.groupeId,
    matiereId: devoir.matiereId,
    periodeId: devoir.periodeId,
  });
}

function weightedAverage(
  rows: Array<{ valeur: string | null; coefficient: string; absent: boolean; dispense: boolean }>,
): { moyenne: number | null; nbNotes: number } {
  let sum = 0;
  let coefSum = 0;
  let nb = 0;
  for (const r of rows) {
    if (r.absent || r.dispense || r.valeur == null) continue;
    const v = Number(r.valeur);
    const c = Number(r.coefficient) || 1;
    if (!Number.isFinite(v)) continue;
    sum += v * c;
    coefSum += c;
    nb += 1;
  }
  if (coefSum <= 0) return { moyenne: null, nbNotes: nb };
  return { moyenne: Math.round((sum / coefSum) * 100) / 100, nbNotes: nb };
}

export async function recomputeMoyennes(
  etablissementId: string,
  opts: { classe: string; groupeId?: string | null; matiereId: string; periodeId: string },
) {
  const db = getDb();
  const eleves = opts.groupeId
    ? await listElevesForGroupe(etablissementId, opts.groupeId)
    : await listElevesForClasse(etablissementId, opts.classe);

  const devoirClauses = [
    eq(noteDevoir.etablissementId, etablissementId),
    eq(noteDevoir.matiereId, opts.matiereId),
    eq(noteDevoir.periodeId, opts.periodeId),
  ];
  if (opts.groupeId) {
    devoirClauses.push(eq(noteDevoir.groupeId, opts.groupeId));
  } else {
    devoirClauses.push(eq(noteDevoir.classe, opts.classe));
    devoirClauses.push(sql`${noteDevoir.groupeId} is null`);
  }

  const devoirs = await db
    .select({ id: noteDevoir.id, coefficient: noteDevoir.coefficient })
    .from(noteDevoir)
    .where(and(...devoirClauses));

  for (const el of eleves) {
    const rows: Array<{
      valeur: string | null;
      coefficient: string;
      absent: boolean;
      dispense: boolean;
    }> = [];

    for (const d of devoirs) {
      const [note] = await db
        .select({
          valeur: noteValeur.valeur,
          absent: noteValeur.absent,
          dispense: noteValeur.dispense,
        })
        .from(noteValeur)
        .where(
          and(
            eq(noteValeur.etablissementId, etablissementId),
            eq(noteValeur.devoirId, d.id),
            eq(noteValeur.eleveId, el.id),
          ),
        )
        .limit(1);
      rows.push({
        valeur: note?.valeur ?? null,
        coefficient: d.coefficient,
        absent: note?.absent ?? false,
        dispense: note?.dispense ?? false,
      });
    }

    const { moyenne, nbNotes } = weightedAverage(rows);
    await db
      .insert(noteMoyenneEleve)
      .values({
        etablissementId,
        eleveId: el.id,
        matiereId: opts.matiereId,
        periodeId: opts.periodeId,
        moyenne: moyenne != null ? String(moyenne) : null,
        nbNotes,
      })
      .onConflictDoUpdate({
        target: [
          noteMoyenneEleve.etablissementId,
          noteMoyenneEleve.eleveId,
          noteMoyenneEleve.matiereId,
          noteMoyenneEleve.periodeId,
        ],
        set: {
          moyenne: moyenne != null ? String(moyenne) : null,
          nbNotes,
          updatedAt: new Date(),
        },
      });
  }
}

export async function listMoyennesClasse(
  etablissementId: string,
  opts: { classe: string; periodeId: string; matiereId?: string },
) {
  return listMoyennesScope(etablissementId, { ...opts, groupeId: null });
}

export async function listMoyennesGroupe(
  etablissementId: string,
  opts: { groupeId: string; periodeId: string; matiereId?: string },
) {
  return listMoyennesScope(etablissementId, {
    classe: "",
    groupeId: opts.groupeId,
    periodeId: opts.periodeId,
    matiereId: opts.matiereId,
  });
}

async function listMoyennesScope(
  etablissementId: string,
  opts: { classe: string; groupeId?: string | null; periodeId: string; matiereId?: string },
) {
  const db = getDb();
  const eleveScope = opts.groupeId
    ? sql`exists (
      select 1 from ${groupePedagogiqueMembre}
      where ${groupePedagogiqueMembre.eleveId} = ${noteMoyenneEleve.eleveId}
      and ${groupePedagogiqueMembre.etablissementId} = ${etablissementId}
      and ${groupePedagogiqueMembre.groupeId} = ${opts.groupeId}
    )`
    : sql`exists (
      select 1 from ${eleve}
      where ${eleve.id} = ${noteMoyenneEleve.eleveId}
      and ${eleve.etablissementId} = ${etablissementId}
      and lower(trim(${eleve.classe})) = lower(${opts.classe.trim()})
    )`;

  const clauses = [
    eq(noteMoyenneEleve.etablissementId, etablissementId),
    eq(noteMoyenneEleve.periodeId, opts.periodeId),
    eleveScope,
  ];
  if (opts.matiereId) clauses.push(eq(noteMoyenneEleve.matiereId, opts.matiereId));

  return db
    .select({
      eleveId: noteMoyenneEleve.eleveId,
      nom: eleve.nom,
      prenom: eleve.prenom,
      matiereId: noteMoyenneEleve.matiereId,
      matiereLibelle: noteMatiere.libelle,
      moyenne: noteMoyenneEleve.moyenne,
      nbNotes: noteMoyenneEleve.nbNotes,
    })
    .from(noteMoyenneEleve)
    .innerJoin(eleve, eq(noteMoyenneEleve.eleveId, eleve.id))
    .innerJoin(noteMatiere, eq(noteMoyenneEleve.matiereId, noteMatiere.id))
    .where(and(...clauses))
    .orderBy(asc(eleve.nom), asc(noteMatiere.code));
}

export async function listMoyennesForEleve(etablissementId: string, eleveId: string) {
  const db = getDb();
  return db
    .select({
      matiereCode: noteMatiere.code,
      matiereLibelle: noteMatiere.libelle,
      periodeId: notePeriode.id,
      periodeCode: notePeriode.code,
      periodeLibelle: notePeriode.libelle,
      periodeStatut: notePeriode.statut,
      moyenne: noteMoyenneEleve.moyenne,
      nbNotes: noteMoyenneEleve.nbNotes,
    })
    .from(noteMoyenneEleve)
    .innerJoin(noteMatiere, eq(noteMoyenneEleve.matiereId, noteMatiere.id))
    .innerJoin(notePeriode, eq(noteMoyenneEleve.periodeId, notePeriode.id))
    .where(
      and(eq(noteMoyenneEleve.etablissementId, etablissementId), eq(noteMoyenneEleve.eleveId, eleveId)),
    )
    .orderBy(asc(notePeriode.ordre), asc(noteMatiere.code));
}

export async function closePeriode(etablissementId: string, periodeId: string) {
  const db = getDb();
  const [row] = await db
    .update(notePeriode)
    .set({ statut: "cloturee", updatedAt: new Date() })
    .where(
      and(eq(notePeriode.etablissementId, etablissementId), eq(notePeriode.id, periodeId)),
    )
    .returning();
  if (!row) throw new Error("Période introuvable.");
  return row;
}
