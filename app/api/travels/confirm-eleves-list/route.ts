import { NextResponse } from "next/server";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { requireAuth } from "@/app/lib/intranet-auth";
import { getJson, putJson } from "@/app/lib/s3-storage";
import {
  applyParticipantElevesToTripData,
  buildElevesListCsv,
} from "@/app/lib/travels-eleves-list";
import { assertTravelsTripAccess } from "@/app/lib/travels-rbac-server";
import { complexNeedsBus } from "@/app/lib/travels-trip-helpers";
import type { TravelsParticipantEleve, TravelsTrip } from "@/app/lib/travels-types";
import { orderEmailForQuote } from "@/app/lib/travels-transport-shared";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
} from "@/app/lib/tenant-mail";
import { buildTransportReplyTo } from "@/app/lib/travel-email-routing";

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

    if (["SEANCE_ANNULEE", "REJETE", "ANNULE"].includes(String(trip.status))) {
      return NextResponse.json({ error: "Dossier clos — confirmation impossible." }, { status: 400 });
    }

    let participants: TravelsParticipantEleve[] = Array.isArray(body.participantEleves)
      ? body.participantEleves
      : Array.isArray(trip.data.participantEleves)
        ? trip.data.participantEleves
        : [];

    participants = participants
      .filter((p) => p && typeof p.ine === "string" && p.ine.trim())
      .map((p) => ({
        ine: String(p.ine).trim(),
        nom: String(p.nom || "").trim(),
        prenom: String(p.prenom || "").trim(),
        classe: p.classe ? String(p.classe).trim() : undefined,
        droitImageOk: p.droitImageOk !== false,
      }));

    if (participants.length === 0) {
      return NextResponse.json({ error: "Au moins un élève est requis pour confirmer la liste." }, { status: 400 });
    }

    const user = await safeCurrentUser();
    const userName = user?.fullName || access.user.fullName || "Utilisateur";
    const now = new Date().toISOString();
    const needsBus = complexNeedsBus(trip);
    const actorEmail =
      access.user.primaryEmailAddress?.emailAddress ||
      access.user.emailAddresses?.[0]?.emailAddress;

    let data = applyParticipantElevesToTripData(trip.data, participants);
    data = {
      ...data,
      listeElevesStatus: "confirmed",
      listeElevesConfirmedAt: now,
      listeElevesConfirmedBy: {
        userId: access.user.id,
        email: actorEmail,
        name: userName,
      },
    };

    let sentTo: string[] = [];
    let transportSkippedReason: string | null = null;

    if (needsBus) {
      const selected = data.selectedBusQuote as Record<string, unknown> | undefined;
      const selectedEmail = orderEmailForQuote(
        selected as { extractedContactEmail?: string; providerEmail?: string; email?: string } | null,
      );
      const recipients: Array<{ name: string; email: string }> = [];

      if (selected && selectedEmail) {
        recipients.push({
          name: String(selected.providerName || "Transporteur"),
          email: selectedEmail,
        });
      } else if (selectedEmail) {
        recipients.push({ name: "Transporteur", email: selectedEmail });
      }

      if (recipients.length === 0) {
        transportSkippedReason =
          "Aucun transporteur retenu avec e-mail — liste confirmée sans envoi. Sélectionnez un devis bus puis reconduirez la confirmation si besoin.";
      } else {
        const smtp = await getTenantSmtpConfig();
        const transporter = smtp ? await createTenantTransporter() : null;
        if (!smtp || !transporter) {
          return NextResponse.json({ error: "SMTP non configuré — impossible d'envoyer au transporteur." }, { status: 503 });
        }

        const csv = buildElevesListCsv(participants);
        const destSlug = String(data.destination || data.title || "sortie").replace(/\s+/g, "_");
        const replyTo = await buildTransportReplyTo();
        const dates =
          data.startDate && data.endDate
            ? `Du ${data.startDate} au ${data.endDate}`
            : data.date || data.startDate || "—";

        for (const r of recipients) {
          await transporter.sendMail({
            from: `"Plateforme Voyages" <${smtp.user}>`,
            to: r.email,
            ...(replyTo ? { replyTo } : {}),
            subject: `Liste des élèves — ${String(data.title || data.destination || tripId)}`,
            text: [
              `Bonjour ${r.name},`,
              "",
              `Veuillez trouver ci-joint la liste nominative des élèves pour la sortie :`,
              `Titre : ${data.title || "—"}`,
              `Destination : ${data.destination || "—"}`,
              `Dates : ${dates}`,
              `Effectif élèves : ${participants.length}`,
              `Accompagnateurs : ${data.nbAccompagnateurs || "—"}`,
              "",
              "Cordialement,",
              "Plateforme Voyages",
            ].join("\n"),
            attachments: [
              {
                filename: `Liste_eleves_${destSlug}.csv`,
                content: Buffer.from(`\uFEFF${csv}`, "utf8"),
                contentType: "text/csv; charset=utf-8",
              },
            ],
          });
          sentTo.push(r.email);
        }
        data.listeEnvoyeeTransporteurAt = now;
      }
    }

    const historyAction = needsBus
      ? sentTo.length > 0
        ? `Liste élèves confirmée (${participants.length}) et envoyée au transporteur`
        : `Liste élèves confirmée (${participants.length})${transportSkippedReason ? ` — ${transportSkippedReason}` : ""}`
      : `Liste élèves confirmée (${participants.length})`;

    const updatedTrip: TravelsTrip = {
      ...trip,
      updatedAt: now,
      data,
      history: [
        ...(Array.isArray(trip.history) ? trip.history : []),
        {
          date: now,
          user: userName,
          action: historyAction,
          note: sentTo.length ? `Destinataires : ${sentTo.join(", ")}` : undefined,
        },
      ],
    };

    await putJson(`travels/${tripId}.json`, updatedTrip);
    const indexHit = await getJson<TravelsTrip[]>("travels/index.json");
    const index = Array.isArray(indexHit?.data) ? indexHit.data : [];
    await putJson(
      "travels/index.json",
      index.map((t) => (t.id === tripId ? { ...t, ...updatedTrip, data: { ...t.data, ...updatedTrip.data } } : t)),
    );

    return NextResponse.json({
      success: true,
      trip: updatedTrip,
      sentTo,
      transportSkippedReason,
    });
  } catch (e) {
    console.error("[confirm-eleves-list]", e);
    return NextResponse.json({ error: "Confirmation impossible" }, { status: 500 });
  }
}
