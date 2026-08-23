import "server-only";

import { getDb } from "@/db/index";
import { dataAccessAudit } from "@/db/schema";

export type DataAccessAction =
  | "list"
  | "read"
  | "export"
  | "download"
  | "search";

export type DataAccessResourceType =
  | "eleves_registry"
  | "eleve_dossier"
  | "personnel"
  | "travel"
  | "document"
  | "settings"
  | "members";

function requestMeta(req?: Request): { ip: string | null; userAgent: string | null } {
  const ip =
    req?.headers.get("x-forwarded-for")?.split(",")[0]?.trim() ||
    req?.headers.get("x-real-ip")?.trim() ||
    null;
  const userAgent = req?.headers.get("user-agent")?.slice(0, 512) || null;
  return { ip, userAgent };
}

export async function writeDataAccessAudit(opts: {
  etablissementId: string;
  userId: string | null;
  resourceType: DataAccessResourceType;
  resourceId?: string | null;
  action: DataAccessAction;
  req?: Request;
  metadata?: Record<string, unknown>;
}): Promise<void> {
  const { ip, userAgent } = requestMeta(opts.req);
  try {
    const db = getDb();
    await db.insert(dataAccessAudit).values({
      etablissementId: opts.etablissementId,
      userId: opts.userId,
      resourceType: opts.resourceType,
      resourceId: opts.resourceId ?? null,
      action: opts.action,
      ipAddress: ip,
      userAgent,
      metadata: opts.metadata ?? null,
    });
  } catch (error) {
    console.error("[data-access-audit]", opts.resourceType, opts.action, error);
  }
}
