import { NextResponse } from "next/server";
import {
  getInstallationConfig,
  listInstallationBookings,
} from "@/app/lib/internat-installation-storage";
import { buildPublicInstallationSlots } from "@/app/lib/internat-installation-slots";

/** Créneaux restants (sans données perso) — page publique. */
export async function GET() {
  try {
    const config = await getInstallationConfig();
    if (!config.enabled) {
      return NextResponse.json({
        enabled: false,
        title: config.title,
        intro: config.intro || null,
        location: config.location || null,
        slots: [],
      });
    }
    const bookings = await listInstallationBookings();
    const slots = buildPublicInstallationSlots(config, bookings);
    return NextResponse.json({
      enabled: true,
      title: config.title,
      intro: config.intro || null,
      location: config.location || null,
      slotDurationMinutes: config.slotDurationMinutes,
      slots,
    });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Impossible de charger les créneaux." },
      { status: 500 },
    );
  }
}
