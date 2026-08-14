export type Tab =
  | "site"
  | "establishments"
  | "notifications"
  | "mef"
  | "prof-room"
  | "requests-routing"
  | "travels"
  | "integrations"
  | "toolbox"
  | "referentiel"
  | "dashboard-links"
  | "utilisateurs";

export type SettingsEstablishmentForm = {
  id: string;
  label: string;
  kind?: string;
  directorName: string;
  directorEmail: string;
  clerkRoleSlugs: string;
  active: boolean;
  signatureS3Key?: string;
  signaturePreviewUrl?: string | null;
};

export type SettingsTravelsConfig = {
  transportProviders: { name: string; email: string }[];
  pdfFooterText?: string;
};

export function linesToList(text: string): string[] {
  return text
    .split(/\r?\n/)
    .map((s) => s.trim())
    .filter(Boolean);
}

export function listToLines(arr: string[]): string {
  return (arr || []).join("\n");
}
