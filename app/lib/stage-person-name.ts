/**
 * Normalisation des noms pour le matching stages (roster, référents, identité).
 * Aligné sur l’internat : apostrophes retirées pour fusionner N'SONI / NSONI.
 */

/** Apostrophes / quotes typographiques à ignorer (N'SONI → nsoni). */
const APOSTROPHE_RE = /[\u0027\u2018\u2019\u0060\u00b4\u02bc]/g;

export function normalizeStagePersonName(str: string | undefined | null): string {
  return String(str ?? "")
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(APOSTROPHE_RE, "")
    .replace(/[-\s]+/g, " ")
    .trim();
}

export function stagePersonNamesMatch(
  a: { nom: string; prenom: string },
  b: { lastName: string; firstName: string },
): boolean {
  return (
    normalizeStagePersonName(a.nom) === normalizeStagePersonName(b.lastName) &&
    normalizeStagePersonName(a.prenom) === normalizeStagePersonName(b.firstName)
  );
}

/**
 * Compare le libellé index (`prénom nom`, prénoms composés inclus) à une fiche
 * élève. Évite de couper « Dane Junior NSONI » en prénom=Dane / nom=Junior NSONI.
 */
export function stageStudentNameMatchesEleve(
  studentName: string,
  eleve: { nom: string; prenom: string },
): boolean {
  const blob = normalizeStagePersonName(studentName);
  if (!blob) return false;
  const prenomNom = normalizeStagePersonName(`${eleve.prenom} ${eleve.nom}`);
  const nomPrenom = normalizeStagePersonName(`${eleve.nom} ${eleve.prenom}`);
  return blob === prenomNom || blob === nomPrenom;
}

/**
 * Décide si une convention sans élève dans la classe cible doit créer une ligne
 * « orpheline » dans le suivi, ou être ignorée.
 *
 * - Élève actif dans une autre classe (ex. Apolline Seglas en 2nde sur une
 *   convention encore taguée 3e) → skip
 * - Élève connu mais sorti (ex. Sohan Pitte) → skip
 * - Aucune fiche registre → orphelin (typo / parent saisi par erreur, à nettoyer)
 */
export type OrphanConventionDisposition = "create_orphan" | "skip_other_class" | "skip_sorti";

export function orphanConventionDisposition(params: {
  /** Présent dans le registre des actifs, mais pas dans la classe consultée. */
  actifInOtherClass: boolean;
  /** Présent dans le registre complet, absent des actifs (sorti / ancien). */
  knownButNotActif: boolean;
}): OrphanConventionDisposition {
  if (params.actifInOtherClass) return "skip_other_class";
  if (params.knownButNotActif) return "skip_sorti";
  return "create_orphan";
}
