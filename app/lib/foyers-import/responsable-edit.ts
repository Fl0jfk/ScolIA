import "server-only";

import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleveFoyerLink, foyerResponsable } from "@/db/schema";
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
};

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
  const changed = JSON.stringify(prev) !== JSON.stringify(after);
  if (changed) {
    await db
      .update(foyerResponsable)
      .set({ ...after, updatedAt: new Date() })
      .where(
        and(
          eq(foyerResponsable.id, opts.responsableId),
          eq(foyerResponsable.etablissementId, opts.etablissementId),
        ),
      );
    const [link] = await db
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
      metadata: { foyerId: before.foyerId, avant: prev, apres: after, compteFamille: !!before.userId },
    });
  }
  return {
    responsable: { id: before.id, foyerId: before.foyerId, ...after },
    compteFamille: !!before.userId,
    changed,
  };
}
