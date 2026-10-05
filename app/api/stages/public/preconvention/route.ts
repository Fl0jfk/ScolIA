import { NextResponse } from "next/server";
import { createPublicPreconventionDraft } from "@/app/lib/stage-workflow";
import { verifyStudentForPreconvention } from "@/app/lib/stage-student-identity";
import { buildStudentStageDossier } from "@/app/lib/stage-student-dossier";
import {
  getStagePeriodsForClass,
  getStageRemindersForClass,
  isClassEligibleForStage,
} from "@/app/lib/stage-periods-config";
import { getStageConstraintsPublicContext } from "@/app/lib/stage-constraints-config";
import { getElevePhotoUrl } from "@/app/lib/eleve-photos";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";
import {
  assertIdentityProof,
  confirmIdentityOtpCode,
  createAndSendIdentityOtp,
  loadIdentityProof,
  subjectsMatch,
  type StageIdentitySubject,
} from "@/app/lib/stage-identity-otp";
import {
  createIdentityRecipientChoiceSession,
  discardIdentityRecipientChoiceSession,
  listStageIdentityRecipientEmails,
  loadIdentityRecipientChoiceSession,
  resolveSelectedRecipientEmails,
} from "@/app/lib/stage-identity-recipients";

const preconventionLimiter = createMemoryRateLimiter({
  windowMs: 10 * 60 * 1000,
  max: 40,
});

const identityLimiter = createMemoryRateLimiter({
  windowMs: 15 * 60 * 1000,
  max: 20,
});

const otpConfirmLimiter = createMemoryRateLimiter({
  windowMs: 15 * 60 * 1000,
  max: 30,
});

const GENERIC_IDENTITY_ERROR =
  "Nom, prénom ou date de naissance incorrects. Vérifiez l'orthographe (comme sur le bulletin), ou contactez le secrétariat.";

async function verifyAndLoadStudent(params: {
  nom: string;
  prenom: string;
  dateNaissance: string;
  classe?: string;
}) {
  const verified = await verifyStudentForPreconvention(params);
  if (!verified.ok) {
    if (verified.reason === "ambiguous") {
      return { ok: false as const, ambiguous: true as const, candidates: verified.candidates };
    }
    return { ok: false as const };
  }

  const { eleve, ...student } = verified.student;
  const eligibility = await isClassEligibleForStage(student.className);
  if (!eligibility.ok) {
    return { ok: false as const, error: eligibility.reason, status: 403 as const };
  }

  const [dossier, reminders, periods, photoUrl, constraints] = await Promise.all([
    buildStudentStageDossier(student),
    getStageRemindersForClass(student.className),
    getStagePeriodsForClass(student.className),
    getElevePhotoUrl(eleve).catch(() => null),
    getStageConstraintsPublicContext({
      level: student.level,
      className: student.className,
    }),
  ]);

  return {
    ok: true as const,
    eleve,
    student,
    dossier,
    photoUrl,
    stageContext: { reminders, periods, constraints },
    parent1Email:
      eleve.parent1Email?.trim() ||
      eleve.parentEmail?.trim() ||
      undefined,
    parent2Email: eleve.parent2Email?.trim() || undefined,
  };
}

function identitySubjectFromLoaded(
  loaded: Extract<Awaited<ReturnType<typeof verifyAndLoadStudent>>, { ok: true }>,
  dateNaissanceFallback: string,
): StageIdentitySubject {
  return {
    nom: loaded.student.lastName,
    prenom: loaded.student.firstName,
    dateNaissance: loaded.student.dateNaissance || dateNaissanceFallback,
    classe: loaded.student.className,
    eleveKey: loaded.eleve.ine?.trim() || undefined,
  };
}

function buildIdentifySuccessPayload(
  loaded: Extract<Awaited<ReturnType<typeof verifyAndLoadStudent>>, { ok: true }>,
  identityProof?: string,
) {
  return {
    success: true as const,
    mode: "dashboard" as const,
    identityProof: identityProof || undefined,
    studentPreview: {
      firstName: loaded.student.firstName,
      lastName: loaded.student.lastName,
      className: loaded.student.className,
      photoUrl: loaded.photoUrl || null,
      parent1Email: loaded.parent1Email || null,
      parent2Email: loaded.parent2Email || null,
      parentPhone:
        loaded.eleve.parent1Phone?.trim() ||
        loaded.eleve.parentPhone?.trim() ||
        null,
      parent2Phone: loaded.eleve.parent2Phone?.trim() || null,
      studentEmail: loaded.eleve.email?.trim() || null,
    },
    dossier: {
      schoolYear: loaded.dossier.schoolYear,
      conventions: loaded.dossier.conventions,
      availablePeriods: loaded.dossier.availablePeriods,
      canCreateNew: loaded.dossier.canCreateNew,
    },
    stageContext: loaded.stageContext,
  };
}

async function buildRecipientChoiceForLoaded(
  loaded: Extract<Awaited<ReturnType<typeof verifyAndLoadStudent>>, { ok: true }>,
  dateNaissance: string,
) {
  const subject = identitySubjectFromLoaded(loaded, dateNaissance);
  const studentName = `${loaded.student.firstName} ${loaded.student.lastName}`.trim();
  const emails = await listStageIdentityRecipientEmails(loaded.eleve);
  return createIdentityRecipientChoiceSession({ subject, studentName, emails });
}

async function sendOtpForChoiceSession(params: {
  recipientSessionId: string;
  selectedRecipientIds: string[];
}) {
  const session = await loadIdentityRecipientChoiceSession(params.recipientSessionId);
  if (!session) {
    return {
      ok: false as const,
      error: "Session expirée. Identifiez-vous à nouveau.",
      status: 403 as const,
    };
  }
  const recipients = resolveSelectedRecipientEmails(session, params.selectedRecipientIds);
  if (recipients.length === 0) {
    return {
      ok: false as const,
      error: "Sélectionnez au moins une adresse e-mail pour recevoir le code.",
      status: 400 as const,
    };
  }

  const otp = await createAndSendIdentityOtp({
    subject: session.subject,
    recipients,
    studentName: session.studentName,
  });
  if (!otp.ok) {
    return { ok: false as const, error: otp.error, status: 403 as const };
  }
  if (otp.sentCount === 0) {
    return {
      ok: false as const,
      error:
        "Impossible d'envoyer le code par e-mail pour le moment. Réessayez plus tard ou contactez le secrétariat.",
      status: 503 as const,
    };
  }

  return {
    ok: true as const,
    challengeId: otp.challengeId,
    maskedRecipients: otp.maskedRecipients,
    recipientSessionId: session.sessionId,
  };
}

/** Identification élève → choix destinataires masqués → OTP → tableau de bord. */
export async function POST(req: Request) {
  try {
    if (!(await preconventionLimiter.allow(clientIpFromRequest(req)))) {
      return NextResponse.json(
        { error: "Trop de tentatives. Réessayez dans quelques minutes." },
        { status: 429 },
      );
    }

    const body = await req.json();
    const action = String(body.action ?? "identify");

    if (action === "confirm_identity_otp") {
      if (!(await otpConfirmLimiter.allow(clientIpFromRequest(req)))) {
        return NextResponse.json(
          { error: "Trop de tentatives. Réessayez dans quelques minutes." },
          { status: 429 },
        );
      }
      const challengeId = String(body.challengeId ?? "").trim();
      const code = String(body.code ?? "").trim();
      if (!challengeId || !code) {
        return NextResponse.json({ error: "Code manquant." }, { status: 400 });
      }

      const confirmed = await confirmIdentityOtpCode({ challengeId, code });
      if (!confirmed.ok) {
        return NextResponse.json({ error: confirmed.error }, { status: 403 });
      }

      let loaded: Awaited<ReturnType<typeof verifyAndLoadStudent>>;
      try {
        loaded = await verifyAndLoadStudent({
          nom: confirmed.subject.nom,
          prenom: confirmed.subject.prenom,
          dateNaissance: confirmed.subject.dateNaissance,
          classe: confirmed.subject.classe,
        });
      } catch (error) {
        console.error("[stages/preconvention] identity reload after otp failed", error);
        return NextResponse.json(
          {
            error:
              "Le service d'identification est temporairement indisponible. Réessayez dans quelques minutes ou contactez le secrétariat.",
          },
          { status: 503 },
        );
      }
      if (!loaded.ok) {
        return NextResponse.json({ error: GENERIC_IDENTITY_ERROR }, { status: 403 });
      }

      const recipientSessionId = String(body.recipientSessionId ?? "").trim();
      if (recipientSessionId) {
        await discardIdentityRecipientChoiceSession(recipientSessionId);
      }

      return NextResponse.json(buildIdentifySuccessPayload(loaded, confirmed.proofToken));
    }

    if (action === "send_identity_otp") {
      if (!(await identityLimiter.allow(clientIpFromRequest(req)))) {
        return NextResponse.json({ error: GENERIC_IDENTITY_ERROR }, { status: 403 });
      }
      const recipientSessionId = String(body.recipientSessionId ?? "").trim();
      const selectedRecipientIds = Array.isArray(body.selectedRecipientIds)
        ? body.selectedRecipientIds.map((x: unknown) => String(x ?? "").trim()).filter(Boolean)
        : [];
      if (!recipientSessionId) {
        return NextResponse.json({ error: "Session destinataires manquante." }, { status: 400 });
      }
      const sent = await sendOtpForChoiceSession({ recipientSessionId, selectedRecipientIds });
      if (!sent.ok) {
        return NextResponse.json({ error: sent.error }, { status: sent.status });
      }
      return NextResponse.json({
        success: true,
        needsOtp: true,
        challengeId: sent.challengeId,
        recipientSessionId: sent.recipientSessionId,
        maskedRecipients: sent.maskedRecipients,
        message:
          "Le code a été envoyé. Vérifiez aussi vos spams / courriers indésirables.",
      });
    }

    const nom = String(body.nom ?? "").trim();
    const prenom = String(body.prenom ?? "").trim();
    const dateNaissance = String(body.dateNaissance ?? "").trim();
    const classe = String(body.classe ?? "").trim() || undefined;
    const identityProof = String(body.identityProof ?? "").trim() || undefined;

    if (!nom || !prenom || !dateNaissance) {
      return NextResponse.json(
        { error: "Le nom, le prénom et la date de naissance sont obligatoires." },
        { status: 400 },
      );
    }

    if (!(await identityLimiter.allow(clientIpFromRequest(req)))) {
      return NextResponse.json({ error: GENERIC_IDENTITY_ERROR }, { status: 403 });
    }

    let loaded: Awaited<ReturnType<typeof verifyAndLoadStudent>>;
    try {
      loaded = await verifyAndLoadStudent({ nom, prenom, dateNaissance, classe });
    } catch (error) {
      console.error("[stages/preconvention] identity lookup failed", error);
      return NextResponse.json(
        {
          error:
            "Le service d'identification est temporairement indisponible. Réessayez dans quelques minutes ou contactez le secrétariat.",
        },
        { status: 503 },
      );
    }
    if (!loaded.ok) {
      if ("ambiguous" in loaded && loaded.ambiguous) {
        return NextResponse.json({
          success: false,
          needsClass: true,
          candidates: loaded.candidates,
          message:
            "Plusieurs élèves correspondent. Sélectionnez votre classe pour continuer.",
        });
      }
      if ("error" in loaded && loaded.error) {
        return NextResponse.json({ error: loaded.error }, { status: loaded.status ?? 403 });
      }
      return NextResponse.json({ error: GENERIC_IDENTITY_ERROR }, { status: 403 });
    }

    const subject = identitySubjectFromLoaded(loaded, dateNaissance);

    if (action === "identify") {
      if (identityProof) {
        const proof = await loadIdentityProof(identityProof);
        if (proof && subjectsMatch(proof.subject, subject)) {
          return NextResponse.json(buildIdentifySuccessPayload(loaded, identityProof));
        }
      }

      const choice = await buildRecipientChoiceForLoaded(loaded, dateNaissance);
      if (!choice.ok) {
        return NextResponse.json({ error: choice.error }, { status: 403 });
      }

      // Un seul destinataire → envoi immédiat (pas d'étape de choix).
      if (choice.publicOptions.length === 1) {
        const sent = await sendOtpForChoiceSession({
          recipientSessionId: choice.session.sessionId,
          selectedRecipientIds: [choice.publicOptions[0]!.id],
        });
        if (!sent.ok) {
          return NextResponse.json({ error: sent.error }, { status: sent.status });
        }
        return NextResponse.json({
          success: true,
          needsOtp: true,
          challengeId: sent.challengeId,
          recipientSessionId: sent.recipientSessionId,
          maskedRecipients: sent.maskedRecipients,
          message:
            "Le code a été envoyé. Vérifiez aussi vos spams / courriers indésirables.",
        });
      }

      return NextResponse.json({
        success: true,
        needsOtpRecipientChoice: true,
        recipientSessionId: choice.session.sessionId,
        recipientOptions: choice.publicOptions,
        message:
          "Choisissez la ou les adresses (masquées) où envoyer le code d'accès.",
      });
    }

    if (action === "resend_identity_otp") {
      const recipientSessionId = String(body.recipientSessionId ?? "").trim();
      const selectedRecipientIds = Array.isArray(body.selectedRecipientIds)
        ? body.selectedRecipientIds.map((x: unknown) => String(x ?? "").trim()).filter(Boolean)
        : [];

      if (recipientSessionId && selectedRecipientIds.length > 0) {
        const sent = await sendOtpForChoiceSession({ recipientSessionId, selectedRecipientIds });
        if (!sent.ok) {
          return NextResponse.json({ error: sent.error }, { status: sent.status });
        }
        return NextResponse.json({
          success: true,
          needsOtp: true,
          challengeId: sent.challengeId,
          recipientSessionId: sent.recipientSessionId,
          maskedRecipients: sent.maskedRecipients,
          message:
            "Un nouveau code a été envoyé. Pensez à vérifier vos spams / courriers indésirables.",
        });
      }

      // Repli : reconstruire le choix (ex. session expirée).
      const choice = await buildRecipientChoiceForLoaded(loaded, dateNaissance);
      if (!choice.ok) {
        return NextResponse.json({ error: choice.error }, { status: 403 });
      }
      if (choice.publicOptions.length === 1) {
        const sent = await sendOtpForChoiceSession({
          recipientSessionId: choice.session.sessionId,
          selectedRecipientIds: [choice.publicOptions[0]!.id],
        });
        if (!sent.ok) {
          return NextResponse.json({ error: sent.error }, { status: sent.status });
        }
        return NextResponse.json({
          success: true,
          needsOtp: true,
          challengeId: sent.challengeId,
          recipientSessionId: sent.recipientSessionId,
          maskedRecipients: sent.maskedRecipients,
          message:
            "Un nouveau code a été envoyé. Pensez à vérifier vos spams / courriers indésirables.",
        });
      }
      return NextResponse.json({
        success: true,
        needsOtpRecipientChoice: true,
        recipientSessionId: choice.session.sessionId,
        recipientOptions: choice.publicOptions,
        message: "Resélectionnez une adresse pour renvoyer le code.",
      });
    }

    if (action === "create") {
      const proofCheck = await assertIdentityProof(identityProof, subject);
      if (!proofCheck.ok) {
        return NextResponse.json({ error: proofCheck.error }, { status: 403 });
      }

      if (!loaded.dossier.canCreateNew) {
        return NextResponse.json(
          {
            error:
              "Vous avez déjà ouvert tous les stages prévus pour cette année. Contactez le secrétariat si vous devez en ajouter un.",
          },
          { status: 400 },
        );
      }

      const periodId = String(body.periodId ?? "").trim() || undefined;
      const selectedPeriod = periodId
        ? loaded.dossier.availablePeriods.find((p) => p.id === periodId)
        : undefined;

      if (periodId && selectedPeriod?.used) {
        return NextResponse.json(
          { error: "Cette période de stage est déjà utilisée par un autre dossier." },
          { status: 400 },
        );
      }

      const parent1Override = String(body.parent1Email ?? "").trim() || undefined;
      const parent2Override = String(body.parent2Email ?? "").trim() || undefined;

      const { convention, studentLink } = await createPublicPreconventionDraft({
        ...loaded.student,
        dateNaissance: loaded.student.dateNaissance || dateNaissance,
        email: loaded.eleve.email?.trim() || undefined,
        parent1Email: parent1Override || loaded.parent1Email,
        parent2Email: parent2Override || loaded.parent2Email,
        parentEmail: parent1Override || loaded.parent1Email,
        matchedEleveIne: loaded.eleve.ine,
        stagePeriodId: selectedPeriod?.id,
        stageLabel: selectedPeriod?.label || String(body.stageLabel ?? "").trim() || undefined,
        periodStart: selectedPeriod?.periodStart,
        periodEnd: selectedPeriod?.periodEnd,
      });

      return NextResponse.json({
        success: true,
        mode: "created",
        conventionId: convention.id,
        studentLink,
        identityProof,
      });
    }

    return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
