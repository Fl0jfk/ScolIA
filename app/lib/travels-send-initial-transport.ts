import "server-only";

import { getTransportProviders } from "@/app/lib/transport-providers";
import { loadBusProgramAttachments } from "@/app/lib/travels-bus-program";
import { buildTransportQuotePdf } from "@/app/lib/travels-transport-quote-pdf";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
} from "@/app/lib/tenant-mail";
import { buildTransportReplyTo, mergeTransportMailCc } from "@/app/lib/travel-email-routing";
import type { TravelsTrip, TravelsTripData } from "@/app/lib/travels-types";

export type SendInitialTransportResult =
  | { ok: true; emailsAttempted: number; emailsFailed: number }
  | { ok: false; error: string; status: number };

/**
 * Envoie la demande de devis initiale à tous les transporteurs configurés.
 * Ne persiste pas le snapshot — l’appelant met à jour le dossier.
 */
export async function sendInitialTransportQuotes(params: {
  tripId: string;
  data: TravelsTripData;
  userName: string;
}): Promise<SendInitialTransportResult> {
  const { tripId, data, userName } = params;
  const destination = String(data.destination || "").trim();
  if (!destination) {
    return { ok: false, error: "Destination manquante pour la demande de devis.", status: 400 };
  }

  const transporteurs = await getTransportProviders();
  if (transporteurs.length === 0) {
    return { ok: false, error: "Aucun transporteur configuré.", status: 400 };
  }

  const smtp = await getTenantSmtpConfig();
  if (!smtp) {
    return { ok: false, error: "SMTP non configuré", status: 503 };
  }
  const transporter = await createTenantTransporter();
  if (!transporter) {
    return { ok: false, error: "SMTP non configuré", status: 503 };
  }

  const busProgramExtra = await loadBusProgramAttachments(data);
  const replyTo = await buildTransportReplyTo();
  let emailsFailed = 0;

  for (const transporteur of transporteurs) {
    try {
      const personalPdf = await buildTransportQuotePdf({
        tripId,
        data,
        userName,
        transporteurName: transporteur.name,
        mode: "initial",
      });
      await transporter.sendMail({
        from: `"Plateforme Voyages" <${smtp.user}>`,
        to: transporteur.email,
        ...(replyTo ? { replyTo } : {}),
        cc: mergeTransportMailCc(),
        subject: `DEMANDE DE DEVIS - ${destination.toUpperCase()} - ${userName}`,
        html: `
          <div style="font-family: sans-serif; line-height: 1.5; color: #334155;">
            <h2>Bonjour ${transporteur.name},</h2>
            <p>Veuillez trouver ci-joint une demande de devis pour un transport scolaire à destination de <strong>${destination}</strong>.</p>
            <p>Le récapitulatif complet ainsi que le programme éventuel sont joints à cet email.</p>
            <div style="margin: 24px 0; padding: 16px; border-radius: 12px; background-color: #f0fdf4; border: 1px solid #86efac;">
              <p style="margin: 0 0 8px; font-weight: bold; color: #166534;">Réponse par e-mail</p>
              <p style="margin: 0; font-size: 14px; color: #14532d;">Répondez directement à cet e-mail en joignant votre devis ou vos questions en PDF si besoin. Votre message sera rattaché automatiquement au dossier de sortie.</p>
            </div>
            <p>Cordialement,<br/>L'administration.</p>
          </div>
        `,
        attachments: [
          {
            filename: `Demande_Transport_${destination.replace(/\s+/g, "_")}.pdf`,
            content: personalPdf,
            contentType: "application/pdf",
          },
          ...busProgramExtra,
        ],
      });
    } catch (sendErr: unknown) {
      emailsFailed += 1;
      const msg = sendErr instanceof Error ? sendErr.message : String(sendErr);
      console.error(`[send-initial-transport] ${transporteur.name}:`, msg);
    }
  }

  return {
    ok: true,
    emailsAttempted: transporteurs.length,
    emailsFailed,
  };
}

export function buildInitialTransportQuoteSnapshot(data: TravelsTripData, sentAt: string) {
  return {
    nbEleves: Number(data.nbEleves) || 0,
    nbAccompagnateurs: Number(data.nbAccompagnateurs) || 0,
    sentAt,
    type: "initial" as const,
  };
}

export function tripPayloadForTransportMail(trip: TravelsTrip) {
  return {
    id: trip.id,
    data: trip.data,
  };
}
