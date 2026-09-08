import { rentreeAccentClasses } from "@/app/lib/rentree-accent-styles";
import type { PortesOuvertesCycle } from "@/app/lib/portes-ouvertes-types";

/** Accents rentrée utilisés pour école / collège / lycée. */
export type PortesOuvertesCycleAccent = "yellow" | "sky" | "pink";

/** Même mapping que `defaultAccentForKind` (préparation rentrée). */
export function rentreeAccentForPortesCycle(cycle: PortesOuvertesCycle): PortesOuvertesCycleAccent {
  if (cycle === "ecole") return "yellow";
  if (cycle === "college") return "sky";
  return "pink";
}

type CycleShell = {
  pageBg: string;
  cardBorder: string;
  cardShadow: string;
  headerBg: string;
  headerBorder: string;
  headerEyebrow: string;
  headerTitle: string;
  addressRing: string;
  link: string;
  fieldFocus: string;
  slotSelected: string;
  slotIdle: string;
  childPanel: string;
  childTitle: string;
};

const SHELL: Record<PortesOuvertesCycleAccent, CycleShell> = {
  yellow: {
    pageBg: "bg-[linear-gradient(180deg,#fffbeb_0%,#f8fafc_42%,#ffffff_100%)]",
    cardBorder: "border-yellow-100/90",
    cardShadow: "shadow-[0_30px_80px_-48px_rgba(234,179,8,0.55)]",
    headerBg: "bg-[radial-gradient(120%_120%_at_0%_0%,#fef9c3_0%,#ffffff_55%)]",
    headerBorder: "border-yellow-100",
    headerEyebrow: "text-yellow-700",
    headerTitle: "text-yellow-950",
    addressRing: "ring-yellow-100",
    link: "font-bold text-yellow-700 underline-offset-2 hover:underline",
    fieldFocus: "focus:border-yellow-400 focus:ring-2 focus:ring-yellow-200",
    slotSelected: "bg-yellow-500 text-white shadow-md shadow-yellow-500/25",
    slotIdle: "bg-slate-50 text-slate-900 ring-1 ring-slate-200/80 hover:bg-yellow-50 hover:ring-yellow-200",
    childPanel: "bg-yellow-50/50 ring-1 ring-yellow-100",
    childTitle: "text-yellow-900",
  },
  sky: {
    pageBg: "bg-[linear-gradient(180deg,#f0f9ff_0%,#f8fafc_42%,#ffffff_100%)]",
    cardBorder: "border-sky-100/90",
    cardShadow: "shadow-[0_30px_80px_-48px_rgba(2,132,199,0.55)]",
    headerBg: "bg-[radial-gradient(120%_120%_at_0%_0%,#e0f2fe_0%,#ffffff_55%)]",
    headerBorder: "border-sky-100",
    headerEyebrow: "text-sky-700",
    headerTitle: "text-sky-950",
    addressRing: "ring-sky-100",
    link: "font-bold text-sky-700 underline-offset-2 hover:underline",
    fieldFocus: "focus:border-sky-400 focus:ring-2 focus:ring-sky-200",
    slotSelected: "bg-sky-600 text-white shadow-md shadow-sky-600/25",
    slotIdle: "bg-slate-50 text-slate-900 ring-1 ring-slate-200/80 hover:bg-sky-50 hover:ring-sky-200",
    childPanel: "bg-sky-50/50 ring-1 ring-sky-100",
    childTitle: "text-sky-900",
  },
  pink: {
    pageBg: "bg-[linear-gradient(180deg,#fdf2f8_0%,#f8fafc_42%,#ffffff_100%)]",
    cardBorder: "border-pink-100/90",
    cardShadow: "shadow-[0_30px_80px_-48px_rgba(219,39,119,0.55)]",
    headerBg: "bg-[radial-gradient(120%_120%_at_0%_0%,#fce7f3_0%,#ffffff_55%)]",
    headerBorder: "border-pink-100",
    headerEyebrow: "text-pink-700",
    headerTitle: "text-pink-950",
    addressRing: "ring-pink-100",
    link: "font-bold text-pink-700 underline-offset-2 hover:underline",
    fieldFocus: "focus:border-pink-400 focus:ring-2 focus:ring-pink-200",
    slotSelected: "bg-pink-600 text-white shadow-md shadow-pink-600/25",
    slotIdle: "bg-slate-50 text-slate-900 ring-1 ring-slate-200/80 hover:bg-pink-50 hover:ring-pink-200",
    childPanel: "bg-pink-50/50 ring-1 ring-pink-100",
    childTitle: "text-pink-900",
  },
};

export function portesOuvertesCycleFormStyles(cycle: PortesOuvertesCycle) {
  const accent = rentreeAccentForPortesCycle(cycle);
  const a = rentreeAccentClasses(accent);
  const shell = SHELL[accent];
  return { accent, ...a, ...shell };
}
