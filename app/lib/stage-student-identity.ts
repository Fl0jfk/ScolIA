import "server-only";

import type { EleveConfig } from "@/app/lib/eleves-config";
import { normalizeEleveDateNaissance } from "@/app/lib/eleves-config";
import { loadElevesRegistry } from "@/app/lib/eleves-registry";

/** Déduit un niveau scolaire à partir du libellé de classe (ex. « 3e2 » → 3e, « 2A » → 2nde). */
export function inferStudentLevelFromClass(className: string): string {
  const raw = className.trim().toLowerCase();
  if (!raw) return "3e";
  const c = raw
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[\s._-]+/g, "");

  // Collège
  if (/^(6e|6eme)/.test(c) || /^6[a-z0-9]/.test(c)) return "6e";
  if (/^(5e|5eme)/.test(c) || /^5[a-z0-9]/.test(c)) return "5e";
  if (/^(4e|4eme)/.test(c) || /^4[a-z0-9]/.test(c)) return "4e";
  if (/^(3e|3eme)/.test(c) || /^3[a-z0-9]/.test(c)) return "3e";

  // Lycée — « 2A », « 2B », « 2nde1 »… = seconde (pas le fallback 3e)
  if (/^(2nde|2de|seconde)/.test(c) || /^2[a-z0-9]/.test(c) || c.includes("2nde") || c.includes("seconde")) {
    return "2nde";
  }
  if (
    /^(1re|1ere|premiere)/.test(c) ||
    /^1[a-z]/.test(c) ||
    /^1st/.test(c) ||
    c.includes("1re") ||
    c.includes("prem")
  ) {
    return "1re";
  }
  if (
    /^(tle|terminale|tale)/.test(c) ||
    /^t[a-z0-9]/.test(c) ||
    c.includes("tle") ||
    c.includes("term")
  ) {
    return "Tle";
  }

  return "3e";
}

function normalizePersonName(str: string): string {
  return str
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[-\s]+/g, " ")
    .trim();
}

function normalizeClassLabel(str: string): string {
  return str
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/\s+/g, "")
    .trim();
}

export type VerifiedStageStudent = {
  eleve: EleveConfig;
  firstName: string;
  lastName: string;
  className: string;
  level: string;
};

export type StageIdentityCandidateClass = {
  className: string;
};

function toVerifiedStudent(eleve: EleveConfig): VerifiedStageStudent {
  const className = String(eleve.classe ?? "").trim();
  return {
    eleve,
    firstName: eleve.prenom.trim(),
    lastName: eleve.nom.trim(),
    className,
    level: inferStudentLevelFromClass(className),
  };
}

/**
 * Vérifie l'identité élève sans exposer de liste publique.
 * Nom + prénom + date de naissance ; si plusieurs homonymes, la classe départage.
 */
export async function verifyStudentForPreconvention(params: {
  nom: string;
  prenom: string;
  dateNaissance: string;
  classe?: string;
}): Promise<
  | { ok: true; student: VerifiedStageStudent }
  | { ok: false; reason: "not_found" }
  | { ok: false; reason: "ambiguous"; candidates: StageIdentityCandidateClass[] }
> {
  const nom = normalizePersonName(params.nom);
  const prenom = normalizePersonName(params.prenom);
  const dob = normalizeEleveDateNaissance(params.dateNaissance);
  if (!nom || !prenom || !dob) return { ok: false, reason: "not_found" };

  const eleves = await loadElevesRegistry();
  const matches = eleves.filter((eleve) => {
    if (normalizePersonName(eleve.nom) !== nom) return false;
    if (normalizePersonName(eleve.prenom) !== prenom) return false;
    const registryDob = normalizeEleveDateNaissance(eleve.dateNaissance);
    return Boolean(registryDob) && registryDob === dob;
  });

  if (matches.length === 0) return { ok: false, reason: "not_found" };

  if (matches.length === 1) {
    return { ok: true, student: toVerifiedStudent(matches[0]!) };
  }

  const classeRaw = String(params.classe ?? "").trim();
  if (!classeRaw) {
    const seen = new Set<string>();
    const candidates: StageIdentityCandidateClass[] = [];
    for (const eleve of matches) {
      const className = String(eleve.classe ?? "").trim();
      if (!className) continue;
      const key = normalizeClassLabel(className);
      if (!key || seen.has(key)) continue;
      seen.add(key);
      candidates.push({ className });
    }
    if (candidates.length < 2) {
      // Homonymes sans classes distinctes exploitables → échec générique.
      return { ok: false, reason: "not_found" };
    }
    return { ok: false, reason: "ambiguous", candidates };
  }

  const wantedClass = normalizeClassLabel(classeRaw);
  const narrowed = matches.filter(
    (eleve) => normalizeClassLabel(String(eleve.classe ?? "")) === wantedClass,
  );

  if (narrowed.length === 1) {
    return { ok: true, student: toVerifiedStudent(narrowed[0]!) };
  }

  return { ok: false, reason: "not_found" };
}
