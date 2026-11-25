import "server-only";

import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleveFoyerLink, foyerResponsable, foyerResponsableLien } from "@/db/schema";
import { isMissingRelationError } from "./lien-parente";
import { recordEleveAccessAudit } from "@/app/lib/eleve-dossier-access";
import type { ResponsableInput } from "./responsable-validate";

export type ResponsableUpdateResult = {
  responsable: {
    id: string;
    foyerId: string;
    nom: string;
    prenom: string;
    email: string | null;
    telephone: string | null;
    autoriteParentale: boolean;
  };
  /** Le responsable se connecte avec un compte famille (userId inchangé). */
  compteFamille: boolean;
  changed: boolean;
  /** Lien de parenté enregistré pour l'élève demandé (null = aucun). Absent si non demandé. */
  lien?: string | null;
};

/** Le lien demandé ne peut pas être enregistré (élève sans rapport avec le foyer, ou migration absente). */
export class LienParenteError extends Error {
  constructor(
    message: string,
    readonly status: 400 | 409,
  ) {
    super(message);
  }
}

/**
 * Modifie nom / prénom / e-mail / téléphone (et autorité parentale) d'un responsable de foyer.
 * N'écrit que dans foyer_responsable (+ journal d'accès) : jamais `eleve`, jamais `userId`.
 */
export async function updateFoyerResponsable(opts: {
  etablissementId: string;
  actorUserId: string | null;
  responsableId: string;
  input: ResponsableInput;
}): Promise<ResponsableUpdateResult | null> {
  const db = getDb();
  const [before] = await db
    .select()
    .from(foyerResponsable)
    .where(
      and(
        eq(foyerResponsable.id, opts.responsableId),
        eq(foyerResponsable.etablissementId, opts.etablissementId),
      ),
    )
    .limit(1);
  if (!before) return null;

  // Lien de parenté avec l'élève du dossier (le lien dépend du couple responsable / enfant).
  const wantsLien = opts.input.lien !== undefined && !!opts.input.eleveId;
  let lienAvant: string | null = null;
  let lienApres: string | null = null;
  if (wantsLien) {
    const eleveId = opts.input.eleveId!;
    const [attached] = await db
      .select({ eleveId: eleveFoyerLink.eleveId })
      .from(eleveFoyerLink)
      .where(
        and(
          eq(eleveFoyerLink.etablissementId, opts.etablissementId),
          eq(eleveFoyerLink.foyerId, before.foyerId),
          eq(eleveFoyerLink.eleveId, eleveId),
        ),
      )
      .limit(1);
    if (!attached) throw new LienParenteError("Cet élève n'est pas rattaché à ce foyer.", 400);
    try {
      const [cur] = await db
        .select({ lien: foyerResponsableLien.lien })
        .from(foyerResponsableLien)
        .where(
          and(
            eq(foyerResponsableLien.etablissementId, opts.etablissementId),
            eq(foyerResponsableLien.responsableId, before.id),
            eq(foyerResponsableLien.eleveId, eleveId),
          ),
        )
        .limit(1);
      lienAvant = cur?.lien ?? null;
    } catch (err) {
      if (isMissingRelationError(err)) {
        throw new LienParenteError("Le lien de parenté n'est pas encore disponible (mise à jour de la base en attente).", 409);
      }
      throw err;
    }
    lienApres = opts.input.lien ? opts.input.lien : null;
  }
  const lienChanged = wantsLien && lienAvant !== lienApres;

  const after = {
    nom: opts.input.nom,
    prenom: opts.input.prenom,
    email: opts.input.email,
    telephone: opts.input.telephone,
    autoriteParentale: opts.input.autoriteParentale ?? before.autoriteParentale,
  };
  const prev = {
    nom: before.nom,
    prenom: before.prenom,
    email: before.email,
    telephone: before.telephone,
    autoriteParentale: before.autoriteParentale,
  };
  const respChanged = JSON.stringify(prev) !== JSON.stringify(after);
  const changed = respChanged || lienChanged;
  if (lienChanged) {
    const eleveId = opts.input.eleveId!;
    if (lienApres === null) {
      await db
        .delete(foyerResponsableLien)
        .where(
          and(
            eq(foyerResponsableLien.etablissementId, opts.etablissementId),
            eq(foyerResponsableLien.responsableId, before.id),
            eq(foyerResponsableLien.eleveId, eleveId),
          ),
        );
    } else {
      await db
        .insert(foyerResponsableLien)
        .values({ etablissementId: opts.etablissementId, responsableId: before.id, eleveId, lien: lienApres })
        .onConflictDoUpdate({
          target: [foyerResponsableLien.responsableId, foyerResponsableLien.eleveId],
          set: { lien: lienApres, updatedAt: new Date() },
        });
    }
  }
  if (changed) {
    if (respChanged) await db
      .update(foyerResponsable)
      .set({ ...after, updatedAt: new Date() })
      .where(
        and(
          eq(foyerResponsable.id, opts.responsableId),
          eq(foyerResponsable.etablissementId, opts.etablissementId),
        ),
      );
    const [link] = opts.input.eleveId ? [{ eleveId: opts.input.eleveId }] : await db
      .select({ eleveId: eleveFoyerLink.eleveId })
      .from(eleveFoyerLink)
      .where(
        and(
          eq(eleveFoyerLink.etablissementId, opts.etablissementId),
          eq(eleveFoyerLink.foyerId, before.foyerId),
        ),
      )
      .limit(1);
    await recordEleveAccessAudit({
      etablissementId: opts.etablissementId,
      actorUserId: opts.actorUserId,
      resourceType: "foyer_responsable",
      resourceId: before.id,
      eleveId: link?.eleveId ?? null,
      action: "update",
      metadata: {
        foyerId: before.foyerId,
        avant: wantsLien ? { ...prev, lien: lienAvant } : prev,
        apres: wantsLien ? { ...after, lien: lienApres } : after,
        compteFamille: !!before.userId,
      },
    });
  }
  return {
    responsable: { id: before.id, foyerId: before.foyerId, ...after },
    compteFamille: !!before.userId,
    changed,
    ...(wantsLien ? { lien: lienApres } : {}),
  };
}
