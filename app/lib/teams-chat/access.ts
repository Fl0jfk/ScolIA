import { NextRequest, NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import {
  getTeamsChatAccessContext,
  TeamsChatUnlinkedError,
} from "@/app/lib/teams-chat/graph";
import { isTeamsChatOverlayEnabled } from "@/app/lib/teams-chat/flag";
import { canUseTeamsChatOverlay } from "@/app/lib/teams-chat/roles";

export { canUseTeamsChatOverlay } from "@/app/lib/teams-chat/roles";

export function graphTokenFromRequest(req: NextRequest): string {
  const header = req.headers.get("x-graph-access-token")?.trim() || "";
  if (header) return header;
  const auth = req.headers.get("authorization")?.trim() || "";
  return auth.replace(/^Bearer\s+/i, "").trim();
}

export async function requireTeamsChatAccess(): Promise<
  { ok: true; userId: string } | { ok: false; response: NextResponse }
> {
  if (!isTeamsChatOverlayEnabled()) {
    return {
      ok: false,
      response: NextResponse.json(
        { error: "Messagerie Teams indisponible.", code: "TEAMS_CHAT_DISABLED" },
        { status: 404 },
      ),
    };
  }

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

export async function requireTeamsChatGraph(req: NextRequest): Promise<
  | {
      ok: true;
      userId: string;
      accessToken: string;
      microsoftUserId: string;
      displayName?: string;
      upn?: string;
    }
  | { ok: false; response: NextResponse }
> {
  const gate = await requireTeamsChatAccess();
  if (!gate.ok) return gate;

  try {
    const ctx = await getTeamsChatAccessContext(gate.userId, graphTokenFromRequest(req));
    return { ok: true, userId: gate.userId, ...ctx };
  } catch (e) {
    if (e instanceof TeamsChatUnlinkedError) {
      return {
        ok: false,
        response: NextResponse.json({ error: e.message, code: "TEAMS_UNLINKED" }, { status: 409 }),
      };
    }
    return {
      ok: false,
      response: NextResponse.json(
        { error: e instanceof Error ? e.message : "Session Microsoft indisponible." },
        { status: 502 },
      ),
    };
  }
}
