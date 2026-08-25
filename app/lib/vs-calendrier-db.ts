import "server-only";

import { and, asc, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { calendrierScolaire, edtCreneau, groupePedagogique, noteMatiere } from "@/db/schema";

export type CalendrierInput = {
  id?: string;
  label: string;
  dateDebut: string;
  dateFin: string;
  type?: string;
  anneeScolaireId?: string | null;
};

export type EdtCreneauInput = {
  id?: string;
  jourSemaine: number;
  heureDebut: string;
  heureFin: string;
  classe?: string;
  groupeId?: string | null;
  matiereId?: string | null;
  enseignantNom?: string;
  salle?: string;
  semaine?: string;
  anneeScolaireId?: string | null;
  /** Autorise l’enregistrement malgré un conflit dur (direction). */
  force?: boolean;
};

export async function listCalendrierEntries(etablissementId: string) {
  const db = getDb();
  return db
    .select()
    .from(calendrierScolaire)
    .where(eq(calendrierScolaire.etablissementId, etablissementId))
    .orderBy(asc(calendrierScolaire.dateDebut));
}

export async function upsertCalendrierEntry(etablissementId: string, input: CalendrierInput) {
  const db = getDb();
  const label = input.label.trim();
  const dateDebut = input.dateDebut.trim();
  const dateFin = input.dateFin.trim();
  if (!label || !dateDebut || !dateFin) {
    throw new Error("Libellé et dates obligatoires.");
  }

  if (input.id) {
    const [row] = await db
      .update(calendrierScolaire)
      .set({
        label,
        dateDebut,
        dateFin,
        type: input.type?.trim() || "vacances",
        anneeScolaireId: input.anneeScolaireId ?? null,
      })
      .where(
        and(eq(calendrierScolaire.etablissementId, etablissementId), eq(calendrierScolaire.id, input.id)),
      )
      .returning();
    return row;
  }

  const [row] = await db
    .insert(calendrierScolaire)
    .values({
      etablissementId,
      label,
      dateDebut,
      dateFin,
      type: input.type?.trim() || "vacances",
      anneeScolaireId: input.anneeScolaireId ?? null,
    })
    .returning();
  return row;
}

export async function deleteCalendrierEntry(etablissementId: string, id: string) {
  const db = getDb();
  await db
    .delete(calendrierScolaire)
    .where(and(eq(calendrierScolaire.etablissementId, etablissementId), eq(calendrierScolaire.id, id)));
}

export async function listEdtCreneaux(
  etablissementId: string,
  opts?: { classe?: string; groupeId?: string },
) {
  const db = getDb();
  const clauses = [eq(edtCreneau.etablissementId, etablissementId)];
  if (opts?.classe?.trim()) {
    clauses.push(eq(edtCreneau.classe, opts.classe.trim()));
  }
  if (opts?.groupeId?.trim()) {
    clauses.push(eq(edtCreneau.groupeId, opts.groupeId.trim()));
  }
  return db
    .select({
      id: edtCreneau.id,
      jourSemaine: edtCreneau.jourSemaine,
      heureDebut: edtCreneau.heureDebut,
      heureFin: edtCreneau.heureFin,
      classe: edtCreneau.classe,
      groupeId: edtCreneau.groupeId,
      groupeCode: groupePedagogique.code,
      matiereId: edtCreneau.matiereId,
      enseignantNom: edtCreneau.enseignantNom,
      salle: edtCreneau.salle,
      semaine: edtCreneau.semaine,
    })
    .from(edtCreneau)
    .leftJoin(groupePedagogique, eq(edtCreneau.groupeId, groupePedagogique.id))
    .where(and(...clauses))
    .orderBy(asc(edtCreneau.jourSemaine), asc(edtCreneau.heureDebut));
}

/** Créneaux pour un jour civil (1=lundi … 7=dimanche, ISO-like). */
export async function listEdtCreneauxForJour(
  etablissementId: string,
  jourSemaine: number,
  opts?: { classe?: string; groupeId?: string },
) {
  const db = getDb();
  const clauses = [
    eq(edtCreneau.etablissementId, etablissementId),
    eq(edtCreneau.jourSemaine, jourSemaine),
  ];
  if (opts?.classe?.trim()) {
    clauses.push(eq(edtCreneau.classe, opts.classe.trim()));
  }
  if (opts?.groupeId?.trim()) {
    clauses.push(eq(edtCreneau.groupeId, opts.groupeId.trim()));
  }
  return db
    .select({
      id: edtCreneau.id,
      jourSemaine: edtCreneau.jourSemaine,
      heureDebut: edtCreneau.heureDebut,
      heureFin: edtCreneau.heureFin,
      classe: edtCreneau.classe,
      groupeId: edtCreneau.groupeId,
      groupeCode: groupePedagogique.code,
      matiereId: edtCreneau.matiereId,
      matiereLibelle: noteMatiere.libelle,
      enseignantNom: edtCreneau.enseignantNom,
      salle: edtCreneau.salle,
      semaine: edtCreneau.semaine,
    })
    .from(edtCreneau)
    .leftJoin(noteMatiere, eq(noteMatiere.id, edtCreneau.matiereId))
    .leftJoin(groupePedagogique, eq(edtCreneau.groupeId, groupePedagogique.id))
    .where(and(...clauses))
    .orderBy(asc(edtCreneau.heureDebut), asc(edtCreneau.classe));
}

/** Jour de la semaine ISO : lundi=1 … dimanche=7 (depuis YYYY-MM-DD). */
export function jourSemaineFromIsoDate(isoDate: string): number {
  const [y, m, d] = isoDate.split("-").map(Number);
  if (!y || !m || !d) return 1;
  const js = new Date(Date.UTC(y, m - 1, d)).getUTCDay();
  return js === 0 ? 7 : js;
}

export async function upsertEdtCreneau(etablissementId: string, input: EdtCreneauInput) {
  const db = getDb();
  const jourSemaine = input.jourSemaine;
  const heureDebut = input.heureDebut.trim();
  const heureFin = input.heureFin.trim();
  if (jourSemaine < 1 || jourSemaine > 7 || !heureDebut || !heureFin) {
    throw new Error("Jour (1–7) et horaires obligatoires.");
  }

  const groupeId = input.groupeId?.trim() || null;
  let classe = input.classe?.trim() || null;
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

  const values = {
    jourSemaine,
    heureDebut,
    heureFin,
    classe,
    groupeId,
    matiereId: input.matiereId ?? null,
    enseignantNom: input.enseignantNom?.trim() || null,
    salle: input.salle?.trim() || null,
    semaine: input.semaine?.trim() || "AB",
    anneeScolaireId: input.anneeScolaireId ?? null,
    updatedAt: new Date(),
  };

  if (!input.force) {
    const { detectEdtConflicts } = await import("@/app/lib/edt-conflicts");
    const existing = await listEdtCreneaux(etablissementId);
    const candidateId = input.id || "__new__";
    const others = existing.filter((c) => c.id !== input.id);
    const conflits = detectEdtConflicts([
      ...others.map((c) => ({
        id: c.id,
        jourSemaine: c.jourSemaine,
        heureDebut: c.heureDebut,
        heureFin: c.heureFin,
        classe: c.classe,
        groupeId: c.groupeId,
        groupeCode: c.groupeCode,
        enseignantNom: c.enseignantNom,
        salle: c.salle,
        semaine: c.semaine,
      })),
      {
        id: candidateId,
        jourSemaine: values.jourSemaine,
        heureDebut: values.heureDebut,
        heureFin: values.heureFin,
        classe: values.classe,
        groupeId: values.groupeId,
        enseignantNom: values.enseignantNom,
        salle: values.salle,
        semaine: values.semaine,
      },
    ]).filter((c) => c.creneauIds.includes(candidateId));

    if (conflits.length) {
      throw new Error(
        `Conflit EDT : ${conflits[0]!.label} (${conflits[0]!.detail}). Corrigez ou forcez l’enregistrement.`,
      );
    }
  }

  if (input.id) {
    const [row] = await db
      .update(edtCreneau)
      .set(values)
      .where(and(eq(edtCreneau.etablissementId, etablissementId), eq(edtCreneau.id, input.id)))
      .returning();
    return row;
  }

  const [row] = await db
    .insert(edtCreneau)
    .values({ etablissementId, ...values })
    .returning();
  return row;
}

export async function deleteEdtCreneau(etablissementId: string, id: string) {
  const db = getDb();
  await db
    .delete(edtCreneau)
    .where(and(eq(edtCreneau.etablissementId, etablissementId), eq(edtCreneau.id, id)));
}

export async function seedDefaultCalendrier(etablissementId: string) {
  const existing = await listCalendrierEntries(etablissementId);
  if (existing.length) return { seeded: false, count: existing.length };

  const defaults: CalendrierInput[] = [
    {
      label: "Toussaint",
      dateDebut: "2025-10-18",
      dateFin: "2025-11-03",
      type: "vacances",
    },
    {
      label: "Noël",
      dateDebut: "2025-12-20",
      dateFin: "2026-01-05",
      type: "vacances",
    },
    {
      label: "Hiver",
      dateDebut: "2026-02-14",
      dateFin: "2026-03-02",
      type: "vacances",
    },
  ];

  for (const entry of defaults) {
    await upsertCalendrierEntry(etablissementId, entry);
  }
  return { seeded: true, count: defaults.length };
}
