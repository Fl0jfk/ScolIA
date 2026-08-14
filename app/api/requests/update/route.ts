import { safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";

import { requireAuth } from "@/app/lib/intranet-auth";
import { REQUEST_STATUSES, RequestAttachment, RequestComment, RequestRecord, RequestStatus, assertEligibleRequestAttachment, finalizeRequestPurgeMetadata, getDefaultRequestBranchForStaffEmail, getRequestPoolEmails, getRequestsIndex, isLeaderForRequestBranch, isUserInRequestPool, notifyRequesterOnly, notifyRequestStatusMilestone, resolveRequestRouteById, saveRequestFile, saveRequestsIndex, uploadBuffersAsRequestAttachments, MAX_REQUEST_ATTACHMENTS_PER_UPLOAD, MANUAL_ONLY_DIRECTION_IDS} from "@/app/lib/requests";
import { isCorbeilleBranchId, normalizeRequestBranchId, normalizeRequestEmail} from "@/app/lib/requests-board";
import { canAccessRequestsStaffBoard } from "@/app/lib/requests-staff-access";
import { isStaffInBranchPool } from "@/app/lib/staff-directory";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";

const LEGACY_ASSIGN_UNIT_TO_ROUTE: Record<string, string> = { comptabilite: "comptabilite", maintenance: "maintenance", "direction-college": "admin_college", "direction-lycee": "admin_lycee", "vie-scolaire": "vie_scolaire_infirmerie", informatique: "maintenance"};

export async function PATCH(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const { userId } = gate.ctx;
    const user = await safeCurrentUser();
    const actorName = user?.fullName || user?.firstName || "Équipe";
    const actorEmail = user?.primaryEmailAddress?.emailAddress || "";
    const actorRoles = rolesFromUserLike(user);
  try {
    const contentType = req.headers.get("content-type") || "";
    let body: Record<string, unknown>;
    let multipartFiles: File[] = [];
    if (contentType.includes("multipart/form-data")) {
      const form = await req.formData();
      const get = (k: string) => {
        const v = form.get(k);
        return typeof v === "string" ? v : "";
      };
      body = {
        action: get("action"),
        id: get("id"),
        status: get("status"),
        comment: get("comment"),
        toRequester: get("toRequester"),
        assignRouteId: get("assignRouteId"),
        assignUnit: get("assignUnit"),
        toCorbeille: get("toCorbeille"),
        targetEmail: get("targetEmail"),
      };
      multipartFiles = form.getAll("files").filter((x): x is File => x instanceof File && x.size > 0).slice(0, MAX_REQUEST_ATTACHMENTS_PER_UPLOAD);
    } else { body = await req.json()}
    const action = String(body?.action ?? "").trim();
    const id = String(body?.id || "");
    const status = String(body?.status || "") as RequestStatus;
    const comment = String(body?.comment || "").trim();
    const toRequester = contentType.includes("multipart/form-data") ? String(body?.toRequester || "") === "true" || String(body?.toRequester || "") === "on" : Boolean(body?.toRequester);
    const assignRouteIdRaw = String(body?.assignRouteId || "").trim();
    const assignUnit = String(body?.assignUnit || "").trim();
    const toCorbeille = contentType.includes("multipart/form-data") ? String(body?.toCorbeille || "") === "true" || String(body?.toCorbeille || "") === "on" : Boolean(body?.toCorbeille);
    if (multipartFiles.length > 0 && ["claim", "release_claim", "claim_self", "delegate_claim"].includes(action)) { return NextResponse.json({ error: "Retirez les pièces jointes pour cette action." }, { status: 400 });}
    if (!id) return NextResponse.json({ error: "id requis" }, { status: 400 });
    const index = await getRequestsIndex();
    const pos = index.findIndex((r) => r.id === id);
    if (pos < 0) return NextResponse.json({ error: "Demande introuvable" }, { status: 404 });
    const current = index[pos] as RequestRecord;
    const now = new Date().toISOString();
    if (action === "delegate_claim") {
      const targetEmail = normalizeRequestEmail(String(body?.targetEmail ?? ""));
      if (!actorEmail) return NextResponse.json({ error: "Email requis" }, { status: 400 });
      if (!targetEmail) return NextResponse.json({ error: "Collaborateur cible requis (targetEmail)" }, { status: 400 });
      if (!(await isLeaderForRequestBranch(current.assignedTo.routeId, current.assignedTo.unit, actorEmail))) { return NextResponse.json({ error: "Seul le responsable du service peut déléguer." }, { status: 403 })}
      const branch = normalizeRequestBranchId(current.assignedTo.routeId, current.assignedTo.unit);
      if (isCorbeilleBranchId(branch)) { return NextResponse.json({ error: "Délégation impossible pour une fiche en corbeille." }, { status: 400 })}
      if (!(await isStaffInBranchPool(branch, targetEmail))) { return NextResponse.json({ error: "Ce collaborateur n’est pas dans le service (table équipe)." }, { status: 403 })}
      const pool = (await getRequestPoolEmails(current)).map(normalizeRequestEmail);
      if (!pool.includes(targetEmail)) { return NextResponse.json({ error: "Ce collaborateur ne figure pas sur la file de cette fiche." }, { status: 403 })}
      if (normalizeRequestEmail(actorEmail) === targetEmail) { return NextResponse.json({ error: "Choisissez un autre collaborateur que vous-même." }, { status: 400 })}
      const delegated: RequestRecord = {
        ...current,
        updatedAt: now,
        status: current.status === "NOUVELLE" ? "EN_COURS" : current.status,
        assignedTo: {
          ...current.assignedTo,
          claimedBy: {
            email: targetEmail,
            name: targetEmail,
            userId: null,
            at: now,
          },
        },
        history: [
          ...current.history,
          {
            at: now,
            by: actorName,
            action: "DELEGATE",
            note: `Délégation à ${targetEmail} par le responsable.`,
          },
        ],
      };
      const finalized = finalizeRequestPurgeMetadata(current, delegated, now);
      index[pos] = finalized;
      await saveRequestFile(finalized);
      await saveRequestsIndex( index);
      return NextResponse.json({ success: true, request: finalized });
    }
    if (action === "claim" || action === "release_claim") {
      if (action === "claim") {
        if (!actorEmail) return NextResponse.json({ error: "Email requis pour prendre en charge" }, { status: 400 });
        if (!(await isUserInRequestPool(current, actorEmail))) {
          return NextResponse.json({ error: "Vous n'êtes pas dans la file de cette demande" }, { status: 403 });
        }
        const existing = current.assignedTo.claimedBy;
        if (existing?.email && normalizeRequestEmail(existing.email) !== normalizeRequestEmail(actorEmail)) {
          return NextResponse.json(
            { error: `Déjà prise en charge par ${existing.name || existing.email}` },
            { status: 409 },
          );
        }
        const fromBrClaim = normalizeRequestBranchId(current.assignedTo.routeId, current.assignedTo.unit);
        let nextCatClaim = current.category;
        let nextAssignedClaim: RequestRecord["assignedTo"] = {
          ...current.assignedTo,
          claimedBy: { email: actorEmail, name: actorName, userId: userId ?? null, at: now },
        };
        if (isCorbeilleBranchId(fromBrClaim)) {
          const dest = await getDefaultRequestBranchForStaffEmail(actorEmail);
          if (dest) {
            const resolved = await resolveRequestRouteById(dest);
            if (resolved) {
              nextCatClaim = resolved.category;
              nextAssignedClaim = {
                ...resolved.assignedTo,
                claimedBy: { email: actorEmail, name: actorName, userId: userId ?? null, at: now },
              };
            }
          }
        }
        const updatedClaim: RequestRecord = {
          ...current,
          updatedAt: now,
          status: current.status === "NOUVELLE" ? "EN_COURS" : current.status,
          category: nextCatClaim,
          assignedTo: nextAssignedClaim,
          history: [
            ...current.history,
            {
              at: now,
              by: actorName,
              action: "CLAIM",
              note: isCorbeilleBranchId(fromBrClaim)
                ? "Prise en charge depuis la corbeille — affectation vers la branche du collaborateur."
                : "Prise en charge (file partagée)",
            },
          ],
        };
        const finalizedClaim = finalizeRequestPurgeMetadata(current, updatedClaim, now);
        index[pos] = finalizedClaim;
        await saveRequestFile(finalizedClaim);
        await saveRequestsIndex( index);
        return NextResponse.json({ success: true, request: finalizedClaim });
      }
      if (toCorbeille) {
        if (!(await isLeaderForRequestBranch(current.assignedTo.routeId, current.assignedTo.unit, actorEmail))) {
          return NextResponse.json({ error: "Seul le responsable du service peut renvoyer la demande à la corbeille." },{ status: 403 });
        }
        const corb = await resolveRequestRouteById("corbeille");
        if (!corb) return NextResponse.json({ error: "Configuration corbeille manquante" }, { status: 500 });
        const updatedBasket: RequestRecord = {
          ...current,
          updatedAt: now,
          status: "NOUVELLE",
          category: corb.category,
          assignedTo: { ...corb.assignedTo, claimedBy: null },
          routing: {
            ...current.routing,
            source: "fallback",
            confidence: 1,
            reason: "Renvoyée à la corbeille établissement par le responsable du service.",
            suggestedRouteId: undefined,
          },
          history: [
            ...current.history,
            {
              at: now,
              by: actorName,
              action: "RELEASE_TO_CORBEILLE",
              note: "Renvoyée à la corbeille (tout personnel) par le responsable.",
            },
          ],
        };
        const finalizedBasket = finalizeRequestPurgeMetadata(current, updatedBasket, now);
        index[pos] = finalizedBasket;
        await saveRequestFile(finalizedBasket);
        await saveRequestsIndex( index);
        return NextResponse.json({ success: true, request: finalizedBasket });
      }
      const existing = current.assignedTo.claimedBy;
      if (!existing?.email) return NextResponse.json({ error: "Aucune prise en charge active" }, { status: 400 });
      if (normalizeRequestEmail(existing.email) !== normalizeRequestEmail(actorEmail)) { return NextResponse.json({ error: "Seul le collaborateur assigné peut libérer la fiche" }, { status: 403 })}
      const updatedRel: RequestRecord = {
        ...current,
        updatedAt: now,
        assignedTo: { ...current.assignedTo, claimedBy: null },
        history: [
          ...current.history,
          { at: now, by: actorName, action: "RELEASE_CLAIM", note: "Fiche remise en file partagée" },
        ],
      };
      const finalizedRel = finalizeRequestPurgeMetadata(current, updatedRel, now);
      index[pos] = finalizedRel;
      await saveRequestFile(finalizedRel);
      await saveRequestsIndex( index);
      return NextResponse.json({ success: true, request: finalizedRel });
    }
    if (action === "claim_self") {
      if (!actorEmail) return NextResponse.json({ error: "Email requis pour prendre en charge" }, { status: 400 });
      if (!(await canAccessRequestsStaffBoard(actorRoles, actorEmail))) { return NextResponse.json({ error: "Réservé au personnel habilité aux demandes" }, { status: 403 })}
      const existingClaim = current.assignedTo.claimedBy?.email;
      if (!existingClaim && !(await isUserInRequestPool(current, actorEmail))) { return NextResponse.json({ error: "Vous ne pouvez pas prendre cette demande." }, { status: 403 })}
      const statusOpt = String(body?.status || "").trim() as RequestStatus;
      let nextStatus = current.status;
      if (statusOpt && REQUEST_STATUSES.includes(statusOpt)) nextStatus = statusOpt;
      else if (current.status === "NOUVELLE") nextStatus = "EN_COURS";
      const existing = current.assignedTo.claimedBy;
      const hadOther = existing?.email && normalizeRequestEmail(existing.email) !== normalizeRequestEmail(actorEmail);
      const note = hadOther ? `Prise en charge personnelle — réattribution (avant : ${existing!.name || existing!.email}).` : "Prise en charge personnelle (y compris hors file d’origine ou si l’IA s’est trompée).";
      const fromBr = normalizeRequestBranchId(current.assignedTo.routeId, current.assignedTo.unit);
      let nextCategory = current.category;
      let nextAssignedTo: RequestRecord["assignedTo"] = { ...current.assignedTo, claimedBy: { email: actorEmail, name: actorName, userId: userId ?? null, at: now }};
      if (isCorbeilleBranchId(fromBr)) {
        const dest = getDefaultRequestBranchForStaffEmail(actorEmail);
        if (dest) {
          const resolved = resolveRequestRouteById(dest);
          if (resolved) {
            nextCategory = resolved.category;
            nextAssignedTo = {
              ...resolved.assignedTo,
              claimedBy: { email: actorEmail, name: actorName, userId: userId ?? null, at: now },
            };
          }
        }
      }
      const updatedSelf: RequestRecord = {
        ...current,
        updatedAt: now,
        status: nextStatus,
        category: nextCategory,
        assignedTo: nextAssignedTo,
        history: [
          ...current.history,
          {
            at: now,
            by: actorName,
            action: "CLAIM_SELF",
            note: isCorbeilleBranchId(fromBr)
              ? `${note} Affectation automatique vers la branche du collaborateur.`
              : note,
          },
        ],
      };
      const finalizedSelf = finalizeRequestPurgeMetadata(current, updatedSelf, now);
      index[pos] = finalizedSelf;
      await saveRequestFile(finalizedSelf);
      await saveRequestsIndex( index);
      return NextResponse.json({ success: true, request: finalizedSelf });
    }
    if (action === "transmit_to_direction") {
      if (!(await canAccessRequestsStaffBoard(actorRoles, actorEmail))) {
        return NextResponse.json({ error: "Réservé au personnel habilité." }, { status: 403 });
      }
      const directionId = assignRouteIdRaw || current.routing?.directionHint?.suggestedQueueId || "";
      if (!MANUAL_ONLY_DIRECTION_IDS.has(directionId)) {
        return NextResponse.json({ error: "Corbeille direction invalide." }, { status: 400 });
      }
      const resolved = await resolveRequestRouteById(directionId);
      if (!resolved) return NextResponse.json({ error: "Configuration direction manquante." }, { status: 500 });
      const updatedDir: RequestRecord = {
        ...current,
        updatedAt: now,
        category: resolved.category,
        assignedTo: { ...resolved.assignedTo, claimedBy: current.assignedTo.claimedBy ?? null },
        routing: {
          ...current.routing,
          directionHint: undefined,
          reason: `${current.routing.reason} — transmise à ${resolved.assignedTo.roleLabel} par ${actorName}.`,
        },
        history: [
          ...current.history,
          {
            at: now,
            by: actorName,
            action: "TRANSMIS_DIRECTION",
            note: `Transmise à ${resolved.assignedTo.roleLabel} (${directionId}).`,
          },
        ],
      };
      const finalizedDir = finalizeRequestPurgeMetadata(current, updatedDir, now);
      index[pos] = finalizedDir;
      await saveRequestFile(finalizedDir);
      await saveRequestsIndex(index);
      return NextResponse.json({ success: true, request: finalizedDir });
    }
    const priorStatusForNotify = current.status;
    let updated: RequestRecord = { ...current, updatedAt: now};
    if (status && REQUEST_STATUSES.includes(status)) {
      if (!(await canAccessRequestsStaffBoard(actorRoles, actorEmail))) {  return NextResponse.json({ error: "Non autorisé" }, { status: 403 });}
      const claimedBy = current.assignedTo.claimedBy?.email;
      const claimedMe = Boolean(claimedBy) && normalizeRequestEmail(claimedBy!) === normalizeRequestEmail(actorEmail);
      const inPool = await isUserInRequestPool(current, actorEmail);
      if (!claimedMe && !inPool) {
        return NextResponse.json({ error: "Vous n'avez pas accès à cette demande." }, { status: 403 });
      }
      if (!claimedBy && (status === "EN_ATTENTE" || status === "TERMINEE")) {
        return NextResponse.json(
          { error: "Prenez d'abord la demande en charge (colonne En cours)." },
          { status: 400 },
        );
      }
      updated = {
        ...updated,
        status,
        history: [
          ...updated.history,
          {
            at: now,
            by: actorName,
            action: "STATUS_CHANGE",
            note: `Nouveau statut: ${status}`,
          },
        ],
      };
    }

    let targetRouteId = assignRouteIdRaw;
    if (!targetRouteId && assignUnit) {
      targetRouteId = LEGACY_ASSIGN_UNIT_TO_ROUTE[assignUnit] || assignUnit;
    }
    if (targetRouteId) {
      if (!(await isLeaderForRequestBranch(current.assignedTo.routeId, current.assignedTo.unit, actorEmail))) {
        return NextResponse.json({ error: "Seul le responsable du service peut renvoyer la demande vers un autre service." },{ status: 403 });
      }
      const resolved = await resolveRequestRouteById(targetRouteId);
      if (!resolved) return NextResponse.json({ error: "Route d'assignation invalide" }, { status: 400 });
      updated = {
        ...updated,
        category: resolved.category,
        assignedTo: resolved.assignedTo,
        routing: {
          ...updated.routing,
          source: "fallback",
          confidence: 1,
          reason: resolved.reason,
          suggestedRouteId: undefined,
        },
        history: [
          ...updated.history,
          {
            at: now,
            by: actorName,
            action: "REASSIGNATION",
            note: `Réassignée à ${resolved.assignedTo.routeId} (${resolved.assignedTo.roleLabel})`,
          },
        ],
      };
    }
    let noteForEmail: string | undefined;
    let closureAttachments: RequestAttachment[] | undefined;
    if (comment.trim() || multipartFiles.length > 0) {
      if (multipartFiles.length > 0 && !(await canAccessRequestsStaffBoard(actorRoles, actorEmail))) {
        return NextResponse.json({ error: "Seul le personnel habilité peut joindre des fichiers aux réponses." }, { status: 403 });
      }
      let commentText = comment.trim();
      let commentAttachments: RequestAttachment[] = [];
      if (multipartFiles.length > 0) {
        for (const f of multipartFiles) {
          const v = assertEligibleRequestAttachment(f.name, f.type, f.size);
          if (!v.ok) return NextResponse.json({ error: v.error }, { status: 400 });
        }
        const bufs = await Promise.all(
          multipartFiles.map(async (f) => ({
            buffer: Buffer.from(await f.arrayBuffer()),
            fileName: f.name,
            contentType: f.type || "application/octet-stream",
          })),
        );
        commentAttachments = await uploadBuffersAsRequestAttachments( id, bufs);
      }
      if (!commentText && commentAttachments.length > 0) commentText = "Pièce(s) jointe(s).";
      if (commentText || commentAttachments.length > 0) {
        const newComment: RequestComment = {
          id: `${Date.now()}_${Math.random().toString(36).slice(2, 7)}`,
          at: now,
          by: actorName,
          byEmail: actorEmail,
          toRequester,
          content: commentText || "Pièce(s) jointe(s).",
          ...(commentAttachments.length > 0 ? { attachments: commentAttachments } : {}),
        };
        updated = {
          ...updated,
          comments: [...updated.comments, newComment],
          history: [
            ...updated.history,
            {
              at: now,
              by: actorName,
              action: "COMMENT",
              note: toRequester ? "Commentaire envoyé au demandeur" : "Commentaire interne",
            },
          ],
        };
        noteForEmail = [ commentText, commentAttachments.length > 0 ? `Fichiers : ${commentAttachments.map((a) => a.fileName).join(", ")}` : ""].filter(Boolean).join("\n");
        if (updated.status === "TERMINEE" && commentAttachments.length > 0) {
          closureAttachments = commentAttachments;
        }
      }
    }
    const finalized = finalizeRequestPurgeMetadata(current, updated, now);
    index[pos] = finalized;
    await saveRequestFile(finalized);
    await saveRequestsIndex( index);
    try {
      const reachedMilestone = finalized.status !== priorStatusForNotify && (finalized.status === "EN_COURS" || finalized.status === "EN_ATTENTE" || finalized.status === "TERMINEE");
      if (reachedMilestone) {
        const publicNote = toRequester ? noteForEmail : undefined;
        await notifyRequestStatusMilestone(finalized, priorStatusForNotify, publicNote, closureAttachments);
      } else if (toRequester && noteForEmail?.trim()) {
        await notifyRequesterOnly(finalized, noteForEmail);
      }
    } catch (mailError) {
      console.error("Request update notification error:", mailError);
    }
    return NextResponse.json({ success: true, request: finalized });
  } catch (error) {
    console.error("Request update error:", error);
    return NextResponse.json({ error: "Erreur mise à jour demande" }, { status: 500 });
  }
}