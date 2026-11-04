import { nameKey } from "@/app/lib/personnel-birthdate-import";

/** Clé de nom (prénom + nom) insensible à la casse, aux accents, aux espaces et tirets. */
export function personNameKey(firstName: string | null | undefined, lastName: string | null | undefined): string {
  return nameKey(firstName ?? "", lastName ?? "");
}

export type FicheLite = {
  active: boolean;
  keys: string[];
  nameKey: string;
  jobTitle?: string | null;
};

/**
 * Fiche liée à un compte : d'abord par clés (id / e-mail), en préférant une fiche active ;
 * sinon repli par nom normalisé (uniquement parmi les fiches actives, si exactement une correspond).
 */
export function findFicheForAccount<F extends FicheLite>(
  account: { keys: string[]; nameKey: string },
  fiches: readonly F[],
): F | undefined {
  const byKeys = fiches.filter((f) => f.keys.some((k) => account.keys.includes(k)));
  if (byKeys.length > 0) return byKeys.find((f) => f.active) ?? byKeys[0];
  if (!account.nameKey) return undefined;
  const byName = fiches.filter((f) => f.active && f.nameKey === account.nameKey);
  return byName.length === 1 ? byName[0] : undefined;
}

/** Une fiche non rattachée est un doublon d'un compte déjà listé si leur nom normalisé est identique. */
export function isFicheDuplicateByName(ficheNameKey: string, listedAccountNameKeys: ReadonlySet<string>): boolean {
  return !!ficheNameKey && listedAccountNameKeys.has(ficheNameKey);
}
