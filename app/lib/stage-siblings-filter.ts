import type { EleveConfig } from "@/app/lib/eleves-config";
import { isEleveActifPourListes } from "@/app/lib/eleve-actif-shared";
import { normMefCode } from "@/app/lib/mef-secteurs";
import { inferSecteurFromFolderName, type Secteur } from "@/app/lib/onedrive-eleves";

export type StageSibling = {
  id: string;
  firstName: string;
  lastName: string;
  className: string;
  current: boolean;
};

function normEmail(raw: unknown): string {
  return String(raw ?? "")
    .trim()
    .toLowerCase();
}

/** Libellés de classe école, sur texte normalisé (sans accents, minuscules). */
const ECOLE_CLASS_RE =
  /^(tps|ps|ms|gs|cp|ce1|ce2|cm1|cm2)([0-9]|[a-f])?(?![a-z0-9])|maternelle|elementaire|primaire|ulis ?ecole/;

function normClassLabel(raw: unknown): string {
  return String(raw ?? "")
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .trim();
}

/**
 * Élève d'école (maternelle / primaire). Libellé de classe, puis secteur explicite, puis MEF.
 * N'utilise JAMAIS `folderName` (« NOM Prénom » : « TAN Emma » y serait lu comme un lycéen).
 */
export function isEcoleEleve(eleve: EleveConfig, mefMap?: Map<string, Secteur> | null): boolean {
  const classeNorm = normClassLabel(eleve.classe);
  if (ECOLE_CLASS_RE.test(classeNorm)) return true;
  if (inferSecteurFromFolderName(String(eleve.classe ?? "")) === "ecole") return true;
  const explicit = String(eleve.secteur ?? "").trim().toLowerCase();
  if (explicit === "ecole") return true;
  if (explicit === "lycee" || explicit === "college") return false;
  const mef = normMefCode(String(eleve.mef ?? eleve.formation ?? ""));
  if (mef && mefMap && mefMap.size > 0) return mefMap.get(mef) === "ecole";
  return false;
}

/**
 * Filtrage pur (sans DB) de la fratrie.
 * Rattachement : e-mail parent (parent1/parent/parent2) ∈ verifiedEmails, ou élève
 * dont l'id figure dans `foyerEleveIds` (foyers dont un responsable a une adresse vérifiée).
 * `eleve.email` n'est JAMAIS un critère.
 */
export function filterStageSiblings(params: {
  eleves: EleveConfig[];
  verifiedEmails: string[];
  foyerEleveIds?: Iterable<string>;
  mefMap?: Map<string, Secteur> | null;
  isClassEligible: (className: string) => boolean;
  currentEleveId?: string;
  now?: Date;
}): StageSibling[] {
  const verified = new Set(params.verifiedEmails.map(normEmail).filter(Boolean));
  if (verified.size === 0) return [];
  const foyerIds = new Set(params.foyerEleveIds ?? []);

  const out: StageSibling[] = [];
  const seen = new Set<string>();
  for (const eleve of params.eleves) {
    const id = String(eleve.id ?? "").trim();
    if (!id || seen.has(id)) continue;
    if (!isEleveActifPourListes({ status: eleve.status, dateSortie: eleve.dateSortie }, params.now)) {
      continue;
    }
    const linked =
      [eleve.parent1Email, eleve.parentEmail, eleve.parent2Email].some((e) =>
        verified.has(normEmail(e)),
      ) || foyerIds.has(id);
    if (!linked) continue;
    if (!String(eleve.dateNaissance ?? "").trim()) continue;
    if (isEcoleEleve(eleve, params.mefMap)) continue;
    const className = String(eleve.classe ?? "").trim();
    if (!params.isClassEligible(className)) continue;

    seen.add(id);
    out.push({
      id,
      firstName: eleve.prenom.trim(),
      lastName: eleve.nom.trim(),
      className,
      current: Boolean(params.currentEleveId) && id === params.currentEleveId,
    });
  }
  return out.sort((a, b) =>
    `${a.lastName} ${a.firstName}`.localeCompare(`${b.lastName} ${b.firstName}`, "fr", {
      sensitivity: "base",
    }),
  );
}
