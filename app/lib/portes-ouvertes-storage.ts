import { getJson, putJson } from "@/app/lib/s3-storage";

type PortesOuvertesRegistration = {
  id: string;
  slotId: string;
  firstName: string;
  lastName: string;
  email: string;
  phone?: string;
  childrenInfo?: string;
  consent: boolean;
  createdAt: string;
};

const KEY = "toolbox/portes-ouvertes/registrations.json";

export async function listPortesOuvertesRegistrations(): Promise<PortesOuvertesRegistration[]> {
  const raw = await getJson<PortesOuvertesRegistration[]>(KEY);
  return Array.isArray(raw?.data) ? raw.data : [];
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

export function countRegistrationsBySlot(
  rows: PortesOuvertesRegistration[],
): Record<string, number> {
  const out: Record<string, number> = {};
  for (const r of rows) {
    out[r.slotId] = (out[r.slotId] || 0) + 1;
  }
  return out;
}
