import type { EnseignantConfig } from "@/app/lib/enseignants-types";
import { normalizeName } from "@/app/lib/ocr-eleve-match";
import { oneDrivePathForEleve } from "@/app/lib/onedrive-eleves";

export type EnseignantMatchCandidateView = {
  kind: "enseignant";
  nom: string;
  prenom: string;
  classe?: string;
  folderName: string;
  folderPath?: string;
  score: number;
  matchedBy: string;
  secteur: string;
};

export type EnseignantMatchDecision = {
  decision: "auto" | "review" | "none";
  enseignant: EnseignantConfig | null;
  candidates: EnseignantMatchCandidateView[];
  reason: string;
  confidence: number;
};

function closeness(a: string, b: string): number {
  if (!a || !b) return 0;
  if (a === b) return 1;
  if (a.includes(b) || b.includes(a)) return 0.92;
  return 0;
}

function toView(e: EnseignantConfig, score: number, matchedBy: string, basePath?: string): EnseignantMatchCandidateView {
  return {
    kind: "enseignant",
    nom: e.nom,
    prenom: e.prenom,
    folderName: e.folderName,
    folderPath: basePath ? oneDrivePathForEleve(basePath, e.folderName) : undefined,
    score,
    matchedBy,
    secteur: e.secteur,
  };
}

export function matchEnseignantFromDocument(params: {
  text: string;
  extractedNom?: string;
  extractedPrenom?: string;
  enseignants: EnseignantConfig[];
  basePathFor?: (secteur: EnseignantConfig["secteur"]) => string | null;
}): EnseignantMatchDecision {
  const enseignants = params.enseignants;
  const none = (reason: string): EnseignantMatchDecision => ({
    decision: "none",
    enseignant: null,
    candidates: [],
    reason,
    confidence: 0,
  });
  if (enseignants.length === 0) return none("liste_enseignants_vide");

  const an = normalizeName(params.extractedNom || "");
  const ap = normalizeName(params.extractedPrenom || "");
  const text = normalizeName(params.text || "");
  if (!an && !ap) return none("identite_absente");

  const scored = enseignants
    .map((e) => {
      const bn = normalizeName(e.nom);
      const bp = normalizeName(e.prenom);
      const nomC = closeness(an, bn);
      const prenomC = closeness(ap, bp);
      const inText = Boolean(bn && bp && text.includes(bn) && text.includes(bp));
      const score = 2.2 * nomC + 1.6 * prenomC + (inText ? 0.4 : 0);
      return { e, score, nomC, prenomC };
    })
    .filter((x) => x.score >= 3.2)
    .sort((a, b) => b.score - a.score);

  if (scored.length === 0) return none("aucun_enseignant");

  const top = scored[0];
  const second = scored[1];
  const unique = !second || top.score > second.score + 0.6;
  const strong = top.nomC >= 0.92 && top.prenomC >= 0.92;
  const candidates = scored.slice(0, 5).map((x) =>
    toView(x.e, x.score, "name", params.basePathFor?.(x.e.secteur) ?? undefined),
  );

  if (unique && strong) {
    return {
      decision: "auto",
      enseignant: top.e,
      candidates,
      reason: "nom_prenom_unique",
      confidence: Math.min(0.99, top.score / 4.2),
    };
  }
  return {
    decision: "review",
    enseignant: null,
    candidates,
    reason: unique ? "score_insuffisant" : "homonyme",
    confidence: 0.55,
  };
}
