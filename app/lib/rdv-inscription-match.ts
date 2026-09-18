/**
 * Matching protégé RDV inscription : pool = élèves liés au contact parent uniquement.
 * Jamais de recherche globale par nom seul.
 */

import { identityKey, normalizePersonPart } from "@/app/lib/eleve-photos-match";
import type { EleveConfig } from "@/app/lib/eleves-config";

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
    status: e.status || "inscrit",
    parents: [],
  }));
}
