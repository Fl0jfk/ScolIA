import { getJson, putJson } from "@/app/lib/s3-storage";
import type { PortesOuvertesRegistration } from "@/app/lib/portes-ouvertes-types";
import { isPortesOuvertesCycle } from "@/app/lib/portes-ouvertes-types";

const KEY = "toolbox/portes-ouvertes/registrations.json";

function parseActor(raw: unknown): { userId: string; name: string } | undefined {
  if (!raw || typeof raw !== "object") return undefined;
  const o = raw as Record<string, unknown>;
  const userId = String(o.userId || "").trim();
  if (!userId) return undefined;
  return {
    userId,
    name: String(o.name || "").trim() || "Accueil",
  };
}

function parseRegistration(raw: unknown): PortesOuvertesRegistration | null {
  if (!raw || typeof raw !== "object") return null;
  const o = raw as Record<string, unknown>;
  const id = String(o.id || "").trim();
  const slotId = String(o.slotId || "").trim();
  const firstName = String(o.firstName || "").trim();
  const lastName = String(o.lastName || "").trim();
  const email = String(o.email || "").trim().toLowerCase();
  if (!id || !slotId || !firstName || !lastName || !email) return null;
  const phone = String(o.phone || "").trim() || undefined;
  const childrenInfo = String(o.childrenInfo || "").trim() || undefined;
  const cycle = isPortesOuvertesCycle(o.cycle) ? o.cycle : undefined;
  const classeSouhaitee = String(o.classeSouhaitee || "").trim() || undefined;
  const source =
    o.source === "accueil" || o.source === "public" ? o.source : undefined;
  return {
    id,
    slotId,
    slotLabel: String(o.slotLabel || "").trim() || undefined,
    slotStartAt: String(o.slotStartAt || "").trim() || undefined,
    slotEndAt: String(o.slotEndAt || "").trim() || undefined,
    firstName,
    lastName,
    email,
    phone,
    childrenInfo,
    cycle,
    classeSouhaitee,
    consent: o.consent === true || o.consent === undefined,
    source,
    recordedBy: parseActor(o.recordedBy),
    lastModifiedBy: parseActor(o.lastModifiedBy),
    createdAt: String(o.createdAt || new Date().toISOString()),
    updatedAt: String(o.updatedAt || "").trim() || undefined,
  };
}

export type { PortesOuvertesRegistration };

export async function listPortesOuvertesRegistrations(): Promise<PortesOuvertesRegistration[]> {
  const raw = await getJson<unknown[]>(KEY);
  if (!Array.isArray(raw?.data)) return [];
  return raw.data.map(parseRegistration).filter((r): r is PortesOuvertesRegistration => Boolean(r));
}

async function savePortesOuvertesRegistrations(rows: PortesOuvertesRegistration[]): Promise<void> {
  await putJson(KEY, rows);
}

export async function addPortesOuvertesRegistration(
  row: Omit<PortesOuvertesRegistration, "id" | "createdAt">,
  existing?: PortesOuvertesRegistration[],
): Promise<PortesOuvertesRegistration> {
  const list = existing ? [...existing] : await listPortesOuvertesRegistrations();
  const entry: PortesOuvertesRegistration = {
    ...row,
    id: `po-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    createdAt: new Date().toISOString(),
  };
  list.push(entry);
  await savePortesOuvertesRegistrations(list);
  return entry;
}

export async function updatePortesOuvertesRegistration(
  id: string,
  patch: Partial<
    Omit<PortesOuvertesRegistration, "id" | "createdAt" | "consent" | "source" | "recordedBy">
  >,
  existing?: PortesOuvertesRegistration[],
): Promise<PortesOuvertesRegistration | null> {
  const list = existing ? [...existing] : await listPortesOuvertesRegistrations();
  const idx = list.findIndex((r) => r.id === id);
  if (idx < 0) return null;
  const updated: PortesOuvertesRegistration = {
    ...list[idx],
    ...patch,
    id: list[idx].id,
    createdAt: list[idx].createdAt,
    consent: list[idx].consent,
    source: list[idx].source,
    recordedBy: list[idx].recordedBy,
    updatedAt: new Date().toISOString(),
  };
  list[idx] = updated;
  await savePortesOuvertesRegistrations(list);
  return updated;
}

export function countRegistrationsBySlot(
  rows: PortesOuvertesRegistration[],
): Record<string, number> {
  const out: Record<string, number> = {};
  for (const r of rows) {
    out[r.slotId] = (out[r.slotId] || 0) + 1;
  }
  return out;
}
