import { NextResponse } from "next/server";
import { avatarErrorResponse, requireAvatarContext } from "@/app/lib/avatar-access";
import { listAvatarVersions } from "@/app/lib/avatar-db";
import { avatarUrl, canManageAvatarOf } from "@/app/lib/avatar-rules";

export const dynamic = "force-dynamic";

/** Liste du personnel et de ses photos, réservée aux administrateurs. */
export async function GET() {
  try {
    const context = await requireAvatarContext();
    if (!context.ok) return context.response;
    if (!context.canManage) {
      return NextResponse.json({ error: "Réservé aux administrateurs." }, { status: 403 });
    }
    const manageable = context.staff.filter((m) => canManageAvatarOf(context.actor, m));
    const versions = await listAvatarVersions(manageable.map((m) => m.userId));
    return NextResponse.json(
      {
        staff: manageable.map((m) => {
          const updatedAt = versions.get(m.userId);
          return {
            userId: m.userId,
            displayName: m.displayName,
            photoUrl: updatedAt ? avatarUrl(m.userId, updatedAt) : null,
          };
        }),
      },
      { headers: { "Cache-Control": "no-store" } },
    );
  } catch (error) {
    return avatarErrorResponse(error);
  }
}
