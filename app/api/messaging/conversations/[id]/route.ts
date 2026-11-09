import { NextResponse } from "next/server";
import { requireMessagingContext } from "@/app/lib/messaging/access";
import { messagingErrorResponse, readJsonBody } from "@/app/lib/messaging/http";
import {
  getConversationDetail,
  hideConversationForUser,
  updateConversationPreferences,
} from "@/app/lib/messaging/service";
import type { MessagingConversationPatchInput } from "@/app/lib/messaging/types";

type Ctx = { params: Promise<{ id: string }> };

export async function GET(_req: Request, ctx: Ctx) {
  const gate = await requireMessagingContext();
  if (!gate.ok) return gate.response;
  try {
    const { id } = await ctx.params;
    const conversation = await getConversationDetail(
      gate.ctx.etablissementId,
      id,
      gate.ctx.userId,
    );
    return NextResponse.json({ conversation });
  } catch (error) {
    return messagingErrorResponse(error, "conversation GET", "Impossible de charger la conversation.");
  }
}

export async function PATCH(req: Request, ctx: Ctx) {
  const gate = await requireMessagingContext();
  if (!gate.ok) return gate.response;
  try {
    const { id } = await ctx.params;
    const raw = await readJsonBody(req);
    const input: MessagingConversationPatchInput = {};
    if (raw.pinned !== undefined) input.pinned = raw.pinned as boolean;
    if (raw.muted !== undefined) input.muted = raw.muted as boolean;
    if (raw.markedUnread !== undefined) input.markedUnread = raw.markedUnread as boolean;
    if (raw.title !== undefined) input.title = raw.title as string;
    const conversation = await updateConversationPreferences(
      gate.ctx.etablissementId,
      id,
      gate.ctx.userId,
      input,
    );
    return NextResponse.json({ conversation });
  } catch (error) {
    return messagingErrorResponse(error, "conversation PATCH", "Modification impossible.");
  }
}

/** « Supprimer pour moi » : les autres participants ne sont pas affectés. */
export async function DELETE(_req: Request, ctx: Ctx) {
  const gate = await requireMessagingContext();
  if (!gate.ok) return gate.response;
  try {
    const { id } = await ctx.params;
    await hideConversationForUser(gate.ctx.etablissementId, id, gate.ctx.userId);
    return NextResponse.json({ ok: true });
  } catch (error) {
    return messagingErrorResponse(error, "conversation DELETE", "Suppression impossible.");
  }
}
