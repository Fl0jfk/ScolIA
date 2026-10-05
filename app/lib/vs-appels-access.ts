import "server-only";

import { isAnyDirectionRole } from "@/app/lib/establishment-catalog";
import { hasGlobalAdminRole, hasRole } from "@/app/lib/intranet-role-utils";
import { vsAppel } from "@/db/schema";
import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";

export type EdtCreneauForAppel = {
  id: string;
  enseignantNom?: string | null;
  classe?: string | null;
  groupeId?: string | null;
};

function normalizePersonName(raw: string): string {
  return raw
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, " ")
    .trim();
}

/** Prof « pur » : pas direction / CPE / accueil / admin établissement. */
export function isProfesseurScopedAppelActor(opts: {
  roles: string[];
  isOrgAdmin: boolean;
}): boolean {
  if (opts.isOrgAdmin || hasGlobalAdminRole(opts.roles)) return false;
  if (isAnyDirectionRole(opts.roles)) return false;
  if (
    hasRole(opts.roles, "cpe") ||
    hasRole(opts.roles, "surveillant") ||
    hasRole(opts.roles, "administratif") ||
    hasRole(opts.roles, "accueil") ||
    hasRole(opts.roles, "admin")
  ) {
    return false;
  }
  return hasRole(opts.roles, "professeur");
}

export function professeurOwnsCreneau(
  creneau: EdtCreneauForAppel,
  actor: { userId: string; displayName: string },
): boolean {
  const target = normalizePersonName(creneau.enseignantNom || "");
  const actorN = normalizePersonName(actor.displayName);
  if (!target || !actorN) return false;
  return actorN === target || actorN.includes(target) || target.includes(actorN);
}

export async function assertProfesseurCanAccessAppel(
  etablissementId: string,
  appelId: string,
  actor: { userId: string; displayName: string; roles: string[]; isOrgAdmin: boolean },
): Promise<void> {
  if (!isProfesseurScopedAppelActor(actor)) return;

  const db = getDb();
  const [appel] = await db
    .select({
      enseignantUserId: vsAppel.enseignantUserId,
      enseignantNom: vsAppel.enseignantNom,
    })
    .from(vsAppel)
    .where(and(eq(vsAppel.etablissementId, etablissementId), eq(vsAppel.id, appelId)))
    .limit(1);
  if (!appel) throw new Error("Appel introuvable.");

  if (appel.enseignantUserId && appel.enseignantUserId !== actor.userId) {
    throw new Error("Cet appel appartient à un autre enseignant.");
  }
  if (appel.enseignantNom && !professeurOwnsCreneau({ id: "", enseignantNom: appel.enseignantNom }, actor)) {
    throw new Error("Cet appel n’est pas rattaché à vos créneaux.");
  }
}

export function filterCreneauxForProfesseur<
  T extends EdtCreneauForAppel,
>(creneaux: T[], actor: { displayName: string; roles: string[]; isOrgAdmin: boolean }): T[] {
  if (!isProfesseurScopedAppelActor(actor)) return creneaux;
  return creneaux.filter((c) => professeurOwnsCreneau(c, { userId: "", displayName: actor.displayName }));
}
