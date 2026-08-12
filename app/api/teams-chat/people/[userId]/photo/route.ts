import { NextRequest, NextResponse } from "next/server";
import { requireTeamsChatGraph } from "@/app/lib/teams-chat/access";
import { fetchUserPhotoBytes } from "@/app/lib/teams-chat/graph";

export async function GET(
  req: NextRequest,
  ctx: { params: Promise<{ userId: string }> },
) {
  const gate = await requireTeamsChatGraph(req);
  if (!gate.ok) return gate.response;

  const { userId } = await ctx.params;
  if (!userId?.trim()) {
    return NextResponse.json({ error: "userId manquant." }, { status: 400 });
  }

  try {
    const photo = await fetchUserPhotoBytes(gate.accessToken, userId);
    if (!photo) return new NextResponse(null, { status: 404 });
    return new NextResponse(new Uint8Array(photo.bytes), {
      status: 200,
      headers: {
        "Content-Type": photo.contentType,
        "Cache-Control": "private, max-age=3600",
      },
    });
  } catch {
    return new NextResponse(null, { status: 404 });
  }
}
