import { safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";

import { loadAppConfig } from "@/app/lib/app-config";
import { requireAuth, isIntranetAdmin, type AuthContext } from "@/app/lib/intranet-auth";
import { loadModuleAccess } from "@/app/lib/module-access-store";
import { userHasProfRoomAdminFlag } from "@/app/lib/module-access";

export function normalizeProfRoomAdminIds(ids: unknown[]): string[] {
  return [...new Set(ids.map((id) => String(id).trim()).filter(Boolean))];
}

export async function isListedProfRoomAdmin(): Promise<boolean> {
  const user = await safeCurrentUser();
  if (!user) return false;

  const access = await loadModuleAccess().catch(() => null);
  if (
    userHasProfRoomAdminFlag(access, {
      userId: user.id,
      businessUserId: user.id,
    })
  ) {
    return true;
  }

  // Legacy : liste dans settings/modules/prof-room.json
  const config = await loadAppConfig();
  const adminIds = config.profRoom.adminExternalUserIds || [];
  return adminIds.includes(user.id);
}

export async function isProfRoomModuleAdmin(): Promise<boolean> {
  if (await isIntranetAdmin()) return true;
  return isListedProfRoomAdmin();
}

export async function requireProfRoomModuleAdmin(): Promise<
  { ok: true; ctx: AuthContext } | { ok: false; response: NextResponse }
> {
  const gate = await requireAuth();
  if (!gate.ok) return gate;
  if (!(await isProfRoomModuleAdmin())) {
    return {
      ok: false,
      response: NextResponse.json(
        { error: "Réservé aux administrateurs du module réservation de salles.", code: "PROF_ROOM_ADMIN_REQUIRED" },
        { status: 403 },
      ),
    };
  }
  return gate;
}
