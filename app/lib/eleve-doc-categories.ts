/**
 * Catégories documents dossier élève (client + serveur).
 * Les tiroirs techniques restent pour le stockage ; l’UI regroupe par catégorie.
 */

export type EleveDocCategorie = "administratif" | "financier" | "sante";

export type EleveDocTiroirId =
  | "scolaire"
  | "inscription"
  | "facturation"
  | "voyages"
  | "sante"
  | "vie_scolaire";

export const DOC_CATEGORIE_ORDER: EleveDocCategorie[] = [
  "administratif",
  "financier",
  "sante",
];

export const DOC_CATEGORIE_LABELS: Record<EleveDocCategorie, string> = {
  administratif: "Administratif",
  financier: "Financier / comptable",
  sante: "Santé",
};

export const TIROIR_LABELS: Record<string, string> = {
  scolaire: "Scolaire",
  inscription: "Inscription",
  facturation: "Facturation",
  voyages: "Voyages",
  sante: "Santé / PAP",
  vie_scolaire: "Vie scolaire",
};

export const TIROIR_TO_CATEGORIE: Record<EleveDocTiroirId, EleveDocCategorie> = {
  scolaire: "administratif",
  inscription: "administratif",
  voyages: "administratif",
  vie_scolaire: "administratif",
  facturation: "financier",
  sante: "sante",
};

export const CATEGORIE_TIROIRS: Record<EleveDocCategorie, EleveDocTiroirId[]> = {
  administratif: ["scolaire", "inscription", "voyages", "vie_scolaire"],
  financier: ["facturation"],
  sante: ["sante"],
};

export function categorieForTiroir(tiroir: string): EleveDocCategorie | null {
  return TIROIR_TO_CATEGORIE[tiroir as EleveDocTiroirId] ?? null;
}

export function tiroirsForCategories(categories: Iterable<EleveDocCategorie>): EleveDocTiroirId[] {
  const out: EleveDocTiroirId[] = [];
  for (const cat of DOC_CATEGORIE_ORDER) {
    // only include if requested
    let wanted = false;
    for (const c of categories) {
      if (c === cat) {
        wanted = true;
        break;
      }
    }
    if (!wanted) continue;
    out.push(...CATEGORIE_TIROIRS[cat]);
  }
  return out;
}
