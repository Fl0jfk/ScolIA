import "server-only";
import { createTenantTransporter, getTenantSmtpConfig } from "@/app/lib/tenant-mail";
import { travelEmailAlertRecipients } from "@/app/lib/travel-email-routing";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";

export type UnmatchedAlertPayload = {
  gmailMessageId: string;
  fromEmail: string;
  subject: string;
  snippet?: string;
  reason?: string | null;
  matchMotif?: string | null;
  messageType?: string | null;
  guessedTripId?: string | null;
  tenantSlug?: string | null;
};

/**
 * Alerte ops quand un mail transport n'a pas pu être rattaché.
 * Nécessite TRAVEL_EMAIL_ALERT_TO + SMTP.
 */
export async function sendTravelEmailUnmatchedAlert(
  payload: UnmatchedAlertPayload,
): Promise<{ sent: boolean; reason?: string }> {
  const recipients = travelEmailAlertRecipients();
  if (!recipients.length) {
    return { sent: false, reason: "TRAVEL_EMAIL_ALERT_TO_absent" };
  }
  const smtp = await getTenantSmtpConfig();
  const transporter = await createTenantTransporter();
  if (!smtp || !transporter) {
    return { sent: false, reason: "smtp_unavailable" };
  }

  let unmatchedUrl = "/travels";
  try {
    unmatchedUrl = await tenantAbsolutePath("/travels");
  } catch {
    /* ignore */
  }

  const lines = [
    `Attention — polling e-mail transport : message non rattaché.`,
    ``,
    `De : ${payload.fromEmail}`,
    `Objet : ${payload.subject || "(sans objet)"}`,
    payload.snippet ? `Extrait : ${payload.snippet.slice(0, 400)}` : "",
    ``,
    `Motif : ${payload.reason || "—"}`,
    payload.matchMotif ? `Détail IA : ${payload.matchMotif}` : "",
    payload.messageType ? `Type suggéré : ${payload.messageType}` : "",
    payload.guessedTripId ? `Séjour suggéré (incertain) : ${payload.guessedTripId}` : "",
    payload.tenantSlug ? `Tenant détecté : ${payload.tenantSlug}` : "",
    `Message id : ${payload.gmailMessageId}`,
    ``,
    `Ouvre la liste des sorties (bandeau unmatched) pour rattacher manuellement :`,
    unmatchedUrl,
    ``,
    `— ScolIA polling`,
  ].filter(Boolean);

  try {
    await transporter.sendMail({
      from: `"ScolIA Polling" <${smtp.user}>`,
      to: recipients.join(", "),
      subject: `[ScolIA] Mail transport non rattaché — ${payload.subject || payload.fromEmail}`.slice(0, 180),
      text: lines.join("\n"),
    });
    return { sent: true };
  } catch (e) {
    console.error("[travel-email-alert]", e);
    return { sent: false, reason: e instanceof Error ? e.message : "send_failed" };
  }
}
