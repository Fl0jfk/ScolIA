import "server-only";

import { loadAppConfig } from "@/app/lib/app-config";
import { createTenantTransporter, getTenantSmtpConfig, sendMailWithTimeout } from "@/app/lib/tenant-mail";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";
import { SantaError, type SantaRecord } from "@/app/lib/secret-santa";
import { santaMail, type SantaMailKind } from "@/app/lib/secret-santa-mail-text";

type Target = { userId: string; displayName: string; email: string };

/**
 * Un message par personne, sans aucun nom tiré. `onFailure` est appelé pour chaque personne non
 * servie (y compris si l'envoi n'est pas configuré) afin d'annuler sa réservation.
 */
export async function sendSantaMails<T extends Target>(
  kind: SantaMailKind,
  santa: SantaRecord,
  targets: T[],
  onFailure: (target: T) => Promise<void> = async () => {},
): Promise<{ sent: number; failed: number }> {
  if (!targets.length) return { sent: 0, failed: 0 };

  let send: (target: T) => Promise<void>;
  try {
    const smtp = await getTenantSmtpConfig();
    const transporter = smtp ? await createTenantTransporter() : null;
    if (!smtp || !transporter) throw new SantaError("L'envoi d'e-mails n'est pas configuré.", 503);
    const bundle = await loadAppConfig();
    const school = bundle.identity.shortName || bundle.identity.name || "Établissement";
    const link = await tenantAbsolutePath(`/secret-santa/${santa.id}`);
    const { subject, lines } = santaMail(kind, santa, link, new Date());
    send = async (target) => {
      await sendMailWithTimeout(transporter, {
        from: `"${school.replace(/["\r\n]/g, "")}" <${smtp.user}>`,
        to: target.email,
        subject,
        text: [`Bonjour ${target.displayName},`, "", ...lines].join("\n"),
      });
    };
  } catch (error) {
    await Promise.all(targets.map(onFailure));
    throw error;
  }

  let sent = 0;
  let failed = 0;
  for (const target of targets) {
    try {
      await send(target);
      sent++;
    } catch (error) {
      failed++;
      console.error("[secret-santa] envoi en échec", error instanceof Error ? error.name : "erreur");
      await onFailure(target);
    }
  }
  return { sent, failed };
}
