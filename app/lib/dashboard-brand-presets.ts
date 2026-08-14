export const DASHBOARD_ACCENT_OPTIONS = [
  { id: "green", label: "Vert (par défaut)", swatch: "#2F6B4A" },
  { id: "blue", label: "Bleu", swatch: "#2563EB" },
  { id: "rose", label: "Rose", swatch: "#E11D48" },
  { id: "violet", label: "Violet", swatch: "#7C3AED" },
  { id: "amber", label: "Ambre", swatch: "#D97706" },
  { id: "teal", label: "Bleu-vert", swatch: "#0D9488" },
] as const;

export type DashboardAccent = (typeof DASHBOARD_ACCENT_OPTIONS)[number]["id"];

export const DEFAULT_DASHBOARD_ACCENT: DashboardAccent = "green";

type DashboardBrandPalette = {
  primary: string;
  dark: string;
  mid: string;
  bright: string;
  soft: string;
  softMuted: string;
  border: string;
  ink: string;
};

const DASHBOARD_BRAND_PRESETS: Record<DashboardAccent, DashboardBrandPalette> = {
  green: {
    primary: "#2F6B4A",
    dark: "#1E4A32",
    mid: "#3D8A5C",
    bright: "#4ADE80",
    soft: "#D1FAE5",
    softMuted: "#ECFDF5",
    border: "#A7F3D0",
    ink: "#14231A",
  },
  blue: {
    primary: "#2563EB",
    dark: "#1D4ED8",
    mid: "#3B82F6",
    bright: "#60A5FA",
    soft: "#DBEAFE",
    softMuted: "#EFF6FF",
    border: "#BFDBFE",
    ink: "#14231A",
  },
  rose: {
    primary: "#E11D48",
    dark: "#BE123C",
    mid: "#F43F5E",
    bright: "#FB7185",
    soft: "#FFE4E6",
    softMuted: "#FFF1F2",
    border: "#FECDD3",
    ink: "#14231A",
  },
  violet: {
    primary: "#7C3AED",
    dark: "#6D28D9",
    mid: "#8B5CF6",
    bright: "#A78BFA",
    soft: "#EDE9FE",
    softMuted: "#F5F3FF",
    border: "#DDD6FE",
    ink: "#14231A",
  },
  amber: {
    primary: "#D97706",
    dark: "#B45309",
    mid: "#F59E0B",
    bright: "#FBBF24",
    soft: "#FEF3C7",
    softMuted: "#FFFBEB",
    border: "#FDE68A",
    ink: "#14231A",
  },
  teal: {
    primary: "#0D9488",
    dark: "#0F766E",
    mid: "#14B8A6",
    bright: "#2DD4BF",
    soft: "#CCFBF1",
    softMuted: "#F0FDFA",
    border: "#99F6E4",
    ink: "#14231A",
  },
};

export function parseDashboardAccent(raw: unknown): DashboardAccent {
  const value = typeof raw === "string" ? raw.trim() : "";
  if (DASHBOARD_ACCENT_OPTIONS.some((o) => o.id === value)) {
    return value as DashboardAccent;
  }
  return DEFAULT_DASHBOARD_ACCENT;
}

export function dashboardBrandCssVars(accent: DashboardAccent): Record<string, string> {
  const p = DASHBOARD_BRAND_PRESETS[accent];
  return {
    "--dash-primary": p.primary,
    "--dash-dark": p.dark,
    "--dash-mid": p.mid,
    "--dash-bright": p.bright,
    "--dash-soft": p.soft,
    "--dash-soft-muted": p.softMuted,
    "--dash-border": p.border,
    "--dash-ink": p.ink,
  };
}
