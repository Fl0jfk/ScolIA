import "server-only";

import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  eleve,
  eleveDocument,
  eleveFoyerLink,
  eleveScolarite,
  factureLigne,
  fdCampagne,
  fdFiche,
  groupePedagogiqueMembre,
  noteCompetenceValeur,
  noteMoyenneEleve,
  noteValeur,
  preinscription,
  rdvInscriptionBooking,
  vsAbsenceEleve,
  vsAppelLigne,
  vsCarnetEntree,
  vsSanction,
} from "@/db/schema";
import { resolveEleveDocumentS3Key } from "@/app/lib/eleve-document-file";
import { deleteObject, listPrefix } from "@/app/lib/s3-storage";
import { cacheInvalidateElevesDossiers } from "@/app/lib/valkey-cache";
import { invalidateElevesRegistryCache } from "@/app/lib/eleves-registry";
import { hasGlobalAdminRole, INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";
import {
  ELEVE_DELETE_PERMANENT_CONFIRM_WORD,
  isEleveDeletePermanentConfirmation,
} from "@/app/lib/eleve-delete-permanent-confirm";

export { ELEVE_DELETE_PERMANENT_CONFIRM_WORD, isEleveDeletePermanentConfirmation };

export function canDeleteElevePermanently(
  roles: string[],
  opts: { orgAdmin?: boolean; platformAdmin?: boolean },
): boolean {
  if (opts.orgAdmin || opts.platformAdmin) return true;
  if (roles.includes("admin") || hasGlobalAdminRole(roles)) return true;
  return INTRANET_DIRECTION_SLUGS.some((slug) => roles.includes(slug));
}

export type DeleteElevePermanentResult = {
  deletedId: string;
  nom: string;
  prenom: string;
  documentsRemoved: number;
  s3ObjectsRemoved: number;
};

/**
 * Efface complètement un élève (fiche + traces liées), pas une sortie / statut « ancien ».
 * Ciblé : un seul `id` + `etablissement_id`. Les factures foyer sont conservées
 * (`facture_ligne.eleve_id` mis à null). Les foyers orphelins restent (facturation).
 */
export async function deleteElevePermanently(input: {
  etablissementId: string;
  eleveId: string;
}): Promise<DeleteElevePermanentResult> {
  const etabId = input.etablissementId;
  const eleveId = input.eleveId;
  const db = getDb();

  const [row] = await db
    .select({
      id: eleve.id,
      nom: eleve.nom,
      prenom: eleve.prenom,
      photoKey: eleve.photoKey,
    })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etabId), eq(eleve.id, eleveId)))
    .limit(1);

  if (!row) {
    throw new EleveDeleteNotFoundError();
  }

  const docs = await db
    .select({
      id: eleveDocument.id,
      s3Key: eleveDocument.s3Key,
      fileUrl: eleveDocument.fileUrl,
    })
    .from(eleveDocument)
    .where(and(eq(eleveDocument.etablissementId, etabId), eq(eleveDocument.eleveId, eleveId)));

  const s3KeysToDelete = new Set<string>();
  for (const doc of docs) {
    const key = await resolveEleveDocumentS3Key(doc).catch(() => null);
    if (key) s3KeysToDelete.add(key);
  }
  if (row.photoKey?.trim()) {
    s3KeysToDelete.add(row.photoKey.trim());
  }

  try {
    const prefixKeys = await listPrefix(`eleves-dossier/${eleveId}/`);
    for (const k of prefixKeys) {
      if (k) s3KeysToDelete.add(k);
    }
  } catch (err) {
    console.warn("[eleve-delete-permanent] listPrefix eleves-dossier", err);
  }

  await db.transaction(async (tx) => {
    // Tables sans FK Drizzle / sans cascade SQL garantie — avant DELETE eleve.
    await tx
      .delete(groupePedagogiqueMembre)
      .where(
        and(
          eq(groupePedagogiqueMembre.etablissementId, etabId),
          eq(groupePedagogiqueMembre.eleveId, eleveId),
        ),
      );
    await tx
      .delete(noteValeur)
      .where(and(eq(noteValeur.etablissementId, etabId), eq(noteValeur.eleveId, eleveId)));
    await tx
      .delete(noteMoyenneEleve)
      .where(
        and(eq(noteMoyenneEleve.etablissementId, etabId), eq(noteMoyenneEleve.eleveId, eleveId)),
      );
    await tx
      .delete(noteCompetenceValeur)
      .where(
        and(
          eq(noteCompetenceValeur.etablissementId, etabId),
          eq(noteCompetenceValeur.eleveId, eleveId),
        ),
      );
    await tx
      .delete(fdFiche)
      .where(and(eq(fdFiche.etablissementId, etabId), eq(fdFiche.eleveId, eleveId)));
    await tx
      .delete(rdvInscriptionBooking)
      .where(
        and(
          eq(rdvInscriptionBooking.etablissementId, etabId),
          eq(rdvInscriptionBooking.eleveId, eleveId),
        ),
      );

    // Conserver les lignes de facture : détacher l’élève seulement.
    await tx
      .update(factureLigne)
      .set({ eleveId: null })
      .where(and(eq(factureLigne.etablissementId, etabId), eq(factureLigne.eleveId, eleveId)));

    // Préinscription : détacher (SET NULL côté schéma, on le fait explicitement).
    await tx
      .update(preinscription)
      .set({ eleveId: null, updatedAt: new Date() })
      .where(and(eq(preinscription.etablissementId, etabId), eq(preinscription.eleveId, eleveId)));

    // Campagnes fiches dialogue : retirer l’id du tableau JSON cibles.
    const campagnes = await tx
      .select({ id: fdCampagne.id, eleveIdsCibles: fdCampagne.eleveIdsCibles })
      .from(fdCampagne)
      .where(eq(fdCampagne.etablissementId, etabId));
    for (const c of campagnes) {
      const ids = Array.isArray(c.eleveIdsCibles) ? c.eleveIdsCibles : [];
      if (!ids.includes(eleveId)) continue;
      await tx
        .update(fdCampagne)
        .set({
          eleveIdsCibles: ids.filter((x) => x !== eleveId),
          updatedAt: new Date(),
        })
        .where(and(eq(fdCampagne.etablissementId, etabId), eq(fdCampagne.id, c.id)));
    }

    // VS : cascade SQL en prod, mais on nettoie explicitement pour cohérence schéma Drizzle.
    await tx
      .delete(vsAppelLigne)
      .where(and(eq(vsAppelLigne.etablissementId, etabId), eq(vsAppelLigne.eleveId, eleveId)));
    await tx
      .delete(vsAbsenceEleve)
      .where(and(eq(vsAbsenceEleve.etablissementId, etabId), eq(vsAbsenceEleve.eleveId, eleveId)));
    await tx
      .delete(vsSanction)
      .where(and(eq(vsSanction.etablissementId, etabId), eq(vsSanction.eleveId, eleveId)));
    await tx
      .delete(vsCarnetEntree)
      .where(and(eq(vsCarnetEntree.etablissementId, etabId), eq(vsCarnetEntree.eleveId, eleveId)));

    // Cascades schéma : documents, scolarité, foyer links — puis la fiche.
    await tx
      .delete(eleveDocument)
      .where(and(eq(eleveDocument.etablissementId, etabId), eq(eleveDocument.eleveId, eleveId)));
    await tx
      .delete(eleveScolarite)
      .where(and(eq(eleveScolarite.etablissementId, etabId), eq(eleveScolarite.eleveId, eleveId)));
    await tx
      .delete(eleveFoyerLink)
      .where(and(eq(eleveFoyerLink.etablissementId, etabId), eq(eleveFoyerLink.eleveId, eleveId)));

    const deleted = await tx
      .delete(eleve)
      .where(and(eq(eleve.etablissementId, etabId), eq(eleve.id, eleveId)))
      .returning({ id: eleve.id });

    if (!deleted.length) {
      throw new EleveDeleteNotFoundError();
    }
  });

  let s3ObjectsRemoved = 0;
  for (const key of s3KeysToDelete) {
    try {
      await deleteObject(key);
      s3ObjectsRemoved += 1;
    } catch (err) {
      console.warn("[eleve-delete-permanent] delete S3", key, err);
    }
  }

  await Promise.all([
    cacheInvalidateElevesDossiers(etabId),
    invalidateElevesRegistryCache(etabId),
  ]);

  return {
    deletedId: row.id,
    nom: row.nom,
    prenom: row.prenom,
    documentsRemoved: docs.length,
    s3ObjectsRemoved,
  };
}

export class EleveDeleteNotFoundError extends Error {
  constructor() {
    super("Élève introuvable.");
    this.name = "EleveDeleteNotFoundError";
  }
}
