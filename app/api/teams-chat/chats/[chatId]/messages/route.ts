import { NextRequest, NextResponse } from "next/server";
import { requireTeamsChatGraph } from "@/app/lib/teams-chat/access";
import { listChatMessages, sendChatMessage } from "@/app/lib/teams-chat/graph";

type Ctx = { params: Promise<{ chatId: string }> };

export async function GET(req: NextRequest, route: Ctx) {
  const gate = await requireTeamsChatGraph(req);
  if (!gate.ok) return gate.response;

  const { chatId } = await route.params;
  if (!chatId?.trim()) {
    return NextResponse.json({ error: "chatId manquant." }, { status: 400 });
  }

  try {
    const messages = await listChatMessages(gate.accessToken, chatId, gate.microsoftUserId);
    return NextResponse.json({ messages });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Impossible de charger les messages." },
      { status: 502 },
    );
  }
}

export async function POST(req: NextRequest, route: Ctx) {
  const gate = await requireTeamsChatGraph(req);
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
    await sendChatMessage(gate.accessToken, chatId, text);
    const messages = await listChatMessages(gate.accessToken, chatId, gate.microsoftUserId);
    return NextResponse.json({ ok: true, messages });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Envoi impossible." },
      { status: 502 },
    );
  }
}
