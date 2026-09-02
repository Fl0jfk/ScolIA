import { resolveSession, safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";

import { getTransportProviders } from "@/app/lib/transport-providers";
import { requireAuth } from "@/app/lib/intranet-auth";
import { getJson, putJson } from "@/app/lib/s3-storage";
import { loadBusProgramAttachments } from "@/app/lib/travels-bus-program";
import { assertTravelsTripAccess } from "@/app/lib/travels-rbac-server";
import { buildTransportQuotePdf, orderEmailForQuote } from "@/app/lib/travels-transport-quote-pdf";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
} from "@/app/lib/tenant-mail";
import type { TransportAmendment, TravelsTrip } from "@/app/lib/travels-types";
import { buildTransportReplyTo, mergeTransportMailCc } from "@/app/lib/travel-email-routing";

type TripRecord = TravelsTrip;

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const session = await resolveSession();
  const userId = session?.userId;
  if (!userId) return NextResponse.json({ error: "Non autorisé" }, { status: 401 });

  try {
    const body = await req.json();
    const tripId = String(body.tripId || "");
    if (!tripId) return NextResponse.json({ error: "tripId requis" }, { status: 400 });

    const hit = await getJson<TripRecord>(`travels/${tripId}.json`);
    const trip = hit?.data;
    if (!trip?.data) return NextResponse.json({ error: "Dossier introuvable" }, { status: 404 });

    const access = await assertTravelsTripAccess(trip, { requireOwnerOrDirection: true });
    if (!access.ok) return NextResponse.json({ error: access.error }, { status: access.status });

    const data = trip.data;
    if (!data.needsBus) {
      return NextResponse.json({ error: "Ce voyage n'a pas de transport bus." }, { status: 400 });
    }

    const user = await safeCurrentUser();
    const userName = body.userName || user?.fullName || "La Providence";

    const currentEffectif = {
      nbEleves: Number(data.nbEleves) || 0,
      nbAccompagnateurs: Number(data.nbAccompagnateurs) || 0,
    };
    const previousSnapshot = data.transportQuoteSnapshot as
      | { nbEleves: number; nbAccompagnateurs: number }
      | undefined;
    const previousEffectif = previousSnapshot
      ? {
          nbEleves: Number(previousSnapshot.nbEleves) || 0,
          nbAccompagnateurs: Number(previousSnapshot.nbAccompagnateurs) || 0,
        }
      : undefined;

    const selected = data.selectedBusQuote as Record<string, any> | undefined;
    const selectedEmail = orderEmailForQuote(selected);
    const recipients: Array<{ name: string; email: string }> = [];

    if (selected && selectedEmail) {
      recipients.push({
        name: String(selected.providerName || "Transporteur"),
        email: selectedEmail,
      });
    } else {
      const providers = await getTransportProviders();
      for (const p of providers) {
        recipients.push({ name: p.name, email: p.email });
      }
    }

    const extraAttachments = await loadBusProgramAttachments(data);
    const smtp = await getTenantSmtpConfig();
    if (!smtp) {
      return NextResponse.json({ error: "SMTP non configuré" }, { status: 503 });
    }
    const transporter = await createTenantTransporter();
    if (!transporter) {
      return NextResponse.json({ error: "SMTP non configuré" }, { status: 503 });
    }

    const destSlug = String(data.destination || "voyage").replace(/\s+/g, "_");
    const replyTo = await buildTransportReplyTo();

    for (const r of recipients) {
      const personalPdf = await buildTransportQuotePdf({
        tripId,
        data,
        userName,
        transporteurName: r.name,
        mode: "amendment",
        previousEffectif,
      });
      const personalAttachments = [
        {
          filename: `Avenant_Transport_${destSlug}.pdf`,
          content: personalPdf,
          contentType: "application/pdf",
        },
        ...extraAttachments,
      ];

      await transporter.sendMail({
        from: `"Plateforme Voyages" <${smtp.user}>`,
        to: r.email,
        ...(replyTo ? { replyTo } : {}),
        cc: mergeTransportMailCc(),
        subject: `AVENANT DEVIS (effectif) - ${String(data.destination).toUpperCase()}`,
        html: `
          <div style="font-family: sans-serif; line-height: 1.5; color: #334155;">
            <h2>Bonjour ${r.name},</h2>
            <p>Nous nous excusons de vous recontacter : le nombre de participants pour le transport vers <strong>${data.destination}</strong> a évolué.</p>
            ${
              previousEffectif
                ? `<p>Ancien effectif : <strong>${previousEffectif.nbEleves + previousEffectif.nbAccompagnateurs} personnes</strong> · Nouvel effectif : <strong>${currentEffectif.nbEleves + currentEffectif.nbAccompagnateurs} personnes</strong>.</p>`
                : `<p>Nouvel effectif : <strong>${currentEffectif.nbEleves + currentEffectif.nbAccompagnateurs} personnes</strong>.</p>`
            }
            <p>Veuillez trouver ci-joint notre demande de <strong>devis rectifié</strong>. Merci de nous adresser votre nouvelle proposition en PDF en répondant à cet e-mail.</p>
            <p>Cordialement,<br/>L'administration.</p>
          </div>
        `,
        attachments: personalAttachments,
      });
    }

    const now = new Date().toISOString();
    const amendmentEntry: TransportAmendment = {
      sentAt: now,
      previousEffectif: previousEffectif || null,
      newEffectif: currentEffectif,
      recipientMode: selected && selectedEmail ? "single" : "all",
      providerName: selected?.providerName || null,
      providerEmail: selectedEmail || null,
    };

    const hadSignedQuote = Boolean(data.signedQuoteUrl);
    let nextStatus = trip.status;
    if (hadSignedQuote && ["EN_ATTENTE_COMPTA", "EN_ATTENTE_DIR_FINAL", "VALIDE", "EN_ATTENTE_BUS_SIGNATURE"].includes(String(trip.status))) {
      nextStatus = "EN_ATTENTE_BUS_SIGNATURE";
    }

    const updatedTrip: TripRecord = {
      ...trip,
      status: nextStatus,
      data: {
        ...data,
        transportQuoteSnapshot: { ...currentEffectif, sentAt: now, type: "amendment" },
        transportDateSnapshot: {
          startDate: data.startDate || data.date,
          endDate: data.endDate,
          startTime: data.startTime,
          endTime: data.endTime,
          sentAt: now,
        },
        transportAmendments: [...(Array.isArray(data.transportAmendments) ? data.transportAmendments : []), amendmentEntry],
        pendingAmendedQuote: true,
        ...(hadSignedQuote ? { signedQuoteUrl: undefined, selectedBusQuote: undefined } : {}),
      },
      history: [
        ...(Array.isArray(trip.history) ? trip.history : []),
        {
          date: now,
          user: userName,
          action:
            selected && selectedEmail
              ? `Avenant devis transport envoyé à ${selected.providerName} (changement d'effectif)`
              : `Avenant devis transport envoyé à tous les transporteurs (changement d'effectif)`,
        },
      ],
    };

    await putJson(`travels/${tripId}.json`, updatedTrip);

    const indexHit = await getJson<TripRecord[]>("travels/index.json");
    const index = Array.isArray(indexHit?.data) ? indexHit.data : [];
    const nextIndex = index.map((t) => (t.id === tripId ? { ...t, ...updatedTrip } : t));
    await putJson("travels/index.json", nextIndex);

    return NextResponse.json({
      success: true,
      trip: updatedTrip,
      sentTo: recipients.map((r) => r.email),
      singleProvider: Boolean(selected && selectedEmail),
    });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : "Erreur inconnue";
    console.error("[send-transport-amendment]", msg);
    return NextResponse.json({ error: "Échec de l'envoi de l'avenant", details: msg }, { status: 500 });
  }
}
