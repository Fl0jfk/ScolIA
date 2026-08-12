import { NextResponse } from "next/server";
import { requireTeamsChatAccess } from "@/app/lib/teams-chat/access";
import { deleteTeamsChatLink } from "@/app/lib/teams-chat/tokens";

export async function POST() {
  const gate = await requireTeamsChatAccess();
  if (!gate.ok) return gate.response;
  await deleteTeamsChatLink(gate.userId);
  return NextResponse.json({ ok: true });
}
