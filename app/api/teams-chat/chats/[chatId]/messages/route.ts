import { NextRequest, NextResponse } from "next/server";
import { requireTeamsChatAccess } from "@/app/lib/teams-chat/access";
import {
  getTeamsChatAccessContext,
  listChatMessages,
  sendChatMessage,
  TeamsChatUnlinkedError,
} from "@/app/lib/teams-chat/graph";

type Ctx = { params: Promise<{ chatId: string }> };

export async function GET(_req: NextRequest, route: Ctx) {
  const gate = await requireTeamsChatAccess();
  if (!gate.ok) return gate.response;

  const { chatId } = await route.params;
  if (!chatId?.trim()) {
    return NextResponse.json({ error: "chatId manquant." }, { status: 400 });
  }

  try {
    const ctx = await getTeamsChatAccessContext(gate.userId);
    const messages = await listChatMessages(ctx.accessToken, chatId, ctx.microsoftUserId);
    return NextResponse.json({ messages });
  } catch (e) {
    if (e instanceof TeamsChatUnlinkedError) {
      return NextResponse.json({ error: e.message, code: "TEAMS_UNLINKED" }, { status: 409 });
    }
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Impossible de charger les messages." },
      { status: 502 },
    );
  }
}

export async function POST(req: NextRequest, route: Ctx) {
  const gate = await requireTeamsChatAccess();
  if (!gate.ok) return gate.response;

  const { chatId } = await route.params;
  if (!chatId?.trim()) {
    return NextResponse.json({ error: "chatId manquant." }, { status: 400 });
  }

  let text = "";
  try {
    const body = (await req.json()) as { text?: string };
    text = body.text?.trim() || "";
  } catch {
    text = "";
  }
  if (!text) {
    return NextResponse.json({ error: "Message vide." }, { status: 400 });
  }

  try {
    const ctx = await getTeamsChatAccessContext(gate.userId);
    await sendChatMessage(ctx.accessToken, chatId, text);
    const messages = await listChatMessages(ctx.accessToken, chatId, ctx.microsoftUserId);
    return NextResponse.json({ ok: true, messages });
  } catch (e) {
    if (e instanceof TeamsChatUnlinkedError) {
      return NextResponse.json({ error: e.message, code: "TEAMS_UNLINKED" }, { status: 409 });
    }
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Envoi impossible." },
      { status: 502 },
    );
  }
}
