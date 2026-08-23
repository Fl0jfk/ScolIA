import { NextResponse } from "next/server";
import {
  canUserSignupOnSession,
  isSvtSubject,
  lockedSessionIdeaForSession,
  lockedSubjectForSession,
  signupRequiresSessionIdea,
} from "@/app/lib/domain-planning-defaults";
import { getDomainPlanningUserDisplay, isDomainCoordinator } from "@/app/lib/domain-planning-auth";
import {
  findSessionById,
  loadSignups,
  saveSignups,
} from "@/app/lib/domain-planning-storage";
import type { DomainPlanningSignup } from "@/app/lib/domain-planning-types";
import { requireAuth, isIntranetAdmin } from "@/app/lib/intranet-auth";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { safeCurrentUser } from "@/app/lib/intranet-session";

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const authUser = await getDomainPlanningUserDisplay();
  const sessionUser = await safeCurrentUser();
  const roles = intranetRolesFromMetadata(sessionUser?.publicMetadata);

  const body = await req.json();
  const sessionId = String(body.sessionId || "").trim();
  const className = String(body.className || "").trim();
  let subject = String(body.subject || "").trim();
  let sessionIdea = String(body.sessionIdea || "").trim();

  if (!sessionId || !className) {
    return NextResponse.json({ error: "Séance et classe obligatoires." }, { status: 400 });
  }

  const session = await findSessionById(sessionId);
  if (!session) return NextResponse.json({ error: "Séance introuvable." }, { status: 404 });

  const isCoordinator =
    (await isIntranetAdmin()) || (await isDomainCoordinator(authUser.userId, "evars"));

  if (session.intervenantConstraint === "fixed_association" && !isCoordinator) {
    return NextResponse.json(
      { error: "Cette séance est gérée par l'association : inscription non disponible." },
      { status: 403 },
    );
  }

  if (!canUserSignupOnSession(session, roles, isCoordinator)) {
    if (session.intervenantConstraint === "psy_inf") {
      return NextResponse.json(
        { error: "Cette séance est réservée aux psychologues et infirmières (rôle intranet requis)." },
        { status: 403 },
      );
    }
    return NextResponse.json(
      { error: "Vous n'avez pas le rôle requis pour vous positionner sur cette séance." },
      { status: 403 },
    );
  }

  const lockedSubject = lockedSubjectForSession(session);
  if (lockedSubject) subject = lockedSubject;

  const lockedIdea = lockedSessionIdeaForSession(session);
  if (lockedIdea) sessionIdea = lockedIdea;

  if (!subject) {
    return NextResponse.json({ error: "Matière obligatoire." }, { status: 400 });
  }

  if (session.intervenantConstraint === "svt_only" && !isSvtSubject(subject) && !isCoordinator) {
    return NextResponse.json(
      { error: "Cette séance est réservée aux professeurs de SVT." },
      { status: 403 },
    );
  }

  if (signupRequiresSessionIdea(session) && !sessionIdea) {
    return NextResponse.json(
      { error: "Merci de décrire votre idée de séance." },
      { status: 400 },
    );
  }

  const existing = await loadSignups();
  const slotTaken = existing.some((s) => s.sessionId === sessionId && s.className === className);
  if (slotTaken) {
    return NextResponse.json({ error: "Un intervenant est déjà positionné sur cette classe." }, { status: 409 });
  }

  const signup: DomainPlanningSignup = {
    id: `signup_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
    sessionId,
    className,
    userId: authUser.userId,
    firstName: authUser.firstName,
    lastName: authUser.lastName,
    email: authUser.email,
    subject,
    sessionIdea: sessionIdea || undefined,
    createdAt: new Date().toISOString(),
    validationStatus: "pending",
    validatedAt: undefined,
    validatedByUserId: undefined,
  };

  await saveSignups([...existing, signup]);
  return NextResponse.json({ ok: true, signup });
}
