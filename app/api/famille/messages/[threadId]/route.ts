import { NextResponse } from "next/server";
import { requireFamilleAccess } from "@/app/lib/famille-auth";
import {
  getFamilleThread,
  listAttachmentsForMessages,
  listFamilleThreadMessages,
  markFamilleThreadRead,
} from "@/app/lib/famille-messaging-db";

export async function GET(
  _req: Request,
  ctx: { params: Promise<{ threadId: string }> },
) {
  const gate = await requireFamilleAccess();
  if (!gate.ok) return gate.response;
  const { threadId } = await ctx.params;
  const thread = await getFamilleThread(gate.ctx.etablissementId, threadId);
  if (!thread) {
    return NextResponse.json({ error: "Conversation introuvable." }, { status: 404 });
  }
  const foyerIds = new Set(gate.ctx.foyers.map((f) => f.id));
  if (!foyerIds.has(thread.foyerId)) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }
  await markFamilleThreadRead(gate.ctx.etablissementId, threadId, "parent");
  const messages = await listFamilleThreadMessages(gate.ctx.etablissementId, threadId);
  const attachments = await listAttachmentsForMessages(
    gate.ctx.etablissementId,
    messages.map((m) => m.id),
  );
  return NextResponse.json({ thread, messages, attachments });
}
