import type { CSSProperties } from "react";
import { dashboardBrandCssVars } from "@/app/lib/dashboard-brand-presets";

export function dashboardBrandStyle(_accent?: string | null): CSSProperties {
  return dashboardBrandCssVars() as CSSProperties;
}

/** Classes Tailwind basées sur les variables CSS — à utiliser sous `.dashboard-themed`. */
export const dash = {
  label: "text-[var(--dash-mid)]",
  ink: "text-[var(--dash-ink)]",
  textPrimary: "text-[var(--dash-primary)]",
  textMid: "text-[var(--dash-mid)]",
  textBright: "text-[var(--dash-bright)]",
  textLime: "text-[var(--dash-lime)]",
  border: "border-[color:var(--dash-border)]",
  borderSoft: "border-[color:var(--dash-border)]/80",
  bgSoft: "bg-[color:var(--dash-soft)]",
  bgSoftMuted: "bg-[color:var(--dash-soft-muted)]",
  bgSoft25: "bg-[color:var(--dash-soft-muted)]/60",
  bgSoft30: "bg-[color:var(--dash-soft-muted)]/80",
  bgSoft50: "bg-[color:var(--dash-soft-muted)]/50",
  bgPrimary: "bg-[var(--dash-primary)]",
  bgLime: "bg-[var(--dash-lime)]",
  bgSurface: "bg-[var(--dash-surface)]",
  ringBright: "ring-[color:var(--dash-bright)]",
  ringBright35: "ring-[color:var(--dash-bright)]/35",
  gradientText:
    "bg-gradient-to-r from-[var(--dash-ink)] via-[var(--dash-mid)] to-[var(--dash-bright)] bg-clip-text text-transparent",
  gradientHeader: "bg-gradient-to-r from-[color:var(--dash-soft-muted)] to-white",
  btnPrimary:
    "cursor-pointer rounded-xl bg-[var(--dash-primary)] font-bold text-white hover:brightness-110 disabled:opacity-50",
  btnPrimaryGrad:
    "bg-[var(--dash-primary)] font-bold text-white hover:brightness-110",
  btnLime:
    "cursor-pointer rounded-xl bg-[var(--dash-lime)] font-bold text-[var(--dash-ink)] hover:brightness-105 disabled:opacity-50",
  hoverPrimary: "hover:text-[var(--dash-primary)]",
  hoverBorder: "hover:border-[color:var(--dash-primary)]/35",
  hoverBgSoft: "hover:bg-[color:var(--dash-soft-muted)]",
  focusBorder: "focus:border-[var(--dash-primary)]",
  focusRing: "focus:ring-[color:var(--dash-bright)]/40",
  spinner: "border-[color:var(--dash-soft)] border-t-[var(--dash-primary)]",
  divider: "border-[color:var(--dash-border)]/80",
  linkBold: "font-bold text-[var(--dash-mid)] hover:text-[var(--dash-primary)]",
  tileBorder: "border-[color:var(--dash-border)]/70",
  tileBorderHover: "hover:border-[color:var(--dash-primary)]/35",
  editZone: "border-dashed border-[color:var(--dash-border)] bg-[color:var(--dash-soft-muted)]/30",
  connectorDone: "bg-[var(--dash-bright)]",
  field:
    "w-full rounded-xl border border-black/8 bg-white px-4 py-3 text-sm font-semibold text-[var(--dash-ink)] outline-none shadow-sm transition focus:border-[var(--dash-primary)] focus:ring-2 focus:ring-[color:var(--dash-lime)]/50",
  fieldLabel: "text-[10px] font-semibold uppercase tracking-[0.18em] text-[var(--dash-mid)]",
} as const;
