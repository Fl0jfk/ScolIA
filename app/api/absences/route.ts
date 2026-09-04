import { safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { NextResponse } from "next/server";

import { DeleteObjectCommand } from "@aws-sdk/client-s3";
import { loadAppConfig } from "@/app/lib/app-config";
import { requireAuth } from "@/app/lib/intranet-auth";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { s3Key } from "@/app/lib/s3-path";
import { normalizeAbsencePeriodInput } from "@/app/lib/absence-period";
import { validateHoursTreatmentForAbsence } from "@/app/lib/absence-hours-treatment";
import {
  canDeclareAbsenceOnBehalf,
  canManageAbsence,
  canViewAbsence,
  canViewCalendar,
  isAbsenceVisibleOnCalendar,
  computeStartEndAt,
  filterAbsenceForViewer,
  resolveAbsenceScope,
  resolveSelfDeclarationScope,
  type AbsenceRecord,
  type AbsenceScope,
  type Etablissement,
} from "@/app/lib/absences-types";
import { listDirectoryMembers } from "@/app/lib/directory-members";
import { getAbsenceDocumentKeys, isDocumentKeyReferenced } from "@/app/lib/absences-documents";
import {
  notifyAbsenceCreated,
  notifyAbsenceValidated,
  notifyAbsenceCreatorValidated,
  notifyAbsenceJustificatifRequested,
  notifyAbsenceJustificatifDeposited,
  notifyAbsenceAdminTreated,
} from "@/app/lib/absences-workflow-mail";
import {
  processorMayAccessValidatedAbsence,
  viewerIsAbsenceProcessor,
} from "@/app/lib/absences-admin-access";
import {
  consolidatePendingAbsencesInIndex,
  getAbsenceIndex,
  getAbsenceRecord,
  applyPostValidationPrivacy,
  purgeExpiredAbsences,
  saveAbsenceRecord,
  saveOrMergeAbsenceRecord,
} from "@/app/lib/absences-storage";
import { absencesDbReady, deleteAbsenceFromDb } from "@/app/lib/absence-db";
import {
  getAbsenceOrLegacyRecord,
  isDocumentKeyReferencedInLegacy,
  mergeLegacyConvocationsForCalendar,
} from "@/app/lib/absences-legacy-convocations";

function isTodayOverlap(record: AbsenceRecord) {
  const today = new Date();
  const dayStart = new Date(today.getFullYear(), today.getMonth(), today.getDate(), 0, 0, 0, 0);
  const dayEnd = new Date(today.getFullYear(), today.getMonth(), today.getDate(), 23, 59, 59, 999);
  const start = new Date(record.data.startAt);
  const end = new Date(record.data.endAt);
  if (Number.isNaN(start.getTime()) || Number.isNaN(end.getTime())) return false;
  return +end >= +dayStart && +start <= +dayEnd;
}

export async function GET(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const { userId } = gate.ctx;
  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);

  const { searchParams } = new URL(req.url);
  const calendarOnly = searchParams.get("calendar") === "true";
  const todayOnly = searchParams.get("today") === "true";

  if (calendarOnly && !canViewCalendar(roles)) {
    return NextResponse.json({ error: "Action non autorisée." }, { status: 403 });
  }

  try {
    let index = await purgeExpiredAbsences(await getAbsenceIndex());

    if (!calendarOnly && !todayOnly) {
      try {
        index = await consolidatePendingAbsencesInIndex(index);
      } catch (consolidateErr) {
        console.error("[api/absences] consolidatePendingAbsencesInIndex", consolidateErr);
      }
    }

    if (calendarOnly || todayOnly) {
      index = await mergeLegacyConvocationsForCalendar(index);
    }
    let establishments: Awaited<ReturnType<typeof loadAppConfig>>["establishments"] = [];
    let notifications: Awaited<ReturnType<typeof loadAppConfig>>["notifications"] | null = null;
    try {
      const bundle = await loadAppConfig();
      establishments = bundle.establishments;
      notifications = bundle.notifications;
    } catch (cfgErr) {
      console.error("[api/absences] loadAppConfig", cfgErr);
    }
    const viewerEmail = user?.primaryEmailAddress?.emailAddress || "";
    const ctx = { establishments, userId };
    const viewer = { email: viewerEmail, userId, roles };
    let visible = index.filter((a) => {
      try {
        return (
          canViewAbsence(a, userId, roles, ctx) ||
          processorMayAccessValidatedAbsence(a, viewer, notifications, establishments)
        );
      } catch (filterErr) {
        console.error(`[api/absences] filter ${a.id}`, filterErr);
        return false;
      }
    });
    if (calendarOnly) {
      visible = visible.filter((a) => isAbsenceVisibleOnCalendar(a, userId, roles));
    }
    if (todayOnly) {
      visible = visible.filter((a) => isTodayOverlap(a));
    }

    visible.sort((a, b) => +new Date(b.createdAt) - +new Date(a.createdAt));
    const payload = visible.map((abs) => {
      try {
        if (processorMayAccessValidatedAbsence(abs, viewer, notifications, establishments)) {
          return abs;
        }
        return filterAbsenceForViewer(abs, userId, roles, ctx);
      } catch (mapErr) {
        console.error(`[api/absences] map ${abs.id}`, mapErr);
        return abs;
      }
    });
    // Sérialisation explicite : évite une 500 HTML opaque si une valeur non-JSON reste.
    try {
      JSON.stringify(payload);
    } catch (serErr) {
      console.error("[api/absences] JSON.stringify payload", serErr);
      return NextResponse.json(
        { error: "Erreur sérialisation absences (donnée invalide)." },
        { status: 500 },
      );
    }
    return NextResponse.json(payload);
  } catch (error) {
    console.error("Absences list error:", error);
    const detail = error instanceof Error ? error.message : "";
    return NextResponse.json(
      {
        error:
          detail && /toISOString|date|iso|postgres|absences|stack|Maximum call/i.test(detail)
            ? `Erreur récupération absences (${detail})`
            : "Erreur récupération absences",
      },
      { status: 500 },
    );
  }
}

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const { userId } = gate.ctx;
  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);

  try {
    const body = await req.json();
    const payload = body?.data || {};

    const now = new Date().toISOString();
    const id = `${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
    const actorName = user?.fullName || user?.firstName || "Utilisateur";
    const actorEmail = user?.primaryEmailAddress?.emailAddress || "";

    const onBehalfRaw = body?.onBehalfOf && typeof body.onBehalfOf === "object" ? body.onBehalfOf : null;
    const onBehalfUserId = onBehalfRaw ? String(onBehalfRaw.userId || "").trim() : "";

    let subjectUserId = userId;
    let subjectName = actorName;
    let subjectEmail = actorEmail;
    let subjectRoles = roles;
    let submittedBy: AbsenceRecord["submittedBy"] = null;

    if (onBehalfUserId) {
      if (!canDeclareAbsenceOnBehalf(roles)) {
        return NextResponse.json(
          { error: "Seuls l’administratif, la comptabilité et la direction peuvent déposer une demande d'autorisation d'absence pour un collègue." },
          { status: 403 },
        );
      }
      if (onBehalfUserId === userId) {
        return NextResponse.json(
          { error: "Pour vous-même, décochez « Pour une autre personne »." },
          { status: 400 },
        );
      }
      const members = await listDirectoryMembers();
      const subject = members.find((m) => m.externalUserId === onBehalfUserId && !m.pending);
      if (!subject) {
        return NextResponse.json({ error: "Collègue introuvable dans l’annuaire." }, { status: 404 });
      }
      subjectUserId = subject.externalUserId;
      subjectName =
        subject.displayName?.trim() ||
        `${subject.firstName ?? ""} ${subject.lastName ?? ""}`.trim() ||
        subject.email;
      subjectEmail = subject.email || "";
      subjectRoles = Array.isArray(subject.roles) ? subject.roles : [];
      submittedBy = {
        userId,
        name: actorName,
        email: actorEmail,
        roles,
      };
    }

    const scope: AbsenceScope = onBehalfUserId
      ? resolveSelfDeclarationScope(subjectRoles, payload.scope)
      : resolveSelfDeclarationScope(roles, payload.scope);
    const etablissement: Etablissement | null =
      scope === "ogec" ? null : (payload.etablissement as Etablissement | null) || null;
    const periodResult = normalizeAbsencePeriodInput({
      periodType: payload.periodType,
      startDate: payload.startDate,
      endDate: payload.endDate,
      startTime: payload.startTime,
      endTime: payload.endTime,
    });
    if (periodResult.error || !periodResult.data) {
      return NextResponse.json({ error: periodResult.error || "Période invalide." }, { status: 400 });
    }
    const period = periodResult.data;
    const reason = String(payload.reason || "").trim();
    const details = String(payload.details || "").trim();
    const justificationPayload = payload?.justification || null;
    const staffPreferredTreatment = payload.staffPreferredTreatment
      ? String(payload.staffPreferredTreatment).trim() || null
      : null;
    const staffPreferredMakeupSlots = payload.staffPreferredMakeupSlots
      ? String(payload.staffPreferredMakeupSlots).trim() || null
      : null;

    if (!reason) {
      return NextResponse.json({ error: "Champs obligatoires manquants." }, { status: 400 });
    }
    if (scope === "professeur" && !etablissement) {
      return NextResponse.json({ error: "Établissement requis pour une absence professeur." }, { status: 400 });
    }

    const { startAt, endAt } = computeStartEndAt({
      periodType: period.periodType,
      startDate: period.startDate,
      endDate: period.endDate,
      startTime: period.startTime,
      endTime: period.endTime,
    });

    const record: AbsenceRecord = {
      id,
      createdAt: now,
      updatedAt: now,
      source: "self",
      displayName: subjectName,
      calendarVisible: false,
      createdBy: {
        userId: subjectUserId,
        name: subjectName,
        email: subjectEmail,
        roles: subjectRoles,
      },
      ...(submittedBy ? { submittedBy } : {}),
      data: {
        scope,
        etablissement: scope === "ogec" ? null : etablissement,
        periodType: period.periodType,
        startDate: period.startDate,
        endDate: period.endDate,
        startTime: period.startTime ?? null,
        endTime: period.endTime ?? null,
        startAt,
        endAt,
        reason,
        details,
      },
      staffPreferredTreatment,
      staffPreferredMakeupSlots,
      workflowStatus: justificationPayload?.fileName && justificationPayload?.fileUrl ? "JUSTIFICATIF_DEPOSE" : "OUVERTE",
      managerDecision: "EN_ATTENTE",
      closedAt: null,
      justificatifRelanceAt: null,
      justification:
        justificationPayload?.fileName && justificationPayload?.fileUrl
          ? {
              fileName: String(justificationPayload.fileName),
              fileUrl: String(justificationPayload.fileUrl),
              uploadedAt: now,
              uploadedBy: actorName,
            }
          : null,
      history: [
        {
          at: now,
          by: actorName,
          action: "CREATION",
          note: submittedBy
            ? `Demande créée par ${actorName} pour le compte de ${subjectName}`
            : "Demande d'autorisation d'absence créée",
        },
        ...(justificationPayload?.fileName && justificationPayload?.fileUrl
          ? [
              {
                at: now,
                by: actorName,
                action: "JUSTIFICATIF_DEPOSE",
                note: "Justificatif ajouté à la création",
              },
            ]
          : []),
      ],
    };

    const index = await purgeExpiredAbsences(await getAbsenceIndex());
    const { record: saved, merged } = await saveOrMergeAbsenceRecord(
      index,
      record,
      actorName,
    );

    if (merged) {
      return NextResponse.json({ success: true, id: saved.id, merged: true });
    }

    try {
      await notifyAbsenceCreated({
        record: saved,
        actorName: submittedBy?.name || actorName,
      });
    } catch (mailErr) {
      console.error("Absences creation mail error:", mailErr);
    }

    return NextResponse.json({ success: true, id: saved.id });
  } catch (error) {
    console.error("Absences create error:", error);
    const detail = error instanceof Error ? error.message : "";
    return NextResponse.json(
      {
        error:
          detail && /postgres|absences|établi|etablissement|date|iso|requis/i.test(detail)
            ? detail
            : "Erreur création absence",
      },
      { status: 500 },
    );
  }
}

export async function PATCH(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const { userId } = gate.ctx;
  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);

  try {
    const body = await req.json();
    const id = String(body?.id || "");
    const action = String(body?.action || "");
    const managerNote = String(body?.managerNote || "").trim();
    const justification = body?.justification || null;
    if (
      !id ||
      ![
        "VALIDER",
        "REFUSER",
        "RELANCER_JUSTIFICATIF",
        "DEPOSER_JUSTIFICATIF",
        "CLOTURER",
        "REOUVRIR",
        "CORRIGER_SCOPE",
        "MODIFIER_CALENDRIER",
        "TRAITER_ADMIN",
      ].includes(action)
    ) {
      return NextResponse.json({ error: "Paramètres invalides." }, { status: 400 });
    }

    const index = await purgeExpiredAbsences(await getAbsenceIndex());
    const current = await getAbsenceRecord(id);
    if (!current) return NextResponse.json({ error: "Absence introuvable" }, { status: 404 });

    const actor = user?.fullName || user?.firstName || "Direction";
    const isOwner = current.createdBy.userId === userId;
    const isSubmitter = current.submittedBy?.userId === userId;
    const bundle = await loadAppConfig();
    const canManage = canManageAbsence(current, roles, {
      establishments: bundle.establishments,
      userId,
    });
    const viewerEmail = user?.primaryEmailAddress?.emailAddress || "";
    const canProcess = viewerIsAbsenceProcessor(
      current,
      { email: viewerEmail, userId, roles },
      bundle.notifications,
      bundle.establishments,
    );

    if (
      action === "DEPOSER_JUSTIFICATIF" &&
      !isOwner &&
      !isSubmitter &&
      !canManage &&
      !canProcess
    ) {
      return NextResponse.json({ error: "Action non autorisée." }, { status: 403 });
    }
    if (action === "TRAITER_ADMIN" && !canProcess) {
      return NextResponse.json({ error: "Action non autorisée." }, { status: 403 });
    }
    if (action === "RELANCER_JUSTIFICATIF" && !canManage && !canProcess) {
      return NextResponse.json({ error: "Action non autorisée." }, { status: 403 });
    }
    if (
      (action === "CLOTURER" ||
        action === "REOUVRIR" ||
        action === "VALIDER" ||
        action === "REFUSER" ||
        action === "CORRIGER_SCOPE" ||
        action === "MODIFIER_CALENDRIER") &&
      !canManage
    ) {
      return NextResponse.json({ error: "Action non autorisée." }, { status: 403 });
    }

    let updated: AbsenceRecord = {
      ...current,
      managerNote,
      updatedAt: new Date().toISOString(),
    };
    let validationRecipients: string[] | undefined;

    if (action === "DEPOSER_JUSTIFICATIF") {
      if (!justification?.fileName || !justification?.fileUrl) {
        return NextResponse.json({ error: "Justificatif invalide." }, { status: 400 });
      }
      updated = {
        ...updated,
        justification: {
          fileName: String(justification.fileName),
          fileUrl: String(justification.fileUrl),
          uploadedAt: new Date().toISOString(),
          uploadedBy: actor,
        },
        justificatifRelanceAt: null,
        workflowStatus: current.workflowStatus === "CLOTUREE" ? "CLOTUREE" : "JUSTIFICATIF_DEPOSE",
        history: [
          ...(current.history || []),
          {
            at: new Date().toISOString(),
            by: actor,
            action: "JUSTIFICATIF_DEPOSE",
          },
        ],
      };
      if (current.managerDecision === "VALIDEE" && updated.workflowStatus !== "CLOTUREE") {
        try {
          await notifyAbsenceJustificatifDeposited(updated);
        } catch (err) {
          console.error("Absences justificatif deposited mail:", err);
        }
      }
    } else if (action === "VALIDER") {
      const treatmentResult = validateHoursTreatmentForAbsence(
        current.data.scope,
        current.data.etablissement,
        body?.hoursTreatment,
      );
      if (!treatmentResult.ok) {
        return NextResponse.json({ error: treatmentResult.error }, { status: 400 });
      }
      const hoursTreatment = treatmentResult.treatment;
      const directionConfirmedMakeupSlots =
        body?.directionConfirmedMakeupSlots
          ? String(body.directionConfirmedMakeupSlots).trim() || null
          : null;
      const decidedAt = new Date().toISOString();
      updated = {
        ...updated,
        managerDecision: "VALIDEE",
        workflowStatus: current.justification?.fileUrl ? "JUSTIFICATIF_DEPOSE" : "A_TRAITER",
        calendarVisible: true,
        closedAt: null,
        hoursTreatment,
        directionConfirmedMakeupSlots,
        history: [
          ...(current.history || []),
          {
            at: decidedAt,
            by: actor,
            action: "DECISION_VALIDEE",
            note: managerNote || undefined,
          },
        ],
      };
      // Mails après persistance — un échec SMTP / config ne doit pas bloquer la validation.
    } else if (action === "REFUSER") {
      const closedAt = new Date().toISOString();
      updated = {
        ...updated,
        managerDecision: "REFUSEE",
        workflowStatus: "CLOTUREE",
        calendarVisible: false,
        closedAt,
        justificatifRelanceAt: null,
        history: [
          ...(current.history || []),
          {
            at: closedAt,
            by: actor,
            action: "DECISION_REFUSEE",
            note: managerNote || undefined,
          },
        ],
      };
    } else if (action === "RELANCER_JUSTIFICATIF") {
      const relanceAt = new Date().toISOString();
      updated = {
        ...updated,
        justificatifRelanceAt: relanceAt,
        history: [
          ...(current.history || []),
          {
            at: relanceAt,
            by: actor,
            action: "RELANCE_JUSTIFICATIF",
            note: managerNote || undefined,
          },
        ],
      };
      try {
        await notifyAbsenceJustificatifRequested({
          record: updated,
          fromProcessor: current.managerDecision === "VALIDEE",
          note: managerNote,
        });
      } catch (mailErr) {
        console.error("Absences relance mail error:", mailErr);
      }
    } else if (action === "TRAITER_ADMIN") {
      if (current.managerDecision !== "VALIDEE") {
        return NextResponse.json(
          { error: "La direction doit d’abord valider l’absence." },
          { status: 400 },
        );
      }
      const treatedAt = new Date().toISOString();
      updated = {
        ...updated,
        workflowStatus: "CLOTUREE",
        closedAt: treatedAt,
        adminTreatedAt: treatedAt,
        adminTreatedBy: actor,
        adminNote: managerNote || null,
        justificatifRelanceAt: null,
        history: [
          ...(current.history || []),
          {
            at: treatedAt,
            by: actor,
            action: "TRAITEMENT_ADMIN",
            note: managerNote || "Dossier traité (rectorat / RH).",
          },
        ],
      };
      try {
        await notifyAbsenceAdminTreated(updated);
      } catch (mailErr) {
        console.error("Absences admin treated mail error:", mailErr);
      }
      updated = await applyPostValidationPrivacy(updated, index);
    } else if (action === "CLOTURER") {
      if (current.managerDecision === "VALIDEE" && !current.adminTreatedAt) {
        return NextResponse.json(
          {
            error:
              "Après validation direction, la clôture administrative se fait dans Traitement RH / rectorat.",
          },
          { status: 400 },
        );
      }
      updated = {
        ...updated,
        workflowStatus: "CLOTUREE",
        closedAt: new Date().toISOString(),
        history: [
          ...(current.history || []),
          {
            at: new Date().toISOString(),
            by: actor,
            action: "CLOTUREE",
            note: managerNote || undefined,
          },
        ],
      };
    } else if (action === "CORRIGER_SCOPE") {
      const newScope: AbsenceScope = body?.scope === "ogec" ? "ogec" : "professeur";
      const etablissement: Etablissement | null =
        newScope === "ogec"
          ? null
          : ((body?.etablissement as Etablissement | null) || current.data.etablissement);
      if (newScope === "professeur" && !etablissement) {
        return NextResponse.json({ error: "Établissement requis pour une absence professeur." }, { status: 400 });
      }
      updated = {
        ...updated,
        data: {
          ...current.data,
          scope: newScope,
          etablissement: newScope === "ogec" ? null : etablissement,
        },
        history: [
          ...(current.history || []),
          {
            at: new Date().toISOString(),
            by: actor,
            action: "CORRECTION_SCOPE",
            note: newScope === "ogec" ? "Personnel OGEC" : `Professeur (${etablissement})`,
          },
        ],
      };
    } else if (action === "MODIFIER_CALENDRIER") {
      const displayName = String(body?.displayName ?? current.displayName).trim();
      const reason = String(body?.reason ?? current.data.reason).trim();
      if (!displayName || !reason) {
        return NextResponse.json({ error: "Nom et motif requis." }, { status: 400 });
      }
      const periodResult = normalizeAbsencePeriodInput({
        periodType: body?.periodType,
        startDate: body?.startDate ?? current.data.startDate,
        endDate: body?.endDate ?? current.data.endDate,
        startTime: body?.startTime ?? current.data.startTime,
        endTime: body?.endTime ?? current.data.endTime,
      });
      if (periodResult.error || !periodResult.data) {
        return NextResponse.json({ error: periodResult.error || "Période invalide." }, { status: 400 });
      }
      const period = periodResult.data;
      const { startAt, endAt } = computeStartEndAt({
        periodType: period.periodType,
        startDate: period.startDate,
        endDate: period.endDate,
        startTime: period.startTime,
        endTime: period.endTime,
      });
      const newScope: AbsenceScope =
        body?.scope === "ogec" || body?.scope === "professeur"
          ? body.scope
          : resolveAbsenceScope(current);
      const etablissement: Etablissement | null =
        newScope === "ogec"
          ? null
          : ((body?.etablissement as Etablissement | null) || current.data.etablissement);
      if (newScope === "professeur" && !etablissement) {
        return NextResponse.json({ error: "Établissement requis pour une absence professeur." }, { status: 400 });
      }
      const scopeNote =
        newScope === "ogec" ? "Personnel OGEC" : `Professeur (${etablissement})`;
      updated = {
        ...updated,
        displayName,
        data: {
          ...current.data,
          scope: newScope,
          etablissement: newScope === "ogec" ? null : etablissement,
          reason,
          periodType: period.periodType,
          startDate: period.startDate,
          endDate: period.endDate,
          startTime: period.startTime ?? null,
          endTime: period.endTime ?? null,
          startAt,
          endAt,
        },
        history: [
          ...(current.history || []),
          {
            at: new Date().toISOString(),
            by: actor,
            action: "MODIFICATION_CALENDRIER",
            note: managerNote || scopeNote,
          },
        ],
      };
    } else if (action === "REOUVRIR") {
      updated = {
        ...updated,
        workflowStatus: "OUVERTE",
        managerDecision: "EN_ATTENTE",
        calendarVisible: false,
        closedAt: null,
        history: [
          ...(current.history || []),
          {
            at: new Date().toISOString(),
            by: actor,
            action: "REOUVERTE",
            note: managerNote || undefined,
          },
        ],
      };
    } else {
      return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
    }

    await saveAbsenceRecord(updated);

    if (action === "VALIDER") {
      try {
        const { recipients } = await notifyAbsenceValidated(updated);
        validationRecipients = recipients;
      } catch (mailErr) {
        console.error("Absences validation mail error:", mailErr);
        validationRecipients = [];
      }
      try {
        await notifyAbsenceCreatorValidated(updated);
      } catch (mailErr) {
        console.error("Absences creator validation mail error:", mailErr);
      }
    }

    return NextResponse.json({
      success: true,
      ...(action === "VALIDER"
        ? {
            calendarVisible: updated.calendarVisible === true,
            validationRecipients: validationRecipients ?? [],
          }
        : {}),
    });
  } catch (error) {
    console.error("Absences patch error:", error);
    const detail = error instanceof Error ? error.message : "";
    return NextResponse.json(
      {
        error:
          detail && /postgres|absences|établi|etablissement|date|iso/i.test(detail)
            ? detail
            : "Erreur mise à jour absence",
      },
      { status: 500 },
    );
  }
}

export async function DELETE(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  if (!canViewCalendar(roles)) return NextResponse.json({ error: "Action non autorisée." }, { status: 403 });

  const { searchParams } = new URL(req.url);
  const id = String(searchParams.get("id") || "").trim();
  if (!id) return NextResponse.json({ error: "Paramètre 'id' manquant." }, { status: 400 });

  try {
    const record = await getAbsenceOrLegacyRecord(id);
    if (!record) return NextResponse.json({ error: "Absence introuvable" }, { status: 404 });
    const bundle = await loadAppConfig();
    if (!canManageAbsence(record, roles, { establishments: bundle.establishments, userId: user?.id })) {
      return NextResponse.json({ error: "Suppression non autorisée." }, { status: 403 });
    }

    const docKeys = getAbsenceDocumentKeys(record);
    const bucket = await getBucketName();
    const s3Client = await getTenantDataS3Client();

    const etabId = await absencesDbReady();
    if (!etabId) {
      return NextResponse.json({ error: "[absences] Postgres requis" }, { status: 500 });
    }
    await deleteAbsenceFromDb(etabId, id);
    const updated = (await getAbsenceIndex()).filter((r) => r.id !== id);

    for (const docKey of docKeys) {
      const stillUsed =
        isDocumentKeyReferenced(updated, docKey) || (await isDocumentKeyReferencedInLegacy(docKey));
      if (!stillUsed) {
        await s3Client.send(new DeleteObjectCommand({ Bucket: bucket, Key: s3Key(docKey) }));
      }
    }

    return NextResponse.json({ success: true });
  } catch (error) {
    console.error("Absences delete error:", error);
    return NextResponse.json({ error: "Erreur suppression absence" }, { status: 500 });
  }
}
