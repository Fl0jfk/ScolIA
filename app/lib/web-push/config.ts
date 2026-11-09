import "server-only";

export type WebPushConfig = {
  publicKey: string;
  privateKey: string;
  subject: string;
};

/** Lit la configuration VAPID. Retourne null si incomplète (fonctionnalité « non configurée »). */
export function readWebPushConfig(): WebPushConfig | null {
  const publicKey = process.env.WEB_PUSH_VAPID_PUBLIC_KEY?.trim() ?? "";
  const privateKey = process.env.WEB_PUSH_VAPID_PRIVATE_KEY?.trim() ?? "";
  const subject = process.env.WEB_PUSH_VAPID_SUBJECT?.trim() ?? "";
  if (!publicKey || !privateKey || !subject) return null;
  if (!/^(mailto:|https:\/\/)/i.test(subject)) return null;
  return { publicKey, privateKey, subject };
}

export const WEB_PUSH_KINDS = ["messagerie", "messagerie_mention", "signal"] as const;
