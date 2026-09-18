/**
 * Catégories documents dossier élève (client + serveur).
 * Les tiroirs techniques restent pour le stockage ; l’UI regroupe par catégorie.
 * Chaque métier ne voit que son silo (sauf direction = vue large, avec grants pour psy/santé).
 */

export type EleveDocCategorie =
  | "administratif"
  | "financier"
  | "sante"
  | "vie_scolaire"
  | "psychologue";

export type EleveDocTiroirId =
  | "scolaire"
  | "inscription"
  | "facturation"
  | "voyages"
  | "sante"
  | "vie_scolaire"
  | "orientation"
  | "psychologue";

export const DOC_CATEGORIE_ORDER: EleveDocCategorie[] = [
  "administratif",
  "vie_scolaire",
  "financier",
  "sante",
  "psychologue",
];

export const DOC_CATEGORIE_LABELS: Record<EleveDocCategorie, string> = {
  administratif: "Administratif",
  vie_scolaire: "Vie scolaire",
  financier: "Financier / comptable",
  sante: "Santé / accompagnement",
  psychologue: "Psychologue",
};

export const TIROIR_LABELS: Record<string, string> = {
  scolaire: "Scolaire",
  inscription: "Inscription",
  facturation: "Facturation",
  voyages: "Voyages",
  sante: "Santé / accompagnement",
  vie_scolaire: "Vie scolaire",
  orientation: "Orientation / fiches de dialogue",
  psychologue: "Psychologue",
};

export const TIROIR_TO_CATEGORIE: Record<EleveDocTiroirId, EleveDocCategorie> = {
  scolaire: "administratif",
  inscription: "administratif",
  voyages: "administratif",
  orientation: "administratif",
  vie_scolaire: "vie_scolaire",
  facturation: "financier",
  sante: "sante",
  psychologue: "psychologue",
};

export const CATEGORIE_TIROIRS: Record<EleveDocCategorie, EleveDocTiroirId[]> = {
  administratif: ["scolaire", "inscription", "voyages", "orientation"],
  vie_scolaire: ["vie_scolaire"],
  financier: ["facturation"],
  sante: ["sante"],
  psychologue: ["psychologue"],
};

export function categorieForTiroir(tiroir: string): EleveDocCategorie | null {
  return TIROIR_TO_CATEGORIE[tiroir as EleveDocTiroirId] ?? null;
}

export function tiroirsForCategories(categories: Iterable<EleveDocCategorie>): EleveDocTiroirId[] {
  const out: EleveDocTiroirId[] = [];
  for (const cat of DOC_CATEGORIE_ORDER) {
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
