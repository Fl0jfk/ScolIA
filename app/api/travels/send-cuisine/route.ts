import { resolveSession, safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";

import { loadAppConfig } from "@/app/lib/app-config";
import { resolveTravelsCuisineEmails } from "@/app/lib/app-config-schemas";
import { getJson, putJson } from "@/app/lib/s3-storage";
import { assertTravelsTripAccess } from "@/app/lib/travels-rbac-server";
import type { TravelsTrip } from "@/app/lib/travels-types";
import { buildCuisineOrderPdfBase64 } from "@/app/lib/travels-cuisine-pdf";
import { CUISINE_DAYS, cuisineDateRangeLabel, type CuisineTripPayload } from "@/app/lib/travels-cuisine-shared";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
} from "@/app/lib/tenant-mail";

type TripData = CuisineTripPayload["data"] & Record<string, unknown>;

type TripRecord = Omit<CuisineTripPayload, "data"> & {
  data: TripData;
  status?: string;
  ownerName?: string;
  history?: Array<{ date?: string; action?: string; user?: string; note?: string }>;
};

function buildCuisineSnapshot(trip: TripRecord) {
  const d = trip.data;
  return {
    sentAt: new Date().toISOString(),
    nbEleves: Number(d.nbEleves) || 0,
    nbAccompagnateurs: Number(d.nbAccompagnateurs) || 0,
    piqueNiqueDetails: d.piqueNiqueDetails,
  };
}

export async function POST(req: Request) {
  const session = await resolveSession();
  const userId = session?.userId;
  if (!userId) return new NextResponse("Non autorisé", { status: 401 });

  try {
    const body = await req.json();
    const mode: "initial" | "amendment" = body.mode === "amendment" ? "amendment" : "initial";
    const tripId = String(body.tripId || body.tripData?.id || "");

    let trip: TripRecord | null = body.tripData || null;
    if (tripId) {
      const hit = await getJson<TripRecord>(`travels/${tripId}.json`);
      if (hit?.data) trip = hit.data;
    }
    if (!trip?.data) {
      return NextResponse.json({ error: "Dossier introuvable" }, { status: 404 });
    }

    const access = await assertTravelsTripAccess(trip as TravelsTrip, { requireOwnerOrDirection: true });
    if (!access.ok) return NextResponse.json({ error: access.error }, { status: access.status });

    const details = trip.data.piqueNiqueDetails;
    if (!details?.active) {
      return NextResponse.json({ error: "Aucune commande cuisine à envoyer" }, { status: 400 });
    }

    if (mode === "amendment" && !trip.data.cuisineOrderSentAt) {
      return NextResponse.json(
        { error: "Aucune commande cuisine précédente — utilisez l'envoi initial." },
        { status: 400 },
      );
    }

    const user = await safeCurrentUser();
    const userName = body.userName || trip.ownerName || user?.fullName || "La Providence";
    const userEmail = body.userEmail || user?.primaryEmailAddress?.emailAddress;
    const organizerEmail = body.organizerEmail || trip.ownerEmail;

    const config = await loadAppConfig();
    const chefEmails = resolveTravelsCuisineEmails(config.notifications);
    const chefEmailLabel = chefEmails.join(" / ");

    const pdfBase64 = await buildCuisineOrderPdfBase64(trip, {
      userName,
      chefEmail: chefEmailLabel,
      amendment: mode === "amendment",
    });

    const dateRange = cuisineDateRangeLabel(trip.data);
    const selectedDays = CUISINE_DAYS.filter((d) => details.daysSelection?.[d.key]);
    const selectedDayNames = selectedDays.map((d) => d.label).join(", ");
    const ccRecipients = [...new Set([userEmail, organizerEmail, trip.ownerEmail].filter(Boolean))];

    const smtp = await getTenantSmtpConfig();
    if (!smtp) {
      return NextResponse.json({ error: "SMTP non configuré" }, { status: 503 });
    }
    const transporter = await createTenantTransporter();
    if (!transporter) {
      return NextResponse.json({ error: "SMTP non configuré" }, { status: 503 });
    }

    const subject =
      mode === "amendment"
        ? `ANNULE ET REMPLACE — Bon de commande cuisine — ${trip.data.title}`
        : `Bon de commande cuisine — ${userName} — ${trip.data.title}`;

    const text =
      mode === "amendment"
        ? [
            "Bonjour,",
            "",
            "Nous nous excusons de ce nouveau message.",
            `Suite à un changement sur le dossier de sortie « ${trip.data.title} » (${dateRange}), veuillez considérer le bon de commande ci-joint comme ANNULE ET REMPLACE toute commande précédemment transmise pour ce projet.`,
            "",
            "Il s'agit de la dernière commande en date pour cette sortie.",
            "",
            `Organisateur : ${userName}`,
            `Effectif actuel : ${trip.data.nbEleves || 0} élève(s), ${trip.data.nbAccompagnateurs || 0} accompagnateur(s)`,
            `Jour(s) de sortie : ${selectedDayNames || "—"}`,
            `Lieu de récupération : ${details.deliveryPlace || "—"} à ${details.deliveryTime || "—"}`,
            "",
            "Merci de votre compréhension.",
            "",
            "Cordialement,",
            userName,
          ].join("\n")
        : [
            "Bonjour,",
            "",
            `Veuillez trouver ci-joint le bon de commande de restauration pour le projet "${trip.data.title}" (${dateRange}).`,
            "",
            `Organisateur : ${userName}`,
            `Jour(s) de sortie : ${selectedDayNames || "—"}`,
            `Lieu de récupération : ${details.deliveryPlace || "—"} à ${details.deliveryTime || "—"}`,
            "",
            "Cordialement,",
            userName,
          ].join("\n");

    await transporter.sendMail({
      from: `"Gestion Sorties La Providence" <${smtp.user}>`,
      to: chefEmails.join(", "),
      cc: ccRecipients.length > 0 ? ccRecipients.join(", ") : undefined,
      subject,
      text,
      attachments: [
        {
          filename: `Commande_Cuisine_${trip.id || "sortie"}.pdf`,
          content: pdfBase64,
          encoding: "base64",
        },
      ],
    });

    const now = new Date().toISOString();
    const snapshot = buildCuisineSnapshot(trip);
    const previousSnapshot = trip.data.cuisineOrderSnapshot;
    const amendmentEntry =
      mode === "amendment"
        ? {
            sentAt: now,
            previousSnapshot: previousSnapshot || null,
            newSnapshot: snapshot,
            actor: user?.fullName || userName,
          }
        : null;

    const updatedTrip: TripRecord = {
      ...trip,
      data: {
        ...trip.data,
        cuisineOrderSentAt: now,
        cuisineOrderSnapshot: snapshot,
        cuisineAmendments: amendmentEntry
          ? [...(Array.isArray(trip.data.cuisineAmendments) ? trip.data.cuisineAmendments : []), amendmentEntry]
          : trip.data.cuisineAmendments,
      },
      history: [
        ...(Array.isArray(trip.history) ? trip.history : []),
        {
          date: now,
          user: user?.fullName || userName,
          action:
            mode === "amendment"
              ? "Commande cuisine renvoyée (annule et remplace)"
              : "Commande cuisine envoyée au chef",
        },
      ],
    };

    if (tripId) {
      await putJson(`travels/${tripId}.json`, updatedTrip);
      const indexHit = await getJson<TripRecord[]>("travels/index.json");
      const index = Array.isArray(indexHit?.data) ? indexHit.data : [];
      const nextIndex = index.map((t) => (t.id === tripId ? { ...t, ...updatedTrip } : t));
      await putJson("travels/index.json", nextIndex);
    }

    return NextResponse.json({ success: true, trip: updatedTrip, mode });
  } catch (error) {
    console.error("Erreur envoi mail cuisine:", error);
    return NextResponse.json({ error: "Erreur lors de l'envoi du mail" }, { status: 500 });
  }
}
