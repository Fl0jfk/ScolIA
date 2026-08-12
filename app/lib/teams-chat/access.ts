import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canUseTeamsChatOverlay } from "@/app/lib/teams-chat/roles";

export { canUseTeamsChatOverlay } from "@/app/lib/teams-chat/roles";

export async function requireTeamsChatAccess(): Promise<
  { ok: true; userId: string } | { ok: false; response: NextResponse }
> {
  const gate = await requireAuth();
  if (!gate.ok) return gate;

  const user = await safeCurrentUser();
  const roles = intranetRolesFromMetadata(user?.publicMetadata);
  if (!user || !canUseTeamsChatOverlay(roles)) {
    return {
      ok: false,
      response: NextResponse.json(
        { error: "Réservé au personnel de l’établissement.", code: "TEAMS_CHAT_FORBIDDEN" },
        { status: 403 },
      ),
    };
  }

  return { ok: true, userId: gate.ctx.userId };
}
