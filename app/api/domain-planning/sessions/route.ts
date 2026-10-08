import { NextResponse } from "next/server";
import {
  DEFAULT_DOMAIN_ID,
  isTransversalNiveau,
  normalizeSessionConstraint,
} from "@/app/lib/domain-planning-defaults";
import {
  getDomainPlanningUserDisplay,
  isAnyDomainCoordinator,
} from "@/app/lib/domain-planning-auth";
import { loadSessions, saveSessions } from "@/app/lib/domain-planning-storage";
import type { DomainPlanningSession } from "@/app/lib/domain-planning-types";
import { requireAuth, isIntranetAdmin } from "@/app/lib/intranet-auth";

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const sessions = await loadSessions();
  return NextResponse.json({ sessions });
}

export async function PUT(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const authUser = await getDomainPlanningUserDisplay();
  const isAdmin = await isIntranetAdmin();
  const isCoordinator = await isAnyDomainCoordinator(authUser.userId);
  if (!isAdmin && !isCoordinator) {
    return NextResponse.json(
      { error: "Réservé aux responsables de domaine." },
      { status: 403 },
    );
  }

  const body = await req.json();
  const raw = body?.sessions;
  if (!Array.isArray(raw)) {
    return NextResponse.json({ error: "Liste de séances invalide." }, { status: 400 });
  }

  const sessions: DomainPlanningSession[] = [];
  for (const item of raw) {
    if (!item || typeof item !== "object") continue;
    const o = item as Record<string, unknown>;
    const id = String(o.id || "").trim();
    const domainId = String(o.domainId || DEFAULT_DOMAIN_ID).trim() || DEFAULT_DOMAIN_ID;
    const theme = String(o.theme || "").trim();
    const intervenantLabel = String(o.intervenantLabel || "").trim();
    if (!id || !intervenantLabel) continue;
    if (!isTransversalNiveau(o.niveau)) continue;
    if (![1, 2, 3].includes(Number(o.seanceNumber))) continue;
    const constraint = normalizeSessionConstraint(o.intervenantConstraint, intervenantLabel);
    if (!constraint) continue;

    sessions.push({
      id,
      domainId,
      niveau: o.niveau,
      seanceNumber: Number(o.seanceNumber) as 1 | 2 | 3,
      theme,
      intervenantLabel,
      intervenantConstraint: constraint,
      mixte: Boolean(o.mixte),
    });
  }

  if (sessions.length === 0) {
    return NextResponse.json({ error: "Aucune séance valide." }, { status: 400 });
  }

  await saveSessions(sessions);
  return NextResponse.json({ ok: true, sessions });
}
