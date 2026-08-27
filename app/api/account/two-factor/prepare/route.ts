import { NextResponse } from "next/server";
import { getBetterAuth } from "@/app/lib/auth-server";
import { consumeRateLimit } from "@/app/lib/rate-limit";
import {
  clearIncompleteTwoFactorSetup,
  ensureTwoFactorVerifiedColumnHealthy,
} from "@/app/lib/two-factor-setup";
import { getDb } from "@/db/index";
import { user } from "@/db/schema";
import { eq } from "drizzle-orm";

/**
 * Prépare (ou remet à zéro) le setup MFA avant génération du QR.
 * Nettoie les secrets orphelins après un abandon mid-flux.
 */
export async function POST(req: Request) {
  const auth = getBetterAuth();
  const session = await auth.api.getSession({ headers: req.headers });
  if (!session?.user?.id) {
    return NextResponse.json({ error: "Non authentifié." }, { status: 401 });
  }

  const userId = session.user.id;
  const fwd =
    req.headers.get("x-real-ip")?.trim() ||
    req.headers.get("x-forwarded-for")?.split(",")[0]?.trim() ||
    "unknown";
  const rate = await consumeRateLimit({
    key: `two-factor-prepare:${userId}:${fwd}`,
    limit: 30,
    windowMs: 15 * 60 * 1000,
  });
  if (!rate.ok) {
    return NextResponse.json({ error: "Trop de tentatives." }, { status: 429 });
  }

  await ensureTwoFactorVerifiedColumnHealthy();

  const db = getDb();
  const [row] = await db
    .select({ twoFactorEnabled: user.twoFactorEnabled })
    .from(user)
    .where(eq(user.id, userId))
    .limit(1);

  if (row?.twoFactorEnabled) {
    return NextResponse.json({
      ok: true,
      alreadyEnabled: true,
      cleared: false,
    });
  }

  const cleared = await clearIncompleteTwoFactorSetup(userId);
  return NextResponse.json({ ok: true, alreadyEnabled: false, cleared });
}
