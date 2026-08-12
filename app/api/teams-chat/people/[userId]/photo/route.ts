import { NextRequest, NextResponse } from "next/server";
import { requireTeamsChatAccess } from "@/app/lib/teams-chat/access";
import {
  fetchUserPhotoBytes,
  getTeamsChatAccessContext,
  TeamsChatUnlinkedError,
} from "@/app/lib/teams-chat/graph";

export async function GET(
  _req: NextRequest,
  ctx: { params: Promise<{ userId: string }> },
) {
  const gate = await requireTeamsChatAccess();
  if (!gate.ok) return gate.response;

  const { userId } = await ctx.params;
  if (!userId?.trim()) {
    return NextResponse.json({ error: "userId manquant." }, { status: 400 });
  }

  try {
    const access = await getTeamsChatAccessContext(gate.userId);
    const photo = await fetchUserPhotoBytes(access.accessToken, userId);
    if (!photo) return new NextResponse(null, { status: 404 });
    return new NextResponse(new Uint8Array(photo.bytes), {
      status: 200,
      headers: {
        "Content-Type": photo.contentType,
        "Cache-Control": "private, max-age=3600",
      },
    });
  } catch (e) {
    if (e instanceof TeamsChatUnlinkedError) {
      return NextResponse.json({ error: e.message, code: "TEAMS_UNLINKED" }, { status: 409 });
    }
    return new NextResponse(null, { status: 404 });
  }
}
