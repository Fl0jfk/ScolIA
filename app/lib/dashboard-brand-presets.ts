/**
 * Palette intranet fixe (langage « lime » Permitly).
 * Les anciens accents multi-couleurs sont ignorés ; `parseDashboardAccent` reste
 * pour compat lecture config / onboarding sans casser les tenants existants.
 */

export const DASHBOARD_ACCENT_OPTIONS = [
  { id: "green", label: "Lime (fixe)", swatch: "#D4FF37" },
] as const;

export type DashboardAccent = (typeof DASHBOARD_ACCENT_OPTIONS)[number]["id"];

export const DEFAULT_DASHBOARD_ACCENT: DashboardAccent = "green";

export type DashboardBrandPalette = {
  primary: string;
  dark: string;
  mid: string;
  bright: string;
  soft: string;
  softMuted: string;
  border: string;
  ink: string;
  surface: string;
  lime: string;
};

/** Palette unique intranet — charcoal + lime néon. */
export const LIME_BRAND_PALETTE: DashboardBrandPalette = {
  primary: "#141414",
  dark: "#0A0A0A",
  mid: "#5A6B4A",
  bright: "#D4FF37",
  soft: "#EEFFA8",
  softMuted: "#F5FFD6",
  border: "#D5E89C",
  ink: "#141414",
  surface: "#F4F5F3",
  lime: "#D4FF37",
};

/** @deprecated Les accents configurables sont retirés — toujours la palette lime. */
const DASHBOARD_BRAND_PRESETS: Record<DashboardAccent, DashboardBrandPalette> = {
  green: LIME_BRAND_PALETTE,
};

export function parseDashboardAccent(_raw?: unknown): DashboardAccent {
  return DEFAULT_DASHBOARD_ACCENT;
}

export function dashboardBrandCssVars(_accent?: DashboardAccent): Record<string, string> {
  const p = LIME_BRAND_PALETTE;
  return {
    "--dash-primary": p.primary,
    "--dash-dark": p.dark,
    "--dash-mid": p.mid,
    "--dash-bright": p.bright,
    "--dash-soft": p.soft,
    "--dash-soft-muted": p.softMuted,
    "--dash-border": p.border,
    "--dash-ink": p.ink,
    "--dash-surface": p.surface,
    "--dash-lime": p.lime,
  };
}
