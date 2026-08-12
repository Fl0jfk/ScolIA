import { NextRequest, NextResponse } from "next/server";
import { requireTeamsChatGraph } from "@/app/lib/teams-chat/access";
import { ensureOneOnOneChat, listTeamsOneOnOneChats } from "@/app/lib/teams-chat/graph";

export async function GET(req: NextRequest) {
  const gate = await requireTeamsChatGraph(req);
  if (!gate.ok) return gate.response;

  try {
    const chats = await listTeamsOneOnOneChats(gate.accessToken, gate.microsoftUserId);
    return NextResponse.json({ chats });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Impossible de lister les conversations." },
      { status: 502 },
    );
  }
}

/** Ouvre ou crée une conversation 1:1 avec un utilisateur Entra. */
export async function POST(req: NextRequest) {
  const gate = await requireTeamsChatGraph(req);
  if (!gate.ok) return gate.response;

  let otherUserId = "";
  try {
    const body = (await req.json()) as { otherUserId?: string };
    otherUserId = body.otherUserId?.trim() || "";
  } catch {
    otherUserId = "";
  }
  if (!otherUserId) {
    return NextResponse.json({ error: "otherUserId manquant." }, { status: 400 });
  }

  try {
    if (otherUserId === gate.microsoftUserId || otherUserId === gate.upn) {
      return NextResponse.json({ error: "Impossible de discuter avec soi-même." }, { status: 400 });
    }
    const chatId = await ensureOneOnOneChat(gate.accessToken, gate.microsoftUserId, otherUserId);
    return NextResponse.json({ chatId });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Impossible d’ouvrir la conversation." },
      { status: 502 },
    );
  }
}
