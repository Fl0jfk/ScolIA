import type {
  InternatInstallationBooking,
  InternatInstallationConfig,
  InternatInstallationPublicSlot,
} from "@/app/lib/internat-types";

function timeToMinutes(hhmm: string): number {
  const [h, m] = hhmm.split(":").map(Number);
  return h * 60 + m;
}

function minutesToTime(total: number): string {
  const h = Math.floor(total / 60);
  const m = total % 60;
  return `${String(h).padStart(2, "0")}:${String(m).padStart(2, "0")}`;
}

/** Clé créneau murale Europe/Paris : `YYYY-MM-DDTHH:mm`. */
function installationSlotKey(date: string, time: string): string {
  return `${date}T${time}`;
}

export function parseInstallationSlotKey(slotStart: string): { date: string; time: string } | null {
  const m = /^(\d{4}-\d{2}-\d{2})T(\d{2}:\d{2})$/.exec(slotStart.trim());
  if (!m) return null;
  return { date: m[1], time: m[2] };
}

export function listGeneratedSlotKeys(config: InternatInstallationConfig): string[] {
  const duration = config.slotDurationMinutes;
  const closed = new Set(config.closedSlots);
  const keys: string[] = [];
  for (const day of config.days) {
    let cursor = timeToMinutes(day.openTime);
    const end = timeToMinutes(day.closeTime);
    if (!(end > cursor)) continue;
    while (cursor + duration <= end) {
      const time = minutesToTime(cursor);
      const key = installationSlotKey(day.date, time);
      if (!closed.has(key)) keys.push(key);
      cursor += duration;
    }
  }
  return keys;
}

export function countBookingsBySlot(
  rows: InternatInstallationBooking[],
): Record<string, number> {
  const out: Record<string, number> = {};
  for (const r of rows) {
    if (!isConfirmedInstallationBooking(r)) continue;
    out[r.slotStart] = (out[r.slotStart] || 0) + 1;
  }
  return out;
}

/** Absent ou `confirmed` : occupe le créneau. `pending` n’occupe pas. */
export function isConfirmedInstallationBooking(row: InternatInstallationBooking): boolean {
  return row.status !== "pending";
}

export function buildPublicInstallationSlots(
  config: InternatInstallationConfig,
  bookings: InternatInstallationBooking[],
): InternatInstallationPublicSlot[] {
  const counts = countBookingsBySlot(bookings);
  const capacity = config.maxFamiliesPerSlot;
  const out: InternatInstallationPublicSlot[] = [];
  for (const slotStart of listGeneratedSlotKeys(config)) {
    const parsed = parseInstallationSlotKey(slotStart);
    if (!parsed) continue;
    const taken = counts[slotStart] || 0;
    const remaining = Math.max(0, capacity - taken);
    if (remaining <= 0) continue;
    out.push({
      slotStart,
      date: parsed.date,
      time: parsed.time,
      remaining,
      capacity,
    });
  }
  return out;
}

export function isValidOpenInstallationSlot(
  config: InternatInstallationConfig,
  slotStart: string,
  bookings: InternatInstallationBooking[],
): boolean {
  if (!config.enabled) return false;
  if (!listGeneratedSlotKeys(config).includes(slotStart)) return false;
  const taken = countBookingsBySlot(bookings)[slotStart] || 0;
  return taken < config.maxFamiliesPerSlot;
}

export function formatInstallationSlotFr(slotStart: string): string {
  const parsed = parseInstallationSlotKey(slotStart);
  if (!parsed) return slotStart;
  const d = new Date(`${parsed.date}T12:00:00`);
  const dateLabel = d.toLocaleDateString("fr-FR", {
    weekday: "long",
    day: "numeric",
    month: "long",
    year: "numeric",
  });
  return `${dateLabel} à ${parsed.time}`;
}
