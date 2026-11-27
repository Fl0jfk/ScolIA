/** Suivi d'un interne (incidents + appels) : filtres, tris et calendrier. Module pur. */
import type {
  InternatIncident,
  InternatIncidentKind,
  InternatRollCallPeriod,
  InternatRollMark,
} from "@/app/lib/internat-types";

export type InternatAppelEntry = {
  date: string;
  period: InternatRollCallPeriod;
  mark: InternatRollMark;
  markLabel: string;
  note?: string;
  validatedBy?: string;
  validatedAt?: string;
};

function searchKey(raw: string): string {
  return raw
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase();
}

const pad2 = (n: number) => String(n).padStart(2, "0");

/** Année scolaire (1er août → 31 juillet) d'une date AAAA-MM-JJ : année de rentrée. */
export function schoolYearOf(date: string): number {
  const year = Number(date.slice(0, 4));
  return Number(date.slice(5, 7)) >= 8 ? year : year - 1;
}

/** Mois « AAAA-MM » de septembre jusqu'au mois donné, dans la même année scolaire. */
export function schoolYearMonthsUpTo(month: string): string[] {
  const start = schoolYearOf(`${month}-01`);
  const out: string[] = [];
  let y = start;
  let m = 8;
  while (`${y}-${pad2(m)}` <= month) {
    out.push(`${y}-${pad2(m)}`);
    m += 1;
    if (m > 12) {
      m = 1;
      y += 1;
    }
  }
  return out;
}

export function shiftMonth(month: string, delta: number): string {
  const total = Number(month.slice(0, 4)) * 12 + Number(month.slice(5, 7)) - 1 + delta;
  return `${Math.floor(total / 12)}-${pad2((total % 12) + 1)}`;
}

export function monthBounds(month: string): { from: string; to: string } {
  const last = new Date(Date.UTC(Number(month.slice(0, 4)), Number(month.slice(5, 7)), 0)).getUTCDate();
  return { from: `${month}-01`, to: `${month}-${pad2(last)}` };
}

/** Cases du calendrier, semaine commençant le lundi ; `null` = case vide avant le 1er. */
export function monthCells(month: string): Array<string | null> {
  const first = new Date(Date.UTC(Number(month.slice(0, 4)), Number(month.slice(5, 7)) - 1, 1));
  const blanks = (first.getUTCDay() + 6) % 7;
  const days = Number(monthBounds(month).to.slice(8));
  return [
    ...Array.from({ length: blanks }, () => null),
    ...Array.from({ length: days }, (_, i) => `${month}-${pad2(i + 1)}`),
  ];
}

export function countAppelMarks(entries: InternatAppelEntry[]): Record<InternatRollMark, number> {
  const out: Record<InternatRollMark, number> = { present: 0, absent: 0, excuse: 0, activite: 0 };
  for (const e of entries) if (e.mark in out) out[e.mark] += 1;
  return out;
}

export function filterAppels(
  entries: InternatAppelEntry[],
  opts: { mark: InternatRollMark | "all"; query: string; ascending: boolean },
): InternatAppelEntry[] {
  const q = searchKey(opts.query.trim());
  return entries
    .filter((e) => opts.mark === "all" || e.mark === opts.mark)
    .filter((e) => {
      if (!q) return true;
      const day = new Date(`${e.date}T12:00:00Z`).toLocaleDateString("fr-FR", {
        weekday: "long",
        day: "numeric",
        month: "long",
        year: "numeric",
        timeZone: "UTC",
      });
      const numeric = e.date.split("-").reverse().join("/");
      return searchKey(
        [day, numeric, e.markLabel, e.note, e.validatedBy, e.period].filter(Boolean).join(" "),
      ).includes(q);
    })
    .sort((a, b) => {
      const cmp = a.date.localeCompare(b.date) || a.period.localeCompare(b.period);
      return opts.ascending ? cmp : -cmp;
    });
}

export function filterIncidents(
  incidents: InternatIncident[],
  opts: { kind: InternatIncidentKind | "all"; schoolYear: number | "all"; query: string; ascending: boolean },
): InternatIncident[] {
  const q = searchKey(opts.query.trim());
  return incidents
    .filter((i) => opts.kind === "all" || i.kind === opts.kind)
    .filter((i) => opts.schoolYear === "all" || schoolYearOf(i.occurredAt) === opts.schoolYear)
    .filter(
      (i) =>
        !q ||
        searchKey(
          [
            i.title,
            i.description,
            i.location,
            i.actionsTaken,
            i.witnesses,
            i.createdBy?.name,
            i.occurredAt.split("-").reverse().join("/"),
          ]
            .filter(Boolean)
            .join(" "),
        ).includes(q),
    )
    .sort((a, b) => {
      const cmp = a.occurredAt.localeCompare(b.occurredAt) || a.createdAt.localeCompare(b.createdAt);
      return opts.ascending ? cmp : -cmp;
    });
}
