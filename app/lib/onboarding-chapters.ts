export const TOTAL_CHAPTERS = 5;
export const ONBOARDING_WIZARD_VERSION = 2;

export const ONBOARDING_CHAPTERS = [
  {
    id: 1,
    eyebrow: "Bienvenue",
    title: "Votre espace Scola",
    subtitle: "Première étape : situer votre établissement dans la plateforme.",
  },
  {
    id: 2,
    eyebrow: "Identité",
    title: "Qui êtes-vous ?",
    subtitle: "Nom court, couleur du tableau de bord et adresse pour la météo.",
  },
  {
    id: 3,
    eyebrow: "Structure",
    title: "Établissements & directions",
    subtitle: "Définissez les niveaux actifs et qui les dirige.",
  },
  {
    id: 4,
    eyebrow: "Contacts",
    title: "Qui fait quoi ?",
    subtitle: "Destinataires métier, modules et raccourcis — tout peut être ajusté plus tard.",
  },
  {
    id: 5,
    eyebrow: "Finalisation",
    title: "Tout est prêt",
    subtitle: "Vérifiez le récapitulatif, puis passez aux licences Microsoft.",
  },
] as const;

/**
 * Normalise l'étape sauvegardée vers les 5 chapitres.
 * Legacy (version < 2) : schéma 1–12.
 */
export function normalizeOnboardingStep(
  saved: number,
  wizardVersion?: number,
): number {
  if (!Number.isFinite(saved) || saved < 1) return 1;
  if (wizardVersion && wizardVersion >= ONBOARDING_WIZARD_VERSION) {
    return Math.min(Math.max(1, Math.floor(saved)), TOTAL_CHAPTERS);
  }
  // Legacy 12-step → 5 chapters
  if (saved >= 12) return 5;
  if (saved >= 5) return 4;
  if (saved === 4) return 2;
  if (saved === 3) return 3;
  if (saved === 2) return 2;
  return 1;
}
