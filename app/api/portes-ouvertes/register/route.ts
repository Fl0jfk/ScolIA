import { NextResponse } from "next/server";
import { loadAppConfig } from "@/app/lib/app-config";
import { getToolboxConfig } from "@/app/lib/toolbox-config";
import {
  addPortesOuvertesRegistration,
  countRegistrationsBySlot,
  listPortesOuvertesRegistrations,
} from "@/app/lib/portes-ouvertes-storage";
import { buildPortesOuvertesIcs } from "@/app/lib/portes-ouvertes-ics";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";
import { createTenantTransporter, getTenantSmtpConfig } from "@/app/lib/tenant-mail";

const portesOuvertesLimiter = createMemoryRateLimiter({
  windowMs: 10 * 60 * 1000,
  max: 10,
});

async function sendPoMail(params: {
  to: string;
  subject: string;
  html: string;
  ics?: string;
}) {
  const smtp = await getTenantSmtpConfig();
  const transporter = await createTenantTransporter();
  if (!smtp || !transporter) return false;
  const bundle = await loadAppConfig();
  const school = bundle.identity.shortName || bundle.identity.name;
  await transporter.sendMail({
    from: `"${school}" <${smtp.user}>`,
    to: params.to,
    subject: params.subject,
    html: params.html,
    attachments: params.ics
      ? [{ filename: "portes-ouvertes.ics", content: params.ics, contentType: "text/calendar" }]
      : undefined,
  });
  return true;
}

export async function POST(req: Request) {
  try {
    if (!portesOuvertesLimiter.allow(clientIpFromRequest(req))) {
      return NextResponse.json(
        { error: "Trop de tentatives. Réessayez dans quelques minutes." },
        { status: 429 },
      );
    }

    const toolbox = await getToolboxConfig();
    const po = toolbox.tools["portes-ouvertes"];
    if (!po.enabled) {
      return NextResponse.json({ error: "Les portes ouvertes ne sont pas activées." }, { status: 403 });
    }

    const body = await req.json();
    const honeypot = String(body.website || body.company || "").trim();
    if (honeypot) {
      return NextResponse.json({ success: true });
    }
    const slotId = String(body.slotId || "").trim();
    const firstName = String(body.firstName || "").trim();
    const lastName = String(body.lastName || "").trim();
    const email = String(body.email || "").trim().toLowerCase();
    const phone = String(body.phone || "").trim() || undefined;
    const childrenInfo = String(body.childrenInfo || "").trim() || undefined;
    const consent = body.consent === true;

    if (!slotId || !firstName || !lastName || !email) {
      return NextResponse.json({ error: "Créneau, nom, prénom et e-mail requis." }, { status: 400 });
    }
    if (!consent) {
      return NextResponse.json({ error: "Veuillez accepter le traitement de vos données." }, { status: 400 });
    }

    const slot = po.slots.find((s) => s.id === slotId);
    if (!slot) {
      return NextResponse.json({ error: "Créneau invalide." }, { status: 400 });
    }

    const registrations = await listPortesOuvertesRegistrations();
    if (slot.maxPlaces) {
      const counts = countRegistrationsBySlot(registrations);
      if ((counts[slotId] || 0) >= slot.maxPlaces) {
        return NextResponse.json({ error: "Ce créneau est complet." }, { status: 409 });
      }
    }

    const entry = await addPortesOuvertesRegistration(
      {
        slotId,
        firstName,
        lastName,
        email,
        phone,
        childrenInfo,
        consent,
      },
      registrations,
    );

    const ics = buildPortesOuvertesIcs({
      title: `${po.title} — ${slot.label}`,
      description: po.intro,
      location: po.address,
      startAt: slot.startAt,
      endAt: slot.endAt,
      uid: `po-${entry.id}@scola`,
    });

    const slotLabel = slot.label;
    const dateStr = new Date(slot.startAt).toLocaleString("fr-FR", {
      dateStyle: "full",
      timeStyle: "short",
    });

    await sendPoMail({
      to: email,
      subject: `Confirmation — ${po.title}`,
      html: `
        <p>Bonjour ${firstName} ${lastName},</p>
        <p>Votre inscription aux <strong>${po.title}</strong> est confirmée.</p>
        <p><strong>Créneau :</strong> ${slotLabel}<br/>
        <strong>Date :</strong> ${dateStr}<br/>
        ${po.address ? `<strong>Adresse :</strong> ${po.address}` : ""}</p>
        <p>Ajoutez l'événement à votre agenda via le fichier joint (.ics).</p>
      `,
      ics,
    });

    if (po.notifyEmail) {
      await sendPoMail({
        to: po.notifyEmail,
        subject: `Nouvelle inscription — ${po.title}`,
        html: `<p>${firstName} ${lastName} (${email}) — créneau ${slotLabel}</p>`,
      });
    }

    return NextResponse.json({ success: true, registrationId: entry.id });
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}
