import { NextResponse } from "next/server";
import { getDomainPlanningUserDisplay, isDomainCoordinator } from "@/app/lib/domain-planning-auth";
import { findSessionById, loadSignups, saveSignups } from "@/app/lib/domain-planning-storage";
import { requireAuth, isIntranetAdmin } from "@/app/lib/intranet-auth";

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const authUser = await getDomainPlanningUserDisplay();
  const { id } = await req.json();
  const signupId = String(id || "").trim();
  if (!signupId) return NextResponse.json({ error: "Identifiant manquant." }, { status: 400 });

  const signups = await loadSignups();
  const target = signups.find((s) => s.id === signupId);
  if (!target) return NextResponse.json({ error: "Positionnement introuvable." }, { status: 404 });

  const session = await findSessionById(target.sessionId);
  const isCoordinator =
    (await isIntranetAdmin()) ||
    (session ? await isDomainCoordinator(authUser.userId, session.domainId) : false);
  const isOwner = target.userId === authUser.userId;

  if (!isCoordinator && !isOwner) {
    return NextResponse.json({ error: "Vous ne pouvez retirer que votre propre positionnement." }, { status: 403 });
  }

  await saveSignups(signups.filter((s) => s.id !== signupId));
  return NextResponse.json({ ok: true });
}
