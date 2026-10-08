import { NextResponse } from "next/server";
import { getDomainPlanningUserDisplay, isDomainCoordinator } from "@/app/lib/domain-planning-auth";
import { findSessionById, loadSignups, saveSignups } from "@/app/lib/domain-planning-storage";
import { signupRequiresSessionIdea } from "@/app/lib/domain-planning-defaults";
import type { DomainPlanningSignupValidationStatus } from "@/app/lib/domain-planning-types";
import { requireAuth, isIntranetAdmin } from "@/app/lib/intranet-auth";

const ACTIONS = new Set(["validate", "changes_requested", "reject"]);

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const authUser = await getDomainPlanningUserDisplay();

  const body = await req.json();
  const id = String(body.id || "").trim();
  const action = String(body.action || "validate").trim();
  const comment = String(body.comment || "").trim();

  if (!id) {
    return NextResponse.json({ error: "Positionnement introuvable." }, { status: 400 });
  }
  if (!ACTIONS.has(action)) {
    return NextResponse.json({ error: "Action de validation invalide." }, { status: 400 });
  }
  if ((action === "changes_requested" || action === "reject") && !comment) {
    return NextResponse.json(
      { error: "Merci d'indiquer un commentaire pour cette décision." },
      { status: 400 },
    );
  }

  const signups = await loadSignups();
  const idx = signups.findIndex((s) => s.id === id);
  if (idx < 0) {
    return NextResponse.json({ error: "Positionnement introuvable." }, { status: 404 });
  }

  const signup = signups[idx];
  const session = await findSessionById(signup.sessionId);

  const canValidate =
    (await isIntranetAdmin()) ||
    (session
      ? await isDomainCoordinator(authUser.userId, session.domainId)
      : false);
  if (!canValidate) {
    return NextResponse.json(
      { error: "Seule la responsable du domaine peut valider les positionnements." },
      { status: 403 },
    );
  }


  if (
    action === "validate" &&
    session &&
    signupRequiresSessionIdea(session) &&
    !signup.sessionIdea?.trim()
  ) {
    return NextResponse.json(
      { error: "L'intervenant doit d'abord proposer une idée de séance." },
      { status: 400 },
    );
  }

  let validationStatus: DomainPlanningSignupValidationStatus;
  if (action === "validate") validationStatus = "validated";
  else if (action === "changes_requested") validationStatus = "changes_requested";
  else validationStatus = "rejected";

  signups[idx] = {
    ...signup,
    validationStatus,
    validatedAt: new Date().toISOString(),
    validatedByUserId: authUser.userId,
    validationComment: action === "validate" ? undefined : comment,
  };

  await saveSignups(signups);
  return NextResponse.json({ ok: true, signup: signups[idx] });
}
