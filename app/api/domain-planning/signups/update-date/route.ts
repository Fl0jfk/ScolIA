import { NextResponse } from "next/server";
import { normalizeSignupSessionDate } from "@/app/lib/domain-planning-defaults";
import { getDomainPlanningUserDisplay, isDomainCoordinator } from "@/app/lib/domain-planning-auth";
import { findSessionById, loadSignups, saveSignups } from "@/app/lib/domain-planning-storage";
import { requireAuth, isIntranetAdmin } from "@/app/lib/intranet-auth";

/**
 * Fixe ou efface la date de séance d'un positionnement.
 * Autorisé à l'intervenant positionné et à la responsable du domaine.
 * Ne touche pas au statut de validation.
 */
export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const authUser = await getDomainPlanningUserDisplay();
  const body = await req.json().catch(() => ({}));
  const id = String(body?.id || "").trim();
  const raw = typeof body?.sessionDate === "string" ? body.sessionDate.trim() : "";
  const sessionDate = normalizeSignupSessionDate(raw);

  if (!id) {
    return NextResponse.json({ error: "Positionnement introuvable." }, { status: 400 });
  }
  if (raw && !sessionDate) {
    return NextResponse.json({ error: "Date de séance invalide." }, { status: 400 });
  }

  const signups = await loadSignups();
  const idx = signups.findIndex((s) => s.id === id);
  if (idx < 0) {
    return NextResponse.json({ error: "Positionnement introuvable." }, { status: 404 });
  }

  const signup = signups[idx];
  const session = await findSessionById(signup.sessionId);
  const isOwner = signup.userId === authUser.userId;
  const isCoordinator =
    (await isIntranetAdmin()) ||
    (session ? await isDomainCoordinator(authUser.userId, session.domainId) : false);

  if (!isOwner && !isCoordinator) {
    return NextResponse.json(
      { error: "Seuls l'intervenant et la responsable du domaine peuvent modifier la date." },
      { status: 403 },
    );
  }

  signups[idx] = { ...signup, sessionDate };
  await saveSignups(signups);
  return NextResponse.json({ ok: true, signup: signups[idx] });
}
