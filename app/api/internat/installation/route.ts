import { NextRequest, NextResponse } from "next/server";
import { requireInternatAccess, requireInternatManage } from "@/app/api/internat/_auth";
import {
  deleteInstallationBooking,
  getInstallationConfig,
  listInstallationBookings,
  normalizeInstallationConfig,
  saveInstallationConfig,
} from "@/app/lib/internat-installation-storage";
import { listGeneratedSlotKeys } from "@/app/lib/internat-installation-slots";
import type { InternatInstallationConfig } from "@/app/lib/internat-types";

export async function GET() {
  const gate = await requireInternatAccess();
  if (!gate.ok) return gate.response;

  const [config, bookings] = await Promise.all([
    getInstallationConfig(),
    listInstallationBookings(),
  ]);
  const slotKeys = listGeneratedSlotKeys(config);
  return NextResponse.json({
    config,
    bookings: bookings.sort((a, b) => a.slotStart.localeCompare(b.slotStart)),
    generatedSlotCount: slotKeys.length,
  });
}

export async function PUT(req: NextRequest) {
  const gate = await requireInternatManage();
  if (!gate.ok) return gate.response;

  try {
    const body = (await req.json()) as Partial<InternatInstallationConfig>;
    const saved = await saveInstallationConfig(normalizeInstallationConfig(body));
    return NextResponse.json({ config: saved });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Enregistrement impossible." },
      { status: 500 },
    );
  }
}

export async function DELETE(req: NextRequest) {
  const gate = await requireInternatManage();
  if (!gate.ok) return gate.response;

  const id = req.nextUrl.searchParams.get("id")?.trim() || "";
  if (!id) {
    return NextResponse.json({ error: "id manquant." }, { status: 400 });
  }
  const ok = await deleteInstallationBooking(id);
  if (!ok) {
    return NextResponse.json({ error: "Inscription introuvable." }, { status: 404 });
  }
  return NextResponse.json({ ok: true });
}
