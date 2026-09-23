import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  broadcastFamilleMessage,
  createFamilleThreadWithMessage,
  getFamilleMessagingSettings,
  listFoyersLight,
  listStaffFamilleThreads,
  replyFamilleThreadMessage,
  type FamilleAttachmentInput,
} from "@/app/lib/famille-messaging-db";
import {
  canBroadcastFromMatrix,
  canInitiateFromMatrix,
  filterFoyersForProfClasses,
  isProfesseurOnly,
} from "@/app/lib/famille-messaging-matrix";
import { listClassesForTeacherUser } from "@/app/lib/class-allocation-teachers";

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const { userId } = gate.ctx;
  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) {
    return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });
  }
  const settings = await getFamilleMessagingSettings(etabId);
  if (!canInitiateFromMatrix(roles, settings, { orgAdmin: user?.orgAdmin })) {
    return NextResponse.json({ error: "Action non autorisée (matrice)." }, { status: 403 });
  }

  let foyers = await listFoyersLight(etabId);
  let assignedClasses: string[] = [];
  if (isProfesseurOnly(roles) && settings.profOwnClassesOnly) {
    assignedClasses = await listClassesForTeacherUser(userId);
    foyers = filterFoyersForProfClasses(foyers, assignedClasses);
  }

  const threads = await listStaffFamilleThreads(etabId);
  return NextResponse.json({
    threads,
    foyers,
    settings,
    canBroadcast: canBroadcastFromMatrix(roles, settings, { orgAdmin: user?.orgAdmin }),
    assignedClasses,
  });
}

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const { userId } = gate.ctx;
  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) {
    return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });
  }
  const settings = await getFamilleMessagingSettings(etabId);
  if (!canInitiateFromMatrix(roles, settings, { orgAdmin: user?.orgAdmin })) {
    return NextResponse.json({ error: "Action non autorisée (matrice)." }, { status: 403 });
  }

  const body = (await req.json().catch(() => ({}))) as {
    action?: string;
    foyerId?: string;
    foyerIds?: string[];
    eleveId?: string;
    sujet?: string;
    corps?: string;
    threadId?: string;
    broadcast?: boolean;
    attachments?: FamilleAttachmentInput[];
  };

  const auteurNom =
    user?.fullName ||
    [user?.firstName, user?.lastName].filter(Boolean).join(" ") ||
    user?.primaryEmailAddress?.emailAddress ||
    "Établissement";

  const attachments = Array.isArray(body.attachments) ? body.attachments : [];

  try {
    if (body.action === "reply") {
      const result = await replyFamilleThreadMessage(etabId, {
        threadId: String(body.threadId || ""),
        auteurCote: "staff",
        auteurUserId: userId,
        auteurNom,
        corps: String(body.corps || ""),
        attachments,
      });
      return NextResponse.json({ success: true, ...result });
    }

    if (body.broadcast) {
      if (!canBroadcastFromMatrix(roles, settings, { orgAdmin: user?.orgAdmin })) {
        return NextResponse.json(
          { error: "Diffusion non autorisée (matrice)." },
          { status: 403 },
        );
      }
      let foyerIds = Array.isArray(body.foyerIds)
        ? body.foyerIds.map(String)
        : [];
      if (!foyerIds.length) {
        let foyers = await listFoyersLight(etabId);
        if (isProfesseurOnly(roles) && settings.profOwnClassesOnly) {
          const classes = await listClassesForTeacherUser(userId);
          foyers = filterFoyersForProfClasses(foyers, classes);
        }
        foyerIds = foyers.map((f) => f.id);
      }
      const created = await broadcastFamilleMessage(etabId, {
        foyerIds,
        sujet: String(body.sujet || ""),
        corps: String(body.corps || ""),
        auteurUserId: userId,
        auteurNom,
        attachments,
      });
      return NextResponse.json({
        success: true,
        broadcast: true,
        count: created.length,
        threads: created.map((c) => c.thread),
      });
    }

    const result = await createFamilleThreadWithMessage(etabId, {
      foyerId: String(body.foyerId || ""),
      eleveId: body.eleveId || null,
      sujet: String(body.sujet || ""),
      corps: String(body.corps || ""),
      auteurUserId: userId,
      auteurNom,
      attachments,
    });
    return NextResponse.json({ success: true, ...result });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Envoi impossible." },
      { status: 400 },
    );
  }
}
