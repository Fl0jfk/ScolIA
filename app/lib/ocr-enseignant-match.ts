import type { EnseignantConfig } from "@/app/lib/enseignants-types";
import { collectOcrEmails, matchEntriesByEmailInText } from "@/app/lib/ocr-email-match";
import { normalizeName, scanStudentsInText } from "@/app/lib/ocr-eleve-match";

function folderPathFor(basePath: string | undefined, folderName: string): string | undefined {
  if (!basePath) return undefined;
  return `${basePath.replace(/\/+$/, "")}/${folderName.replace(/^\/+/, "").trim()}`;
}

function enseignantEmails(e: EnseignantConfig): string[] {
  return collectOcrEmails(e.email, e.emailPro);
}

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

function cleanExtractedName(raw?: string | null): string {
  const s = String(raw ?? "").trim();
  if (!s || /^non[_\s-]?trouv/i.test(s)) return "";
  return s;
}

function closeness(a: string, b: string): number {
  if (!a || !b) return 0;
  if (a === b) return 1;
  if (a.includes(b) || b.includes(a)) return 0.92;
  return 0;
}

function toView(
  e: EnseignantConfig,
  score: number,
  matchedBy: string,
  basePath?: string,
): EnseignantMatchCandidateView {
  return {
    kind: "enseignant",
    nom: e.nom,
    prenom: e.prenom,
    folderName: e.folderName,
    folderPath: folderPathFor(basePath, e.folderName),
    score,
    matchedBy,
    secteur: e.secteur,
  };
}

function nomLengthOk(e: Pick<EnseignantConfig, "nom">): boolean {
  return normalizeName(e.nom).replace(/\s+/g, "").length >= 4;
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

  const textRaw = params.text || "";
  const basePathOf = (e: EnseignantConfig) => params.basePathFor?.(e.secteur) ?? undefined;

  const emailHits = matchEntriesByEmailInText(textRaw, enseignants, enseignantEmails);
  if (emailHits.length === 1) {
    const e = emailHits[0]!;
    return {
      decision: "auto",
      enseignant: e,
      candidates: [toView(e, 5, "email", basePathOf(e))],
      reason: "email_dans_document",
      confidence: 0.97,
    };
  }
  if (emailHits.length > 1) {
    return {
      decision: "review",
      enseignant: null,
      candidates: emailHits.slice(0, 5).map((e) => toView(e, 5, "email", basePathOf(e))),
      reason: "email_ambigu",
      confidence: 0.7,
    };
  }

  // Scan nom+prénom dans le texte OCR — indépendant de l'extraction IA.
  const scanned = scanStudentsInText(textRaw, enseignants).filter(nomLengthOk);
  if (scanned.length === 1) {
    const e = scanned[0]!;
    return {
      decision: "auto",
      enseignant: e,
      candidates: [toView(e, 4.6, "scan_texte", basePathOf(e))],
      reason: "nom_prenom_dans_texte",
      confidence: 0.95,
    };
  }
  if (scanned.length > 1) {
    return {
      decision: "review",
      enseignant: null,
      candidates: scanned.slice(0, 5).map((e) => toView(e, 4.2, "scan_texte", basePathOf(e))),
      reason: "plusieurs_enseignants_dans_texte",
      confidence: 0.6,
    };
  }

  const an = normalizeName(cleanExtractedName(params.extractedNom));
  const ap = normalizeName(cleanExtractedName(params.extractedPrenom));
  if (!an || !ap) return none("identite_absente");

  const scored = enseignants
    .map((e) => {
      const bn = normalizeName(e.nom);
      const bp = normalizeName(e.prenom);
      const nomC = closeness(an, bn);
      const prenomC = closeness(ap, bp);
      const score = 2.2 * nomC + 1.6 * prenomC;
      return { e, score, nomC, prenomC };
    })
    .filter((x) => x.score >= 3.2)
    .sort((a, b) => b.score - a.score);

  if (scored.length === 0) return none("aucun_enseignant");

  const top = scored[0]!;
  const second = scored[1];
  const unique = !second || top.score > second.score + 0.6;
  const strong = top.nomC >= 0.92 && top.prenomC >= 0.92;
  const candidates = scored.slice(0, 5).map((x) => toView(x.e, x.score, "name", basePathOf(x.e)));

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
