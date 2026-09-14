/**
 * Prénom à afficher pour un acteur stages (validation admin, historique, PDF).
 * Better Auth expose souvent `name` / fullName sans `firstName` séparé.
 */
export function stageActorFirstName(
  user:
    | {
        firstName?: string | null;
        lastName?: string | null;
        fullName?: string | null;
      }
    | null
    | undefined,
): string {
  const first = user?.firstName?.trim();
  if (first) return first;

  const full = user?.fullName?.trim();
  if (full) {
    const token = full.split(/\s+/).find(Boolean);
    if (token && !/^utilisateur$/i.test(token) && !token.includes("@")) {
      return token;
    }
  }

  return "Administratif";
}

/** Libellé PDF « Validée par X » — prénom seul ; évite le fallback « Utilisateur ». */
export function stageAdminReviewPdfName(byName: string | undefined | null): string {
  const raw = String(byName ?? "").trim();
  if (!raw || /^utilisateur$/i.test(raw)) return "l'administration";
  const token = raw.split(/\s+/).find(Boolean);
  return token || "l'administration";
}
