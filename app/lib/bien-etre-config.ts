import "server-only";

import { getJson, putJson } from "@/app/lib/s3-storage";
import type { BienEtreConfig } from "@/app/lib/bien-etre-types";

const CONFIG_KEY = "settings/bien-etre.json";

function defaultBienEtreConfig(): BienEtreConfig {
  return {
    enabled: false,
    psychologistEmail: "",
    retentionDays: 90,
    welcomeMessage:
      "Bonjour, je suis là pour t'écouter. Ce que tu dis ici n'est pas enregistré tant que tu ne fais pas de signalement. Comment te sens-tu aujourd'hui ?",
  };
}

export async function getBienEtreConfig(): Promise<BienEtreConfig> {
  const raw = await getJson<BienEtreConfig>(CONFIG_KEY);
  if (!raw?.data) return defaultBienEtreConfig();
  const d = raw.data;
  return {
    enabled: d.enabled === true,
    psychologistEmail: String(d.psychologistEmail || "").trim(),
    retentionDays: Math.max(7, Math.min(365, Number(d.retentionDays) || 90)),
    welcomeMessage: String(d.welcomeMessage || defaultBienEtreConfig().welcomeMessage).trim(),
    notificationFromEmail: String(d.notificationFromEmail || "").trim() || undefined,
  };
}

export async function saveBienEtreConfig(config: BienEtreConfig): Promise<void> {
  await putJson(CONFIG_KEY, {
    ...config,
    psychologistEmail: config.psychologistEmail.trim(),
    welcomeMessage: config.welcomeMessage?.trim() || defaultBienEtreConfig().welcomeMessage,
    notificationFromEmail: config.notificationFromEmail?.trim() || undefined,
  });
}
