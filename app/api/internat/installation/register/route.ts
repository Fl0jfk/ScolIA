import { NextResponse } from "next/server";
import { loadAppConfig } from "@/app/lib/app-config";
import {
  addInstallationBooking,
  deleteInstallationBooking,
  generateInstallationConfirmToken,
  getInstallationConfig,
  INSTALLATION_PENDING_TTL_MS,
  listInstallationBookings,
} from "@/app/lib/internat-installation-storage";
import {
  formatInstallationSlotFr,
  isValidOpenInstallationSlot,
  parseInstallationSlotKey,
} from "@/app/lib/internat-installation-slots";
import { escapeHtml } from "@/app/lib/escape-html";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";
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
        needsEmailVerification: true,
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

    const smtp = await getTenantSmtpConfig();
    const transporter = await createTenantTransporter();
    if (!smtp || !transporter) {
      return NextResponse.json(
        {
          error:
            "La confirmation par e-mail n’est pas configurée. Contactez l’établissement.",
        },
        { status: 503 },
      );
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

    const confirmToken = generateInstallationConfirmToken();
    const entry = await addInstallationBooking({
      slotStart,
      studentFirstName,
      studentLastName,
      parentPhone,
      parentEmail,
      status: "pending",
      confirmToken,
      expiresAt: new Date(Date.now() + INSTALLATION_PENDING_TTL_MS).toISOString(),
    });

    try {
      const bundle = await loadAppConfig();
      const school = bundle.identity.shortName || bundle.identity.name;
      const slotLabel = formatInstallationSlotFr(slotStart);
      const confirmUrl = await tenantAbsolutePath(
        `/api/internat/installation/confirm?token=${encodeURIComponent(confirmToken)}`,
      );
      await transporter.sendMail({
        from: `"${school}" <${smtp.user}>`,
        to: parentEmail,
        subject: `Confirmez votre rendez-vous — ${config.title}`,
        html: `
          <p>Bonjour,</p>
          <p>Pour réserver le créneau d’installation internat de
          <strong>${escapeHtml(`${studentFirstName} ${studentLastName}`)}</strong>
          (${escapeHtml(slotLabel)}), cliquez sur ce lien (valable 2 heures) :</p>
          <p><a href="${escapeHtml(confirmUrl)}">${escapeHtml(confirmUrl)}</a></p>
          <p>Le créneau n’est réservé qu’après ce clic. Si vous n’êtes pas à l’origine de ce message, ignorez-le.</p>
        `,
      });
    } catch (e) {
      console.error("[internat/installation] pending mail", e);
      await deleteInstallationBooking(entry.id);
      return NextResponse.json(
        { error: "Impossible d’envoyer l’e-mail de confirmation. Réessayez plus tard." },
        { status: 500 },
      );
    }

    return NextResponse.json({ success: true, needsEmailVerification: true });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Inscription impossible." },
      { status: 500 },
    );
  }
}
