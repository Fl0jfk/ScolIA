import { NextResponse } from "next/server";
import {
  applyOutingDecision,
  sanitizeOutingForTokenView,
} from "@/app/lib/internat-outing-decision";
import { findOutingByToken, saveInternatOuting } from "@/app/lib/internat-storage";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";

const outingDecisionLimiter = createMemoryRateLimiter({
  windowMs: 10 * 60 * 1000,
  max: 10,
});

export async function GET(req: Request) {
  const { searchParams } = new URL(req.url);
  const token = String(searchParams.get("token") || "").trim();
  if (!token) {
    return NextResponse.json({ error: "Jeton manquant." }, { status: 400 });
  }

  const outing = await findOutingByToken(token);
  if (!outing) {
    return NextResponse.json({ error: "Lien invalide ou expiré." }, { status: 404 });
  }

  const view = sanitizeOutingForTokenView(outing, token);
  if (!view) {
    return NextResponse.json({ error: "Lien invalide." }, { status: 404 });
  }

  return NextResponse.json({ outing: view });
}

export async function POST(req: Request) {
  if (!outingDecisionLimiter.allow(clientIpFromRequest(req))) {
    return NextResponse.json(
      { error: "Trop de tentatives. Réessayez dans quelques minutes." },
      { status: 429 },
    );
  }

  const body = await req.json().catch(() => ({}));
  const token = String(body.token || "").trim();
  const decision = body.decision === "refuse" ? "refuse" : body.decision === "approve" ? "approve" : null;
  const decidedBy = String(body.respondentName || body.decidedBy || "").trim() || undefined;
  const note = String(body.note || "").trim() || undefined;

  if (!token || !decision) {
    return NextResponse.json({ error: "Jeton et décision requis." }, { status: 400 });
  }

  const outing = await findOutingByToken(token);
  if (!outing) {
    return NextResponse.json({ error: "Lien invalide ou expiré." }, { status: 404 });
  }

  const clientIp = clientIpFromRequest(req);

  try {
    const { outing: updated, alreadyDecided } = await applyOutingDecision({
      kind: "token",
      token,
      decision,
      decidedBy,
      note,
      clientIp,
      outing,
    });
    await saveInternatOuting(updated);
    const view = sanitizeOutingForTokenView(updated, token);
    return NextResponse.json({
      ok: true,
      alreadyDecided,
      outing: view,
      status: updated.status,
    });
  } catch (e: unknown) {
    const message = e instanceof Error ? e.message : "Décision impossible.";
    return NextResponse.json({ error: message }, { status: 400 });
  }
}
