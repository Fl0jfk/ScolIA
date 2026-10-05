import "server-only";

import { getTransportProviders, type TransportProvider } from "@/app/lib/transport-providers";
import { loadBusProgramAttachments } from "@/app/lib/travels-bus-program";
import { buildTransportQuotePdf } from "@/app/lib/travels-transport-quote-pdf";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
} from "@/app/lib/tenant-mail";
import { buildTransportReplyTo, mergeTransportMailCc } from "@/app/lib/travel-email-routing";
import type { TravelsTrip, TravelsTripData } from "@/app/lib/travels-types";

export type SendInitialTransportResult =
  | {
      ok: true;
      emailsAttempted: number;
      emailsFailed: number;
      sentTo: string[];
      skippedAlreadyQuoted: number;
    }
  | { ok: false; error: string; status: number };

export type TransportQuoteSendKind = "initial" | "reminder";

function normalizeEmail(value: unknown): string {
  return String(value || "")
    .trim()
    .toLowerCase();
}

/** E-mails déjà associés à un devis reçu (expéditeur ou contact extrait). */
export function emailsAlreadyQuoted(receivedDevis: unknown): Set<string> {
  const out = new Set<string>();
  if (!Array.isArray(receivedDevis)) return out;
  for (const raw of receivedDevis) {
    if (!raw || typeof raw !== "object") continue;
    const quote = raw as Record<string, unknown>;
    const candidates = [quote.providerEmail, quote.extractedContactEmail, quote.fromEmail];
    for (const c of candidates) {
      const n = normalizeEmail(c);
      if (n.includes("@")) out.add(n);
    }
  }
  return out;
}

function filterProvidersForReminder(
  providers: TransportProvider[],
  receivedDevis: unknown,
  forceAll: boolean,
): { recipients: TransportProvider[]; skippedAlreadyQuoted: number } {
  if (forceAll) {
    return { recipients: providers, skippedAlreadyQuoted: 0 };
  }
  const quoted = emailsAlreadyQuoted(receivedDevis);
  if (quoted.size === 0) {
    return { recipients: providers, skippedAlreadyQuoted: 0 };
  }
  const recipients: TransportProvider[] = [];
  let skippedAlreadyQuoted = 0;
  for (const p of providers) {
    if (quoted.has(normalizeEmail(p.email))) {
      skippedAlreadyQuoted += 1;
    } else {
      recipients.push(p);
    }
  }
  return { recipients, skippedAlreadyQuoted };
}

/**
 * Envoie la demande de devis (initiale ou relance) aux transporteurs configurés.
 * Ne persiste pas le snapshot — l’appelant met à jour le dossier.
 */
export async function sendInitialTransportQuotes(params: {
  tripId: string;
  data: TravelsTripData;
  userName: string;
  kind?: TransportQuoteSendKind;
  /** Relance : ignorer le filtre « déjà devis reçu » et renvoyer à tous. */
  forceAllProviders?: boolean;
  receivedDevis?: unknown;
}): Promise<SendInitialTransportResult> {
  const {
    tripId,
    data,
    userName,
    kind = "initial",
    forceAllProviders = false,
    receivedDevis,
  } = params;
  const destination = String(data.destination || "").trim();
  if (!destination) {
    return { ok: false, error: "Destination manquante pour la demande de devis.", status: 400 };
  }

  const transporteurs = await getTransportProviders();
  if (transporteurs.length === 0) {
    return { ok: false, error: "Aucun transporteur configuré.", status: 400 };
  }

  const { recipients, skippedAlreadyQuoted } =
    kind === "reminder"
      ? filterProvidersForReminder(transporteurs, receivedDevis, forceAllProviders)
      : { recipients: transporteurs, skippedAlreadyQuoted: 0 };

  if (recipients.length === 0) {
    return {
      ok: false,
      error:
        "Tous les transporteurs configurés ont déjà un devis rattaché à ce dossier. Aucune relance envoyée.",
      status: 400,
    };
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
  const sentTo: string[] = [];
  const destUpper = destination.toUpperCase();
  const isReminder = kind === "reminder";
  const subject = isReminder
    ? `RELANCE DEMANDE DE DEVIS - ${destUpper} - ${userName}`
    : `DEMANDE DE DEVIS - ${destUpper} - ${userName}`;

  for (const transporteur of recipients) {
    try {
      const personalPdf = await buildTransportQuotePdf({
        tripId,
        data,
        userName,
        transporteurName: transporteur.name,
        mode: "initial",
      });
      const introHtml = isReminder
        ? `
            <h2>Bonjour ${transporteur.name},</h2>
            <p>Nous nous permettons de <strong>relancer</strong> notre demande de devis pour un transport scolaire à destination de <strong>${destination}</strong>.</p>
            <p>Nous n’avons pas encore reçu votre proposition. Vous trouverez à nouveau ci-joint le récapitulatif complet ainsi que le programme éventuel.</p>
            <p>Si vous avez déjà répondu, merci de nous le signaler en répondant à cet e-mail (avec le devis en pièce jointe si besoin).</p>
          `
        : `
            <h2>Bonjour ${transporteur.name},</h2>
            <p>Veuillez trouver ci-joint une demande de devis pour un transport scolaire à destination de <strong>${destination}</strong>.</p>
            <p>Le récapitulatif complet ainsi que le programme éventuel sont joints à cet email.</p>
          `;
      await transporter.sendMail({
        from: `"Plateforme Voyages" <${smtp.user}>`,
        to: transporteur.email,
        ...(replyTo ? { replyTo } : {}),
        cc: mergeTransportMailCc(),
        subject,
        html: `
          <div style="font-family: sans-serif; line-height: 1.5; color: #334155;">
            ${introHtml}
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
      sentTo.push(transporteur.email);
    } catch (sendErr: unknown) {
      emailsFailed += 1;
      const msg = sendErr instanceof Error ? sendErr.message : String(sendErr);
      console.error(`[send-initial-transport] ${transporteur.name}:`, msg);
    }
  }

  return {
    ok: true,
    emailsAttempted: recipients.length,
    emailsFailed,
    sentTo,
    skippedAlreadyQuoted,
  };
}

export function buildInitialTransportQuoteSnapshot(
  data: TravelsTripData,
  sentAt: string,
  type: "initial" | "reminder" = "initial",
) {
  return {
    nbEleves: Number(data.nbEleves) || 0,
    nbAccompagnateurs: Number(data.nbAccompagnateurs) || 0,
    sentAt,
    type,
  };
}

export function tripPayloadForTransportMail(trip: TravelsTrip) {
  return {
    id: trip.id,
    data: trip.data,
  };
}
