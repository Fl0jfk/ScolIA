import { NextResponse } from "next/server";
import { resolveSession } from "@/app/lib/intranet-session";

import { requireAuth } from "@/app/lib/intranet-auth";
import { getJson, putJson } from "@/app/lib/s3-storage";
import { computeTripReminders } from "@/app/lib/travels-trip-helpers";
import type { TravelsTrip } from "@/app/lib/travels-types";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
} from "@/app/lib/tenant-mail";

function appBaseUrl() {
  return (process.env.NEXT_PUBLIC_APP_URL || "").trim().replace(/\/$/, "");
}

/** GET : rappels calculés pour tous les dossiers ou un tripId. */
export async function GET(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const url = new URL(req.url);
  const tripId = url.searchParams.get("tripId");

  const indexHit = await getJson<TravelsTrip[]>("travels/index.json");
  const index = Array.isArray(indexHit?.data) ? indexHit.data : [];

  if (tripId) {
    const hit = await getJson<TravelsTrip>(`travels/${tripId}.json`);
    const trip = hit?.data;
    if (!trip) return NextResponse.json({ error: "Introuvable" }, { status: 404 });
    return NextResponse.json({ reminders: computeTripReminders(trip) });
  }

  const reminders: Array<ReturnType<typeof computeTripReminders>[number] & {
    tripTitle?: string;
    tripDestination?: string;
  }> = [];
  for (const summary of index.slice(0, 200)) {
    if (!summary?.id) continue;
    const hit = await getJson<TravelsTrip>(`travels/${summary.id}.json`);
    if (hit?.data) {
      const trip = hit.data;
      reminders.push(
        ...computeTripReminders(trip).map((r) => ({
          ...r,
          tripTitle: trip.data?.title,
          tripDestination: trip.data?.destination,
        })),
      );
    }
  }

  return NextResponse.json({ reminders, count: reminders.length });
}

/** POST : envoie un e-mail de rappel interne (créateur) pour un dossier. */
export async function POST(req: Request) {
  const cronSecret = process.env.TRAVELS_CRON_SECRET;
  const body = await req.json();
  const isCron = cronSecret && body.cronSecret === cronSecret;

  if (!isCron) {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;
  } else {
    const session = await resolveSession();
    const userId = session?.userId;
    if (!userId && !isCron) return NextResponse.json({ error: "Non autorisé" }, { status: 401 });
  }

  try {
    const tripId = String(body.tripId || "");
    const reminderId = String(body.reminderId || "");
    if (!tripId) return NextResponse.json({ error: "tripId requis" }, { status: 400 });

    const hit = await getJson<TravelsTrip>(`travels/${tripId}.json`);
    const trip = hit?.data;
    if (!trip?.ownerEmail) {
      return NextResponse.json({ error: "Dossier ou email créateur introuvable" }, { status: 404 });
    }

    const reminders = computeTripReminders(trip);
    const reminder = reminderId ? reminders.find((r) => r.id === reminderId) : reminders[0];
    if (!reminder) return NextResponse.json({ error: "Aucun rappel applicable" }, { status: 400 });

    const already = trip.data.remindersSent?.[reminder.type];
    if (already && (reminder.type === "bus_liste_j3" || reminder.type === "com_parents_j0")) {
      return NextResponse.json({
        success: true,
        reminder,
        skipped: true,
        reason: "Rappel déjà envoyé",
      });
    }

    const smtp = await getTenantSmtpConfig();
    if (!smtp) {
      return NextResponse.json({ error: "SMTP non configuré" }, { status: 503 });
    }
    const transporter = await createTenantTransporter();
    if (!transporter) {
      return NextResponse.json({ error: "SMTP non configuré" }, { status: 503 });
    }

    const base = appBaseUrl();
    const path = reminder.href || `/travels/${tripId}`;
    const link = base ? `${base}${path}` : path;

    let subject = `Rappel sortie — ${trip.data.title || tripId}`;
    let bodyLines = [
      `Bonjour ${trip.ownerName || ""},`,
      "",
      reminder.label,
      "",
      `Dossier : ${trip.data.title}`,
      `Destination : ${trip.data.destination}`,
      `Statut actuel : ${trip.status}`,
      "",
      `Ouvrir le dossier : ${link}`,
      "",
      "Cordialement,",
      "Plateforme Voyages",
    ];

    if (reminder.type === "bus_liste_j3") {
      subject = `Confirmez la liste des élèves — ${trip.data.title || tripId}`;
      bodyLines = [
        `Bonjour ${trip.ownerName || ""},`,
        "",
        "La sortie approche : merci de confirmer la liste des élèves sur la plateforme.",
        "Une fois confirmée, la liste sera envoyée automatiquement au transporteur.",
        "",
        reminder.label,
        "",
        `Lien : ${link}`,
        "",
        "Cordialement,",
        "Plateforme Voyages",
      ];
    } else if (reminder.type === "com_parents_j0") {
      subject = `Communication parents disponible — ${trip.data.title || tripId}`;
      bodyLines = [
        `Bonjour ${trip.ownerName || ""},`,
        "",
        "C'est le jour du départ : vous pouvez communiquer aux parents (messages et photos) depuis l'onglet Communication.",
        "Cette proposition est optionnelle et non bloquante.",
        "",
        `Lien : ${link}`,
        "",
        "Cordialement,",
        "Plateforme Voyages",
      ];
    }

    await transporter.sendMail({
      from: `"Plateforme Voyages" <${smtp.user}>`,
      to: trip.ownerEmail,
      subject,
      text: bodyLines.join("\n"),
    });

    const now = new Date().toISOString();
    const updatedTrip: TravelsTrip = {
      ...trip,
      updatedAt: now,
      data: {
        ...trip.data,
        remindersSent: {
          ...(trip.data.remindersSent || {}),
          [reminder.type]: now,
        },
      },
    };
    await putJson(`travels/${tripId}.json`, updatedTrip);

    return NextResponse.json({ success: true, reminder });
  } catch (e) {
    console.error("[reminders]", e);
    return NextResponse.json({ error: "Envoi rappel impossible" }, { status: 500 });
  }
}
