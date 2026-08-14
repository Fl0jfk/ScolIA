import { getJson, putJson } from "@/app/lib/s3-storage";

type ClassAllocationAuditEntry = {
  at: string;
  action: string;
  actor: string;
  details?: Record<string, unknown>;
};

export async function appendClassAllocationAudit(entry: ClassAllocationAuditEntry): Promise<void> {
  const key = "class-allocation/audit-log.json";
  const hit = await getJson<ClassAllocationAuditEntry[]>(key);
  const list = Array.isArray(hit?.data) ? hit.data : [];
  list.push(entry);
  await putJson(key, list.slice(-2000));
}
