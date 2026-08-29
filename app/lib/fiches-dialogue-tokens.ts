import "server-only";

import { randomBytes, randomInt } from "crypto";
import { and, eq, isNull } from "drizzle-orm";
import { getDb } from "@/db/index";
import { fdToken, type FdTokenRow } from "@/db/schema";

export function generateFdToken(): string {
  return randomBytes(32).toString("base64url");
}

export function generateFdSecureCode(): string {
  return String(randomInt(100_000, 1_000_000));
}

export function normalizeFdEmail(email: string): string {
  return email.trim().toLowerCase();
}

export async function createFdAccessToken(params: {
  etablissementId: string;
  ficheId: string;
  etapeId?: string | null;
  email?: string | null;
  purpose: string;
  expiresInDays?: number;
}): Promise<FdTokenRow> {
  const db = getDb();
  const token = generateFdToken();
  const secureCode = generateFdSecureCode();
  const expiresAt =
    params.expiresInDays && params.expiresInDays > 0
      ? new Date(Date.now() + params.expiresInDays * 24 * 60 * 60 * 1000)
      : null;

  const [row] = await db
    .insert(fdToken)
    .values({
      etablissementId: params.etablissementId,
      ficheId: params.ficheId,
      etapeId: params.etapeId ?? null,
      token,
      secureCode,
      email: params.email ? normalizeFdEmail(params.email) : null,
      purpose: params.purpose,
      expiresAt,
    })
    .returning();

  return row;
}

export async function resolveFdToken(token: string): Promise<FdTokenRow | null> {
  const db = getDb();
  const [row] = await db
    .select()
    .from(fdToken)
    .where(and(eq(fdToken.token, token), isNull(fdToken.revokedAt)))
    .limit(1);
  if (!row) return null;
  if (row.expiresAt && row.expiresAt.getTime() < Date.now()) return null;
  return row;
}

export async function resolveFdTokenBySecureCode(
  email: string,
  code: string,
): Promise<FdTokenRow | null> {
  const db = getDb();
  const normalized = normalizeFdEmail(email);
  const [row] = await db
    .select()
    .from(fdToken)
    .where(
      and(
        eq(fdToken.email, normalized),
        eq(fdToken.secureCode, code.trim()),
        isNull(fdToken.revokedAt),
      ),
    )
    .limit(1);
  if (!row) return null;
  if (row.expiresAt && row.expiresAt.getTime() < Date.now()) return null;
  return row;
}

export async function revokeFdTokensForFiche(
  ficheId: string,
  purpose?: string,
): Promise<void> {
  const db = getDb();
  const conditions = [eq(fdToken.ficheId, ficheId), isNull(fdToken.revokedAt)];
  if (purpose) conditions.push(eq(fdToken.purpose, purpose));
  await db
    .update(fdToken)
    .set({ revokedAt: new Date() })
    .where(and(...conditions));
}
