import "server-only";

import { loadAppConfig } from "@/app/lib/app-config";
import { createTenantTransporter, getTenantSmtpConfig, sendMailWithTimeout } from "@/app/lib/tenant-mail";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";
import { releaseInvitation } from "@/app/lib/secret-santa-db";
import { SantaError, isRevealDue, type SantaParticipantRecord, type SantaRecord } from "@/app/lib/secret-santa";

type Target = SantaParticipantRecord & { email: string };

const dateParis = (date: Date) =>
  new Intl.DateTimeFormat("fr-FR", {
    timeZone: "Europe/Paris",
    weekday: "long",
    day: "numeric",
    month: "long",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);

/** Un message par personne, sans aucun nom tiré. Les échecs sont comptés et restent à renvoyer. */
export async function sendSantaInvitations(
  santa: SantaRecord,
  targets: Target[],
): Promise<{ sent: number; failed: number }> {
  if (!targets.length) return { sent: 0, failed: 0 };
  const releaseAll = () => Promise.all(targets.map((t) => releaseInvitation(santa.id, t.userId, t.invitedAt)));

  let send: (target: Target) => Promise<void>;
  try {
    const smtp = await getTenantSmtpConfig();
    const transporter = smtp ? await createTenantTransporter() : null;
    if (!smtp || !transporter) throw new SantaError("L'envoi d'e-mails n'est pas configuré.", 503);
    const bundle = await loadAppConfig();
    const school = bundle.identity.shortName || bundle.identity.name || "Établissement";
    const link = await tenantAbsolutePath(`/secret-santa/${santa.id}`);
    const lines = [
      `${santa.organizerName} vous invite au Secret Santa « ${santa.title} ».`,
      "",
      ...(isRevealDue(santa.revealAt, new Date())
        ? ["Le tirage a eu lieu : ouvrez votre cadeau pour découvrir à qui l'offrir."]
        : [
            `Le tirage aura lieu ${dateParis(santa.revealAt)} (heure de Paris).`,
            "À cette heure-là, tout le monde découvre en même temps à qui offrir un cadeau.",
          ]),
      santa.budget ? `Budget : ${santa.budget}` : null,
      santa.message ? `\n${santa.message}` : null,
      "",
      `Votre lien (connexion à ScolIA demandée) : ${link}`,
      "",
      "Chut : le nom que vous découvrirez doit rester secret.",
    ].filter((line): line is string => line !== null);
    send = async (target) => {
      await sendMailWithTimeout(transporter, {
        from: `"${school.replace(/["\r\n]/g, "")}" <${smtp.user}>`,
        to: target.email,
        subject: `Secret Santa : ${santa.title}`.replace(/[\r\n]+/g, " "),
        text: [`Bonjour ${target.displayName},`, "", ...lines].join("\n"),
      });
    };
  } catch (error) {
    await releaseAll();
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
      console.error("[secret-santa] envoi en échec", error instanceof Error ? error.message : "erreur");
      await releaseInvitation(santa.id, target.userId, target.invitedAt);
    }
  }
  return { sent, failed };
}
