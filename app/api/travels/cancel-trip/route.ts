import { NextResponse } from "next/server";
import { loadAppConfig } from "@/app/lib/app-config";
import { resolveTravelsCuisineEmails } from "@/app/lib/app-config-schemas";
import { requireAuth } from "@/app/lib/intranet-auth";
import { getJson, putJson } from "@/app/lib/s3-storage";
import { buildTravelsMailPreviewFromConfig } from "@/app/lib/travels-mail-preview";
import { assertTravelsTripAccess } from "@/app/lib/travels-rbac-server";
import { tripDateRangeLabel } from "@/app/lib/travels-trip-helpers";
import type { TravelsTrip } from "@/app/lib/travels-types";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
} from "@/app/lib/tenant-mail";
import { mergeTransportMailCc } from "@/app/lib/travel-email-routing";

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  try {
    const body = await req.json();
    const tripId = String(body.tripId || "");
    const reason = String(body.reason || "").trim();
    const notifyTransport = body.notifyTransport !== false;
    const notifyCuisine = body.notifyCuisine !== false;

    if (!tripId) return NextResponse.json({ error: "tripId requis" }, { status: 400 });

    const hit = await getJson<TravelsTrip>(`travels/${tripId}.json`);
    const trip = hit?.data;
    if (!trip) return NextResponse.json({ error: "Dossier introuvable" }, { status: 404 });

    const access = await assertTravelsTripAccess(trip, { requireOwnerOrDirection: true });
    if (!access.ok) return NextResponse.json({ error: access.error }, { status: access.status });

    if (["ANNULE", "SEANCE_ANNULEE"].includes(trip.status)) {
      return NextResponse.json({ error: "Ce dossier est déjà annulé." }, { status: 400 });
    }

    const config = await loadAppConfig();
    const chefEmails = resolveTravelsCuisineEmails(config.notifications, "");

    const now = new Date().toISOString();
    const actor = access.user.fullName || "Administration";
    const smtp = await getTenantSmtpConfig();
    if (!smtp) {
      return NextResponse.json({ error: "SMTP non configuré" }, { status: 503 });
    }
    const transporter = await createTenantTransporter();
    if (!transporter) {
      return NextResponse.json({ error: "SMTP non configuré" }, { status: 503 });
    }

    const emailsSent: string[] = [];

    if (notifyTransport && trip.data.needsBus) {
      const preview = await buildTravelsMailPreviewFromConfig(trip, "cancel_trip_transport", { userName: actor });
      for (const to of preview.to) {
        await transporter.sendMail({
          from: `"Plateforme Voyages" <${smtp.user}>`,
          to,
          cc: mergeTransportMailCc(),
          subject: preview.subject,
          text: reason ? `${preview.text}\n\nMotif : ${reason}` : preview.text,
        });
        emailsSent.push(to);
      }
    }

    if (notifyCuisine && trip.data.cuisineOrderSentAt && chefEmails.length > 0) {
      const preview = await buildTravelsMailPreviewFromConfig(trip, "cancel_trip_cuisine", {
        userName: actor,
        chefEmails,
      });
      await transporter.sendMail({
        from: `"Gestion Sorties La Providence" <${smtp.user}>`,
        to: chefEmails.join(", "),
        cc: trip.ownerEmail || undefined,
        subject: preview.subject,
        text: reason ? `${preview.text}\n\nMotif : ${reason}` : preview.text,
      });
      emailsSent.push(...chefEmails);
    }

    const updatedTrip: TravelsTrip = {
      ...trip,
      status: "ANNULE",
      data: {
        ...trip.data,
        cancelledAt: now,
        cancelReason: reason || undefined,
      },
      history: [
        ...(trip.history || []),
        {
          date: now,
          user: actor,
          action: "ANNULE",
          note: reason || "Sortie annulée",
        },
      ],
    };

    await putJson(`travels/${tripId}.json`, updatedTrip);
    const indexHit = await getJson<TravelsTrip[]>("travels/index.json");
    const index = Array.isArray(indexHit?.data) ? indexHit.data : [];
    await putJson(
      "travels/index.json",
      index.map((t) => (t.id === tripId ? { ...t, status: "ANNULE", data: { ...t.data, ...updatedTrip.data } } : t)),
    );

    return NextResponse.json({
      success: true,
      trip: updatedTrip,
      emailsSent,
      dateRange: tripDateRangeLabel(trip.data),
    });
  } catch (e) {
    console.error("[cancel-trip]", e);
    return NextResponse.json({ error: "Annulation impossible" }, { status: 500 });
  }
}
