import type { RgpdPlatformDpaId, RgpdQuestionnaireAnswers } from "@/app/lib/rgpd-types";

type RgpdPlatformDpaEntry = {
  id: RgpdPlatformDpaId;
  name: string;
  provider: string;
  purposes: string;
  dataCategories: string;
  dataLocation: string;
  dpaLabel: string;
  dpaUrl: string;
  defaultEnabled: boolean;
};

/** Plateformes Scola / établissement avec DPA publics connus. */
export const RGPD_PLATFORM_DPAS: RgpdPlatformDpaEntry[] = [
  {
    id: "microsoft365",
    name: "Microsoft 365",
    provider: "Microsoft Ireland Operations Ltd.",
    purposes:
      "Messagerie, bureautique, stockage OneDrive, Teams, authentification Microsoft (selon licences établissement).",
    dataCategories: "Identité, e-mails, fichiers, métadonnées d'usage",
    dataLocation: "Union européenne / EEE (selon offre et configuration tenant)",
    dpaLabel: "Microsoft Products and Services Data Protection Addendum (DPA)",
    dpaUrl:
      "https://www.microsoft.com/licensing/docs/view/Microsoft-Products-and-Services-Data-Protection-Addendum-DPA",
    defaultEnabled: true,
  },
  {
    id: "aws",
    name: "Amazon Web Services (AWS)",
    provider: "Amazon Web Services EMEA SARL",
    purposes:
      "Hébergement cloud, stockage S3 des documents et données de l'intranet Scola.",
    dataCategories: "Fichiers, documents scolaires, métadonnées techniques, journaux",
    dataLocation: "Région UE (ex. eu-west-3 Paris) selon configuration",
    dpaLabel: "AWS GDPR Data Processing Addendum (DPA)",
    dpaUrl: "https://aws.amazon.com/compliance/gdpr-center/",
    defaultEnabled: true,
  },
  {
    id: "mistral",
    name: "Mistral AI",
    provider: "Mistral AI",
    purposes:
      "Traitements d'IA : OCR documents, analyse de conformité RGPD, assistance incidents (selon modules activés).",
    dataCategories: "Extraits de documents, textes saisis, contexte métier limité au traitement",
    dataLocation: "Union européenne (offre API / hébergement UE Mistral)",
    dpaLabel: "Mistral AI Data Processing Agreement",
    dpaUrl: "https://legal.mistral.ai/terms/data-processing-agreement",
    defaultEnabled: true,
  },
  {
    id: "clerk",
    name: "Clerk",
    provider: "Clerk Inc.",
    purposes: "Authentification des utilisateurs de l'intranet, gestion des comptes et rôles.",
    dataCategories: "Identité, e-mail professionnel, métadonnées de session",
    dataLocation: "Union européenne / EEE (selon configuration tenant)",
    dpaLabel: "Clerk Data Processing Agreement",
    dpaUrl: "https://clerk.com/legal/dpa",
    defaultEnabled: true,
  },
];

const DEFAULT_PLATFORM_DPA_FLAGS: Record<RgpdPlatformDpaId, boolean> = {
  microsoft365: true,
  aws: true,
  mistral: true,
  clerk: true,
};

export function resolvePlatformDpaFlags(
  raw?: Partial<Record<RgpdPlatformDpaId, boolean>>,
): Record<RgpdPlatformDpaId, boolean> {
  const out = { ...DEFAULT_PLATFORM_DPA_FLAGS };
  for (const entry of RGPD_PLATFORM_DPAS) {
    if (typeof raw?.[entry.id] === "boolean") {
      out[entry.id] = raw[entry.id]!;
    }
  }
  return out;
}

export function getEnabledPlatformDpas(
  answers: RgpdQuestionnaireAnswers,
): RgpdPlatformDpaEntry[] {
  const flags = resolvePlatformDpaFlags(answers.platformDpas);
  return RGPD_PLATFORM_DPAS.filter((p) => flags[p.id]);
}

function formatPlatformDpaLine(entry: RgpdPlatformDpaEntry): string {
  return `${entry.name} (${entry.provider}) — ${entry.purposes} — Localisation : ${entry.dataLocation} — DPA : ${entry.dpaLabel} (${entry.dpaUrl})`;
}

export function formatPlatformDpaBullet(entry: RgpdPlatformDpaEntry): string {
  return `${entry.name} (${entry.provider}) : ${entry.purposes} | Données : ${entry.dataCategories} | ${entry.dataLocation} | DPA : ${entry.dpaLabel} — ${entry.dpaUrl}`;
}
