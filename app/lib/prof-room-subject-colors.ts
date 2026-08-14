/** Presets Tailwind (valeur historique du module). */
export const PROF_ROOM_COLOR_PRESETS: { label: string; value: string }[] = [
  { label: "Bleu", value: "bg-blue-600 text-white" },
  { label: "Rouge", value: "bg-red-600 text-white" },
  { label: "Ambre", value: "bg-amber-700 text-white" },
  { label: "Rose", value: "bg-pink-600 text-white" },
  { label: "Rose clair", value: "bg-rose-500 text-white" },
  { label: "Rose EVARS", value: "bg-rose-600 text-white" },
  { label: "Pierre", value: "bg-stone-600 text-white" },
  { label: "Émeraude", value: "bg-emerald-600 text-white" },
  { label: "Jaune", value: "bg-yellow-500 text-white" },
  { label: "Orange", value: "bg-orange-600 text-white" },
  { label: "Fuchsia", value: "bg-fuchsia-600 text-white" },
  { label: "Violet", value: "bg-violet-600 text-white" },
  { label: "Ardoise", value: "bg-slate-400 text-white" },
  { label: "Indigo", value: "bg-indigo-600 text-white" },
  { label: "Cyan", value: "bg-cyan-600 text-white" },
  { label: "Sarcelle", value: "bg-teal-600 text-white" },
  { label: "Citron", value: "bg-lime-600 text-white" },
  { label: "Zinc", value: "bg-zinc-500 text-white" },
  { label: "Gris foncé", value: "bg-slate-600 text-white" },
];

const PRESET_TO_HEX: Record<string, string> = {
  "bg-blue-600 text-white": "#2563eb",
  "bg-red-600 text-white": "#dc2626",
  "bg-amber-700 text-white": "#b45309",
  "bg-pink-600 text-white": "#db2777",
  "bg-rose-500 text-white": "#f43f5e",
  "bg-rose-600 text-white": "#e11d48",
  "bg-stone-600 text-white": "#57534e",
  "bg-emerald-600 text-white": "#059669",
  "bg-yellow-500 text-white": "#eab308",
  "bg-orange-600 text-white": "#ea580c",
  "bg-fuchsia-600 text-white": "#c026d3",
  "bg-violet-600 text-white": "#7c3aed",
  "bg-slate-400 text-white": "#94a3b8",
  "bg-indigo-600 text-white": "#4f46e5",
  "bg-cyan-600 text-white": "#0891b2",
  "bg-teal-600 text-white": "#0d9488",
  "bg-lime-600 text-white": "#65a30d",
  "bg-zinc-500 text-white": "#71717a",
  "bg-slate-600 text-white": "#475569",
};

export function isHexSubjectColor(value: string): boolean {
  return /^#[0-9A-Fa-f]{6}$/.test(value.trim());
}

export function subjectColorToHex(value: string): string {
  if (isHexSubjectColor(value)) return value.trim().toLowerCase();
  return PRESET_TO_HEX[value] || "#475569";
}

function hexLuminance(hex: string): number {
  const h = hex.replace("#", "");
  const r = parseInt(h.slice(0, 2), 16) / 255;
  const g = parseInt(h.slice(2, 4), 16) / 255;
  const b = parseInt(h.slice(4, 6), 16) / 255;
  const lin = (c: number) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}

function hexToRgb(hex: string): [number, number, number] {
  const h = hex.replace("#", "");
  return [parseInt(h.slice(0, 2), 16), parseInt(h.slice(2, 4), 16), parseInt(h.slice(4, 6), 16)];
}

function rgbToHex(r: number, g: number, b: number): string {
  const byte = (n: number) => Math.max(0, Math.min(255, Math.round(n))).toString(16).padStart(2, "0");
  return `#${byte(r)}${byte(g)}${byte(b)}`;
}

function mixRgb(
  a: [number, number, number],
  b: [number, number, number],
  t: number,
): [number, number, number] {
  return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
}

export function getSubjectColorPresentation(value: string): {
  className?: string;
  style?: { backgroundColor: string; color: string };
} {
  if (isHexSubjectColor(value)) {
    const hex = value.trim();
    return {
      style: {
        backgroundColor: hex,
        color: hexLuminance(hex) > 0.45 ? "#0f172a" : "#ffffff",
      },
    };
  }
  return { className: value || "bg-slate-600 text-white" };
}

/** Fond de tuile planning : couleur matière un peu plus dense, texte contrasté. */
export function getReservationTilePresentation(value: string): {
  style: { backgroundColor: string; color: string };
} {
  const hex = subjectColorToHex(value);
  let rgb = hexToRgb(hex);
  if (hexLuminance(hex) > 0.42) {
    rgb = mixRgb(rgb, [20, 35, 26], 0.32);
  } else {
    rgb = mixRgb(rgb, [15, 23, 42], 0.12);
  }
  const backgroundColor = rgbToHex(rgb[0], rgb[1], rgb[2]);
  return {
    style: {
      backgroundColor,
      color: hexLuminance(backgroundColor) > 0.45 ? "#14231A" : "#ffffff",
    },
  };
}

export function isPresetSubjectColor(value: string): boolean {
  return PROF_ROOM_COLOR_PRESETS.some((p) => p.value === value);
}
