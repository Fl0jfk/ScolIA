import type { Establishment, EstablishmentKind } from "@/app/lib/app-config-schemas";
import { internatEligibleEstablishments, matchEstablishment } from "@/app/lib/establishment-catalog";
import { inferEstablishmentKind } from "@/app/lib/establishment-visual";
import { niveauFromClasse, type InternatNiveau } from "@/app/lib/internat-level";

const COLLEGE_NIVEAUX = new Set<InternatNiveau>(["6e", "5e", "4e", "3e"]);
const LYCEE_NIVEAUX = new Set<InternatNiveau>(["2nde", "1re", "Tle"]);

function fold(raw: string | null | undefined): string {
  return String(raw || "")
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[_\s-]+/g, " ")
    .trim();
}

function kindFromClasse(classe: string | null | undefined): "college" | "lycee" | null {
  const niveau = niveauFromClasse(classe);
  if (!niveau) return null;
  if (COLLEGE_NIVEAUX.has(niveau)) return "college";
  if (LYCEE_NIVEAUX.has(niveau)) return "lycee";
  return null;
}

function kindFromLabel(label: string | null | undefined): "college" | "lycee" | null {
  const blob = fold(label);
  if (!blob) return null;
  const hasCollege = blob.includes("college");
  const hasLycee = blob.includes("lycee");
  if (hasCollege && !hasLycee) return "college";
  if (hasLycee && !hasCollege) return "lycee";
  return null;
}

/**
 * Kind collège/lycée d’un interne pour PDF, mails et filtres direction.
 *
 * Priorité :
 * 1. Niveau de classe (fiable même si l’étiquette établissement est fausse)
 * 2. Kind catalogue si le libellé matche un établissement
 * 3. Heuristique sur un libellé non ambigu
 *
 * Les libellés mixtes (« Collège-Lycée ») ne forcent plus tout en collège.
 */
export function resolveInternatStudentKind(
  student: { etablissement?: string | null; classe?: string | null },
  establishments: Establishment[] = [],
): "college" | "lycee" | null {
  const fromClasse = kindFromClasse(student.classe);
  if (fromClasse) return fromClasse;

  const eligible = internatEligibleEstablishments(establishments);
  if (eligible.length > 0 && student.etablissement) {
    const hit = matchEstablishment(eligible, student.etablissement);
    if (hit) {
      const k = inferEstablishmentKind(hit);
      if (k === "college" || k === "lycee") return k;
    }
  }

  return kindFromLabel(student.etablissement);
}

export function isInternatStudentOfKind(
  student: { etablissement?: string | null; classe?: string | null },
  kind: "college" | "lycee",
  establishments: Establishment[] = [],
): boolean {
  return resolveInternatStudentKind(student, establishments) === kind;
}

export function establishmentKindLabel(kind: EstablishmentKind | "college" | "lycee"): string {
  if (kind === "college") return "Collège";
  if (kind === "lycee") return "Lycée";
  if (kind === "ecole") return "École";
  return "Établissement";
}
