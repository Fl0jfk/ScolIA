import { NextResponse } from "next/server";
import {
  canUserSignupOnSession,
  isSvtSubject,
  lockedSessionIdeaForSession,
  lockedSubjectForSession,
  signupRequiresSessionIdea,
  TRANSVERSAL_NIVEAU_LABELS,
} from "@/app/lib/domain-planning-defaults";
import { getDomainPlanningUserDisplay, isDomainCoordinator } from "@/app/lib/domain-planning-auth";
import {
  findDomainById,
  findSessionById,
  loadSignups,
  saveSignups,
} from "@/app/lib/domain-planning-storage";
import type { DomainPlanningSignup } from "@/app/lib/domain-planning-types";
import { requireAuth, isIntranetAdmin } from "@/app/lib/intranet-auth";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { resolveMemberProfileById } from "@/app/lib/members-db";
import { createTenantTransporter, getTenantSmtpConfig } from "@/app/lib/tenant-mail";

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
  const notifyEmail = Boolean(body.notifyEmail);
  const onBehalfRaw =
    body?.onBehalfOf && typeof body.onBehalfOf === "object" ? body.onBehalfOf : null;
  const onBehalfUserId = onBehalfRaw ? String(onBehalfRaw.userId || "").trim() : "";

  if (!sessionId || !className) {
    return NextResponse.json({ error: "Séance et classe obligatoires." }, { status: 400 });
  }

  const session = await findSessionById(sessionId);
  if (!session) return NextResponse.json({ error: "Séance introuvable." }, { status: 404 });

  const isCoordinator =
    (await isIntranetAdmin()) ||
    (await isDomainCoordinator(authUser.userId, session.domainId));

  if (onBehalfUserId && !isCoordinator) {
    return NextResponse.json(
      { error: "Seul un responsable de domaine peut positionner un collègue." },
      { status: 403 },
    );
  }

  if (onBehalfUserId && onBehalfUserId === authUser.userId) {
    return NextResponse.json(
      { error: "Pour vous-même, décochez « Positionner un collègue »." },
      { status: 400 },
    );
  }

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

  let targetUserId = authUser.userId;
  let targetFirstName = authUser.firstName;
  let targetLastName = authUser.lastName;
  let targetEmail = authUser.email;
  let assignmentKind: DomainPlanningSignup["assignmentKind"] = "self";
  let submittedBy: DomainPlanningSignup["submittedBy"];
  let createdByUserId: string | undefined;

  if (onBehalfUserId) {
    const profile = await resolveMemberProfileById(onBehalfUserId);
    if (!profile) {
      return NextResponse.json(
        { error: "Personne introuvable dans l'annuaire." },
        { status: 404 },
      );
    }
    targetUserId = profile.id;
    targetFirstName = profile.firstName?.trim() || "";
    targetLastName = (profile.lastName?.trim() || "").toUpperCase();
    targetEmail = profile.email?.trim() || "";
    if (!targetEmail) {
      return NextResponse.json(
        { error: "E-mail de la personne sélectionnée introuvable." },
        { status: 400 },
      );
    }
    assignmentKind = "coordinator";
    createdByUserId = authUser.userId;
    submittedBy = {
      userId: authUser.userId,
      name: [authUser.firstName, authUser.lastName].filter(Boolean).join(" ").trim() || authUser.email,
      email: authUser.email,
    };
  }

  const existing = await loadSignups();
  const slotTaken = existing.some((s) => s.sessionId === sessionId && s.className === className);
  if (slotTaken) {
    return NextResponse.json({ error: "Un intervenant est déjà positionné sur cette classe." }, { status: 409 });
  }

  const nowIso = new Date().toISOString();
  const isOnBehalf = assignmentKind === "coordinator";

  const signup: DomainPlanningSignup = {
    id: `signup_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
    sessionId,
    className,
    userId: targetUserId,
    firstName: targetFirstName,
    lastName: targetLastName,
    email: targetEmail,
    subject,
    sessionIdea: sessionIdea || undefined,
    createdAt: nowIso,
    validationStatus: isOnBehalf ? "validated" : "pending",
    validatedAt: isOnBehalf ? nowIso : undefined,
    validatedByUserId: isOnBehalf ? authUser.userId : undefined,
    createdByUserId,
    submittedBy,
    assignmentKind,
  };

  await saveSignups([...existing, signup]);

  if (isOnBehalf && notifyEmail && targetEmail) {
    try {
      const smtp = await getTenantSmtpConfig();
      const transporter = smtp ? await createTenantTransporter() : null;
      if (transporter && smtp) {
        const domain = await findDomainById(session.domainId);
        const domainName = domain?.name?.trim() || "Enseignements transversaux";
        const niveauLabel =
          TRANSVERSAL_NIVEAU_LABELS[session.niveau] || session.niveau;
        const actorName = submittedBy?.name || authUser.email;
        const greeting = targetFirstName || "Bonjour";
        await transporter.sendMail({
          from: `"Enseignements transversaux" <${smtp.user}>`,
          to: targetEmail,
          subject: `Positionnement — ${domainName}`,
          html: `
            <div style="font-family:sans-serif;max-width:600px;margin:0 auto;">
              <h2 style="color:#7c3aed;">Vous avez été positionné(e) — ${domainName}</h2>
              <p>Bonjour ${greeting},</p>
              <p>
                <strong>${actorName}</strong> vous a positionné(e) sur la séance suivante :
              </p>
              <ul>
                <li>Niveau : <strong>${niveauLabel}</strong></li>
                <li>Séance : <strong>${session.seanceNumber}</strong></li>
                <li>Classe : <strong>${className}</strong></li>
                <li>Thème : <strong>${session.theme || "—"}</strong></li>
                <li>Matière : <strong>${subject}</strong></li>
                ${sessionIdea ? `<li>Idée de séance : <em>${sessionIdea}</em></li>` : ""}
              </ul>
              <p>Ce positionnement est <strong>déjà validé</strong>.</p>
            </div>
          `,
        });
      }
    } catch (err: unknown) {
      console.error("[domain-planning/signups/create] notify email failed:", err);
    }
  }

  return NextResponse.json({ ok: true, signup });
}
