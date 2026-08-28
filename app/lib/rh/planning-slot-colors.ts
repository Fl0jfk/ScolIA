/** Couleurs pastel stables par libellé (matière, mission, poste…). */

export type PlanningSlotColor = {
  bg: string;
  border: string;
  title: string;
  meta: string;
  /** Couleurs jsPDF [r,g,b] 0–255 */
  pdfFill: [number, number, number];
  pdfStroke: [number, number, number];
};

const PALETTE: PlanningSlotColor[] = [
  {
    bg: "bg-indigo-100",
    border: "border-indigo-300",
    title: "text-indigo-950",
    meta: "text-indigo-800/80",
    pdfFill: [224, 231, 255],
    pdfStroke: [129, 140, 248],
  },
  {
    bg: "bg-violet-100",
    border: "border-violet-300",
    title: "text-violet-950",
    meta: "text-violet-800/80",
    pdfFill: [237, 233, 254],
    pdfStroke: [167, 139, 250],
  },
  {
    bg: "bg-sky-100",
    border: "border-sky-300",
    title: "text-sky-950",
    meta: "text-sky-800/80",
    pdfFill: [224, 242, 254],
    pdfStroke: [56, 189, 248],
  },
  {
    bg: "bg-emerald-100",
    border: "border-emerald-300",
    title: "text-emerald-950",
    meta: "text-emerald-800/80",
    pdfFill: [209, 250, 229],
    pdfStroke: [52, 211, 153],
  },
  {
    bg: "bg-amber-100",
    border: "border-amber-300",
    title: "text-amber-950",
    meta: "text-amber-900/80",
    pdfFill: [254, 243, 199],
    pdfStroke: [251, 191, 36],
  },
  {
    bg: "bg-rose-100",
    border: "border-rose-300",
    title: "text-rose-950",
    meta: "text-rose-800/80",
    pdfFill: [255, 228, 230],
    pdfStroke: [251, 113, 133],
  },
  {
    bg: "bg-teal-100",
    border: "border-teal-300",
    title: "text-teal-950",
    meta: "text-teal-800/80",
    pdfFill: [204, 251, 241],
    pdfStroke: [45, 212, 191],
  },
  {
    bg: "bg-fuchsia-100",
    border: "border-fuchsia-300",
    title: "text-fuchsia-950",
    meta: "text-fuchsia-800/80",
    pdfFill: [250, 232, 255],
    pdfStroke: [232, 121, 249],
  },
];

const DAY_HEADER_CLASS =
  "border-l border-slate-100 px-1.5 py-2 text-center bg-gradient-to-b from-slate-50 to-white";

const DAY_HEADER_COLORS: Record<number, string> = {
  1: "from-indigo-50/90 to-white",
  2: "from-sky-50/90 to-white",
  3: "from-emerald-50/90 to-white",
  4: "from-amber-50/90 to-white",
  5: "from-violet-50/90 to-white",
};

export function planningSlotColor(label: string): PlanningSlotColor {
  const key = label.trim().toLowerCase() || "cours";
  let hash = 0;
  for (let i = 0; i < key.length; i += 1) {
    hash = (hash + key.charCodeAt(i) * (i + 3)) % 9973;
  }
  return PALETTE[hash % PALETTE.length]!;
}

export function planningDayHeaderClass(day: number): string {
  const tint = DAY_HEADER_COLORS[day] || "from-slate-50 to-white";
  return `${DAY_HEADER_CLASS} bg-gradient-to-b ${tint}`;
}

export function planningWeekTabClass(active: boolean, week: "A" | "B"): string {
  if (!active) return "bg-white border border-slate-200 text-slate-600";
  return week === "A"
    ? "bg-indigo-600 text-white shadow-sm"
    : "bg-violet-600 text-white shadow-sm";
}

export function planningSlotCardClass(label: string): string {
  const c = planningSlotColor(label);
  return `h-full rounded-lg border px-1.5 py-1 text-[10px] leading-tight overflow-hidden shadow-sm ${c.bg} ${c.border}`;
}

export function planningSlotTimeClass(label: string): string {
  return `font-bold tabular-nums ${planningSlotColor(label).title}`;
}

export function planningSlotTitleTextClass(label: string): string {
  return `font-semibold truncate ${planningSlotColor(label).title}`;
}

export function planningSlotMetaTextClass(label: string): string {
  return `truncate ${planningSlotColor(label).meta}`;
}
