import "server-only";

import { randomUUID } from "crypto";
import { getDb } from "@/db/index";
import { eleve, eleveScolarite } from "@/db/schema";
import { buildEleveFolderName } from "@/app/lib/eleves-config";
import { ensureEleveFoyerFromParentContacts } from "@/app/lib/ent-core-db";
import { isValidParentEmail } from "@/app/lib/eleves-parent-emails";

export const MAX_PREINSCRIT_PARENTS = 4;

export type PreinscritParentContact = {
  firstName?: string | null;
  lastName?: string | null;
  email?: string | null;
  phone?: string | null;
};

export type CreateElevePreinscritInput = {
  etablissementId: string;
  nom: string;
  prenom: string;
  /**
   * Contact principal (rétrocompat RDV / anciens appels).
   * Si `parents` est fourni, le 1er e-mail valide de la liste prime.
   */
  parentEmail?: string | null;
  parentPhone?: string | null;
  /** Identité du parent (peut différer du nom de l’élève). */
  parentFirstName?: string | null;
  parentLastName?: string | null;
  /**
   * Jusqu’à 4 personnes du foyer. Chaque e-mail est reconnu pour le matching
   * RDV inscription (gate e-mail → pool enfants liés).
   */
  parents?: PreinscritParentContact[];
  classe?: string | null;
  siteId?: string | null;
  /** Prefixe source_key (rdv-inscription | manuel | …). */
  sourcePrefix?: string;
};

function normalizeParents(
  input: CreateElevePreinscritInput,
): Array<{
  firstName: string | null;
  lastName: string | null;
  email: string | null;
  phone: string | null;
}> {
  const fromList = (input.parents || [])
    .map((p) => ({
      firstName: String(p.firstName || "").trim() || null,
      lastName: String(p.lastName || "").trim() || null,
      email: String(p.email || "").trim().toLowerCase() || null,
      phone: String(p.phone || "").trim() || null,
    }))
    .filter((p) => p.firstName || p.lastName || p.email || p.phone)
    .slice(0, MAX_PREINSCRIT_PARENTS);

  if (fromList.length > 0) return fromList;

  const email = String(input.parentEmail || "").trim().toLowerCase() || null;
  const phone = String(input.parentPhone || "").trim() || null;
  const firstName = String(input.parentFirstName || "").trim() || null;
  const lastName = String(input.parentLastName || "").trim() || null;
  if (!email && !phone && !firstName && !lastName) return [];
  return [{ firstName, lastName, email, phone }];
}

/**
 * Crée un élève `preinscrit` avec contacts foyer (requis pour le matching RDV).
 * Jusqu’à 4 responsables : e-mails stockés sur la fiche (parent1/2) + foyer_responsable.
 */
export async function createElevePreinscrit(
  input: CreateElevePreinscritInput,
): Promise<{ id: string; nom: string; prenom: string }> {
  const nom = input.nom.trim();
  const prenom = input.prenom.trim();
  const classe = input.classe?.trim() || null;
  const siteId = input.siteId?.trim() || null;
  const parents = normalizeParents(input);

  if (!nom || !prenom) {
    throw new Error("Nom et prénom requis.");
  }
  if (parents.length === 0) {
    throw new Error("Au moins un contact foyer (e-mail parent) est requis.");
  }

  for (let i = 0; i < parents.length; i++) {
    const email = parents[i]!.email;
    if (email && !isValidParentEmail(email)) {
      throw new Error(`E-mail parent ${i + 1} invalide.`);
    }
  }

  const primaryEmail = parents.find((p) => p.email)?.email || null;
  if (!primaryEmail || !isValidParentEmail(primaryEmail)) {
    throw new Error("Au moins un e-mail parent valide est requis.");
  }

  const parent1 = parents[0]!;
  const parent2 = parents[1] || null;
  const parent1Email = parent1.email || primaryEmail;
  const parent1Phone = parent1.phone;
  const parent2Email = parent2?.email || null;
  const parent2Phone = parent2?.phone || null;

  const db = getDb();
  const id = randomUUID();
  const prefix = (input.sourcePrefix || "manuel").trim() || "manuel";
  const folderName = buildEleveFolderName(nom, prenom);

  await db.insert(eleve).values({
    id,
    etablissementId: input.etablissementId,
    sourceKey: `${prefix}:${id}`,
    nom,
    prenom,
    folderName,
    classe,
    parentEmail: parent1Email,
    parentPhone: parent1Phone,
    parent1Email,
    parent1Phone,
    parent2Email,
    parent2Phone,
    status: "preinscrit",
    secteur: siteId,
  });

  await db.insert(eleveScolarite).values({
    etablissementId: input.etablissementId,
    eleveId: id,
    siteId,
    classe,
    statut: "prevue",
  });

  try {
    await ensureEleveFoyerFromParentContacts(input.etablissementId, id, {
      nom,
      prenom,
      parentEmail: parent1Email,
      parentPhone: parent1Phone,
      parent1Email,
      parent1Phone,
      parent2Email,
      parent2Phone,
      parentFirstName: parent1.firstName,
      parentLastName: parent1.lastName,
      responsables: parents.map((p) => ({
        prenom: p.firstName,
        nom: p.lastName,
        email: p.email,
        telephone: p.phone,
      })),
    });
  } catch (e) {
    console.error("[createElevePreinscrit] foyer:", e);
  }

  return { id, nom, prenom };
}
