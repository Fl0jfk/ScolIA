/** Récapitulatif de stages d'un enfant pour l'écran famille (pur, sans DB). Aucun jeton ni donnée personnelle. */

export type SiblingStagePeriod = {
  id: string;
  label: string;
  periodStart: string;
  periodEnd: string;
  chosen: boolean;
  companyName?: string;
};

export type SiblingStageConvention = {
  id: string;
  label?: string;
  companyName?: string;
  status: string;
  statusLabel?: string;
};

export type SiblingStageSummary = {
  periods: SiblingStagePeriod[];
  conventions: SiblingStageConvention[];
};

export const EMPTY_SIBLING_STAGE_SUMMARY: SiblingStageSummary = { periods: [], conventions: [] };

type DossierLike = {
  availablePeriods: Array<{
    id: string;
    label: string;
    periodStart: string;
    periodEnd: string;
    used: boolean;
    conventionId?: string;
  }>;
  conventions: Array<{
    id: string;
    status: string;
    statusLabel?: string;
    stageLabel?: string;
    companyName?: string;
  }>;
};

const INACTIVE_STATUSES = new Set(["cancelled", "archived"]);

/** Champs explicitement copiés : ne jamais propager de jeton (studentAccessToken…) ni de lien. */
export function buildSiblingStageSummary(dossier: DossierLike): SiblingStageSummary {
  const byId = new Map(dossier.conventions.map((c) => [c.id, c]));
  return {
    periods: dossier.availablePeriods.map((p) => {
      const rawCompany = p.conventionId ? byId.get(p.conventionId)?.companyName?.trim() : "";
      const company = rawCompany && rawCompany !== "—" ? rawCompany : "";
      return {
        id: p.id,
        label: p.label,
        periodStart: p.periodStart,
        periodEnd: p.periodEnd,
        chosen: Boolean(p.used),
        ...(p.used && company ? { companyName: company } : {}),
      };
    }),
    conventions: dossier.conventions
      .filter((c) => !INACTIVE_STATUSES.has(c.status))
      .map((c) => ({
        id: c.id,
        status: c.status,
        ...(c.stageLabel?.trim() ? { label: c.stageLabel.trim() } : {}),
        ...(c.companyName?.trim() && c.companyName.trim() !== "—"
          ? { companyName: c.companyName.trim() }
          : {}),
        ...(c.statusLabel?.trim() ? { statusLabel: c.statusLabel.trim() } : {}),
      })),
  };
}

/** Enfant courant en premier, puis par prénom (insensible à la casse et aux accents). */
export function sortSiblingsCurrentFirst<T extends { current: boolean; firstName: string; lastName: string }>(
  list: T[],
): T[] {
  return [...list].sort((a, b) => {
    if (a.current !== b.current) return a.current ? -1 : 1;
    return (
      a.firstName.localeCompare(b.firstName, "fr", { sensitivity: "base" }) ||
      a.lastName.localeCompare(b.lastName, "fr", { sensitivity: "base" })
    );
  });
}
