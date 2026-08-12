import { NextRequest, NextResponse } from "next/server";
import { requireTeamsChatAccess } from "@/app/lib/teams-chat/access";
import {
  getTeamsChatAccessContext,
  searchTeamsChatPeople,
  TeamsChatUnlinkedError,
} from "@/app/lib/teams-chat/graph";

export async function GET(req: NextRequest) {
  const gate = await requireTeamsChatAccess();
  if (!gate.ok) return gate.response;

  const q = req.nextUrl.searchParams.get("q")?.trim() || "";
  try {
    const ctx = await getTeamsChatAccessContext(gate.userId);
    const people = await searchTeamsChatPeople(ctx.accessToken, q);
    return NextResponse.json({
      people: people.filter((p) => p.id !== ctx.microsoftUserId),
    });
  } catch (e) {
    if (e instanceof TeamsChatUnlinkedError) {
      return NextResponse.json({ error: e.message, code: "TEAMS_UNLINKED" }, { status: 409 });
    }
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Recherche impossible." },
      { status: 502 },
    );
  }
}
