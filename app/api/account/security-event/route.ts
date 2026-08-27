import { NextResponse } from "next/server";
import { getBetterAuth } from "@/app/lib/auth-server";
import { consumeRateLimit } from "@/app/lib/rate-limit";
import {
  writeSecurityAudit,
  type SecurityAuditAction,
} from "@/app/lib/security-audit";
import { forcePromoteTwoFactorEnabled } from "@/app/lib/two-factor-setup";

const ALLOWED: SecurityAuditAction[] = ["two_factor_enabled", "two_factor_disabled"];

/** Journalisation côté client pour événements 2FA (session requise). */
export async function POST(req: Request) {
  const auth = getBetterAuth();
  const session = await auth.api.getSession({ headers: req.headers });
  if (!session?.user?.id) {
    return NextResponse.json({ error: "Non authentifié." }, { status: 401 });
  }

  const fwd = req.headers.get("x-forwarded-for")?.split(",")[0]?.trim() || "unknown";
  const rate = await consumeRateLimit({
    key: `security-event:${session.user.id}:${fwd}`,
    limit: 20,
    windowMs: 15 * 60 * 1000,
  });
  if (!rate.ok) {
    return NextResponse.json({ error: "Trop de tentatives." }, { status: 429 });
  }

  let action: string;
  try {
    const body = (await req.json()) as { action?: string };
    action = String(body.action ?? "");
  } catch {
    return NextResponse.json({ error: "JSON invalide." }, { status: 400 });
  }

  if (!ALLOWED.includes(action as SecurityAuditAction)) {
    return NextResponse.json({ error: "Action non autorisée." }, { status: 400 });
  }

  let promoted = false;
  if (action === "two_factor_enabled") {
    promoted = await forcePromoteTwoFactorEnabled(session.user.id);
  }

  await writeSecurityAudit({
    userId: session.user.id,
    action: action as SecurityAuditAction,
    req,
    metadata: action === "two_factor_enabled" ? { promoted } : undefined,
  });

  return NextResponse.json({ ok: true, promoted });
}
