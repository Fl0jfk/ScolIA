import { loadAppConfig } from "@/app/lib/app-config";
import { listDirectoryMembers } from "@/app/lib/directory-members";
import { createTenantTransporter, getTenantSmtpConfig } from "@/app/lib/tenant-mail";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";

function inviterLabel(params: {
  firstName?: string | null;
  lastName?: string | null;
  email?: string | null;
}): string {
  const name = `${params.firstName ?? ""} ${params.lastName ?? ""}`.trim();
  return name || params.email?.trim() || "Un collègue";
}

/**
 * Envoie un e-mail aux membres nouvellement invités sur un dossier partagé
 * (Cloud personnel → Dossiers partagés).
 */
export async function notifySharedFolderInvites(params: {
  shareId: string;
  shareName: string;
  inviteeUserIds: string[];
  inviter: {
    firstName?: string | null;
    lastName?: string | null;
    email?: string | null;
  };
}): Promise<{ sent: number; skipped: number }> {
  const inviteeIds = [...new Set(params.inviteeUserIds.filter(Boolean))];
  if (inviteeIds.length === 0) return { sent: 0, skipped: 0 };

  const smtp = await getTenantSmtpConfig();
  if (!smtp) return { sent: 0, skipped: inviteeIds.length };
  const transporter = await createTenantTransporter();
  if (!transporter) return { sent: 0, skipped: inviteeIds.length };

  const bundle = await loadAppConfig();
  const orgName = bundle.identity.shortName || bundle.identity.name || "La Providence";
  const who = inviterLabel(params.inviter);
  const folderName = params.shareName.trim() || "Dossier partagé";
  const folderLink = await tenantAbsolutePath(
    `/documents?shareId=${encodeURIComponent(params.shareId)}`,
  );
  const signInLink = await tenantAbsolutePath("/sign-in");

  const members = await listDirectoryMembers();
  const byId = new Map(members.filter((m) => m.externalUserId).map((m) => [m.externalUserId, m]));

  let sent = 0;
  let skipped = 0;

  for (const userId of inviteeIds) {
    const member = byId.get(userId);
    const to = member?.email?.trim();
    if (!member || !to) {
      skipped += 1;
      continue;
    }

    const greetingName = member.firstName || member.displayName || "";
    const text = [
      greetingName ? `Bonjour ${greetingName},` : "Bonjour,",
      "",
      `${who} vous a invité à consulter le dossier partagé « ${folderName} ».`,
      "",
      "Connectez-vous avec votre compte habituel :",
      folderLink,
      "",
      `(Page de connexion : ${signInLink})`,
      "",
      "Une fois connecté :",
      "1. Ouvrez le module Cloud personnel",
      "2. Dans la colonne de gauche, section « Dossiers partagés »",
      `3. Ouvrez le dossier « ${folderName} »`,
      "",
      "Cordialement,",
      orgName,
    ].join("\n");

    await transporter.sendMail({
      from: `"Cloud personnel ${orgName}" <${smtp.user}>`,
      to,
      subject: `${who} vous a partagé le dossier « ${folderName} »`,
      text,
    });
    sent += 1;
  }

  return { sent, skipped };
}
