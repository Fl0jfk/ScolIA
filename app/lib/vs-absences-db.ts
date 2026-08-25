import "server-only";

import { and, asc, desc, eq, inArray, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve, groupePedagogiqueMembre, vsAbsenceEleve, vsAppel, vsAppelLigne } from "@/db/schema";
import { matchInternatStudent } from "@/app/lib/eleve-dossier-synthese";
import { getInternatStudents } from "@/app/lib/internat-storage";
import type { InternatStudent } from "@/app/lib/internat-types";

export type AppelLigneStatut = "present" | "absent" | "retard" | "dispense";
export type AbsenceType = "absence" | "retard";
export type AbsenceStatut = "a_traiter" | "justifiee" | "non_justifiee" | "classee";

export type VsAppelLigneInput = {
  eleveId: string;
  statut: AppelLigneStatut;
  retardMinutes?: number | null;
  note?: string | null;
};

function isNonPresent(statut: string): statut is "absent" | "retard" {
  return statut === "absent" || statut === "retard";
}

export type VsEleveAppelRow = {
  id: string;
  nom: string;
  prenom: string;
  ine: string | null;
  photoKey: string | null;
  classe: string | null;
  folderName: string | null;
};

export async function listElevesForClasse(
  etablissementId: string,
  classe: string,
): Promise<VsEleveAppelRow[]> {
  const db = getDb();
  const trimmed = classe.trim();
  return db
    .select({
      id: eleve.id,
      nom: eleve.nom,
      prenom: eleve.prenom,
      ine: eleve.ine,
      photoKey: eleve.photoKey,
      classe: eleve.classe,
      folderName: eleve.folderName,
    })
    .from(eleve)
    .where(
      and(
        eq(eleve.etablissementId, etablissementId),
        sql`lower(trim(${eleve.classe})) = lower(${trimmed})`,
        eq(eleve.status, "inscrit"),
      ),
    )
    .orderBy(asc(eleve.nom), asc(eleve.prenom));
}

/** Élèves d’un groupe pédagogique (LV2, option…) pour l’appel — mêmes champs photo que la classe. */
export async function listElevesForGroupeAppel(
  etablissementId: string,
  groupeId: string,
): Promise<VsEleveAppelRow[]> {
  const db = getDb();
  const gid = groupeId.trim();
  if (!gid) return [];

  return db
    .select({
      id: eleve.id,
      nom: eleve.nom,
      prenom: eleve.prenom,
      ine: eleve.ine,
      photoKey: eleve.photoKey,
      classe: eleve.classe,
      folderName: eleve.folderName,
    })
    .from(groupePedagogiqueMembre)
    .innerJoin(eleve, eq(groupePedagogiqueMembre.eleveId, eleve.id))
    .where(
      and(
        eq(groupePedagogiqueMembre.etablissementId, etablissementId),
        eq(groupePedagogiqueMembre.groupeId, gid),
        eq(eleve.etablissementId, etablissementId),
        eq(eleve.status, "inscrit"),
      ),
    )
    .orderBy(asc(eleve.nom), asc(eleve.prenom));
}

export async function listAppelsForDate(etablissementId: string, dateAppel: string) {
  const db = getDb();
  return db
    .select({
      id: vsAppel.id,
      classe: vsAppel.classe,
      creneauId: vsAppel.creneauId,
      heureDebut: vsAppel.heureDebut,
      heureFin: vsAppel.heureFin,
      matiereLibelle: vsAppel.matiereLibelle,
      statut: vsAppel.statut,
      enseignantNom: vsAppel.enseignantNom,
    })
    .from(vsAppel)
    .where(and(eq(vsAppel.etablissementId, etablissementId), eq(vsAppel.dateAppel, dateAppel)))
    .orderBy(asc(vsAppel.heureDebut), asc(vsAppel.classe));
}

export async function getOrCreateAppel(
  etablissementId: string,
  input: {
    dateAppel: string;
    classe: string;
    creneauId?: string | null;
    heureDebut?: string | null;
    heureFin?: string | null;
    matiereLibelle?: string | null;
    enseignantUserId?: string | null;
    enseignantNom?: string | null;
  },
) {
  const db = getDb();
  const classe = input.classe.trim();
  if (!classe || !input.dateAppel) throw new Error("Classe et date obligatoires.");

  if (input.creneauId) {
    const [existing] = await db
      .select()
      .from(vsAppel)
      .where(
        and(
          eq(vsAppel.etablissementId, etablissementId),
          eq(vsAppel.creneauId, input.creneauId),
          eq(vsAppel.dateAppel, input.dateAppel),
        ),
      )
      .limit(1);
    if (existing) return existing;
  } else {
    const clauses = [
      eq(vsAppel.etablissementId, etablissementId),
      eq(vsAppel.dateAppel, input.dateAppel),
      eq(vsAppel.classe, classe),
    ];
    const rows = await db
      .select()
      .from(vsAppel)
      .where(and(...clauses))
      .orderBy(desc(vsAppel.createdAt))
      .limit(5);
    const match = rows.find((r) => {
      if (input.heureDebut) return r.heureDebut === input.heureDebut;
      return !r.creneauId;
    });
    if (match) return match;
  }

  const [created] = await db
    .insert(vsAppel)
    .values({
      etablissementId,
      dateAppel: input.dateAppel,
      creneauId: input.creneauId || null,
      classe,
      heureDebut: input.heureDebut || null,
      heureFin: input.heureFin || null,
      matiereLibelle: input.matiereLibelle || null,
      enseignantUserId: input.enseignantUserId || null,
      enseignantNom: input.enseignantNom || null,
      statut: "en_cours",
    })
    .returning();
  return created;
}

export async function getAppelWithLignes(etablissementId: string, appelId: string) {
  const db = getDb();
  const [appel] = await db
    .select()
    .from(vsAppel)
    .where(and(eq(vsAppel.etablissementId, etablissementId), eq(vsAppel.id, appelId)))
    .limit(1);
  if (!appel) return null;

  const lignes = await db
    .select({
      eleveId: vsAppelLigne.eleveId,
      statut: vsAppelLigne.statut,
      retardMinutes: vsAppelLigne.retardMinutes,
      note: vsAppelLigne.note,
      nom: eleve.nom,
      prenom: eleve.prenom,
      ine: eleve.ine,
      photoKey: eleve.photoKey,
      folderName: eleve.folderName,
    })
    .from(vsAppelLigne)
    .innerJoin(eleve, eq(eleve.id, vsAppelLigne.eleveId))
    .where(
      and(eq(vsAppelLigne.etablissementId, etablissementId), eq(vsAppelLigne.appelId, appelId)),
    )
    .orderBy(asc(eleve.nom), asc(eleve.prenom));

  return { appel, lignes };
}

export async function saveAppelLignes(
  etablissementId: string,
  appelId: string,
  lignes: VsAppelLigneInput[],
): Promise<{ saved: number }> {
  const db = getDb();
  const [appel] = await db
    .select()
    .from(vsAppel)
    .where(and(eq(vsAppel.etablissementId, etablissementId), eq(vsAppel.id, appelId)))
    .limit(1);
  if (!appel) throw new Error("Appel introuvable.");
  if (appel.statut === "clos") throw new Error("Appel déjà clos.");

  let saved = 0;
  for (const ligne of lignes) {
    const statut = ligne.statut;
    if (!["present", "absent", "retard", "dispense"].includes(statut)) continue;

    await db
      .insert(vsAppelLigne)
      .values({
        etablissementId,
        appelId,
        eleveId: ligne.eleveId,
        statut,
        retardMinutes: statut === "retard" ? ligne.retardMinutes ?? null : null,
        note: ligne.note?.trim() || null,
        updatedAt: new Date(),
      })
      .onConflictDoUpdate({
        target: [vsAppelLigne.etablissementId, vsAppelLigne.appelId, vsAppelLigne.eleveId],
        set: {
          statut,
          retardMinutes: statut === "retard" ? ligne.retardMinutes ?? null : null,
          note: ligne.note?.trim() || null,
          updatedAt: new Date(),
        },
      });
    saved += 1;

    if (isNonPresent(statut)) {
      await upsertAbsenceFromAppelLigne(etablissementId, appel, ligne.eleveId, statut);
    } else {
      await db
        .delete(vsAbsenceEleve)
        .where(
          and(
            eq(vsAbsenceEleve.etablissementId, etablissementId),
            eq(vsAbsenceEleve.appelId, appelId),
            eq(vsAbsenceEleve.eleveId, ligne.eleveId),
            eq(vsAbsenceEleve.statut, "a_traiter"),
          ),
        );
    }
  }

  await db
    .update(vsAppel)
    .set({ updatedAt: new Date() })
    .where(and(eq(vsAppel.etablissementId, etablissementId), eq(vsAppel.id, appelId)));

  return { saved };
}

async function upsertAbsenceFromAppelLigne(
  etablissementId: string,
  appel: typeof vsAppel.$inferSelect,
  eleveId: string,
  type: "absent" | "retard",
) {
  const db = getDb();
  const absenceType: AbsenceType = type === "retard" ? "retard" : "absence";
  await db
    .insert(vsAbsenceEleve)
    .values({
      etablissementId,
      eleveId,
      appelId: appel.id,
      dateDebut: appel.dateAppel,
      dateFin: appel.dateAppel,
      type: absenceType,
      statut: "a_traiter",
      justifie: false,
    })
    .onConflictDoUpdate({
      target: [vsAbsenceEleve.etablissementId, vsAbsenceEleve.appelId, vsAbsenceEleve.eleveId],
      set: {
        type: absenceType,
        updatedAt: new Date(),
      },
    });
}

export async function closeAppel(etablissementId: string, appelId: string) {
  const db = getDb();
  const [row] = await db
    .update(vsAppel)
    .set({ statut: "clos", closAt: new Date(), updatedAt: new Date() })
    .where(and(eq(vsAppel.etablissementId, etablissementId), eq(vsAppel.id, appelId)))
    .returning();
  return row ?? null;
}

export async function listAbsencesATraiter(
  etablissementId: string,
  opts?: { statut?: AbsenceStatut; limit?: number },
) {
  const db = getDb();
  const statut = opts?.statut || "a_traiter";
  const limit = opts?.limit ?? 100;

  return db
    .select({
      id: vsAbsenceEleve.id,
      eleveId: vsAbsenceEleve.eleveId,
      eleveNom: eleve.nom,
      elevePrenom: eleve.prenom,
      eleveClasse: eleve.classe,
      eleveIne: eleve.ine,
      eleveFolderName: eleve.folderName,
      elevePhotoKey: eleve.photoKey,
      appelId: vsAbsenceEleve.appelId,
      dateDebut: vsAbsenceEleve.dateDebut,
      dateFin: vsAbsenceEleve.dateFin,
      type: vsAbsenceEleve.type,
      statut: vsAbsenceEleve.statut,
      justifie: vsAbsenceEleve.justifie,
      motif: vsAbsenceEleve.motif,
      noteCpe: vsAbsenceEleve.noteCpe,
      relanceAt: vsAbsenceEleve.relanceAt,
      createdAt: vsAbsenceEleve.createdAt,
    })
    .from(vsAbsenceEleve)
    .innerJoin(eleve, eq(eleve.id, vsAbsenceEleve.eleveId))
    .where(
      and(eq(vsAbsenceEleve.etablissementId, etablissementId), eq(vsAbsenceEleve.statut, statut)),
    )
    .orderBy(desc(vsAbsenceEleve.dateDebut), asc(eleve.nom))
    .limit(limit);
}

/** Enrichit les absences avec le rattachement internat (même fiche élève). */
export async function enrichAbsencesWithInternat<
  T extends {
    eleveNom: string;
    elevePrenom: string;
    eleveIne?: string | null;
    eleveFolderName?: string | null;
  },
>(
  absences: T[],
): Promise<
  Array<
    T & {
      interne: boolean;
      internatStudentId: string | null;
      internatRoomId: string | null;
    }
  >
> {
  let roster: InternatStudent[] = [];
  try {
    roster = await getInternatStudents();
  } catch {
    roster = [];
  }

  return absences.map((a) => {
    const match = matchInternatStudent(roster, {
      nom: a.eleveNom,
      prenom: a.elevePrenom,
      ine: a.eleveIne,
      folderName: a.eleveFolderName,
    });
    return {
      ...a,
      interne: Boolean(match),
      internatStudentId: match?.id ?? null,
      internatRoomId: match?.roomId ?? null,
    };
  });
}

export async function updateAbsenceCpe(
  etablissementId: string,
  absenceId: string,
  input: {
    statut?: AbsenceStatut;
    justifie?: boolean;
    motif?: string | null;
    noteCpe?: string | null;
    traiteParUserId?: string | null;
  },
) {
  const db = getDb();
  const patch: Partial<typeof vsAbsenceEleve.$inferInsert> = {
    updatedAt: new Date(),
  };
  if (input.statut) patch.statut = input.statut;
  if (input.justifie != null) patch.justifie = input.justifie;
  if (input.motif !== undefined) patch.motif = input.motif?.trim() || null;
  if (input.noteCpe !== undefined) patch.noteCpe = input.noteCpe?.trim() || null;
  if (input.statut && input.statut !== "a_traiter") {
    patch.traiteAt = new Date();
    patch.traiteParUserId = input.traiteParUserId || null;
  }
  if (input.statut === "a_traiter") {
    patch.traiteAt = null;
    patch.traiteParUserId = null;
  }

  const [row] = await db
    .update(vsAbsenceEleve)
    .set(patch)
    .where(
      and(eq(vsAbsenceEleve.etablissementId, etablissementId), eq(vsAbsenceEleve.id, absenceId)),
    )
    .returning();
  return row ?? null;
}

export async function markAbsenceRelance(etablissementId: string, absenceIds: string[]) {
  if (!absenceIds.length) return 0;
  const db = getDb();
  const result = await db
    .update(vsAbsenceEleve)
    .set({ relanceAt: new Date(), updatedAt: new Date() })
    .where(
      and(
        eq(vsAbsenceEleve.etablissementId, etablissementId),
        inArray(vsAbsenceEleve.id, absenceIds),
      ),
    )
    .returning({ id: vsAbsenceEleve.id });
  return result.length;
}

export async function countAbsencesATraiter(etablissementId: string): Promise<number> {
  const db = getDb();
  const [row] = await db
    .select({ n: sql<number>`count(*)::int` })
    .from(vsAbsenceEleve)
    .where(
      and(
        eq(vsAbsenceEleve.etablissementId, etablissementId),
        eq(vsAbsenceEleve.statut, "a_traiter"),
      ),
    );
  return row?.n ?? 0;
}

/** Absences à traiter avec motif famille (en attente de validation CPE). */
export async function countAbsencesJustifFamilleEnAttente(
  etablissementId: string,
): Promise<number> {
  const db = getDb();
  const [row] = await db
    .select({ n: sql<number>`count(*)::int` })
    .from(vsAbsenceEleve)
    .where(
      and(
        eq(vsAbsenceEleve.etablissementId, etablissementId),
        eq(vsAbsenceEleve.statut, "a_traiter"),
        eq(vsAbsenceEleve.justifie, false),
        sql`nullif(trim(${vsAbsenceEleve.motif}), '') is not null`,
      ),
    );
  return row?.n ?? 0;
}

/** Historique absences / retards d'un élève (dossier). */
export async function listAbsencesForEleve(
  etablissementId: string,
  eleveId: string,
  opts?: { limit?: number },
) {
  const db = getDb();
  const limit = opts?.limit ?? 50;
  return db
    .select({
      id: vsAbsenceEleve.id,
      dateDebut: vsAbsenceEleve.dateDebut,
      dateFin: vsAbsenceEleve.dateFin,
      type: vsAbsenceEleve.type,
      statut: vsAbsenceEleve.statut,
      justifie: vsAbsenceEleve.justifie,
      motif: vsAbsenceEleve.motif,
      noteCpe: vsAbsenceEleve.noteCpe,
      appelId: vsAbsenceEleve.appelId,
      createdAt: vsAbsenceEleve.createdAt,
    })
    .from(vsAbsenceEleve)
    .where(
      and(eq(vsAbsenceEleve.etablissementId, etablissementId), eq(vsAbsenceEleve.eleveId, eleveId)),
    )
    .orderBy(desc(vsAbsenceEleve.dateDebut), desc(vsAbsenceEleve.createdAt))
    .limit(limit);
}

/** Absences / retards sur une date (lien internat + stats). */
export async function listAbsencesForDate(
  etablissementId: string,
  dateIso: string,
): Promise<
  Array<{
    id: string;
    eleveId: string;
    eleveNom: string;
    elevePrenom: string;
    eleveIne: string | null;
    eleveFolderName: string | null;
    type: string;
    statut: string;
    justifie: boolean;
    motif: string | null;
  }>
> {
  const db = getDb();
  return db
    .select({
      id: vsAbsenceEleve.id,
      eleveId: vsAbsenceEleve.eleveId,
      eleveNom: eleve.nom,
      elevePrenom: eleve.prenom,
      eleveIne: eleve.ine,
      eleveFolderName: eleve.folderName,
      type: vsAbsenceEleve.type,
      statut: vsAbsenceEleve.statut,
      justifie: vsAbsenceEleve.justifie,
      motif: vsAbsenceEleve.motif,
    })
    .from(vsAbsenceEleve)
    .innerJoin(eleve, eq(eleve.id, vsAbsenceEleve.eleveId))
    .where(
      and(
        eq(vsAbsenceEleve.etablissementId, etablissementId),
        eq(vsAbsenceEleve.dateDebut, dateIso),
      ),
    )
    .orderBy(asc(eleve.nom), asc(eleve.prenom));
}

function parseHm(raw: string | null | undefined): number | null {
  const m = String(raw || "")
    .trim()
    .match(/^(\d{1,2}):(\d{2})/);
  if (!m) return null;
  return Number(m[1]) * 60 + Number(m[2]);
}

export type AppelManquantRow = {
  creneauId: string;
  classe: string | null;
  heureDebut: string;
  heureFin: string;
  matiereLibelle: string | null;
  enseignantNom: string | null;
  salle: string | null;
  appelId: string | null;
  appelStatut: string | null;
};

/**
 * Créneaux du jour déjà commencés sans appel clôturé
 * (équivalent « fautes enseignants » Charlemagne, vocabulaire non punitif).
 */
export async function listAppelsManquants(
  etablissementId: string,
  opts?: { dateAppel?: string; nowMinutes?: number },
): Promise<AppelManquantRow[]> {
  const { listEdtCreneauxForJour, jourSemaineFromIsoDate } = await import(
    "@/app/lib/vs-calendrier-db"
  );
  const { parisDateKey, getParisParts } = await import("@/app/lib/paris-time");

  const dateAppel = opts?.dateAppel || parisDateKey(new Date());
  const parts = getParisParts(new Date());
  const nowMinutes = opts?.nowMinutes ?? parts.hour * 60 + parts.minute;
  const jour = jourSemaineFromIsoDate(dateAppel);

  const creneaux = await listEdtCreneauxForJour(etablissementId, jour);
  const started = creneaux.filter((c) => {
    const start = parseHm(c.heureDebut);
    const hasScope = Boolean(c.classe?.trim()) || Boolean(c.groupeId);
    return start != null && start <= nowMinutes && hasScope;
  });
  if (!started.length) return [];

  const db = getDb();
  const appels = await db
    .select({
      id: vsAppel.id,
      creneauId: vsAppel.creneauId,
      statut: vsAppel.statut,
    })
    .from(vsAppel)
    .where(and(eq(vsAppel.etablissementId, etablissementId), eq(vsAppel.dateAppel, dateAppel)));

  const byCreneau = new Map(
    appels.filter((a) => a.creneauId).map((a) => [a.creneauId as string, a]),
  );

  const manquants: AppelManquantRow[] = [];
  for (const c of started) {
    const appel = byCreneau.get(c.id);
    if (appel?.statut === "clos") continue;
    manquants.push({
      creneauId: c.id,
      classe: c.classe || c.groupeCode,
      heureDebut: c.heureDebut,
      heureFin: c.heureFin,
      matiereLibelle: c.matiereLibelle,
      enseignantNom: c.enseignantNom,
      salle: c.salle,
      appelId: appel?.id ?? null,
      appelStatut: appel?.statut ?? null,
    });
  }
  return manquants;
}

export async function countAppelsManquants(etablissementId: string): Promise<number> {
  const rows = await listAppelsManquants(etablissementId);
  return rows.length;
}
