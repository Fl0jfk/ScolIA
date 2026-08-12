import { NextRequest, NextResponse } from "next/server";
import { requireTeamsChatAccess } from "@/app/lib/teams-chat/access";
import {
  ensureOneOnOneChat,
  getTeamsChatAccessContext,
  listTeamsOneOnOneChats,
  TeamsChatUnlinkedError,
} from "@/app/lib/teams-chat/graph";

export async function GET() {
  const gate = await requireTeamsChatAccess();
  if (!gate.ok) return gate.response;

  try {
    const ctx = await getTeamsChatAccessContext(gate.userId);
    const chats = await listTeamsOneOnOneChats(ctx.accessToken, ctx.microsoftUserId);
    return NextResponse.json({ chats });
  } catch (e) {
    if (e instanceof TeamsChatUnlinkedError) {
      return NextResponse.json({ error: e.message, code: "TEAMS_UNLINKED" }, { status: 409 });
    }
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Impossible de lister les conversations." },
      { status: 502 },
    );
  }
}

/** Ouvre ou crée une conversation 1:1 avec un utilisateur Entra. */
export async function POST(req: NextRequest) {
  const gate = await requireTeamsChatAccess();
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
    const ctx = await getTeamsChatAccessContext(gate.userId);
    if (otherUserId === ctx.microsoftUserId) {
      return NextResponse.json({ error: "Impossible de discuter avec soi-même." }, { status: 400 });
    }
    const chatId = await ensureOneOnOneChat(ctx.accessToken, ctx.microsoftUserId, otherUserId);
    return NextResponse.json({ chatId });
  } catch (e) {
    if (e instanceof TeamsChatUnlinkedError) {
      return NextResponse.json({ error: e.message, code: "TEAMS_UNLINKED" }, { status: 409 });
    }
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Impossible d’ouvrir la conversation." },
      { status: 502 },
    );
  }
}
