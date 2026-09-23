import "server-only";

import { and, asc, desc, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { paieElement, paiePeriode, personnel } from "@/db/schema";

const NATURES = ["absence", "heure", "prime", "retenue"] as const;

function isNature(v: string): v is (typeof NATURES)[number] {
  return (NATURES as readonly string[]).includes(v);
}

export async function listPaiePeriodes(etablissementId: string) {
  const db = getDb();
  return db
    .select()
    .from(paiePeriode)
    .where(eq(paiePeriode.etablissementId, etablissementId))
    .orderBy(desc(paiePeriode.dateDebut));
}

export async function createPaiePeriode(
  etablissementId: string,
  input: { label: string; dateDebut: string; dateFin: string },
) {
  const db = getDb();
  const label = input.label.trim();
  if (!label) throw new Error("Libellé période obligatoire.");
  const dateDebut = input.dateDebut.slice(0, 10);
  const dateFin = input.dateFin.slice(0, 10);
  if (dateFin < dateDebut) throw new Error("Date de fin avant la date de début.");

  const [row] = await db
    .insert(paiePeriode)
    .values({
      etablissementId,
      label: label.slice(0, 120),
      dateDebut,
      dateFin,
      statut: "brouillon",
    })
    .returning();
  if (!row) throw new Error("Création période impossible.");
  return row;
}

export async function figerPaiePeriode(etablissementId: string, periodeId: string) {
  const db = getDb();
  const [row] = await db
    .update(paiePeriode)
    .set({ statut: "figee", updatedAt: new Date() })
    .where(and(eq(paiePeriode.etablissementId, etablissementId), eq(paiePeriode.id, periodeId)))
    .returning();
  if (!row) throw new Error("Période introuvable.");
  return row;
}

export async function listPaieElements(etablissementId: string, periodeId: string) {
  const db = getDb();
  return db
    .select({
      id: paieElement.id,
      periodeId: paieElement.periodeId,
      personnelId: paieElement.personnelId,
      nature: paieElement.nature,
      libelle: paieElement.libelle,
      quantite: paieElement.quantite,
      montant: paieElement.montant,
      absenceId: paieElement.absenceId,
      createdAt: paieElement.createdAt,
      personnelNom: personnel.lastName,
      personnelPrenom: personnel.firstName,
      personnelDisplay: personnel.displayName,
    })
    .from(paieElement)
    .leftJoin(personnel, eq(personnel.id, paieElement.personnelId))
    .where(
      and(eq(paieElement.etablissementId, etablissementId), eq(paieElement.periodeId, periodeId)),
    )
    .orderBy(asc(personnel.lastName), asc(paieElement.nature));
}

export async function createPaieElement(
  etablissementId: string,
  input: {
    periodeId: string;
    personnelId: string;
    nature: string;
    libelle: string;
    quantite?: string | number | null;
    montant?: string | number | null;
    absenceId?: string | null;
  },
) {
  const db = getDb();
  const [periode] = await db
    .select()
    .from(paiePeriode)
    .where(
      and(eq(paiePeriode.etablissementId, etablissementId), eq(paiePeriode.id, input.periodeId)),
    )
    .limit(1);
  if (!periode) throw new Error("Période introuvable.");
  if (periode.statut === "figee") throw new Error("Période figée — modification impossible.");

  if (!isNature(input.nature)) throw new Error("Nature invalide (absence|heure|prime|retenue).");
  const libelle = input.libelle.trim();
  if (!libelle) throw new Error("Libellé obligatoire.");
  const personnelId = input.personnelId.trim();
  if (!personnelId) throw new Error("Personnel obligatoire.");

  const [row] = await db
    .insert(paieElement)
    .values({
      etablissementId,
      periodeId: input.periodeId,
      personnelId,
      nature: input.nature,
      libelle: libelle.slice(0, 200),
      quantite:
        input.quantite != null && String(input.quantite).trim() !== ""
          ? String(Number(input.quantite))
          : null,
      montant:
        input.montant != null && String(input.montant).trim() !== ""
          ? Number(input.montant).toFixed(2)
          : null,
      absenceId: input.absenceId || null,
    })
    .returning();
  if (!row) throw new Error("Création élément impossible.");
  return row;
}

export async function listPersonnelLight(etablissementId: string) {
  const db = getDb();
  return db
    .select({
      id: personnel.id,
      nom: personnel.lastName,
      prenom: personnel.firstName,
      displayName: personnel.displayName,
      category: personnel.category,
    })
    .from(personnel)
    .where(and(eq(personnel.etablissementId, etablissementId), eq(personnel.active, true)))
    .orderBy(asc(personnel.lastName), asc(personnel.firstName))
    .limit(200);
}

/** Crée un agent de démo si aucun personnel (labo local / Value Gate). */
export async function ensurePersonnelDemo(etablissementId: string) {
  const existing = await listPersonnelLight(etablissementId);
  if (existing.length) return existing;
  const db = getDb();
  const id = `paie-demo-${etablissementId.slice(0, 8)}`;
  await db
    .insert(personnel)
    .values({
      id,
      etablissementId,
      email: "agent.demo@localhost.dev",
      firstName: "Alice",
      lastName: "DEMO",
      displayName: "Alice DEMO",
      category: "enseignant",
      jobTitle: "Professeur",
      active: true,
    })
    .onConflictDoNothing();
  return listPersonnelLight(etablissementId);
}

export async function loadPaieHub(etablissementId: string, periodeId?: string) {
  const personnelList = await ensurePersonnelDemo(etablissementId);
  const periodes = await listPaiePeriodes(etablissementId);
  const selectedId = periodeId || periodes[0]?.id;
  const elements = selectedId ? await listPaieElements(etablissementId, selectedId) : [];
  return {
    periodes,
    personnel: personnelList,
    periodeId: selectedId || null,
    elements,
  };
}
