import { calendarDateKeyParis } from "@/app/lib/domain-planning-dates";

/** Lundi → vendredi de la semaine courante (fuseau Paris). */
export function schoolWeekDaysParis(): { key: string; short: string }[] {
  const now = new Date();
  const wdStr = new Intl.DateTimeFormat("en-US", {
    timeZone: "Europe/Paris",
    weekday: "short",
  }).format(now);
  const wd = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"].indexOf(wdStr);
  const daysFromMonday = wd === 0 ? -6 : 1 - wd;

  const days: { key: string; short: string }[] = [];
  for (let i = 0; i < 5; i++) {
    const d = new Date(now);
    d.setDate(d.getDate() + daysFromMonday + i);
    const key = d.toLocaleDateString("en-CA", { timeZone: "Europe/Paris" });
    const short = d
      .toLocaleDateString("fr-FR", { timeZone: "Europe/Paris", weekday: "short" })
      .replace(".", "")
      .slice(0, 3);
    days.push({ key, short });
  }
  return days;
}

/** Index 0=lundi … 4=vendredi dans la semaine scolaire courante ; -1 si hors semaine. */
export function todaySchoolWeekDayIndex(): number {
  const todayKey = calendarDateKeyParis();
  return schoolWeekDaysParis().findIndex((d) => d.key === todayKey);
}

/** « Jean Dupont » → « Dupont Jean » */
function formatNomPrenom(displayName: string): string {
  const parts = displayName.trim().split(/\s+/).filter(Boolean);
  if (parts.length < 2) return displayName.trim();
  const nom = parts[parts.length - 1];
  const prenom = parts.slice(0, -1).join(" ");
  return `${nom} ${prenom}`;
}
