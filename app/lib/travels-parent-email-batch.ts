export type BatchedEmailSendResult = {
  sentCount: number;
  totalCount: number;
  completed: boolean;
  sentEmails: string[];
  error?: unknown;
};

/** Log SMTP sans adresses rejetées (objet brut nodemailer). */
export function smtpErrorForLog(error: unknown): { message: string; code?: string } {
  if (error instanceof Error) {
    const code =
      typeof (error as { code?: unknown }).code === "string"
        ? (error as { code: string }).code
        : undefined;
    return code ? { message: error.message, code } : { message: error.message };
  }
  return { message: String(error) };
}

/** Parents déjà notifiés (calendrier .ics) — évite les doublons après échec partiel. */
export function remainingParentEmailsToNotify(
  allEmails: string[],
  alreadyNotified: string[] | undefined,
): string[] {
  const sent = new Set((alreadyNotified || []).map((e) => e.trim().toLowerCase()).filter(Boolean));
  return allEmails.filter((e) => !sent.has(e.trim().toLowerCase()));
}

/** Liste notifiée effective : réinitialisée si le calendrier a changé depuis le dernier envoi. */
export function parentCalendarNotifiedEmailsForHash(
  storedEmails: string[] | undefined,
  storedHash: string | undefined,
  currentHash: string,
): string[] {
  if (!storedHash || storedHash !== currentHash) return [];
  return storedEmails || [];
}

export function mergeNotifiedParentEmails(
  existing: string[] | undefined,
  newlySent: string[],
): string[] {
  const seen = new Set((existing || []).map((e) => e.trim().toLowerCase()).filter(Boolean));
  const out = [...(existing || [])];
  for (const email of newlySent) {
    const key = email.trim().toLowerCase();
    if (!key || seen.has(key)) continue;
    seen.add(key);
    out.push(email.trim());
  }
  return out;
}

/**
 * Envoie des e-mails par lots ; en cas d’échec, conserve le décompte et la liste des lots déjà partis.
 */
export async function sendParentEmailsInBatches(
  emails: string[],
  batchSize: number,
  sendBatch: (batch: string[]) => Promise<void>,
): Promise<BatchedEmailSendResult> {
  const sentEmails: string[] = [];
  if (batchSize < 1) {
    return { sentCount: 0, totalCount: emails.length, completed: emails.length === 0, sentEmails };
  }
  try {
    for (let i = 0; i < emails.length; i += batchSize) {
      const batch = emails.slice(i, i + batchSize);
      await sendBatch(batch);
      sentEmails.push(...batch);
    }
    return {
      sentCount: sentEmails.length,
      totalCount: emails.length,
      completed: true,
      sentEmails,
    };
  } catch (error) {
    return {
      sentCount: sentEmails.length,
      totalCount: emails.length,
      completed: false,
      sentEmails,
      error,
    };
  }
}
