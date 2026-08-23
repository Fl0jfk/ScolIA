import "server-only";

import { getDb } from "@/db/index";
import { securityAuditLog } from "@/db/schema";

export type SecurityAuditAction =
  | "password_changed"
  | "email_change_requested"
  | "email_change_confirmed"
  | "email_change_immediate"
  | "two_factor_enabled"
  | "two_factor_disabled"
  | "account_claimed";

export async function writeSecurityAudit(opts: {
  userId: string | null;
  action: SecurityAuditAction;
  req?: Request;
  metadata?: Record<string, unknown>;
}): Promise<void> {
  const ip =
    opts.req?.headers.get("x-forwarded-for")?.split(",")[0]?.trim() ||
    opts.req?.headers.get("x-real-ip")?.trim() ||
    null;
  const userAgent = opts.req?.headers.get("user-agent")?.slice(0, 512) || null;
  try {
    const db = getDb();
    await db.insert(securityAuditLog).values({
      userId: opts.userId,
      action: opts.action,
      ipAddress: ip,
      userAgent,
      metadata: opts.metadata ?? null,
    });
  } catch (error) {
    console.error("[security-audit]", opts.action, error);
  }
}
