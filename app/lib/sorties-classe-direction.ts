import type { Establishment } from "@/app/lib/app-config-schemas";
import { matchEstablishment } from "@/app/lib/establishment-catalog";
import { inferEstablishmentKind } from "@/app/lib/establishment-visual";
import { resolveInternatStudentKind } from "@/app/lib/internat-etablissement-kind";
import type { SortieNiveau } from "@/app/lib/sorties-classe";

/**
 * Quelle direction doit valider pour cet élève. Le NIVEAU de l'élève fait foi, pas le libellé de sa fiche :
 * un libellé mixte (« Collège-Lycée ») ou faux ne doit jamais confier un lycéen à la direction du collège.
 * Niveau inconnu : réservé à la direction générale (et à l'administrateur).
 */
export function groupeDirection(
  student: { etablissement?: string | null; classe?: string | null },
  establishments: Establishment[],
): { groupe: string; niveau: SortieNiveau; email?: string } {
  const label = String(student.etablissement || "").trim();
  const niveau = resolveInternatStudentKind(student, establishments);
  if (!niveau) return { groupe: label ? `${label} (niveau à préciser)` : "Niveau à préciser", niveau: null };

  const duNiveau = establishments.filter((e) => inferEstablishmentKind(e) === niveau);
  const fiche = label ? matchEstablishment(establishments, label) : undefined;
  const retenu =
    fiche && inferEstablishmentKind(fiche) === niveau
      ? fiche
      : (duNiveau.find((e) => e.directorEmail?.trim()) ?? duNiveau[0]);
  return {
    groupe: retenu?.label?.trim() || (niveau === "college" ? "Collège" : "Lycée"),
    niveau,
    email: retenu?.directorEmail?.trim() || undefined,
  };
}
