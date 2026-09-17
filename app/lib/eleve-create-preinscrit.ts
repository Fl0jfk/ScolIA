import "server-only";

import { randomUUID } from "crypto";
import { getDb } from "@/db/index";
import { eleve, eleveScolarite } from "@/db/schema";
import { buildEleveFolderName } from "@/app/lib/eleves-config";
import { ensureEleveFoyerFromParentContacts } from "@/app/lib/ent-core-db";
import { isValidParentEmail } from "@/app/lib/eleves-parent-emails";

export type CreateElevePreinscritInput = {
  etablissementId: string;
  nom: string;
  prenom: string;
  parentEmail: string;
  parentPhone?: string | null;
  classe?: string | null;
  siteId?: string | null;
  /** Prefixe source_key (rdv-inscription | manuel | …). */
  sourcePrefix?: string;
};

/**
 * Crée un élève `preinscrit` avec contacts parent (requis pour le matching RDV).
 */
export async function createElevePreinscrit(
  input: CreateElevePreinscritInput,
): Promise<{ id: string; nom: string; prenom: string }> {
  const nom = input.nom.trim();
  const prenom = input.prenom.trim();
  const parentEmail = input.parentEmail.trim().toLowerCase();
  const parentPhone = String(input.parentPhone || "").trim();
  const classe = input.classe?.trim() || null;
  const siteId = input.siteId?.trim() || null;

  if (!nom || !prenom) {
    throw new Error("Nom et prénom requis.");
  }
  if (!isValidParentEmail(parentEmail)) {
    throw new Error("E-mail parent invalide.");
  }

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
    parentEmail,
    parentPhone: parentPhone || null,
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
      parentEmail,
      parentPhone: parentPhone || null,
    });
  } catch (e) {
    console.error("[createElevePreinscrit] foyer:", e);
  }

  return { id, nom, prenom };
}
