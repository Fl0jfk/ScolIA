/**
 * Envoi du bon de commande cuisine au chef (logique partagée API + confirmation liste élèves).
 */

import { loadAppConfig } from "@/app/lib/app-config";
import { resolveTravelsCuisineEmails } from "@/app/lib/app-config-schemas";
import { getJson, putJson } from "@/app/lib/s3-storage";
import { isListeElevesConfirmed } from "@/app/lib/travels-eleves-list";
import { buildCuisineOrderPdfBase64 } from "@/app/lib/travels-cuisine-pdf";
import {
  CUISINE_DAYS,
  cuisineDateRangeLabel,
  type CuisineTripPayload,
} from "@/app/lib/travels-cuisine-shared";
import type { TravelsTrip, TravelsTripData } from "@/app/lib/travels-types";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
} from "@/app/lib/tenant-mail";

type TripData = CuisineTripPayload["data"] & Record<string, unknown>;

export type CuisineTripRecord = Omit<CuisineTripPayload, "data"> & {
  data: TripData;
  status?: string;
  ownerName?: string;
  ownerEmail?: string;
  history?: Array<{ date?: string; action?: string; user?: string; note?: string }>;
};

function buildCuisineSnapshot(trip: CuisineTripRecord) {
  const d = trip.data;
  return {
    sentAt: new Date().toISOString(),
    nbEleves: Number(d.nbEleves) || 0,
    nbAccompagnateurs: Number(d.nbAccompagnateurs) || 0,
    piqueNiqueDetails: d.piqueNiqueDetails,
  };
}

export type SendCuisineOrderParams = {
  trip: CuisineTripRecord;
  tripId: string;
  mode?: "initial" | "amendment";
  userName?: string;
  userEmail?: string | null;
  organizerEmail?: string | null;
  /** Si false, n’exige pas la liste confirmée (réservé aux avenants déjà envoyés). */
  requireListeElevesConfirmed?: boolean;
  /** Persiste le trip mis à jour (défaut true). */
  persist?: boolean;
};

export type SendCuisineOrderResult =
  | { ok: true; trip: CuisineTripRecord; mode: "initial" | "amendment" }
  | { ok: false; error: string; status: number };

/**
 * Envoie le PDF + mail au chef, met à jour cuisineOrderSentAt / historique.
 */
export async function sendCuisineOrderForTrip(
  params: SendCuisineOrderParams,
): Promise<SendCuisineOrderResult> {
  const mode = params.mode === "amendment" ? "amendment" : "initial";
  const trip = params.trip;
  const tripId = String(params.tripId || "").trim();
  const requireListe =
    params.requireListeElevesConfirmed !== false && mode === "initial";

  const details = trip.data.piqueNiqueDetails;
  if (!details?.active) {
    return { ok: false, error: "Aucune commande cuisine à envoyer", status: 400 };
  }

  if (mode === "amendment" && !trip.data.cuisineOrderSentAt) {
    return {
      ok: false,
      error: "Aucune commande cuisine précédente — utilisez l'envoi initial.",
      status: 400,
    };
  }

  if (requireListe && !isListeElevesConfirmed(trip.data as TravelsTripData)) {
    return {
      ok: false,
      error:
        "La liste des élèves doit être confirmée (onglet Élèves) avant l’envoi de la commande cuisine au chef.",
      status: 400,
    };
  }

  const userName = params.userName || trip.ownerName || "La Providence";
  const userEmail = params.userEmail || undefined;
  const organizerEmail = params.organizerEmail || trip.ownerEmail;

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
    return { ok: false, error: "SMTP non configuré", status: 503 };
  }
  const transporter = await createTenantTransporter();
  if (!transporter) {
    return { ok: false, error: "SMTP non configuré", status: 503 };
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
        filename: `Commande_Cuisine_${trip.id || tripId || "sortie"}.pdf`,
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
          actor: userName,
        }
      : null;

  const updatedTrip: CuisineTripRecord = {
    ...trip,
    data: {
      ...trip.data,
      cuisineOrderSentAt: now,
      cuisineOrderSnapshot: snapshot,
      cuisineAmendments: amendmentEntry
        ? [
            ...(Array.isArray(trip.data.cuisineAmendments) ? trip.data.cuisineAmendments : []),
            amendmentEntry,
          ]
        : trip.data.cuisineAmendments,
    },
    history: [
      ...(Array.isArray(trip.history) ? trip.history : []),
      {
        date: now,
        user: userName,
        action:
          mode === "amendment"
            ? "Commande cuisine renvoyée (annule et remplace)"
            : "Commande cuisine envoyée au chef",
      },
    ],
  };

  if (params.persist !== false && tripId) {
    await putJson(`travels/${tripId}.json`, updatedTrip);
    const indexHit = await getJson<CuisineTripRecord[]>("travels/index.json");
    const index = Array.isArray(indexHit?.data) ? indexHit.data : [];
    const nextIndex = index.map((t) => (t.id === tripId ? { ...t, ...updatedTrip } : t));
    await putJson("travels/index.json", nextIndex);
  }

  return { ok: true, trip: updatedTrip, mode };
}

/** Recharge le trip S3 puis envoie la cuisine (utile après une autre mutation). */
export async function sendCuisineOrderByTripId(
  tripId: string,
  opts: Omit<SendCuisineOrderParams, "trip" | "tripId"> & {
    tripOverride?: TravelsTrip;
  },
): Promise<SendCuisineOrderResult> {
  let trip: CuisineTripRecord | null = (opts.tripOverride as CuisineTripRecord | undefined) || null;
  if (!trip) {
    const hit = await getJson<CuisineTripRecord>(`travels/${tripId}.json`);
    trip = hit?.data || null;
  }
  if (!trip?.data) {
    return { ok: false, error: "Dossier introuvable", status: 404 };
  }
  return sendCuisineOrderForTrip({ ...opts, trip, tripId });
}
