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
        name?: string | null;
      }
    | null
    | undefined,
): string {
  const first = user?.firstName?.trim();
  if (first) return first;

  const full = (user?.fullName ?? user?.name)?.trim();
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
  if (!raw || /^utilisateur$/i.test(raw) || /^administratif$/i.test(raw)) {
    return "l'administration";
  }
  const token = raw.split(/\s+/).find(Boolean);
  return token || "l'administration";
}

/**
 * Résout le prénom pour la bannière PDF (y compris anciennes validations
 * enregistrées comme « Utilisateur » via l'id adminReview.by).
 */
export async function resolveStageAdminReviewPdfName(adminReview: {
  by: string;
  byName: string;
}): Promise<string> {
  const fromStored = stageAdminReviewPdfName(adminReview.byName);
  if (fromStored !== "l'administration") return fromStored;

  try {
    const { resolveMemberProfileById } = await import("@/app/lib/members-db");
    const profile = await resolveMemberProfileById(adminReview.by);
    if (!profile) return fromStored;
    return stageActorFirstName({
      firstName: profile.firstName,
      lastName: profile.lastName,
      name: profile.name,
      fullName: profile.name,
    });
  } catch {
    return fromStored;
  }
}
