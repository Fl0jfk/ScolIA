import { calendarDateKeyParis } from "@/app/lib/domain-planning-dates";

export const WIZARD_DATE_OTHER = "__OTHER_DATE__";

export function wizardStep(current: number, total: number, body: string): string {
  return `Étape ${current}/${total} — ${body}`;
}

export function addDaysToKey(dateKey: string, days: number): string {
  const [y, m, d] = dateKey.split("-").map(Number);
  const next = new Date(Date.UTC(y!, m! - 1, d! + days, 12, 0, 0));
  return next.toISOString().slice(0, 10);
}

export function weekdayLabelFr(dateKey: string): string {
  return new Intl.DateTimeFormat("fr-FR", {
    timeZone: "UTC",
    weekday: "long",
    day: "numeric",
    month: "short",
  }).format(new Date(`${dateKey}T12:00:00Z`));
}

function isWeekendKey(dateKey: string): boolean {
  const wd = new Date(`${dateKey}T12:00:00Z`).getUTCDay();
  return wd === 0 || wd === 6;
}

/** Aujourd'hui, demain, jours ouvrés suivants + « Autre date ». */
export function buildDateQuickOptions(
  today: string = calendarDateKeyParis(),
): Array<{ value: string; label: string }> {
  const opts: Array<{ value: string; label: string }> = [
    { value: today, label: `Aujourd'hui (${weekdayLabelFr(today)})` },
  ];
  const tomorrow = addDaysToKey(today, 1);
  opts.push({ value: tomorrow, label: `Demain (${weekdayLabelFr(tomorrow)})` });
  let cursor = tomorrow;
  let added = 0;
  while (added < 5) {
    cursor = addDaysToKey(cursor, 1);
    if (isWeekendKey(cursor)) continue;
    opts.push({ value: cursor, label: weekdayLabelFr(cursor) });
    added += 1;
  }
  opts.push({ value: WIZARD_DATE_OTHER, label: "Autre date…" });
  return opts;
}
