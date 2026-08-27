import "server-only";

import { and, desc, eq, ne } from "drizzle-orm";
import { getDb } from "@/db/index";
import { session } from "@/db/schema";
import {
  describeUserAgent,
  type SessionDevicePublic,
} from "@/app/lib/session-device";

export type { SessionDevicePublic };

export type SessionDeviceRow = {
  id: string;
  token: string;
  createdAt: Date;
  updatedAt: Date;
  expiresAt: Date;
  ipAddress: string | null;
  userAgent: string | null;
};

export function toPublicSession(
  row: SessionDeviceRow,
  currentSessionId: string | null,
): SessionDevicePublic {
  return {
    id: row.id,
    createdAt: row.createdAt.toISOString(),
    updatedAt: row.updatedAt.toISOString(),
    expiresAt: row.expiresAt.toISOString(),
    ipAddress: row.ipAddress,
    userAgent: row.userAgent,
    deviceLabel: describeUserAgent(row.userAgent),
    current: Boolean(currentSessionId && row.id === currentSessionId),
  };
}

export async function listSessionsForAuthUserId(
  authUserId: string,
): Promise<SessionDeviceRow[]> {
  const db = getDb();
  const rows = await db
    .select({
      id: session.id,
      token: session.token,
      createdAt: session.createdAt,
      updatedAt: session.updatedAt,
      expiresAt: session.expiresAt,
      ipAddress: session.ipAddress,
      userAgent: session.userAgent,
    })
    .from(session)
    .where(eq(session.userId, authUserId))
    .orderBy(desc(session.updatedAt));
  return rows;
}

export async function revokeSessionByIdForUser(
  authUserId: string,
  sessionId: string,
): Promise<boolean> {
  const db = getDb();
  const deleted = await db
    .delete(session)
    .where(and(eq(session.userId, authUserId), eq(session.id, sessionId)))
    .returning({ id: session.id });
  return deleted.length > 0;
}

export async function revokeOtherSessionsForUser(
  authUserId: string,
  keepSessionId: string,
): Promise<number> {
  const db = getDb();
  const deleted = await db
    .delete(session)
    .where(and(eq(session.userId, authUserId), ne(session.id, keepSessionId)))
    .returning({ id: session.id });
  return deleted.length;
}

export async function revokeAllSessionsForUser(authUserId: string): Promise<number> {
  const db = getDb();
  const deleted = await db
    .delete(session)
    .where(eq(session.userId, authUserId))
    .returning({ id: session.id });
  return deleted.length;
}
