import "server-only";

import type { EleveConfig } from "@/app/lib/eleves-config";
import { normalizeEleveDateNaissance } from "@/app/lib/eleves-config";
import { findEleveByIne } from "@/app/lib/eleves-registry";

/** Déduit un niveau scolaire à partir du libellé de classe (ex. « 3e2 » → 3e). */
export function inferStudentLevelFromClass(className: string): string {
  const c = className.trim().toLowerCase();
  if (!c) return "3e";
  if (/(^|\s|[^0-9])6[eè]/.test(c) || c.startsWith("6")) return "6e";
  if (/(^|\s|[^0-9])5[eè]/.test(c) || c.startsWith("5")) return "5e";
  if (/(^|\s|[^0-9])4[eè]/.test(c) || c.startsWith("4")) return "4e";
  if (/(^|\s|[^0-9])3[eè]/.test(c) || c.startsWith("3")) return "3e";
  if (c.includes("2nde") || c.includes("seconde")) return "2nde";
  if (c.includes("1re") || c.includes("prem")) return "1re";
  if (c.includes("tle") || c.includes("term")) return "Tle";
  return "3e";
}

export type VerifiedStageStudent = {
  eleve: EleveConfig;
  firstName: string;
  lastName: string;
  className: string;
  level: string;
};

/**
 * Vérifie l'identité élève sans exposer de liste publique.
 * INE + date de naissance (registre eleves.json / BDD) — message d'erreur générique si échec.
 */
export async function verifyStudentForPreconvention(params: {
  ine: string;
  dateNaissance: string;
}): Promise<{ ok: true; student: VerifiedStageStudent } | { ok: false }> {
  const ine = params.ine.trim().toUpperCase();
  const dob = normalizeEleveDateNaissance(params.dateNaissance);
  if (!ine || !dob) return { ok: false };

  const eleve = await findEleveByIne(ine);
  if (!eleve) return { ok: false };

  const registryDob = normalizeEleveDateNaissance(eleve.dateNaissance);
  if (!registryDob || registryDob !== dob) return { ok: false };

  const className = String(eleve.classe ?? "").trim();
  return {
    ok: true,
    student: {
      eleve,
      firstName: eleve.prenom.trim(),
      lastName: eleve.nom.trim(),
      className,
      level: inferStudentLevelFromClass(className),
    },
  };
}
