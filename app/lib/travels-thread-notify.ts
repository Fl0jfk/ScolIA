import "server-only";

import { loadAppConfig } from "@/app/lib/app-config";
import { listStaffMembersLite } from "@/app/lib/members-db";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
  getTenantSmtpFromAddress,
} from "@/app/lib/tenant-mail";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";
import { selectTravelThreadEmailRecipients } from "@/app/lib/travels-thread-recipients";
import type { TravelsTrip } from "@/app/lib/travels-types";

const MAX_EXCERPT = 1500;

/**
 * Prévient par e-mail le public du fil interne (créateur, compta, direction concernée)
 * qu’un message vient d’être posté, sauf son auteur. Un mail par destinataire
 * (aucune adresse exposée aux autres). Ne lève jamais : un échec d’envoi n’annule pas le message.
 */
export async function notifyTravelThreadMessage(params: {
  trip: TravelsTrip;
  etablissementId: string;
  authorUserIds: string[];
  authorEmail?: string | null;
  authorLabel: string;
  roleLabel: string;
  text: string;
}): Promise<{ sent: number; failed: number }> {
  try {
    const smtp = await getTenantSmtpConfig();
    if (!smtp) {
      console.warn("[travels-thread-notify] SMTP non configuré — pas d’e-mail.");
      return { sent: 0, failed: 0 };
    }

    const [bundle, members] = await Promise.all([
      loadAppConfig(),
      listStaffMembersLite(params.etablissementId),
    ]);
    const recipients = selectTravelThreadEmailRecipients({
      trip: params.trip,
      members,
      establishments: bundle.establishments ?? [],
      authorUserIds: params.authorUserIds,
      authorEmail: params.authorEmail,
    });
    if (!recipients.length) return { sent: 0, failed: 0 };

    const transporter = await createTenantTransporter();
    if (!transporter) return { sent: 0, failed: 0 };
    const fromAddress = (await getTenantSmtpFromAddress()) || smtp.user;

    const d = params.trip.data || {};
    const title = String(d.title || "Sans titre");
    const destination = String(d.destination || "").trim();
    const link = await tenantAbsolutePath(`/travels/${params.trip.id}?tab=messages`);
    const orgName = bundle.identity.shortName || bundle.identity.name || "l’établissement";
    const excerpt =
      params.text.length > MAX_EXCERPT ? `${params.text.slice(0, MAX_EXCERPT)}…` : params.text;
    const author = `${params.authorLabel} (${params.roleLabel})`;

    const results = await Promise.allSettled(
      recipients.map((r) =>
        transporter.sendMail({
          from: `"Plateforme Voyages" <${fromAddress}>`,
          to: r.email,
          subject: `[Travels] Nouveau message de ${params.authorLabel} — ${title}`,
          text: [
            `Bonjour${r.name && r.name !== r.email ? ` ${r.name}` : ""},`,
            ``,
            `${author} a écrit un message sur le séjour « ${title} »${destination ? ` (${destination})` : ""} :`,
            ``,
            ...excerpt.split("\n").map((line) => `> ${line}`),
            ``,
            `Pour répondre, ouvrez la messagerie du séjour :`,
            link,
            ``,
            `Merci de ne pas répondre directement à cet e-mail : la réponse ne serait pas transmise.`,
            ``,
            `Cordialement,`,
            `Plateforme Voyages — ${orgName}`,
          ].join("\n"),
        }),
      ),
    );
    const failed = results.filter((r) => r.status === "rejected").length;
    if (failed) {
      console.warn(`[travels-thread-notify] ${failed}/${results.length} e-mail(s) en échec.`);
    }
    return { sent: results.length - failed, failed };
  } catch (err) {
    console.error("[travels-thread-notify]", err);
    return { sent: 0, failed: 0 };
  }
}
