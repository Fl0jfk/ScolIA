import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { resolveSession, safeCurrentUser } from "@/app/lib/intranet-session";
import { getJson, putJson } from "@/app/lib/s3-storage";
import { assertTravelsTripAccess } from "@/app/lib/travels-rbac-server";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
} from "@/app/lib/tenant-mail";
import { buildTransportReplyTo } from "@/app/lib/travel-email-routing";
import type { TravelsTrip } from "@/app/lib/travels-types";

/** Réponse e-mail au transporteur depuis le fil du séjour. */
export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const session = await resolveSession();
  if (!session?.userId) {
    return NextResponse.json({ error: "Non autorisé" }, { status: 401 });
  }

  let body: {
    tripId?: string;
    toEmail?: string;
    subject?: string;
    bodyText?: string;
    replyToMessageId?: string;
  };
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: "JSON invalide" }, { status: 400 });
  }

  const tripId = String(body.tripId || "").trim();
  const toEmail = String(body.toEmail || "").trim();
  const bodyText = String(body.bodyText || "").trim();
  if (!tripId || !toEmail || bodyText.length < 2) {
    return NextResponse.json(
      { error: "tripId, toEmail et bodyText requis" },
      { status: 400 },
    );
  }
  if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(toEmail)) {
    return NextResponse.json({ error: "Adresse e-mail invalide" }, { status: 400 });
  }

  const hit = await getJson<TravelsTrip>(`travels/${tripId}.json`);
  const trip = hit?.data;
  if (!trip?.data) {
    return NextResponse.json({ error: "Dossier introuvable" }, { status: 404 });
  }

  const access = await assertTravelsTripAccess(trip, { requireOwnerOrDirection: true });
  if (!access.ok) {
    return NextResponse.json({ error: access.error }, { status: access.status });
  }

  const smtp = await getTenantSmtpConfig();
  const transporter = await createTenantTransporter();
  if (!smtp || !transporter) {
    return NextResponse.json({ error: "SMTP non configuré" }, { status: 503 });
  }

  const user = await safeCurrentUser();
  const userName = user?.fullName || user?.primaryEmailAddress?.emailAddress || "Administration";
  const dest = String(trip.data.destination || trip.data.title || "sortie");
  const subject =
    String(body.subject || "").trim() ||
    `Re: Transport scolaire — ${dest}`;
  const replyTo = await buildTransportReplyTo();
  const now = new Date().toISOString();

  try {
    await transporter.sendMail({
      from: `"Plateforme Voyages" <${smtp.user}>`,
      to: toEmail,
      ...(replyTo ? { replyTo } : {}),
      subject,
      text: [
        bodyText,
        ``,
        `—`,
        `${userName}`,
        `Établissement — module Sorties scolaires`,
      ].join("\n"),
    });
  } catch (e) {
    console.error("[reply-transport-email]", e);
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Échec envoi" },
      { status: 500 },
    );
  }

  const outboundMsg = {
    id: `out-${Date.now()}`,
    gmailMessageId: `outbound-${Date.now()}`,
    fromEmail: smtp.user,
    toEmail,
    subject,
    messageType: "reponse_generique",
    summary: bodyText.slice(0, 500),
    details: bodyText,
    receivedAt: now,
    source: "app_reply",
    direction: "outbound" as const,
    replyToMessageId: body.replyToMessageId || null,
  };

  const data = { ...trip.data };
  const existing = Array.isArray(data.transportEmailMessages) ? data.transportEmailMessages : [];
  data.transportEmailMessages = [outboundMsg, ...existing].slice(0, 50);

  const history = Array.isArray(trip.history) ? [...trip.history] : [];
  history.unshift({
    date: now,
    user: userName,
    action: "Réponse e-mail transporteur",
    note: `À ${toEmail} — ${bodyText.slice(0, 120)}`,
  });

  const updated: TravelsTrip = {
    ...trip,
    data,
    history: history.slice(0, 200),
  };
  await putJson(`travels/${tripId}.json`, updated);

  const indexHit = await getJson<TravelsTrip[]>("travels/index.json");
  const index = Array.isArray(indexHit?.data) ? indexHit.data : [];
  await putJson(
    "travels/index.json",
    index.map((t) => (String(t.id) === tripId ? { ...t, ...updated } : t)),
  );

  return NextResponse.json({ success: true, message: outboundMsg, trip: updated });
}
