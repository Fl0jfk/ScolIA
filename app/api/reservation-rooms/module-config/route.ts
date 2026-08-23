import { NextResponse } from "next/server";
import { loadAppConfig, saveProfRoomModule } from "@/app/lib/app-config";
import { parseProfRoomModule, type ProfRoomModuleConfig } from "@/app/lib/app-config-schemas";
import { requireAuth } from "@/app/lib/intranet-auth";
import { isProfRoomModuleAdmin, normalizeProfRoomAdminIds, requireProfRoomModuleAdmin } from "@/app/lib/prof-room-auth";
import { withDefaultProfRoomSubjects } from "@/app/lib/prof-room-defaults";

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  try {
    const config = await loadAppConfig();
    const merged = withDefaultProfRoomSubjects(config.profRoom);
    const { adminExternalUserIds, ...moduleConfig } = merged;
    const isAdmin = await isProfRoomModuleAdmin();
    return NextResponse.json({
      config: moduleConfig,
      ...(isAdmin ? { adminExternalUserIds: adminExternalUserIds || [] } : {}),
    });
  } catch (err: unknown) {
    console.error("GET /module-config:", err);
    return NextResponse.json({ error: "Impossible de charger la configuration." }, { status: 500 });
  }
}

export async function PUT(req: Request) {
  const gate = await requireProfRoomModuleAdmin();
  if (!gate.ok) return gate.response;
  try {
    const body = await req.json();
    const current = await loadAppConfig();
    let adminExternalUserIds = current.profRoom.adminExternalUserIds || [];

    if (Array.isArray(body?.adminExternalUserIds)) {
      adminExternalUserIds = normalizeProfRoomAdminIds(body.adminExternalUserIds);
    }

    const merged: ProfRoomModuleConfig = {
      ...current.profRoom,
      subjectColors:
        body?.subjectColors && typeof body.subjectColors === "object"
          ? parseProfRoomModule({ subjectColors: body.subjectColors }).subjectColors
          : current.profRoom.subjectColors,
      classesByPole:
        body?.classesByPole && typeof body.classesByPole === "object"
          ? parseProfRoomModule({ classesByPole: body.classesByPole }).classesByPole
          : current.profRoom.classesByPole,
      bookingHorizonDays:
        typeof body?.bookingHorizonDays === "number"
          ? body.bookingHorizonDays
          : current.profRoom.bookingHorizonDays,
      hoursStart: current.profRoom.hoursStart,
      hoursEnd: current.profRoom.hoursEnd,
      adminExternalUserIds,
    };
    await saveProfRoomModule(merged);
    const { adminExternalUserIds: savedAdminIds, ...moduleConfig } = merged;
    return NextResponse.json({
      success: true,
      config: moduleConfig,
      adminExternalUserIds: savedAdminIds,
    });
  } catch (err: unknown) {
    console.error("PUT /module-config:", err);
    const msg = err instanceof Error ? err.message : "Impossible d'enregistrer la configuration.";
    return NextResponse.json({ error: msg }, { status: 400 });
  }
}
