import { getTransportProviders, type TransportProvider } from "@/app/lib/transport-providers";
import { CUISINE_DAYS, cuisineDateRangeLabel } from "@/app/lib/travels-cuisine-shared";
import { orderEmailForQuote } from "@/app/lib/travels-transport-shared";
import { tripDateRangeLabel } from "@/app/lib/travels-trip-helpers";
import type { TravelsTrip } from "@/app/lib/travels-types";

export type MailPreviewType =
  | "transport_amendment"
  | "transport_initial"
  | "cuisine_initial"
  | "cuisine_amendment"
  | "cancel_trip_transport"
  | "cancel_trip_cuisine";

type MailPreviewResult = {
  type: MailPreviewType;
  to: string[];
  cc?: string[];
  subject: string;
  text: string;
  html?: string;
  attachments: { filename: string; description: string }[];
};

function buildTravelsMailPreview(
  trip: TravelsTrip,
  type: MailPreviewType,
  opts?: { userName?: string; chefEmail?: string; transportProviders?: TransportProvider[] },
): MailPreviewResult {
  const userName = opts?.userName || trip.ownerName || "Administration";
  const transportProviders = opts?.transportProviders ?? [];
  const data = trip.data;
  const dest = String(data.destination || "voyage");
  const dateRange = tripDateRangeLabel(data);
  const details = data.piqueNiqueDetails as {
    active?: boolean;
    deliveryPlace?: string;
    deliveryTime?: string;
    daysSelection?: Record<string, boolean>;
  } | undefined;

  if (type === "transport_amendment") {
    const selected = data.selectedBusQuote;
    const selectedEmail = orderEmailForQuote(selected);
    const recipients =
      selected && selectedEmail
        ? [{ name: String(selected.providerName || "Transporteur"), email: selectedEmail }]
        : transportProviders.map((p) => ({ name: p.name, email: p.email }));

    const snap = data.transportQuoteSnapshot;
    const prevTotal = snap ? Number(snap.nbEleves) + Number(snap.nbAccompagnateurs || 0) : null;
    const newTotal = Number(data.nbEleves) + Number(data.nbAccompagnateurs || 0);

    return {
      type,
      to: recipients.map((r) => r.email),
      subject: `🚗 AVENANT DEVIS (effectif) - ${dest.toUpperCase()} - Réf. ${trip.id}`,
      text: [
        `Bonjour,`,
        ``,
        `Le nombre de participants pour le transport vers ${dest} a évolué.`,
        prevTotal != null ? `Ancien effectif : ${prevTotal} personnes.` : "",
        `Nouvel effectif : ${newTotal} personnes.`,
        `Veuillez trouver ci-joint la demande de devis rectifié. Réf. ${trip.id}`,
        ``,
        `Cordialement,`,
        `L'administration`,
      ]
        .filter(Boolean)
        .join("\n"),
      attachments: [{ filename: `Avenant_Transport_${dest.replace(/\s+/g, "_")}.pdf`, description: "Demande avenant transport" }],
    };
  }

  if (type === "transport_initial") {
    return {
      type,
      to: transportProviders.map((p) => p.email),
      subject: `🚗 DEMANDE DE DEVIS - ${dest.toUpperCase()} - ${userName}`,
      text: `Demande de devis transport vers ${dest} (${dateRange}). Réf. ${trip.id}`,
      attachments: [{ filename: `Demande_Transport_${dest.replace(/\s+/g, "_")}.pdf`, description: "Demande initiale" }],
    };
  }

  if (type === "cuisine_initial" || type === "cuisine_amendment") {
    const chefEmail = opts?.chefEmail || "chef@restauration";
    const selectedDays = CUISINE_DAYS.filter((d) => details?.daysSelection?.[d.key]).map((d) => d.label);
    const isAmendment = type === "cuisine_amendment";

    return {
      type,
      to: [chefEmail],
      cc: [trip.ownerEmail].filter(Boolean) as string[],
      subject: isAmendment
        ? `ANNULE ET REMPLACE — Bon de commande cuisine — ${data.title}`
        : `Bon de commande cuisine — ${userName} — ${data.title}`,
      text: isAmendment
        ? [
            "Bonjour,",
            "",
            "Nous nous excusons de ce nouveau message.",
            `Suite à un changement sur le dossier « ${data.title} » (${dateRange}), veuillez considérer le bon ci-joint comme ANNULE ET REMPLACE toute commande précédente.`,
            "Il s'agit de la dernière commande en date pour cette sortie.",
            "",
            `Organisateur : ${userName}`,
            `Jour(s) : ${selectedDays.join(", ") || "—"}`,
            "",
            "Cordialement,",
            userName,
          ].join("\n")
        : [
            `Bon de commande restauration pour « ${data.title} » (${cuisineDateRangeLabel(data)}).`,
            `Organisateur : ${userName}`,
            `Jours : ${selectedDays.join(", ")}`,
          ].join("\n"),
      attachments: [{ filename: `Commande_Cuisine_${trip.id}.pdf`, description: "Bon de commande cuisine" }],
    };
  }

  if (type === "cancel_trip_transport") {
    const selected = data.selectedBusQuote;
    const email = orderEmailForQuote(selected);
    return {
      type,
      to: email ? [email] : transportProviders.map((p) => p.email),
      subject: `ANNULATION transport — ${dest} — Réf. ${trip.id}`,
      text: [
        "Bonjour,",
        "",
        `Nous vous informons de l'annulation du transport scolaire prévu pour ${dest} (${dateRange}).`,
        "Merci de ne pas maintenir votre réservation pour ce dossier.",
        "",
        "Cordialement,",
        "L'administration",
      ].join("\n"),
      attachments: [],
    };
  }

  return {
    type: "cancel_trip_cuisine",
    to: [opts?.chefEmail || "chef@restauration"],
    subject: `ANNULATION commande cuisine — ${data.title}`,
    text: [
      "Bonjour,",
      "",
      `Nous vous informons de l'annulation de la commande restauration pour la sortie « ${data.title} » (${dateRange}).`,
      "Merci d'ignorer toute commande précédemment transmise pour ce projet.",
      "",
      "Cordialement,",
      "L'administration",
    ].join("\n"),
    attachments: [],
  };
}

export async function buildTravelsMailPreviewFromConfig(
  trip: TravelsTrip,
  type: MailPreviewType,
  opts?: { userName?: string; chefEmail?: string },
): Promise<MailPreviewResult> {
  const transportProviders = await getTransportProviders();
  return buildTravelsMailPreview(trip, type, { ...opts, transportProviders });
}
