import { NextResponse } from "next/server";
import { requireAppUser } from "@/app/lib/app-session";
import { requireFamilleAccess } from "@/app/lib/famille-auth";
import {
  countUnreadNotifsForFoyers,
  getFamilleMessagingSettings,
  listFamilleThreadsForFoyers,
  replyFamilleThreadMessage,
  type FamilleAttachmentInput,
} from "@/app/lib/famille-messaging-db";

export async function GET() {
  const gate = await requireFamilleAccess();
  if (!gate.ok) return gate.response;
  const foyerIds = gate.ctx.foyers.map((f) => f.id);
  const [threads, unreadCount, settings] = await Promise.all([
    listFamilleThreadsForFoyers(gate.ctx.etablissementId, foyerIds),
    countUnreadNotifsForFoyers(gate.ctx.etablissementId, foyerIds),
    getFamilleMessagingSettings(gate.ctx.etablissementId),
  ]);
  return NextResponse.json({
    enfants: gate.ctx.enfants,
    foyers: gate.ctx.foyers,
    threads,
    unreadCount,
    allowParentAttachments: settings.allowParentAttachments,
  });
}

export async function POST(req: Request) {
  const gate = await requireFamilleAccess();
  if (!gate.ok) return gate.response;

  const body = (await req.json().catch(() => ({}))) as {
    action?: string;
    threadId?: string;
    corps?: string;
    attachments?: FamilleAttachmentInput[];
  };
  if (body.action !== "reply" || !body.threadId?.trim()) {
    return NextResponse.json({ error: "Action invalide." }, { status: 400 });
  }

  const foyerIds = new Set(gate.ctx.foyers.map((f) => f.id));
  const threads = await listFamilleThreadsForFoyers(gate.ctx.etablissementId, [...foyerIds]);
  const owned = threads.find((t) => t.id === body.threadId.trim());
  if (!owned || !foyerIds.has(owned.foyerId)) {
    return NextResponse.json({ error: "Conversation introuvable." }, { status: 404 });
  }

  const settings = await getFamilleMessagingSettings(gate.ctx.etablissementId);
  const attachments = Array.isArray(body.attachments) ? body.attachments : [];
  if (attachments.length && !settings.allowParentAttachments) {
    return NextResponse.json(
      { error: "Les pièces jointes parents sont désactivées." },
      { status: 403 },
    );
  }

  const appUser = await requireAppUser();
  const auteurNom = appUser.ok
    ? [appUser.user.firstName, appUser.user.lastName].filter(Boolean).join(" ") ||
      appUser.user.name ||
      appUser.user.email ||
      "Parent"
    : gate.ctx.email;

  try {
    const result = await replyFamilleThreadMessage(gate.ctx.etablissementId, {
      threadId: owned.id,
      auteurCote: "parent",
      auteurUserId: gate.ctx.authUserId,
      auteurNom,
      corps: String(body.corps || ""),
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
