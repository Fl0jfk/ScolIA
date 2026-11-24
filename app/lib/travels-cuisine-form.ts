import { CUISINE_DAYS as PDF_DAYS, CUISINE_ROWS as PDF_ROWS } from "@/app/lib/travels-cuisine-shared";

export const CUISINE_DAYS_UI = PDF_DAYS.map((d) => ({
  key: d.key,
  label: d.label.slice(0, 3) + ".",
  fullLabel: d.label,
}));

export const CUISINE_ROWS_UI = PDF_ROWS.map((r) => ({
  key: r.key,
  label: r.label.trim(),
  type: r.key === "other" ? ("text" as const) : ("number" as const),
}));

export function emptyCuisineDetails() {
  const emptyDay = {
    picnicTotal: "",
    picnicNoPork: "",
    picnicVeg: "",
    selfAdults: "",
    selfStudents: "",
    coffee: "",
    juice: "",
    cakes: "",
    pastries: "",
    other: "",
  };
  return {
    active: false,
    deliveryTime: "",
    deliveryPlace: "Self",
    daysSelection: { lundi: false, mardi: false, mercredi: false, jeudi: false, vendredi: false },
    orders: {
      lundi: { ...emptyDay },
      mardi: { ...emptyDay },
      mercredi: { ...emptyDay },
      jeudi: { ...emptyDay },
      vendredi: { ...emptyDay },
    },
  };
}

export function getTotalMeals(details: {
  active?: boolean;
  daysSelection?: Record<string, boolean>;
  orders?: Record<string, { picnicTotal?: string }>;
  picnicTotal?: string | number;
} | null | undefined): number {
  if (!details?.active) return 0;
  if (details.orders) {
    return CUISINE_DAYS_UI.reduce((sum, { key }) => {
      if (!details.daysSelection?.[key]) return sum;
      const val = Number(details.orders?.[key]?.picnicTotal || 0);
      return sum + (Number.isFinite(val) ? val : 0);
    }, 0);
  }
  const legacy = Number(details.picnicTotal || 0);
  return Number.isFinite(legacy) ? legacy : 0;
}

/**
 * Nombre de jours de restauration cochés. Dès qu'il y a des clés de date ISO (circuit v2), seules les dates réelles
 * comptent : le miroir « jour de semaine » (`vendredi`…) recopie la même date et ne doit pas la compter deux fois.
 * Structure historique (v1, séries) : jours de semaine uniquement, inchangé.
 */
export function countCuisineDays(daysSelection: Record<string, boolean> | null | undefined): number {
  const checked = Object.entries(daysSelection || {}).filter(([, v]) => Boolean(v));
  const iso = checked.filter(([k]) => /^\d{4}-\d{2}-\d{2}$/.test(k));
  return iso.length > 0 ? iso.length : checked.length;
}
