import { safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { NextResponse } from "next/server";

import { DeleteObjectCommand } from "@aws-sdk/client-s3";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
} from "@/app/lib/tenant-mail";
import { loadAppConfig, getEstablishmentByLabel } from "@/app/lib/app-config";
import { requireAuth } from "@/app/lib/intranet-auth";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { s3Key } from "@/app/lib/s3-path";
import { formatAbsencePeriod, normalizeAbsencePeriodInput } from "@/app/lib/absence-period";
import {
  formatHoursTreatmentCreatorMailLine,
  formatHoursTreatmentMailLine,
  validateHoursTreatmentForAbsence,
} from "@/app/lib/absence-hours-treatment";
import {
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
import { tenantAbsolutePath } from "@/app/lib/tenant-context";
import { getAbsenceDocumentKeys, isDocumentKeyReferenced } from "@/app/lib/absences-documents";
import {
  formatJustificatifMailLine,
  loadAbsenceValidationAttachments,
} from "@/app/lib/absences-notify";
import {
  consolidatePendingAbsencesInIndex,
  getAbsenceIndex,
  getAbsenceRecord,
  applyPostValidationPrivacy,
  purgeExpiredAbsences,
  saveAbsenceIndex,
  saveAbsenceRecord,
  saveOrMergeAbsenceRecord,
} from "@/app/lib/absences-storage";
import {
  deleteLegacyConvocation,
  getAbsenceOrLegacyRecord,
  isDocumentKeyReferencedInLegacy,
  mergeLegacyConvocationsForCalendar,
} from "@/app/lib/absences-legacy-convocations";

function recordKey(id: string) {
  return `absences/${id}.json`;
}

async function resolveDecisionTarget(scope: AbsenceScope, etablissement: Etablissement | null) {
  const bundle = await loadAppConfig();
  if (scope === "ogec") {
    const lycee =
      bundle.establishments.find((e) => e.id === "lycee") || bundle.establishments[bundle.establishments.length - 1];
    return {
      roleLabel: lycee ? `Direction ${lycee.label}` : "Direction",
      name: lycee?.directorName || bundle.identity.name,
      email: lycee?.directorEmail || "",
    };
  }
  const est = etablissement ? getEstablishmentByLabel(bundle, etablissement) : null;
  if (est) {
    return {
      roleLabel: `Direction ${est.label}`,
      name: est.directorName || est.label,
      email: est.directorEmail || "",
    };
  }
  return { roleLabel: "Direction", name: bundle.identity.name, email: "" };
}

async function getMailer() {
  const smtp = await getTenantSmtpConfig();
  if (!smtp) return null;
  const transporter = await createTenantTransporter();
  if (!transporter) return null;
  return { smtp, transporter };
}

async function resolveValidationRecipients(record: AbsenceRecord) {
  const n = (await loadAppConfig()).notifications;
  if (record.data.scope === "ogec") {
    return [...n.absencesNotifyOgecCompta].filter(Boolean);
  }
  if (record.data.etablissement === "École") {
    return n.absencesNotifyProfEcole?.email ? [n.absencesNotifyProfEcole.email] : [];
  }
  if (record.data.etablissement === "Collège") {
    const email = n.absencesNotifyProfCollege?.email || n.absencesNotifyProfCollegeLycee?.email;
    return email ? [email] : [];
  }
  if (record.data.etablissement === "Lycée") {
    const email = n.absencesNotifyProfLycee?.email || n.absencesNotifyProfCollegeLycee?.email;
    return email ? [email] : [];
  }
  return n.absencesNotifyProfCollegeLycee?.email ? [n.absencesNotifyProfCollegeLycee.email] : [];
}

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
      index = await consolidatePendingAbsencesInIndex(index);
    }

    if (calendarOnly || todayOnly) {
      index = await mergeLegacyConvocationsForCalendar(index);
    }
    let visible = index.filter((a) => canViewAbsence(a, userId, roles));
    if (calendarOnly) {
      visible = visible.filter((a) => isAbsenceVisibleOnCalendar(a, userId, roles));
    }
    if (todayOnly) {
      visible = visible.filter((a) => isTodayOverlap(a));
    }

    visible.sort((a, b) => +new Date(b.createdAt) - +new Date(a.createdAt));
    const payload = visible.map((abs) => filterAbsenceForViewer(abs, userId, roles));
    return NextResponse.json(payload);
  } catch (error) {
    console.error("Absences list error:", error);
    return NextResponse.json({ error: "Erreur récupération absences" }, { status: 500 });
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

    const scope: AbsenceScope = resolveSelfDeclarationScope(roles, payload.scope);
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

    if (!reason) {
      return NextResponse.json({ error: "Champs obligatoires manquants." }, { status: 400 });
    }
    if (scope === "professeur" && !etablissement) {
      return NextResponse.json({ error: "Établissement requis pour une absence professeur." }, { status: 400 });
    }

    const now = new Date().toISOString();
    const id = `${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
    const creatorName = user?.fullName || user?.firstName || "Utilisateur";
    const creatorEmail = user?.primaryEmailAddress?.emailAddress || "";
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
      displayName: creatorName,
      calendarVisible: false,
      createdBy: {
        userId,
        name: creatorName,
        email: creatorEmail,
        roles,
      },
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
              uploadedBy: creatorName,
            }
          : null,
      history: [
        {
          at: now,
          by: creatorName,
          action: "CREATION",
          note: "Déclaration d'absence créée",
        },
        ...(justificationPayload?.fileName && justificationPayload?.fileUrl
          ? [
              {
                at: now,
                by: creatorName,
                action: "JUSTIFICATIF_DEPOSE",
                note: "Justificatif ajouté à la création",
              },
            ]
          : []),
      ],
    };

    const index = await purgeExpiredAbsences(await getAbsenceIndex());
    const { index: nextIndex, record: saved, merged } = await saveOrMergeAbsenceRecord(
      index,
      record,
      creatorName,
    );
    await saveAbsenceIndex(nextIndex);

    if (merged) {
      return NextResponse.json({ success: true, id: saved.id, merged: true });
    }

    try {
      const target = await resolveDecisionTarget(scope, scope === "ogec" ? null : etablissement);
      const mail = await getMailer();
      const absencesLink = await tenantAbsolutePath("/rh?tab=absences");
      if (mail) {
      const { smtp, transporter } = mail;
      await transporter.sendMail({
        from: `"Absences" <${smtp.user}>`,
        to: target.email,
        subject: `Nouvelle absence à traiter — ${scope === "ogec" ? "Personnel OGEC" : `Professeur ${etablissement || ""}`}`.trim(),
        text: [
          `Bonjour ${target.name},`,
          ``,
          `Une nouvelle absence a été déclarée et nécessite votre décision.`,
          ``,
          `Type : ${scope === "ogec" ? "Personnel OGEC" : "Professeur"}`,
          `Établissement : ${scope === "ogec" ? "OGEC" : etablissement || "—"}`,
          `Créateur : ${creatorName} (${creatorEmail || "email non renseigné"})`,
          `Période : ${formatAbsencePeriod(saved.data)}`,
          `Motif : ${reason}`,
          details ? `Détails : ${details}` : "",
          justificationPayload?.fileName ? `Justificatif fourni : ${justificationPayload.fileName}` : `Justificatif : en attente`,
          ``,
          `Action attendue : Valider / Refuser / Relancer justificatif`,
          ``,
          `Espace Absences: ${absencesLink}`,
        ]
          .filter(Boolean)
          .join("\n"),
      });
      }
    } catch (mailErr) {
      console.error("Absences creation mail error:", mailErr);
    }

    return NextResponse.json({ success: true, id: saved.id });
  } catch (error) {
    console.error("Absences create error:", error);
    return NextResponse.json({ error: "Erreur création absence" }, { status: 500 });
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
      ].includes(action)
    ) {
      return NextResponse.json({ error: "Paramètres invalides." }, { status: 400 });
    }

    const index = await purgeExpiredAbsences(await getAbsenceIndex());
    const current = await getAbsenceRecord(id);
    if (!current) return NextResponse.json({ error: "Absence introuvable" }, { status: 404 });

    const actor = user?.fullName || user?.firstName || "Direction";
    const isOwner = current.createdBy.userId === userId;
    const canManage = canManageAbsence(current, roles);

    if (action === "DEPOSER_JUSTIFICATIF" && !isOwner && !canManage) {
      return NextResponse.json({ error: "Action non autorisée." }, { status: 403 });
    }
    if (
      (action === "CLOTURER" ||
        action === "REOUVRIR" ||
        action === "VALIDER" ||
        action === "REFUSER" ||
        action === "RELANCER_JUSTIFICATIF" ||
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
      const closedAt = new Date().toISOString();
      updated = {
        ...updated,
        managerDecision: "VALIDEE",
        workflowStatus: "CLOTUREE",
        calendarVisible: true,
        closedAt,
        justificatifRelanceAt: null,
        hoursTreatment,
        history: [
          ...(current.history || []),
          {
            at: closedAt,
            by: actor,
            action: "DECISION_VALIDEE",
            note: managerNote || undefined,
          },
        ],
      };

      const recipients = await resolveValidationRecipients(updated);
      let validationMailSent = false;
      if (recipients.length > 0) {
        try {
          const mail = await getMailer();
          if (mail) {
          const { smtp, transporter } = mail;
          const mailAttachments = await loadAbsenceValidationAttachments(updated);
          const justificatifLine = formatJustificatifMailLine(updated, mailAttachments);
          const scope = resolveAbsenceScope(updated);
          const treatmentLine = formatHoursTreatmentMailLine(updated.hoursTreatment!, scope);
          await transporter.sendMail({
            from: `"Absences" <${smtp.user}>`,
            to: recipients.join(","),
            subject: `Absence validée — ${updated.createdBy.name}`,
            text: [
              `Bonjour,`,
              ``,
              `Une absence a été validée.`,
              `Personne : ${updated.createdBy.name}`,
              `Type : ${scope === "ogec" ? "Personnel OGEC" : "Professeur"}`,
              `Établissement : ${scope === "ogec" ? "OGEC" : updated.data.etablissement || "—"}`,
              `Période : ${formatAbsencePeriod(updated.data)}`,
              `Motif : ${updated.data.reason}`,
              updated.data.details ? `Détails : ${updated.data.details}` : "",
              justificatifLine,
              treatmentLine,
              mailAttachments.length > 0 ? `` : undefined,
              mailAttachments.length > 0
                ? `Les justificatifs et documents sont en pièce(s) jointe(s) à ce message.`
                : undefined,
            ]
              .filter(Boolean)
              .join("\n"),
            attachments: mailAttachments.map((a) => ({
              filename: a.filename,
              content: a.content,
              contentType: a.contentType,
            })),
          });
          validationMailSent = true;
          }
        } catch (mailErr) {
          console.error("Absences validation mail error:", mailErr);
        }
      }

      if (validationMailSent || recipients.length === 0) {
        updated = await applyPostValidationPrivacy(updated, index);
      }

      try {
        if (updated.createdBy.email) {
          const mail = await getMailer();
          if (mail) {
          const { smtp, transporter } = mail;
          await transporter.sendMail({
            from: `"Absences" <${smtp.user}>`,
            to: updated.createdBy.email,
            subject: "Votre absence a été validée",
            text: [
              `Bonjour ${updated.createdBy.name},`,
              ``,
              `Votre absence a bien été validée.`,
              ``,
              `Période : ${formatAbsencePeriod(updated.data)}`,
              `Motif : ${updated.data.reason}`,
              updated.data.details ? `Détails : ${updated.data.details}` : "",
              formatHoursTreatmentCreatorMailLine(updated.hoursTreatment!, updated.data.scope),
              ``,
              `Cordialement,`,
              `L'établissement`,
            ]
              .filter(Boolean)
              .join("\n"),
          });
          }
        }
      } catch (mailErr) {
        console.error("Absences creator validation mail error:", mailErr);
      }
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
        if (current.createdBy.email) {
          const mail = await getMailer();
          if (mail) {
          const { smtp, transporter } = mail;
          const absencesLink = await tenantAbsolutePath("/rh?tab=absences");
          await transporter.sendMail({
            from: `"Absences" <${smtp.user}>`,
            to: current.createdBy.email,
            subject: "Complément souhaité — justificatif d'absence",
            text: [
              `Bonjour ${current.createdBy.name},`,
              ``,
              current.justification?.fileName
                ? `La direction a examiné le justificatif déjà transmis (${current.justification.fileName}) et souhaiterait un complément ou un autre document pour finaliser votre dossier.`
                : `Votre déclaration d'absence a bien été reçue.`,
              ``,
              current.justification?.fileName
                ? `Merci de déposer le document complémentaire sur l'espace Absences. Il remplacera le fichier actuel.`
                : `La direction vous invite à déposer, si vous le souhaitez et si votre situation le nécessite, une pièce justificative pour compléter votre dossier.`,
              current.justification?.fileName
                ? ``
                : `Ce document n'est pas toujours obligatoire : cela dépend du type d'absence. En revanche, si vous en disposez, merci de le transmettre.`,
              ``,
              `Période : ${formatAbsencePeriod(current.data)}`,
              `Motif : ${current.data.reason}`,
              managerNote ? `Message de la direction : ${managerNote}` : "",
              ``,
              `Pour déposer votre justificatif :`,
              absencesLink || "Espace Absences de l'intranet",
              ``,
              `Cordialement,`,
              `La direction`,
            ]
              .filter(Boolean)
              .join("\n"),
          });
          }
        }
      } catch (mailErr) {
        console.error("Absences relance mail error:", mailErr);
      }
    } else if (action === "CLOTURER") {
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

    const pos = index.findIndex((a) => a.id === id);
    if (pos >= 0) index[pos] = updated;
    await saveAbsenceIndex(index);

    return NextResponse.json({ success: true });
  } catch (error) {
    console.error("Absences patch error:", error);
    return NextResponse.json({ error: "Erreur mise à jour absence" }, { status: 500 });
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
    if (!canManageAbsence(record, roles)) {
      return NextResponse.json({ error: "Suppression non autorisée." }, { status: 403 });
    }

    const docKeys = getAbsenceDocumentKeys(record);
    const bucket = await getBucketName();
    const s3Client = await getTenantDataS3Client();

    const absenceFile = await getAbsenceRecord(id);
    let updated = await getAbsenceIndex();

    if (absenceFile) {
      await s3Client.send(
        new DeleteObjectCommand({
          Bucket: bucket,
          Key: s3Key(recordKey(id)),
        }),
      );
      updated = updated.filter((r) => r.id !== id);
      await saveAbsenceIndex(updated);
    } else {
      await deleteLegacyConvocation(id);
    }

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
