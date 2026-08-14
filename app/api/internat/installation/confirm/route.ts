import { NextRequest, NextResponse } from "next/server";
import { loadAppConfig } from "@/app/lib/app-config";
import { buildCalendarEventIcs } from "@/app/lib/calendar-ics";
import { escapeHtml } from "@/app/lib/escape-html";
import {
  deleteInstallationBooking,
  findInstallationBookingByConfirmToken,
  getInstallationConfig,
  listInstallationBookings,
  markInstallationBookingConfirmed,
} from "@/app/lib/internat-installation-storage";
import {
  countBookingsBySlot,
  formatInstallationSlotFr,
  isConfirmedInstallationBooking,
  isValidOpenInstallationSlot,
  parseInstallationSlotKey,
} from "@/app/lib/internat-installation-slots";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";
import { createTenantTransporter, getTenantSmtpConfig } from "@/app/lib/tenant-mail";

async function redirectToConfirme(query: Record<string, string>) {
  const base = await tenantAbsolutePath("/internat/installation/confirme");
  const u = new URL(base);
  for (const [k, v] of Object.entries(query)) u.searchParams.set(k, v);
  return NextResponse.redirect(u);
}

export async function GET(req: NextRequest) {
  const token = req.nextUrl.searchParams.get("token")?.trim() ?? "";
  if (!token) {
    return redirectToConfirme({ erreur: "lien_invalide" });
  }

  try {
    const booking = await findInstallationBookingByConfirmToken(token);
    if (!booking) {
      return redirectToConfirme({ erreur: "lien_invalide" });
    }

    if (isConfirmedInstallationBooking(booking)) {
      return redirectToConfirme({ ok: "1", slot: booking.slotStart });
    }

    const config = await getInstallationConfig();
    if (!config.enabled) {
      return redirectToConfirme({ erreur: "lien_invalide" });
    }

    const bookings = await listInstallationBookings();
    if (!isValidOpenInstallationSlot(config, booking.slotStart, bookings)) {
      await deleteInstallationBooking(booking.id);
      return redirectToConfirme({ erreur: "creneau_complet" });
    }

    const confirmed = await markInstallationBookingConfirmed(booking.id);
    if (!confirmed) {
      return redirectToConfirme({ erreur: "lien_invalide" });
    }

    const after = await listInstallationBookings();
    const taken = countBookingsBySlot(after)[confirmed.slotStart] || 0;
    if (taken > config.maxFamiliesPerSlot) {
      await deleteInstallationBooking(confirmed.id);
      return redirectToConfirme({ erreur: "creneau_complet" });
    }

    const parsed = parseInstallationSlotKey(confirmed.slotStart);
    const slotLabel = formatInstallationSlotFr(confirmed.slotStart);
    const smtp = await getTenantSmtpConfig();
    const transporter = await createTenantTransporter();
    if (parsed && smtp && transporter) {
      const ics = buildCalendarEventIcs({
        title: `${config.title} — ${confirmed.studentFirstName} ${confirmed.studentLastName}`,
        description: config.intro,
        location: config.location,
        startDate: parsed.date,
        startTime: parsed.time,
        durationMinutes: config.slotDurationMinutes,
        uid: `internat-install-${confirmed.id}@scola`,
        prodId: "-//Scola//Installation internat//FR",
      });
      const bundle = await loadAppConfig();
      const school = bundle.identity.shortName || bundle.identity.name;
      const studentLabel = escapeHtml(
        `${confirmed.studentFirstName} ${confirmed.studentLastName}`,
      );
      await transporter.sendMail({
        from: `"${school}" <${smtp.user}>`,
        to: confirmed.parentEmail,
        subject: `Confirmation — ${config.title}`,
        html: `
          <p>Bonjour,</p>
          <p>Votre rendez-vous d’installation internat est confirmé pour
          <strong>${studentLabel}</strong>.</p>
          <p><strong>Créneau :</strong> ${escapeHtml(slotLabel)}<br/>
          ${config.location ? `<strong>Lieu :</strong> ${escapeHtml(config.location)}<br/>` : ""}
          <strong>Téléphone indiqué :</strong> ${escapeHtml(confirmed.parentPhone)}</p>
          <p>Ajoutez l’événement à votre agenda via le fichier joint (.ics).</p>
        `,
        attachments: [
          {
            filename: "installation-internat.ics",
            content: ics,
            contentType: "text/calendar",
          },
        ],
      });
    }

    return redirectToConfirme({ ok: "1", slot: confirmed.slotStart });
  } catch (e) {
    console.error("[internat/installation] confirm", e);
    return redirectToConfirme({ erreur: "lien_invalide" });
  }
}
