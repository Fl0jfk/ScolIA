import { getJson, putJson } from "@/app/lib/s3-storage";
import {
  INTERNAT_S3,
  type InternatInstallationBooking,
  type InternatInstallationConfig,
} from "@/app/lib/internat-types";

export const DEFAULT_INSTALLATION_CONFIG: InternatInstallationConfig = {
  enabled: false,
  title: "Installation internat — prise de rendez-vous",
  intro:
    "Choisissez un créneau pour venir prendre possession de la chambre. Un e-mail de confirmation vous sera envoyé.",
  location: "",
  slotDurationMinutes: 30,
  maxFamiliesPerSlot: 1,
  days: [],
  closedSlots: [],
};

function normalizeDay(raw: unknown): InternatInstallationConfig["days"][number] | null {
  if (!raw || typeof raw !== "object") return null;
  const o = raw as Record<string, unknown>;
  const date = String(o.date || "").trim();
  const openTime = String(o.openTime || "").trim();
  const closeTime = String(o.closeTime || "").trim();
  if (!/^\d{4}-\d{2}-\d{2}$/.test(date)) return null;
  if (!/^\d{2}:\d{2}$/.test(openTime) || !/^\d{2}:\d{2}$/.test(closeTime)) return null;
  return { date, openTime, closeTime };
}

export function normalizeInstallationConfig(
  raw: Partial<InternatInstallationConfig> | null | undefined,
): InternatInstallationConfig {
  const base = DEFAULT_INSTALLATION_CONFIG;
  const days = Array.isArray(raw?.days)
    ? raw!.days.map(normalizeDay).filter((d): d is NonNullable<typeof d> => Boolean(d))
    : [];
  const closedSlots = Array.isArray(raw?.closedSlots)
    ? raw!.closedSlots.map((s) => String(s).trim()).filter(Boolean)
    : [];
  const duration = Number(raw?.slotDurationMinutes);
  const maxFam = Number(raw?.maxFamiliesPerSlot);
  return {
    enabled: raw?.enabled === true,
    title: String(raw?.title || base.title).trim() || base.title,
    intro: String(raw?.intro ?? base.intro ?? "").trim() || undefined,
    location: String(raw?.location ?? "").trim() || undefined,
    slotDurationMinutes:
      Number.isFinite(duration) && duration >= 5 && duration <= 180
        ? Math.round(duration)
        : base.slotDurationMinutes,
    maxFamiliesPerSlot:
      Number.isFinite(maxFam) && maxFam >= 1 && maxFam <= 50
        ? Math.round(maxFam)
        : base.maxFamiliesPerSlot,
    days: days.sort((a, b) => a.date.localeCompare(b.date) || a.openTime.localeCompare(b.openTime)),
    closedSlots,
  };
}

export async function getInstallationConfig(): Promise<InternatInstallationConfig> {
  const hit = await getJson<InternatInstallationConfig>(INTERNAT_S3.installationConfig);
  return normalizeInstallationConfig(hit?.data);
}

export async function saveInstallationConfig(
  config: InternatInstallationConfig,
): Promise<InternatInstallationConfig> {
  const normalized = normalizeInstallationConfig(config);
  await putJson(INTERNAT_S3.installationConfig, normalized);
  return normalized;
}

export async function listInstallationBookings(): Promise<InternatInstallationBooking[]> {
  const hit = await getJson<InternatInstallationBooking[]>(INTERNAT_S3.installationBookings);
  return Array.isArray(hit?.data) ? hit.data : [];
}

export async function saveInstallationBookings(
  rows: InternatInstallationBooking[],
): Promise<void> {
  await putJson(INTERNAT_S3.installationBookings, rows);
}

export function countBookingsBySlot(
  rows: InternatInstallationBooking[],
): Record<string, number> {
  const out: Record<string, number> = {};
  for (const r of rows) {
    out[r.slotStart] = (out[r.slotStart] || 0) + 1;
  }
  return out;
}

export async function addInstallationBooking(
  row: Omit<InternatInstallationBooking, "id" | "createdAt">,
): Promise<InternatInstallationBooking> {
  const list = await listInstallationBookings();
  const entry: InternatInstallationBooking = {
    ...row,
    id: `inst-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    createdAt: new Date().toISOString(),
  };
  list.push(entry);
  await saveInstallationBookings(list);
  return entry;
}

export async function deleteInstallationBooking(id: string): Promise<boolean> {
  const list = await listInstallationBookings();
  const next = list.filter((b) => b.id !== id);
  if (next.length === list.length) return false;
  await saveInstallationBookings(next);
  return true;
}
