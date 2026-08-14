import type { RentreeAccent } from "@/app/lib/rentree-types";

type RentreeAccentStyle = {
  pillActive: string;
  pillIdle: string;
  badge: string;
  sectionTitle: string;
  cta: string;
  hero: string;
};

const STYLES: Record<RentreeAccent, RentreeAccentStyle> = {
  yellow: {
    pillActive: "bg-yellow-500 text-white border-yellow-500",
    pillIdle: "bg-white text-slate-700 border-slate-200 hover:border-yellow-300",
    badge: "bg-yellow-50 text-yellow-700 border-yellow-100",
    sectionTitle: "text-yellow-700",
    cta: "bg-yellow-500 hover:bg-yellow-600 text-white",
    hero: "from-yellow-500 via-yellow-500/60 to-white",
  },
  sky: {
    pillActive: "bg-sky-600 text-white border-sky-600",
    pillIdle: "bg-white text-slate-700 border-slate-200 hover:border-sky-300",
    badge: "bg-sky-50 text-sky-700 border-sky-100",
    sectionTitle: "text-sky-700",
    cta: "bg-sky-600 hover:bg-sky-700 text-white",
    hero: "from-sky-600 via-sky-600/60 to-white",
  },
  pink: {
    pillActive: "bg-pink-600 text-white border-pink-600",
    pillIdle: "bg-white text-slate-700 border-slate-200 hover:border-pink-300",
    badge: "bg-pink-50 text-pink-700 border-pink-100",
    sectionTitle: "text-pink-700",
    cta: "bg-pink-600 hover:bg-pink-700 text-white",
    hero: "from-pink-600 via-pink-600/60 to-white",
  },
  green: {
    pillActive: "bg-emerald-600 text-white border-emerald-600",
    pillIdle: "bg-white text-slate-700 border-slate-200 hover:border-emerald-300",
    badge: "bg-emerald-50 text-emerald-800 border-emerald-100",
    sectionTitle: "text-emerald-700",
    cta: "bg-emerald-600 hover:bg-emerald-700 text-white",
    hero: "from-emerald-600 via-emerald-600/60 to-white",
  },
  blue: {
    pillActive: "bg-blue-600 text-white border-blue-600",
    pillIdle: "bg-white text-slate-700 border-slate-200 hover:border-blue-300",
    badge: "bg-blue-50 text-blue-800 border-blue-100",
    sectionTitle: "text-blue-700",
    cta: "bg-blue-600 hover:bg-blue-700 text-white",
    hero: "from-blue-600 via-blue-600/60 to-white",
  },
  rose: {
    pillActive: "bg-rose-600 text-white border-rose-600",
    pillIdle: "bg-white text-slate-700 border-slate-200 hover:border-rose-300",
    badge: "bg-rose-50 text-rose-800 border-rose-100",
    sectionTitle: "text-rose-700",
    cta: "bg-rose-600 hover:bg-rose-700 text-white",
    hero: "from-rose-600 via-rose-600/60 to-white",
  },
  violet: {
    pillActive: "bg-violet-600 text-white border-violet-600",
    pillIdle: "bg-white text-slate-700 border-slate-200 hover:border-violet-300",
    badge: "bg-violet-50 text-violet-800 border-violet-100",
    sectionTitle: "text-violet-700",
    cta: "bg-violet-600 hover:bg-violet-700 text-white",
    hero: "from-violet-600 via-violet-600/60 to-white",
  },
  amber: {
    pillActive: "bg-amber-500 text-white border-amber-500",
    pillIdle: "bg-white text-slate-700 border-slate-200 hover:border-amber-300",
    badge: "bg-amber-50 text-amber-900 border-amber-100",
    sectionTitle: "text-amber-800",
    cta: "bg-amber-500 hover:bg-amber-600 text-white",
    hero: "from-amber-500 via-amber-500/60 to-white",
  },
  teal: {
    pillActive: "bg-teal-600 text-white border-teal-600",
    pillIdle: "bg-white text-slate-700 border-slate-200 hover:border-teal-300",
    badge: "bg-teal-50 text-teal-800 border-teal-100",
    sectionTitle: "text-teal-700",
    cta: "bg-teal-600 hover:bg-teal-700 text-white",
    hero: "from-teal-600 via-teal-600/60 to-white",
  },
};

export function rentreeAccentClasses(accent: RentreeAccent): RentreeAccentStyle {
  return STYLES[accent] ?? STYLES.violet;
}
