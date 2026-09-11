import { NextResponse } from "next/server";
import { loadAppConfig } from "@/app/lib/app-config";
import { resolveTravelsCuisineListePaniersEmails } from "@/app/lib/app-config-schemas";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { getJson, putJson } from "@/app/lib/s3-storage";
import {
  buildPanierRepasListCsv,
  clampPanierRepasAssignments,
  countPanierRepasAssigned,
} from "@/app/lib/travels-eleves-list";
import { getTotalMeals } from "@/app/lib/travels-cuisine-form";
import { cuisineDateRangeLabel } from "@/app/lib/travels-cuisine-shared";
import { assertTravelsTripAccess } from "@/app/lib/travels-rbac-server";
import type { TravelsParticipantEleve, TravelsTrip } from "@/app/lib/travels-types";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
  sendMailWithTimeout,
} from "@/app/lib/tenant-mail";

/**
 * Envoie la liste nominative des élèves avec panier repas
 * aux destinataires configurés (notifications.travelsCuisineListePaniers).
 */
export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  try {
    const body = await req.json();
    const tripId = String(body.tripId || "");
    if (!tripId) return NextResponse.json({ error: "tripId requis" }, { status: 400 });

    const hit = await getJson<TravelsTrip>(`travels/${tripId}.json`);
    const trip = hit?.data;
    if (!trip?.data) return NextResponse.json({ error: "Dossier introuvable" }, { status: 404 });

    const access = await assertTravelsTripAccess(trip, { requireOwnerOrDirection: true });
    if (!access.ok) return NextResponse.json({ error: access.error }, { status: access.status });

    const details = trip.data.piqueNiqueDetails as
      | { active?: boolean; deliveryPlace?: string; deliveryTime?: string }
      | undefined;
    if (!details?.active) {
      return NextResponse.json(
        { error: "Aucune commande cuisine active sur ce dossier." },
        { status: 400 },
      );
    }

    const mealsOrdered = getTotalMeals(
      trip.data.piqueNiqueDetails as Parameters<typeof getTotalMeals>[0],
    );
    if (mealsOrdered <= 0) {
      return NextResponse.json(
        { error: "Aucun panier repas n’a été commandé (picnicTotal = 0)." },
        { status: 400 },
      );
    }

    let participants: TravelsParticipantEleve[] = Array.isArray(body.participantEleves)
      ? body.participantEleves
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
      return NextResponse.json(
        { error: "Sélectionnez au moins un élève avec panier repas." },
        { status: 400 },
      );
    }

    const config = await loadAppConfig();
    const toEmails = resolveTravelsCuisineListePaniersEmails(config.notifications);
    if (toEmails.length === 0) {
      return NextResponse.json(
        {
          error:
            "Aucun destinataire configuré. Renseignez « Liste paniers repas » ou « Cuisine » dans Paramètres → Notifications.",
        },
        { status: 400 },
      );
    }

    const smtp = await getTenantSmtpConfig();
    const transporter = smtp ? await createTenantTransporter() : null;
    if (!smtp || !transporter) {
      return NextResponse.json({ error: "SMTP non configuré" }, { status: 503 });
    }

    const user = await safeCurrentUser();
    const userName =
      user?.fullName || access.user.fullName || trip.ownerName || "Organisateur";
    const userEmail =
      user?.primaryEmailAddress?.emailAddress ||
      access.user.primaryEmailAddress?.emailAddress ||
      trip.ownerEmail;
    const dateRange = cuisineDateRangeLabel(trip.data);
    const title = String(trip.data.title || "Sortie");
    const csv = buildPanierRepasListCsv(participants);
    const now = new Date().toISOString();

    const lines = withPanier
      .slice()
      .sort((a, b) =>
        `${a.nom} ${a.prenom}`.localeCompare(`${b.nom} ${b.prenom}`, "fr", {
          sensitivity: "base",
        }),
      )
      .map(
        (p, i) =>
          `${i + 1}. ${p.nom} ${p.prenom}${p.classe ? ` (${p.classe})` : ""}`,
      );

    const subject = `Liste paniers repas — ${title} — ${withPanier.length}/${mealsOrdered}`;
    const text = [
      "Bonjour,",
      "",
      `Voici la liste nominative des élèves avec panier repas pour « ${title} » (${dateRange}).`,
      "",
      `Organisateur : ${userName}`,
      `Paniers attribués : ${withPanier.length} / ${mealsOrdered} commandés`,
      `Lieu de récupération : ${details.deliveryPlace || "—"} à ${details.deliveryTime || "—"}`,
      "",
      "Élèves :",
      ...lines,
      "",
      "La liste CSV est jointe.",
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
      attachments: [
        {
          filename: `Liste_paniers_${tripId}.csv`,
          content: Buffer.from(`\uFEFF${csv}`, "utf8"),
          contentType: "text/csv; charset=utf-8",
        },
      ],
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
          action: `Liste paniers repas envoyée (${withPanier.length}/${mealsOrdered})`,
        },
      ],
    };

    await putJson(`travels/${tripId}.json`, updatedTrip);

    return NextResponse.json({
      ok: true,
      trip: updatedTrip,
      sentTo: toEmails,
      count: withPanier.length,
      mealsOrdered,
      remaining: Math.max(0, mealsOrdered - withPanier.length),
    });
  } catch (e) {
    console.error("[send-panier-repas-list]", e);
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Envoi impossible" },
      { status: 500 },
    );
  }
}
