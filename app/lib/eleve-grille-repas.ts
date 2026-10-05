/**
 * Grille repas Lun–Ven (Administratif → Passage / facturation).
 * Stockée en jsonb sur eleve_scolarite.grille_repas.
 */

export type MealDayKey = "lun" | "mar" | "mer" | "jeu" | "ven";

export type EleveGrilleRepasDay = {
  midi: boolean;
  soir: boolean;
  etude: boolean;
  garderie: boolean;
  sortSeul: boolean;
};

export type EleveGrilleRepas = Record<MealDayKey, EleveGrilleRepasDay>;

export const MEAL_DAY_ORDER: Array<{ key: MealDayKey; label: string }> = [
  { key: "lun", label: "Lun" },
  { key: "mar", label: "Mar" },
  { key: "mer", label: "Mer" },
  { key: "jeu", label: "Jeu" },
  { key: "ven", label: "Ven" },
];

export function emptyGrilleRepasDay(): EleveGrilleRepasDay {
  return { midi: false, soir: false, etude: false, garderie: false, sortSeul: false };
}

export function emptyGrilleRepas(): EleveGrilleRepas {
  return {
    lun: emptyGrilleRepasDay(),
    mar: emptyGrilleRepasDay(),
    mer: emptyGrilleRepasDay(),
    jeu: emptyGrilleRepasDay(),
    ven: emptyGrilleRepasDay(),
  };
}

function asBool(v: unknown): boolean {
  return v === true || v === 1 || v === "1" || v === "true";
}

/** Parse JSON BDD / body API → grille normalisée, ou null si structure invalide. */
export function parseEleveGrilleRepas(
  raw: unknown,
  opts?: { allowEmpty?: boolean },
): EleveGrilleRepas | null {
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) return null;
  const obj = raw as Record<string, unknown>;
  const out = emptyGrilleRepas();
  let any = false;
  let seenDay = false;
  for (const { key } of MEAL_DAY_ORDER) {
    const day = obj[key];
    if (!day || typeof day !== "object" || Array.isArray(day)) continue;
    seenDay = true;
    const d = day as Record<string, unknown>;
    out[key] = {
      midi: asBool(d.midi),
      soir: asBool(d.soir),
      etude: asBool(d.etude),
      garderie: asBool(d.garderie),
      sortSeul: asBool(d.sortSeul ?? d.sort_seul),
    };
    if (
      out[key].midi ||
      out[key].soir ||
      out[key].etude ||
      out[key].garderie ||
      out[key].sortSeul
    ) {
      any = true;
    }
  }
  if (!seenDay) return null;
  if (!any && !opts?.allowEmpty) return null;
  return out;
}

export function countMidiFromGrille(grille: EleveGrilleRepas): number {
  return MEAL_DAY_ORDER.reduce((n, d) => n + (grille[d.key].midi ? 1 : 0), 0);
}

export function toggleGrilleCell(
  grille: EleveGrilleRepas,
  day: MealDayKey,
  field: keyof EleveGrilleRepasDay,
): EleveGrilleRepas {
  return {
    ...grille,
    [day]: {
      ...grille[day],
      [field]: !grille[day][field],
    },
  };
}

/** Initialise une grille éditable depuis l’affichage synthèse (déduit ou saisi). */
export function grilleFromMealDays(
  days: Array<{
    key: MealDayKey;
    midi: boolean;
    soir: boolean;
    etude?: boolean;
    garderie?: boolean;
    sortSeul?: boolean;
  }>,
): EleveGrilleRepas {
  const g = emptyGrilleRepas();
  for (const d of days) {
    g[d.key] = {
      midi: Boolean(d.midi),
      soir: Boolean(d.soir),
      etude: Boolean(d.etude),
      garderie: Boolean(d.garderie),
      sortSeul: Boolean(d.sortSeul),
    };
  }
  return g;
}

const ISO_DATE_RE = /^\d{4}-\d{2}-\d{2}$/;

const WEEKDAY_SHORT_TO_MEAL: Record<string, MealDayKey> = {
  Mon: "lun",
  Tue: "mar",
  Wed: "mer",
  Thu: "jeu",
  Fri: "ven",
};

/**
 * Jour de grille (Lun–Ven) pour une date calendaire AAAA-MM-JJ (Europe/Paris).
 * Week-end / invalide → null.
 */
export function mealDayKeyFromIsoDate(iso: string): MealDayKey | null {
  const key = iso.trim().slice(0, 10);
  if (!ISO_DATE_RE.test(key)) return null;
  const [y, m, d] = key.split("-").map(Number) as [number, number, number];
  const anchor = new Date(Date.UTC(y, m - 1, d, 12, 0, 0));
  const wd = new Intl.DateTimeFormat("en-US", {
    timeZone: "Europe/Paris",
    weekday: "short",
  }).format(anchor);
  return WEEKDAY_SHORT_TO_MEAL[wd] ?? null;
}
