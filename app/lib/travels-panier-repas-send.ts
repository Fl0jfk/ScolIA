/**
 * Envoi de la liste nominative « qui mange » (paniers repas)
 * aux destinataires Paramètres → Liste paniers repas.
 * Peut joindre le PDF de commande cuisine pour la collègue décompte.
 */

import { loadAppConfig } from "@/app/lib/app-config";
import { resolveTravelsCuisineListePaniersEmails } from "@/app/lib/app-config-schemas";
import { getJson, putJson } from "@/app/lib/s3-storage";
import {
  buildPanierRepasListCsv,
  clampPanierRepasAssignments,
  countPanierRepasAssigned,
  getCuisineMealsOrdered,
} from "@/app/lib/travels-eleves-list";
import { cuisineDateRangeLabel } from "@/app/lib/travels-cuisine-shared";
import { buildCuisineOrderPdfBase64 } from "@/app/lib/travels-cuisine-pdf";
import type { TravelsParticipantEleve, TravelsTrip } from "@/app/lib/travels-types";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
  sendMailWithTimeout,
} from "@/app/lib/tenant-mail";

export type SendPanierRepasListParams = {
  trip: TravelsTrip;
  tripId: string;
  participants?: TravelsParticipantEleve[];
  userName?: string;
  userEmail?: string | null;
  /** Joindre le PDF de commande cuisine (pour la collègue décompte repas). */
  attachCuisineOrderPdf?: boolean;
  persist?: boolean;
};

export type SendPanierRepasListResult =
  | {
      ok: true;
      trip: TravelsTrip;
      sentTo: string[];
      count: number;
      mealsOrdered: number;
    }
  | { ok: false; error: string; status: number };

export async function sendPanierRepasListForTrip(
  params: SendPanierRepasListParams,
): Promise<SendPanierRepasListResult> {
  const trip = params.trip;
  const tripId = String(params.tripId || trip.id || "").trim();
  const details = trip.data.piqueNiqueDetails as
    | { active?: boolean; deliveryPlace?: string; deliveryTime?: string }
    | undefined;

  if (!details?.active) {
    return { ok: false, error: "Aucune commande cuisine active sur ce dossier.", status: 400 };
  }

  const mealsOrdered = getCuisineMealsOrdered(trip.data);
  if (mealsOrdered <= 0) {
    return {
      ok: false,
      error: "Aucun panier repas n’a été commandé (total = 0).",
      status: 400,
    };
  }

  let participants: TravelsParticipantEleve[] = Array.isArray(params.participants)
    ? params.participants
    : Array.isArray(trip.data.participantEleves)
      ? trip.data.participantEleves
      : [];

  participants = clampPanierRepasAssignments(
    participants
      .filter((p) => p && typeof p.ine === "string" && p.ine.trim())
      .map((p) => ({
        ine: String(p.ine).trim(),
        nom: String(p.nom || "").trim(),
        prenom: String(p.prenom || "").trim(),
        classe: p.classe ? String(p.classe).trim() : undefined,
        droitImageOk: p.droitImageOk !== false,
        panierRepas: p.panierRepas === true,
      })),
    mealsOrdered,
  );

  const withPanier = participants.filter((p) => p.panierRepas === true);
  if (withPanier.length === 0) {
    return {
      ok: false,
      error: "Sélectionnez au moins un élève avec panier repas (« qui mange »).",
      status: 400,
    };
  }
  if (withPanier.length !== mealsOrdered) {
    return {
      ok: false,
      error: `Attribuez exactement ${mealsOrdered} panier(s) (« qui mange ») — actuellement ${withPanier.length}/${mealsOrdered}.`,
      status: 400,
    };
  }

  const config = await loadAppConfig();
  const toEmails = resolveTravelsCuisineListePaniersEmails(config.notifications);
  if (toEmails.length === 0) {
    return {
      ok: false,
      error:
        "Aucun destinataire configuré. Renseignez « Liste paniers repas » ou « Cuisine » dans Paramètres → Notifications.",
      status: 400,
    };
  }

  const smtp = await getTenantSmtpConfig();
  const transporter = smtp ? await createTenantTransporter() : null;
  if (!smtp || !transporter) {
    return { ok: false, error: "SMTP non configuré", status: 503 };
  }

  const userName = params.userName || trip.ownerName || "Organisateur";
  const userEmail = params.userEmail || trip.ownerEmail;
  const dateRange = cuisineDateRangeLabel(trip.data);
  const title = String(trip.data.title || "Sortie");
  const csv = buildPanierRepasListCsv(participants);
  const now = new Date().toISOString();

  const lines = withPanier
    .slice()
    .sort((a, b) =>
      `${a.nom} ${a.prenom}`.localeCompare(`${b.nom} ${b.prenom}`, "fr", { sensitivity: "base" }),
    )
    .map(
      (p, i) => `${i + 1}. ${p.nom} ${p.prenom}${p.classe ? ` (${p.classe})` : ""}`,
    );

  const attachments: Array<{
    filename: string;
    content: Buffer | string;
    contentType?: string;
    encoding?: string;
  }> = [
    {
      filename: `Liste_paniers_${tripId}.csv`,
      content: Buffer.from(`\uFEFF${csv}`, "utf8"),
      contentType: "text/csv; charset=utf-8",
    },
  ];

  let cuisinePdfAttached = false;
  if (params.attachCuisineOrderPdf !== false) {
    try {
      const pdfBase64 = await buildCuisineOrderPdfBase64(trip, {
        userName,
        chefEmail: toEmails.join(" / "),
        amendment: false,
      });
      attachments.push({
        filename: `Commande_Cuisine_${tripId}.pdf`,
        content: pdfBase64,
        encoding: "base64",
        contentType: "application/pdf",
      });
      cuisinePdfAttached = true;
    } catch (err) {
      console.warn("[send-panier-repas] PDF cuisine non joint:", err);
    }
  }

  const subject = `Qui mange + commande cuisine — ${title} — ${withPanier.length}/${mealsOrdered}`;
  const text = [
    "Bonjour,",
    "",
    `Voici la commande cuisine et la liste nominative des élèves qui mangent pour « ${title} » (${dateRange}).`,
    "",
    `Organisateur : ${userName}`,
    `Paniers / qui mange : ${withPanier.length} / ${mealsOrdered} commandés`,
    `Lieu de récupération : ${details.deliveryPlace || "—"} à ${details.deliveryTime || "—"}`,
    "",
    "Élèves qui mangent :",
    ...lines,
    "",
    cuisinePdfAttached
      ? "Pièces jointes : bon de commande cuisine (PDF) + liste nominative (CSV)."
      : "Pièce jointe : liste nominative (CSV).",
    "",
    "Cordialement,",
    userName,
  ].join("\n");

  const ccRecipients = [...new Set([userEmail, trip.ownerEmail].filter(Boolean))] as string[];

  await sendMailWithTimeout(transporter, {
    from: `"Gestion Sorties La Providence" <${smtp.user}>`,
    to: toEmails.join(", "),
    cc: ccRecipients.length > 0 ? ccRecipients.join(", ") : undefined,
    subject,
    text,
    attachments,
  });

  const snapshot = {
    sentAt: now,
    count: withPanier.length,
    mealsOrdered,
    eleves: withPanier.map((p) => ({
      ine: p.ine,
      nom: p.nom,
      prenom: p.prenom,
      ...(p.classe ? { classe: p.classe } : {}),
    })),
  };

  const updatedTrip: TravelsTrip = {
    ...trip,
    data: {
      ...trip.data,
      participantEleves: participants,
      panierRepasListSentAt: now,
      panierRepasListSnapshot: snapshot,
    },
    history: [
      ...(trip.history || []),
      {
        date: now,
        user: userName,
        action: `Liste « qui mange » envoyée à la collègue décompte (${withPanier.length}/${mealsOrdered})`,
      },
    ],
  };

  if (params.persist !== false && tripId) {
    await putJson(`travels/${tripId}.json`, updatedTrip);
    const indexHit = await getJson<TravelsTrip[]>("travels/index.json");
    const index = Array.isArray(indexHit?.data) ? indexHit.data : [];
    await putJson(
      "travels/index.json",
      index.map((t) =>
        t.id === tripId ? { ...t, ...updatedTrip, data: { ...t.data, ...updatedTrip.data } } : t,
      ),
    );
  }

  return {
    ok: true,
    trip: updatedTrip,
    sentTo: toEmails,
    count: countPanierRepasAssigned(participants),
    mealsOrdered,
  };
}
