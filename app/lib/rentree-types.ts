/** Types partagés — page publique rentrée et configuration boîte à outils. */

export type RentreeLevel = "ecole" | "college" | "lycee";

export type RentreeAccent =
  | "yellow"
  | "sky"
  | "pink"
  | "green"
  | "blue"
  | "rose"
  | "violet"
  | "amber"
  | "teal";

export const RENTREE_ACCENT_OPTIONS: { id: RentreeAccent; label: string }[] = [
  { id: "yellow", label: "Jaune (école)" },
  { id: "sky", label: "Bleu ciel (collège)" },
  { id: "pink", label: "Rose (lycée)" },
  { id: "green", label: "Vert" },
  { id: "blue", label: "Bleu" },
  { id: "rose", label: "Rose vif" },
  { id: "violet", label: "Violet" },
  { id: "amber", label: "Ambre" },
  { id: "teal", label: "Bleu-vert" },
];

export type RentreeLinkKind = "pdf" | "link" | "submission";

/** Zone de dépôt famille : fichier → e-mail destinataire après confirmation. */
export type RentreeSubmissionConfig = {
  recipientEmails: string[];
};

export type RentreeLinkItem = {
  /** Identifiant stable (surtout pour les dépôts). */
  id?: string;
  title: string;
  description?: string;
  href: string;
  kind?: RentreeLinkKind;
  submission?: RentreeSubmissionConfig;
};

export function newRentreeItemId(): string {
  return `rs_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`;
}

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

export function parseRentreeRecipientEmails(raw: unknown): string[] {
  const parts: string[] = [];
  if (Array.isArray(raw)) {
    for (const v of raw) parts.push(...String(v).split(/[,;\n]+/));
  } else if (typeof raw === "string") {
    parts.push(...raw.split(/[,;\n]+/));
  }
  const seen = new Set<string>();
  const out: string[] = [];
  for (const p of parts) {
    const email = p.trim().toLowerCase();
    if (!EMAIL_RE.test(email) || seen.has(email)) continue;
    seen.add(email);
    out.push(email);
  }
  return out;
}

export type RentreeSection = {
  title: string;
  items: RentreeLinkItem[];
};

/** Page rentrée liée à un établissement (paramètres généraux). */
export type RentreeEstablishmentPage = {
  establishmentId: string;
  label: string;
  accent: RentreeAccent;
  sections: RentreeSection[];
};

/** Ancien modèle par niveau — conservé pour migration. */
export type RentreeLinksByLevel = {
  level: RentreeLevel;
  label: string;
  accent: RentreeAccent;
  sections: RentreeSection[];
};
