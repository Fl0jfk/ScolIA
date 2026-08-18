import type { EleveConfig } from "./eleves-config";
import { normalizeEleveDateNaissance } from "./eleves-config";

export type DocOrigin = "interne" | "externe" | "inconnu";
export type EleveMatchDecisionKind = "auto" | "review" | "none";

export type ExtractedIdentity = {
  nom?: string;
  prenom?: string;
  ine?: string;
  classe?: string;
  dateNaissance?: string;
  parents?: Array<{ nom?: string; prenom?: string }>;
  origine?: DocOrigin;
};

export type EleveMatchCandidateView = {
  ine: string;
  nom: string;
  prenom: string;
  classe?: string;
  folderName: string;
  folderPath?: string;
  score: number;
  matchedBy: string;
};

export type EleveMatchDecision = {
  decision: EleveMatchDecisionKind;
  eleve: EleveConfig | null;
  confidence: number;
  matchedBy: string | null;
  candidates: EleveMatchCandidateView[];
  reason: string;
  origin: DocOrigin;
};

const PARTICLES = new Set(["de", "du", "des", "le", "la", "d", "l", "von", "van", "di"]);

const INTERNAL_HINTS =
  /\b(bulletin|releve de notes|relevé de notes|pronote|charlemagne|certificat de scolarite|certificat de scolarité|livret scolaire|conseil de classe)\b/i;
const EXTERNAL_HINTS =
  /\b(carte nationale|cni|passeport|caf|mutuelle|attestation d.?assurance|securite sociale|sécurité sociale|cpam|ameli|livret de famille)\b/i;

export function normalizeName(str: string): string {
  return String(str || "")
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[-\s']+/g, " ")
    .replace(/[^a-z0-9\s]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

export function normalizeIne(str: string): string {
  return String(str || "")
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toUpperCase()
    .replace(/[^A-Z0-9]/g, "");
}

export function normalizeClass(str: string): string {
  return String(str || "")
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toUpperCase()
    .replace(/[^A-Z0-9]/g, "");
}

function foldOcrLetters(s: string): string {
  return s.replace(/0/g, "o").replace(/1/g, "l").replace(/5/g, "s").replace(/8/g, "b");
}

function foldIneCandidate(raw: string): string {
  const up = String(raw || "")
    .toUpperCase()
    .replace(/[^A-Z0-9]/g, "");
  if (up.length < 10) return up;
  const body = up
    .slice(0, -1)
    .replace(/O/g, "0")
    .replace(/[IL]/g, "1")
    .replace(/S/g, "5")
    .replace(/B/g, "8");
  return body + up.slice(-1);
}

function nameTokens(str: string): string[] {
  return normalizeName(str)
    .split(" ")
    .filter((t) => t.length >= 2 && !PARTICLES.has(t));
}

function levenshtein(a: string, b: string): number {
  if (a === b) return 0;
  if (!a.length) return b.length;
  if (!b.length) return a.length;
  let prev = Array.from({ length: b.length + 1 }, (_, i) => i);
  for (let i = 1; i <= a.length; i++) {
    const curr = [i];
    for (let j = 1; j <= b.length; j++) {
      const cost = a[i - 1] === b[j - 1] ? 0 : 1;
      curr[j] = Math.min(curr[j - 1] + 1, prev[j] + 1, prev[j - 1] + cost);
    }
    prev = curr;
  }
  return prev[b.length];
}

function closeness(a: string, b: string): number {
  if (!a || !b) return 0;
  if (a === b) return 1;
  const maxLen = Math.max(a.length, b.length);
  if (maxLen === 0) return 0;
  const dist = levenshtein(a, b);
  const maxAllowed = maxLen <= 6 ? 1 : maxLen <= 12 ? 2 : 3;
  if (dist > maxAllowed) return 0;
  return 1 - dist / maxLen;
}

function tokenCloseness(aTokens: string[], bTokens: string[]): number {
  if (aTokens.length === 0 || bTokens.length === 0) return 0;
  const scores = aTokens.map((at) => Math.max(...bTokens.map((bt) => closeness(at, bt))));
  return scores.reduce((s, v) => s + v, 0) / Math.max(aTokens.length, bTokens.length);
}

export function extractInesFromText(text: string): string[] {
  const raw = String(text || "");
  const found = new Set<string>();
  const re = /\b[0-9OIlS]{9,11}[A-Z]\b/gi;
  let m: RegExpExecArray | null;
  while ((m = re.exec(raw))) {
    const norm = normalizeIne(foldIneCandidate(m[0]));
    if (norm.length >= 10 && norm.length <= 12) found.add(norm);
  }
  return [...found];
}

export function classifyDocumentOrigin(text: string, extracted?: ExtractedIdentity): DocOrigin {
  if (extracted?.origine === "interne" || extracted?.origine === "externe") return extracted.origine;
  const blob = `${text}\n${extracted?.classe || ""}`;
  const hasIne = extractInesFromText(text).length > 0 || Boolean(normalizeIne(extracted?.ine || ""));
  if (INTERNAL_HINTS.test(blob) || (hasIne && /\b(bulletin|releve|certificat)\b/i.test(blob))) {
    return "interne";
  }
  if (EXTERNAL_HINTS.test(blob)) return "externe";
  return hasIne ? "interne" : "inconnu";
}

type TextIndex = {
  norm: string;
  folded: string;
  tokens: Set<string>;
};

function buildTextIndex(text: string): TextIndex {
  const norm = normalizeName(text);
  const folded = foldOcrLetters(norm);
  return {
    norm,
    folded,
    tokens: new Set([...norm.split(" ").filter(Boolean), ...folded.split(" ").filter(Boolean)]),
  };
}

function tokenInIndex(token: string, index: TextIndex): boolean {
  if (!token) return false;
  if (index.tokens.has(token) || index.tokens.has(foldOcrLetters(token))) return true;
  if (token.length >= 4 && (index.norm.includes(token) || index.folded.includes(token))) return true;
  return false;
}

export function studentNameInText(eleve: Pick<EleveConfig, "nom" | "prenom">, text: string): boolean {
  return studentNameInIndex(eleve, buildTextIndex(text));
}

function studentNameInIndex(eleve: Pick<EleveConfig, "nom" | "prenom">, index: TextIndex): boolean {
  const nomTok = nameTokens(eleve.nom);
  const prenomTok = nameTokens(eleve.prenom);
  if (nomTok.length === 0 || prenomTok.length === 0) return false;
  const nomOk = nomTok.every((t) => tokenInIndex(t, index));
  const prenomOk = tokenInIndex(prenomTok[0], index);
  return nomOk && prenomOk;
}

export function scanStudentsInText<T extends Pick<EleveConfig, "nom" | "prenom">>(
  text: string,
  students: T[],
): T[] {
  if (!text.trim() || students.length === 0) return [];
  const index = buildTextIndex(text);
  return students.filter((s) => studentNameInIndex(s, index));
}

function ineDistance(a: string, b: string): number {
  if (!a || !b) return 99;
  if (a === b) return 0;
  if (a.length !== b.length) return 99;
  let d = 0;
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) d++;
  return d;
}

function classesMatch(a?: string, b?: string): boolean {
  const na = normalizeClass(a || "");
  const nb = normalizeClass(b || "");
  if (!na || !nb) return false;
  return na === nb || na.includes(nb) || nb.includes(na);
}

function dobsMatch(a?: string, b?: string): boolean {
  const na = normalizeEleveDateNaissance(a || "");
  const nb = normalizeEleveDateNaissance(b || "");
  return Boolean(na && nb && na === nb);
}

function toView(eleve: EleveConfig, score: number, matchedBy: string): EleveMatchCandidateView {
  return {
    ine: eleve.ine || "",
    nom: eleve.nom,
    prenom: eleve.prenom,
    classe: eleve.classe,
    folderName: eleve.folderName,
    score: Math.round(score * 100) / 100,
    matchedBy,
  };
}

function uniqueByIne(list: EleveConfig[]): EleveConfig[] {
  const seen = new Set<string>();
  const out: EleveConfig[] = [];
  for (const e of list) {
    const key = e.ine ? `ine:${normalizeIne(e.ine)}` : `n:${normalizeName(e.nom)}|${normalizeName(e.prenom)}|${e.classe || ""}`;
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(e);
  }
  return out;
}

function scoreExtractedAgainst(eleve: EleveConfig, extracted: ExtractedIdentity, text: string) {
  const an = normalizeName(extracted.nom || "");
  const ap = normalizeName(extracted.prenom || "");
  const bn = normalizeName(eleve.nom);
  const bp = normalizeName(eleve.prenom);
  const nomTokA = nameTokens(extracted.nom || "");
  const prenomTokA = nameTokens(extracted.prenom || "");
  const nomTokB = nameTokens(eleve.nom);
  const prenomTokB = nameTokens(eleve.prenom);

  const nomDirect = closeness(an, bn) || tokenCloseness(nomTokA, nomTokB);
  const prenomDirect = closeness(ap, bp) || tokenCloseness(prenomTokA, prenomTokB);
  const nomInv = closeness(an, bp);
  const prenomInv = closeness(ap, bn);
  const inverted = nomInv >= 0.88 && prenomInv >= 0.88 && nomDirect + prenomDirect < nomInv + prenomInv;

  const nomCloseness = inverted ? nomInv : nomDirect;
  const prenomCloseness = inverted ? prenomInv : prenomDirect;
  const classMatched = classesMatch(extracted.classe, eleve.classe);
  const dobMatched = dobsMatch(extracted.dateNaissance, eleve.dateNaissance);
  const inText = studentNameInText(eleve, text);

  const score =
    2.3 * nomCloseness +
    1.6 * prenomCloseness +
    (classMatched ? 0.45 : 0) +
    (dobMatched ? 0.7 : 0) +
    (inText ? 0.4 : 0);

  return { nomCloseness, prenomCloseness, classMatched, dobMatched, inText, score, inverted };
}

export function matchEleveFromDocument(params: {
  text: string;
  eleves: EleveConfig[];
  extracted?: ExtractedIdentity;
}): EleveMatchDecision {
  const text = params.text || "";
  const eleves = uniqueByIne(params.eleves || []);
  const extracted = params.extracted || {};
  const origin = classifyDocumentOrigin(text, extracted);
  const none = (reason: string): EleveMatchDecision => ({
    decision: "none",
    eleve: null,
    confidence: 0,
    matchedBy: null,
    candidates: [],
    reason,
    origin,
  });

  if (eleves.length === 0) return none("liste_eleves_vide");

  const ines = [
    ...extractInesFromText(text),
    ...(extracted.ine && extracted.ine !== "non_trouvé" ? [normalizeIne(extracted.ine)] : []),
  ].filter(Boolean);

  for (const ine of ines) {
    const exact = eleves.filter((e) => e.ine && normalizeIne(e.ine) === ine);
    if (exact.length === 1) {
      return {
        decision: "auto",
        eleve: exact[0],
        confidence: 1,
        matchedBy: "ine",
        candidates: [toView(exact[0], 4, "ine")],
        reason: "ine_exact",
        origin,
      };
    }
    if (exact.length > 1) {
      return {
        decision: "review",
        eleve: null,
        confidence: 0.7,
        matchedBy: "ine",
        candidates: exact.slice(0, 3).map((e) => toView(e, 4, "ine")),
        reason: "ine_ambigu",
        origin,
      };
    }
  }

  const fuzzyHits: EleveConfig[] = [];
  for (const ine of ines) {
    for (const e of eleves) {
      if (!e.ine) continue;
      const dist = ineDistance(ine, normalizeIne(e.ine));
      if (dist === 1) fuzzyHits.push(e);
    }
  }
  const fuzzyUnique = uniqueByIne(fuzzyHits);
  if (fuzzyUnique.length === 1) {
    return {
      decision: "auto",
      eleve: fuzzyUnique[0],
      confidence: 0.96,
      matchedBy: "ine_flou",
      candidates: [toView(fuzzyUnique[0], 3.8, "ine_flou")],
      reason: "ine_1_substitution_unique",
      origin,
    };
  }
  if (fuzzyUnique.length > 1) {
    return {
      decision: "review",
      eleve: null,
      confidence: 0.65,
      matchedBy: "ine_flou",
      candidates: fuzzyUnique.slice(0, 3).map((e) => toView(e, 3.5, "ine_flou")),
      reason: "ine_flou_ambigu",
      origin,
    };
  }

  const scanned = scanStudentsInText(text, eleves);
  if (scanned.length === 1 && nameTokens(scanned[0].nom).join("").length >= 4) {
    const extractedPrenom = normalizeName(extracted.prenom || "");
    const childPrenom = normalizeName(scanned[0].prenom);
    const sameFamilyDifferentPrenom =
      Boolean(extractedPrenom) &&
      extractedPrenom !== childPrenom &&
      closeness(extractedPrenom, childPrenom) < 0.8 &&
      nameTokens(extracted.nom || "").some((t) => nameTokens(scanned[0].nom).includes(t));
    return {
      decision: "auto",
      eleve: scanned[0],
      confidence: sameFamilyDifferentPrenom ? 0.92 : 0.94,
      matchedBy: sameFamilyDifferentPrenom ? "scan_texte_enfant" : "scan_texte",
      candidates: [toView(scanned[0], 3.7, sameFamilyDifferentPrenom ? "scan_texte_enfant" : "scan_texte")],
      reason: sameFamilyDifferentPrenom ? "parent_extrait_enfant_dans_texte" : "nom_prenom_uniques_dans_texte",
      origin,
    };
  }
  if (scanned.length > 1 && scanned.length <= 4) {
    return {
      decision: "review",
      eleve: null,
      confidence: 0.6,
      matchedBy: "scan_texte",
      candidates: scanned.slice(0, 3).map((e) => toView(e, 3.2, "scan_texte")),
      reason: "plusieurs_eleves_dans_texte",
      origin,
    };
  }

  const hasNom = Boolean(extracted.nom && extracted.nom !== "non_trouvé" && nameTokens(extracted.nom).length > 0);
  const hasPrenom = Boolean(
    extracted.prenom && extracted.prenom !== "non_trouvé" && nameTokens(extracted.prenom).length > 0,
  );
  if (!hasNom || !hasPrenom) {
    return none(hasNom || hasPrenom ? "un_seul_champ_nom_prenom" : "pas_de_nom_prenom");
  }

  const scored = eleves
    .map((eleve) => ({ eleve, signals: scoreExtractedAgainst(eleve, extracted, text) }))
    .filter((s) => s.signals.nomCloseness >= 0.88 && s.signals.prenomCloseness >= 0.55 && s.signals.score > 0)
    .sort((a, b) => b.signals.score - a.signals.score)
    .slice(0, 5);

  if (scored.length === 0) return none("aucun_candidat_nom_prenom");

  const best = scored[0];
  const second = scored[1]?.signals.score ?? 0;
  const gap = best.signals.score - second;
  const views = scored.slice(0, 3).map((s) => toView(s.eleve, s.signals.score, "nom_prenom"));

  const autoInternal =
    origin !== "externe" &&
    best.signals.nomCloseness >= 0.9 &&
    best.signals.prenomCloseness >= 0.72 &&
    (best.signals.classMatched ||
      best.signals.inText ||
      best.signals.dobMatched ||
      best.signals.nomCloseness >= 0.98 ||
      (scored.length === 1 && best.signals.prenomCloseness >= 0.78)) &&
    (scored.length === 1 || gap >= 0.35);

  const autoExternal =
    best.signals.nomCloseness >= 0.94 &&
    best.signals.prenomCloseness >= 0.88 &&
    (best.signals.inText || best.signals.dobMatched) &&
    (scored.length === 1 || gap >= 0.45);

  if (autoInternal || autoExternal) {
    return {
      decision: "auto",
      eleve: best.eleve,
      confidence: Math.min(0.99, 0.7 + best.signals.score / 10),
      matchedBy: best.signals.dobMatched ? "nom_prenom_ddn" : best.signals.classMatched ? "nom_prenom_classe" : "nom_prenom",
      candidates: views,
      reason: autoExternal ? "score_externe_convergent" : "score_interne_convergent",
      origin,
    };
  }

  return {
    decision: "review",
    eleve: null,
    confidence: Math.min(0.85, best.signals.score / 5),
    matchedBy: "nom_prenom",
    candidates: views,
    reason: "shortlist_a_valider",
    origin,
  };
}
