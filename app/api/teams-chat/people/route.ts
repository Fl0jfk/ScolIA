import { NextRequest, NextResponse } from "next/server";
import { requireTeamsChatGraph } from "@/app/lib/teams-chat/access";
import { searchTeamsChatPeople } from "@/app/lib/teams-chat/graph";

export async function GET(req: NextRequest) {
  const gate = await requireTeamsChatGraph(req);
  if (!gate.ok) return gate.response;

  const q = req.nextUrl.searchParams.get("q")?.trim() || "";
  try {
    const people = await searchTeamsChatPeople(gate.accessToken, q);
    const me = (gate.upn || "").toLowerCase();
    return NextResponse.json({
      people: people.filter((p) => {
        const id = p.id.toLowerCase();
        const upn = (p.userPrincipalName || "").toLowerCase();
        return id !== gate.microsoftUserId.toLowerCase() && id !== me && upn !== me;
      }),
    });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Recherche impossible." },
      { status: 502 },
    );
  }
}
