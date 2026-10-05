/**
 * Matching protégé RDV inscription :
 * - par contact parent (e-mail / téléphone connus) ;
 * - ou, derrière gate e-mail, par identité élève (nom + prénom + date de naissance).
 * Jamais de recherche globale par nom seul.
 */

import { identityKey, normalizePersonPart } from "@/app/lib/eleve-photos-match";
import {
  normalizeEleveDateNaissance,
  type EleveConfig,
} from "@/app/lib/eleves-config";

export type RdvMatchParent = {
  prenom: string;
  nom: string;
  email: string | null;
  rang: number;
};

export type RdvMatchCandidate = {
  id: string;
  prenom: string;
  /** Nom complet pour confirmation parent (déjà filtré par contact). */
  nom: string;
  classe: string | null;
  status: string;
  /** Responsables foyer liés (si connus) — pour déduire qui sera présent. */
  parents: RdvMatchParent[];
};

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

function normalizeParentEmail(email: string): string {
  return email.trim().toLowerCase();
}

function collectEleveParentEmails(eleve: EleveConfig): string[] {
  const set = new Set<string>();
  for (const raw of [eleve.parentEmail, eleve.parent1Email, eleve.parent2Email]) {
    const e = normalizeParentEmail(String(raw || ""));
    if (e && EMAIL_RE.test(e)) set.add(e);
  }
  return [...set];
}

/** Normalise un téléphone FR pour comparaison (chiffres, 33→0). */
export function normalizeParentPhone(raw: string): string {
  let digits = String(raw || "").replace(/\D+/g, "");
  if (digits.startsWith("33") && digits.length >= 11) {
    digits = `0${digits.slice(2)}`;
  }
  if (digits.startsWith("0033") && digits.length >= 13) {
    digits = `0${digits.slice(4)}`;
  }
  return digits;
}

export function collectEleveParentPhones(eleve: EleveConfig): string[] {
  const set = new Set<string>();
  for (const raw of [eleve.parentPhone, eleve.parent1Phone, eleve.parent2Phone]) {
    const p = normalizeParentPhone(String(raw || ""));
    if (p.length >= 8) set.add(p);
  }
  return [...set];
}

export function elevesMatchingParentContact(
  eleves: EleveConfig[],
  parentEmail: string,
  parentPhone?: string,
): EleveConfig[] {
  const email = normalizeParentEmail(parentEmail);
  const phone = parentPhone ? normalizeParentPhone(parentPhone) : "";
  if (!email && !phone) return [];

  return eleves.filter((e) => {
    if (email && collectEleveParentEmails(e).includes(email)) return true;
    if (phone && collectEleveParentPhones(e).includes(phone)) return true;
    return false;
  });
}

function nameTokens(nom: string, prenom: string): string[] {
  return [
    ...normalizePersonPart(nom).split(" "),
    ...normalizePersonPart(prenom).split(" "),
  ].filter(Boolean);
}

/** Score nom/prénom dans un pool déjà restreint au foyer contact. */
export function scoreEleveNameMatch(
  eleve: EleveConfig,
  nom: string,
  prenom: string,
): number {
  const exactKey = identityKey(nom, prenom);
  if (identityKey(eleve.nom, eleve.prenom) === exactKey) return 100;

  const wanted = nameTokens(nom, prenom);
  const have = nameTokens(eleve.nom, eleve.prenom);
  if (wanted.length < 2 || have.length < 2) return 0;

  const haveSet = new Set(have);
  let hit = 0;
  for (const t of wanted) {
    if (haveSet.has(t)) hit += 1;
  }
  if (hit < 2) return 0;
  // Tous les jetons du côté saisie présents côté élève.
  if (hit === wanted.length) return 80;
  // Chevauchement partiel (noms composés).
  if (hit >= Math.min(wanted.length, have.length)) return 60;
  return hit >= 2 ? 40 : 0;
}

export function matchRdvInscriptionCandidates(opts: {
  eleves: EleveConfig[];
  parentEmail: string;
  parentPhone?: string;
  studentLastName: string;
  studentFirstName: string;
  limit?: number;
}): RdvMatchCandidate[] {
  const pool = elevesMatchingParentContact(
    opts.eleves,
    opts.parentEmail,
    opts.parentPhone,
  );
  if (!pool.length) return [];

  const scored = pool
    .filter((e) => Boolean(e.id))
    .map((e) => ({
      eleve: e,
      score: scoreEleveNameMatch(e, opts.studentLastName, opts.studentFirstName),
    }))
    .filter((x) => x.score > 0)
    .sort((a, b) => b.score - a.score || a.eleve.nom.localeCompare(b.eleve.nom, "fr"));

  const limit = Math.min(8, Math.max(1, opts.limit ?? 5));
  return scored.slice(0, limit).map(({ eleve: e }) => ({
    id: e.id!,
    prenom: e.prenom,
    nom: e.nom,
    classe: e.classe?.trim() || null,
    status: e.status || "preinscrit",
    parents: [],
  }));
}

/**
 * Matching identité (foyer séparé / contact absent de l’extract) :
 * date de naissance exacte + au moins le nom OU le prénom
 * (casse / accents ignorés). À n’appeler qu’après gate e-mail.
 */
export function personPartEquals(a: string, b: string): boolean {
  const left = normalizePersonPart(a);
  const right = normalizePersonPart(b);
  return Boolean(left) && left === right;
}

/** Score 2 = nom+prénom, 1 = un seul côté — null si pas de match. */
export function scoreRdvIdentityMatch(
  eleve: EleveConfig,
  opts: { studentLastName: string; studentFirstName: string; dateNaissance: string },
): number | null {
  const dob = normalizeEleveDateNaissance(opts.dateNaissance);
  const eDob = normalizeEleveDateNaissance(eleve.dateNaissance || "");
  if (!dob || !eDob || dob !== eDob) return null;

  const last = opts.studentLastName.trim();
  const first = opts.studentFirstName.trim();
  if (!last && !first) return null;

  const nomOk = last ? personPartEquals(eleve.nom, last) : false;
  const prenomOk = first ? personPartEquals(eleve.prenom, first) : false;
  if (!nomOk && !prenomOk) return null;
  return (nomOk ? 1 : 0) + (prenomOk ? 1 : 0);
}

export function matchRdvInscriptionByIdentity(opts: {
  eleves: EleveConfig[];
  studentLastName: string;
  studentFirstName: string;
  dateNaissance: string;
  limit?: number;
}): RdvMatchCandidate[] {
  const dob = normalizeEleveDateNaissance(opts.dateNaissance);
  if (!dob) return [];
  const first = opts.studentFirstName.trim();
  const last = opts.studentLastName.trim();
  if (!first && !last) return [];

  const scored = opts.eleves
    .filter((e) => Boolean(e.id))
    .map((e) => {
      const score = scoreRdvIdentityMatch(e, {
        studentLastName: last,
        studentFirstName: first,
        dateNaissance: dob,
      });
      if (score == null) return null;
      return { eleve: e, score };
    })
    .filter((x): x is { eleve: EleveConfig; score: number } => x !== null)
    .sort((a, b) => b.score - a.score || a.eleve.nom.localeCompare(b.eleve.nom, "fr"));

  const limit = Math.min(5, Math.max(1, opts.limit ?? 3));
  return scored.slice(0, limit).map(({ eleve: e }) => ({
    id: e.id!,
    prenom: e.prenom,
    nom: e.nom,
    classe: e.classe?.trim() || null,
    status: e.status || "preinscrit",
    parents: [],
  }));
}

/** L’élève correspond à l’identité saisie (naissance + nom et/ou prénom). */
export function eleveMatchesRdvIdentity(
  eleve: EleveConfig,
  opts: { studentLastName: string; studentFirstName: string; dateNaissance: string },
): boolean {
  return scoreRdvIdentityMatch(eleve, opts) != null;
}
