export type Tab =
  | "site"
  | "establishments"
  | "notifications"
  | "mef"
  | "prof-room"
  | "requests-routing"
  | "travels"
  | "integrations"
  | "referentiel"
  | "annees"
  | "siecle"
  | "dashboard-links"
  | "utilisateurs"
  | "identite"
  | "module-access";

export type SettingsEstablishmentForm = {
  id: string;
  label: string;
  kind?: string;
  directorName: string;
  directorEmail: string;
  directorExternalUserId: string;
  colorHex: string;
  roleSlugs: string;
  active: boolean;
  grades?: string;
  signatureS3Key?: string;
  signaturePreviewUrl?: string | null;
};

export function emptySettingsEstablishmentForm(): SettingsEstablishmentForm {
  return {
    id: "",
    label: "",
    kind: "custom",
    directorName: "",
    directorEmail: "",
    directorExternalUserId: "",
    colorHex: "#8B5CF6",
    roleSlugs: "",
    active: true,
    signaturePreviewUrl: null,
  };
}

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
