import { NextResponse } from "next/server";

import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canSignTravelsDirectionForEtab } from "@/app/lib/establishments";
import { getJson, putJson } from "@/app/lib/s3-storage";
import { createTenantTransporter, getTenantSmtpConfig } from "@/app/lib/tenant-mail";
import { buildTransportReplyTo, mergeTransportMailCc } from "@/app/lib/travel-email-routing";
import { orderEmailForQuote } from "@/app/lib/travels-transport-shared";
import { isGlobalAdminUser } from "@/app/lib/travels-roles";
import {
  BUS_QUOTE_CANCEL_ALLOWED_STATUSES,
  busQuoteCancelWindow,
} from "@/app/lib/travels-bus-quote-cancel";
import type { TravelsTrip } from "@/app/lib/travels-types";

function esc(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

/**
 * Annule le devis de transport signé : mail d'annulation au transporteur, retour à l'étape
 * logistique pour en choisir un autre. Réservé à la direction (ou admin général),
 * jusqu'à 14 jours avant le départ.
 */
export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  try {
    const body = (await req.json()) as { tripId?: string; reason?: string };
    const tripId = String(body?.tripId || "").trim();
    const reason = String(body?.reason || "").trim().slice(0, 500);
    if (!tripId) return NextResponse.json({ error: "tripId requis." }, { status: 400 });

    const user = await safeCurrentUser();
    if (!user) return NextResponse.json({ error: "Non autorisé" }, { status: 401 });

    const hit = await getJson<TravelsTrip>(`travels/${tripId}.json`);
    const trip = hit?.data;
    if (!trip?.data) return NextResponse.json({ error: "Dossier introuvable." }, { status: 404 });
    const data = trip.data as TravelsTrip["data"] & Record<string, unknown>;

    const etab = typeof data.etablissement === "string" ? data.etablissement : null;
    if (!isGlobalAdminUser(user) && !(await canSignTravelsDirectionForEtab(user, etab))) {
      return NextResponse.json(
        { error: "Seule la direction de l'établissement concerné peut annuler un devis signé." },
        { status: 403 },
      );
    }

    if (!data.signedQuoteUrl) {
      return NextResponse.json({ error: "Aucun devis signé sur ce dossier." }, { status: 400 });
    }
    if (!(BUS_QUOTE_CANCEL_ALLOWED_STATUSES as readonly string[]).includes(String(trip.status))) {
      return NextResponse.json(
        { error: "Le statut du dossier ne permet pas d'annuler le devis signé." },
        { status: 400 },
      );
    }
    const win = busQuoteCancelWindow(
      String(data.startDate || data.date || ""),
    );
    if (!win.open) {
      return NextResponse.json({ error: win.reason }, { status: 403 });
    }

    const selected = (data.selectedBusQuote || null) as Record<string, unknown> | null;
    const providerName = String(selected?.providerName || "Transporteur");
    const providerEmail = selected ? orderEmailForQuote(selected as never) : null;
    if (!providerEmail) {
      return NextResponse.json(
        { error: "Adresse e-mail du transporteur introuvable : impossible d'envoyer l'annulation." },
        { status: 400 },
      );
    }

    const smtp = await getTenantSmtpConfig();
    const transporter = await createTenantTransporter();
    if (!smtp || !transporter) {
      return NextResponse.json({ error: "SMTP non configuré" }, { status: 503 });
    }
    const replyTo = await buildTransportReplyTo();
    const title = String(data.title || data.destination || "Sortie scolaire");
    const destination = String(data.destination || "");
    const startLabel = String(data.startDate || data.date || "");

    await transporter.sendMail({
      from: `"Gestion Voyages" <${smtp.user}>`,
      to: providerEmail,
      ...(replyTo ? { replyTo } : {}),
      cc: mergeTransportMailCc(),
      subject: `ANNULATION de commande : ${title}`,
      html: `
        <div style="font-family: sans-serif; line-height: 1.5; color: #333;">
          <p>Bonjour <strong>${esc(providerName)}</strong>,</p>
          <p>Nous sommes au regret de vous informer que nous <strong>annulons la commande</strong> (devis signé) relative au projet <strong>${esc(title)}</strong>${destination ? ` — ${esc(destination)}` : ""}${startLabel ? `, prévu le ${esc(startLabel)}` : ""}.</p>
          ${reason ? `<p>Motif : ${esc(reason)}</p>` : ""}
          <p>Merci de bien vouloir nous confirmer la bonne réception de cette annulation en répondant à cet e-mail.</p>
          <p>Cordialement,</p>
          <p><em>L'administration de l'établissement</em></p>
        </div>
      `,
    });

    const now = new Date().toISOString();
    const actor = user.fullName || user.firstName || "Direction";
    const cancelled = {
      providerName,
      providerEmail,
      signedQuoteUrl: data.signedQuoteUrl,
      signedQuoteS3Key: data.signedQuoteS3Key || null,
      cancelledAt: now,
      cancelledBy: actor,
      reason: reason || null,
    };
    const prevCancelled = Array.isArray(data.cancelledBusQuotes) ? data.cancelledBusQuotes : [];
    const selectedId = String(selected?.id || "");
    const receivedDevis = (Array.isArray(trip.receivedDevis) ? trip.receivedDevis : []).filter(
      (q) => !selectedId || String((q as { id?: string }).id) !== selectedId,
    );

    const updated: TravelsTrip = {
      ...trip,
      status: "PROF_LOGISTICS",
      updatedAt: now,
      receivedDevis,
      data: {
        ...data,
        signedQuoteUrl: undefined,
        signedQuoteS3Key: undefined,
        selectedBusQuote: undefined,
        cancelledBusQuotes: [...prevCancelled, cancelled],
      },
      history: [
        ...(Array.isArray(trip.history) ? trip.history : []),
        {
          date: now,
          user: actor,
          action: "DEVIS_BUS_SIGNE_ANNULE",
          note: `Devis signé annulé (${providerName})${reason ? ` — ${reason}` : ""}. Annulation envoyée au transporteur.`,
        },
      ],
    } as TravelsTrip;

    await putJson(`travels/${tripId}.json`, updated);
    const indexHit = await getJson<TravelsTrip[]>("travels/index.json");
    const index = Array.isArray(indexHit?.data) ? indexHit.data : [];
    await putJson(
      "travels/index.json",
      index.map((t) => (t.id === tripId ? { ...t, status: updated.status } : t)),
    );

    return NextResponse.json({ success: true, trip: updated });
  } catch (error) {
    console.error("[cancel-signed-bus-quote]", error);
    return NextResponse.json({ error: "Erreur lors de l'annulation du devis signé." }, { status: 500 });
  }
}
