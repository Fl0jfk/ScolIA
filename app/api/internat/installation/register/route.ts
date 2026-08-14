import { NextResponse } from "next/server";
import { loadAppConfig } from "@/app/lib/app-config";
import { buildCalendarEventIcs } from "@/app/lib/calendar-ics";
import {
  addInstallationBooking,
  deleteInstallationBooking,
  getInstallationConfig,
  listInstallationBookings,
} from "@/app/lib/internat-installation-storage";
import {
  formatInstallationSlotFr,
  isValidOpenInstallationSlot,
  parseInstallationSlotKey,
} from "@/app/lib/internat-installation-slots";
import { escapeHtml } from "@/app/lib/escape-html";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";
import { createTenantTransporter, getTenantSmtpConfig } from "@/app/lib/tenant-mail";

/** 10 POST / IP / 10 min — même famille que chatbot / portail parents. */
const internatRegisterLimiter = createMemoryRateLimiter({
  windowMs: 10 * 60 * 1000,
  max: 10,
});

export async function POST(req: Request) {
  try {
    if (!internatRegisterLimiter.allow(clientIpFromRequest(req))) {
      return NextResponse.json(
        { error: "Trop de tentatives. Réessayez dans quelques minutes." },
        { status: 429 },
      );
    }

    const body = (await req.json()) as Record<string, unknown>;
    const honeypot = String(body.website || body.company || "").trim();
    if (honeypot) {
      return NextResponse.json({
        success: true,
        bookingId: "ignored",
        slotLabel: "votre créneau",
      });
    }

    const slotStart = String(body.slotStart || "").trim();
    const studentFirstName = String(body.studentFirstName || "").trim();
    const studentLastName = String(body.studentLastName || "").trim();
    const parentPhone = String(body.parentPhone || "").trim();
    const parentEmail = String(body.parentEmail || "").trim().toLowerCase();

    if (!slotStart || !studentFirstName || !studentLastName || !parentPhone || !parentEmail) {
      return NextResponse.json(
        { error: "Créneau, nom et prénom de l’élève, téléphone et e-mail du parent sont requis." },
        { status: 400 },
      );
    }
    if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(parentEmail)) {
      return NextResponse.json({ error: "E-mail parent invalide." }, { status: 400 });
    }
    if (!parseInstallationSlotKey(slotStart)) {
      return NextResponse.json({ error: "Créneau invalide." }, { status: 400 });
    }

    const config = await getInstallationConfig();
    if (!config.enabled) {
      return NextResponse.json(
        { error: "Les prises de rendez-vous ne sont pas ouvertes pour le moment." },
        { status: 403 },
      );
    }

    const bookings = await listInstallationBookings();
    if (!isValidOpenInstallationSlot(config, slotStart, bookings)) {
      return NextResponse.json(
        { error: "Ce créneau n’est plus disponible." },
        { status: 409 },
      );
    }

    const entry = await addInstallationBooking({
      slotStart,
      studentFirstName,
      studentLastName,
      parentPhone,
      parentEmail,
    });

    // Recontrôle après écriture (course simple) : si surcharge, on annule.
    const after = await listInstallationBookings();
    const sameSlot = after.filter((b) => b.slotStart === slotStart);
    if (sameSlot.length > config.maxFamiliesPerSlot) {
      await deleteInstallationBooking(entry.id);
      return NextResponse.json(
        { error: "Ce créneau vient d’être complet. Choisissez un autre horaire." },
        { status: 409 },
      );
    }

    const parsed = parseInstallationSlotKey(slotStart)!;
    const slotLabel = formatInstallationSlotFr(slotStart);
    const ics = buildCalendarEventIcs({
      title: `${config.title} — ${studentFirstName} ${studentLastName}`,
      description: config.intro,
      location: config.location,
      startDate: parsed.date,
      startTime: parsed.time,
      durationMinutes: config.slotDurationMinutes,
      uid: `internat-install-${entry.id}@scola`,
      prodId: "-//Scola//Installation internat//FR",
    });

    const smtp = await getTenantSmtpConfig();
    const transporter = await createTenantTransporter();
    if (smtp && transporter) {
      const bundle = await loadAppConfig();
      const school = bundle.identity.shortName || bundle.identity.name;
      const studentLabel = escapeHtml(`${studentFirstName} ${studentLastName}`);
      await transporter.sendMail({
        from: `"${school}" <${smtp.user}>`,
        to: parentEmail,
        subject: `Confirmation — ${config.title}`,
        html: `
          <p>Bonjour,</p>
          <p>Votre rendez-vous d’installation internat est confirmé pour
          <strong>${studentLabel}</strong>.</p>
          <p><strong>Créneau :</strong> ${escapeHtml(slotLabel)}<br/>
          ${config.location ? `<strong>Lieu :</strong> ${escapeHtml(config.location)}<br/>` : ""}
          <strong>Téléphone indiqué :</strong> ${escapeHtml(parentPhone)}</p>
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

    return NextResponse.json({ success: true, bookingId: entry.id, slotLabel });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Inscription impossible." },
      { status: 500 },
    );
  }
}
