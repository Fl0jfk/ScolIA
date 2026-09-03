import { parseParisDateTime } from "@/app/lib/paris-time";
import type { PortesOuvertesSlot } from "@/app/lib/toolbox-types";

export type PortesOuvertesSlotIntervalMinutes = 15 | 30 | 60;

function pad2(n: number): string {
  return String(n).padStart(2, "0");
}

function formatSlotLabel(startHm: string, endHm: string): string {
  const pretty = (hm: string) => {
    const [h, m] = hm.split(":").map(Number);
    if (!m) return `${h} h`;
    return `${h} h ${pad2(m)}`;
  };
  return `${pretty(startHm)} – ${pretty(endHm)}`;
}

function addMinutesToHm(hm: string, minutes: number): string {
  const [h, m] = hm.split(":").map(Number);
  const total = (h || 0) * 60 + (m || 0) + minutes;
  const nh = Math.floor(total / 60);
  const nm = total % 60;
  return `${pad2(nh)}:${pad2(nm)}`;
}

function hmToMinutes(hm: string): number {
  const [h, m] = hm.split(":").map(Number);
  return (h || 0) * 60 + (m || 0);
}

/**
 * Génère une grille de créneaux pour une journée (quart / demi-heure / heure).
 * Ex. 08:30 → 12:00 par 30 min → 08:30–09:00, 09:00–09:30, …
 */
export function generatePortesOuvertesSlots(params: {
  date: string;
  startTime: string;
  endTime: string;
  intervalMinutes: PortesOuvertesSlotIntervalMinutes;
  maxPlaces?: number;
}): PortesOuvertesSlot[] {
  const date = params.date.trim();
  const startTime = params.startTime.trim();
  const endTime = params.endTime.trim();
  const interval = params.intervalMinutes;
  if (!/^\d{4}-\d{2}-\d{2}$/.test(date)) return [];
  if (!/^\d{1,2}:\d{2}$/.test(startTime) || !/^\d{1,2}:\d{2}$/.test(endTime)) return [];

  const startMin = hmToMinutes(startTime);
  const endMin = hmToMinutes(endTime);
  if (endMin <= startMin) return [];

  const slots: PortesOuvertesSlot[] = [];
  let cursor = startMin;
  let i = 0;
  while (cursor + interval <= endMin) {
    const startHm = `${pad2(Math.floor(cursor / 60))}:${pad2(cursor % 60)}`;
    const endHm = addMinutesToHm(startHm, interval);
    const startDate = parseParisDateTime(date, startHm);
    const endDate = parseParisDateTime(date, endHm);
    if (!startDate || !endDate) break;
    const id = `slot-${date.replace(/-/g, "")}-${startHm.replace(":", "")}-${i}-${Date.now().toString(36)}`;
    slots.push({
      id,
      label: formatSlotLabel(startHm, endHm),
      startAt: startDate.toISOString(),
      endAt: endDate.toISOString(),
      maxPlaces: params.maxPlaces && params.maxPlaces > 0 ? params.maxPlaces : undefined,
    });
    cursor += interval;
    i += 1;
  }
  return slots;
}
