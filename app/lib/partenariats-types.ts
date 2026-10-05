/**
 * Types & helpers — Partenariats & offres.
 */

export const PARTENARIAT_KINDS = ["info", "inscription"] as const;
export type PartenariatKind = (typeof PARTENARIAT_KINDS)[number];

export const PARTENARIAT_CYCLES = ["ecole", "college", "lycee"] as const;
export type PartenariatCycle = (typeof PARTENARIAT_CYCLES)[number];

export const PARTENARIAT_CYCLE_LABELS: Record<PartenariatCycle, string> = {
  ecole: "École",
  college: "Collège",
  lycee: "Lycée",
};

/** Niveaux « métier » (pas les divisions 3A/3B). */
export const PARTENARIAT_NIVEAUX_BY_CYCLE: Record<PartenariatCycle, { id: string; label: string }[]> =
  {
    ecole: [
      { id: "tps", label: "TPS" },
      { id: "ps", label: "PS" },
      { id: "ms", label: "MS" },
      { id: "gs", label: "GS" },
      { id: "cp", label: "CP" },
      { id: "ce1", label: "CE1" },
      { id: "ce2", label: "CE2" },
      { id: "cm1", label: "CM1" },
      { id: "cm2", label: "CM2" },
    ],
    college: [
      { id: "6eme", label: "6ᵉ" },
      { id: "5eme", label: "5ᵉ" },
      { id: "4eme", label: "4ᵉ" },
      { id: "3eme", label: "3ᵉ" },
    ],
    lycee: [
      { id: "2nde", label: "2ⁿᵈᵉ" },
      { id: "1ere", label: "1ʳᵉ" },
      { id: "terminale", label: "Terminale" },
    ],
  };

export const PARTENARIAT_ALL_NIVEAUX = PARTENARIAT_CYCLES.flatMap(
  (c) => PARTENARIAT_NIVEAUX_BY_CYCLE[c],
);

export const PARTENARIAT_INSCRIPTION_STATUSES = [
  "soumise",
  "validee",
  "refusee",
  "annulee",
] as const;
export type PartenariatInscriptionStatus = (typeof PARTENARIAT_INSCRIPTION_STATUSES)[number];

export type PartenariatCtaLink = {
  label: string;
  url: string;
};

export type PartenariatTarif = {
  label: string;
  amountCents?: number | null;
  amountLabel?: string;
  note?: string;
};

export type PartenariatOffreRecord = {
  id: string;
  etablissementId: string;
  slug: string;
  title: string;
  shortDescription: string;
  body: string;
  logoS3Key: string | null;
  logoUrl: string | null;
  kind: PartenariatKind;
  cycles: PartenariatCycle[];
  niveaux: string[];
  categoryLabel: string;
  contactName: string;
  contactRole: string;
  contactEmail: string;
  contactPhone: string;
  partnerContactName: string;
  partnerContactRole: string;
  partnerContactEmail: string;
  partnerContactPhone: string;
  ctaLinks: PartenariatCtaLink[];
  tarifs: PartenariatTarif[];
  demarche: string;
  engagementText: string;
  requireSignature: boolean;
  notifyEmail: string | null;
  maxPlaces: number | null;
  inscriptionOpensAt: string | null;
  inscriptionClosesAt: string | null;
  enabled: boolean;
  sortOrder: number;
  publishedAt: string | null;
  createdAt: string;
  updatedAt: string;
  /** Rempli côté listes admin / public. */
  evenementsCount?: number;
  inscriptionsCount?: number;
  placesRemaining?: number | null;
};

export type PartenariatEvenementRecord = {
  id: string;
  etablissementId: string;
  offreId: string;
  title: string;
  startsAt: string;
  endsAt: string | null;
  location: string;
  notes: string;
  createdAt: string;
  updatedAt: string;
};

export type PartenariatInscriptionRecord = {
  id: string;
  etablissementId: string;
  offreId: string;
  eleveFirstName: string;
  eleveLastName: string;
  eleveNiveau: string;
  eleveClasse: string;
  eleveBirthDate: string | null;
  parentFirstName: string;
  parentLastName: string;
  parentEmail: string;
  parentPhone: string;
  engagementAcceptedAt: string;
  signatureS3Key: string | null;
  status: PartenariatInscriptionStatus;
  adminNote: string;
  createdAt: string;
  updatedAt: string;
};

/** Payload catalogue public (sans emails internes sensibles hors contacts affichés). */
export type PartenariatOffrePublicCard = {
  slug: string;
  title: string;
  shortDescription: string;
  logoUrl: string | null;
  kind: PartenariatKind;
  cycles: PartenariatCycle[];
  niveaux: string[];
  categoryLabel: string;
};

export type PartenariatOffrePublicDetail = PartenariatOffrePublicCard & {
  body: string;
  contactName: string;
  contactRole: string;
  contactEmail: string;
  contactPhone: string;
  partnerContactName: string;
  partnerContactRole: string;
  partnerContactEmail: string;
  partnerContactPhone: string;
  ctaLinks: PartenariatCtaLink[];
  tarifs: PartenariatTarif[];
  demarche: string;
  engagementText: string;
  requireSignature: boolean;
  maxPlaces: number | null;
  placesRemaining: number | null;
  inscriptionOpen: boolean;
  inscriptionOpensAt: string | null;
  inscriptionClosesAt: string | null;
  schoolName: string;
  evenements: Array<{
    id: string;
    title: string;
    startsAt: string;
    endsAt: string | null;
    location: string;
    notes: string;
  }>;
};

export const DEFAULT_ENGAGEMENT_TEXT =
  "Je m’engage à régler les sommes dues liées à cette inscription selon les modalités communiquées par l’établissement.";

export function isPartenariatKind(v: unknown): v is PartenariatKind {
  return typeof v === "string" && (PARTENARIAT_KINDS as readonly string[]).includes(v);
}

export function isPartenariatCycle(v: unknown): v is PartenariatCycle {
  return typeof v === "string" && (PARTENARIAT_CYCLES as readonly string[]).includes(v);
}

export function isPartenariatInscriptionStatus(v: unknown): v is PartenariatInscriptionStatus {
  return (
    typeof v === "string" &&
    (PARTENARIAT_INSCRIPTION_STATUSES as readonly string[]).includes(v)
  );
}

export function sanitizeCycles(input: unknown): PartenariatCycle[] {
  if (!Array.isArray(input)) return [];
  const out: PartenariatCycle[] = [];
  for (const c of input) {
    if (isPartenariatCycle(c) && !out.includes(c)) out.push(c);
  }
  return PARTENARIAT_CYCLES.filter((c) => out.includes(c));
}

export function sanitizeNiveaux(input: unknown): string[] {
  if (!Array.isArray(input)) return [];
  const allowed = new Set(PARTENARIAT_ALL_NIVEAUX.map((n) => n.id));
  const out: string[] = [];
  for (const raw of input) {
    const id = String(raw || "")
      .trim()
      .toLowerCase();
    if (id && allowed.has(id) && !out.includes(id)) out.push(id);
  }
  return out;
}

export function sanitizeCtaLinks(input: unknown): PartenariatCtaLink[] {
  if (!Array.isArray(input)) return [];
  const out: PartenariatCtaLink[] = [];
  for (const row of input) {
    if (!row || typeof row !== "object") continue;
    const label = String((row as { label?: unknown }).label || "").trim().slice(0, 120);
    const url = String((row as { url?: unknown }).url || "").trim().slice(0, 2000);
    if (!label || !url) continue;
    out.push({ label, url });
  }
  return out.slice(0, 20);
}

export function sanitizeTarifs(input: unknown): PartenariatTarif[] {
  if (!Array.isArray(input)) return [];
  const out: PartenariatTarif[] = [];
  for (const row of input) {
    if (!row || typeof row !== "object") continue;
    const label = String((row as { label?: unknown }).label || "").trim().slice(0, 200);
    if (!label) continue;
    const amountLabel = String((row as { amountLabel?: unknown }).amountLabel || "")
      .trim()
      .slice(0, 80);
    const note = String((row as { note?: unknown }).note || "").trim().slice(0, 500);
    const centsRaw = (row as { amountCents?: unknown }).amountCents;
    let amountCents: number | null = null;
    if (typeof centsRaw === "number" && Number.isFinite(centsRaw)) {
      amountCents = Math.round(centsRaw);
    } else if (typeof centsRaw === "string" && centsRaw.trim()) {
      const n = Number(centsRaw);
      if (Number.isFinite(n)) amountCents = Math.round(n);
    }
    out.push({
      label,
      amountCents,
      amountLabel: amountLabel || undefined,
      note: note || undefined,
    });
  }
  return out.slice(0, 30);
}

export function slugifyPartenariat(input: string): string {
  return (
    input
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "")
      .slice(0, 80) || "offre"
  );
}

export function niveauLabel(id: string): string {
  const hit = PARTENARIAT_ALL_NIVEAUX.find((n) => n.id === id);
  return hit?.label || id;
}

export function cycleLabels(cycles: PartenariatCycle[]): string {
  return cycles.map((c) => PARTENARIAT_CYCLE_LABELS[c]).join(" · ");
}

/** Offre visible pour un filtre cycle (+ niveau optionnel). */
export function offreMatchesFilters(
  offre: { cycles: PartenariatCycle[]; niveaux: string[] },
  cycle: PartenariatCycle | null,
  niveau: string | null,
): boolean {
  if (cycle) {
    if (offre.cycles.length > 0 && !offre.cycles.includes(cycle)) return false;
  }
  if (niveau) {
    if (offre.niveaux.length > 0 && !offre.niveaux.includes(niveau)) return false;
  }
  return true;
}

export function isInscriptionWindowOpen(offre: {
  kind: PartenariatKind;
  enabled: boolean;
  inscriptionOpensAt: string | null;
  inscriptionClosesAt: string | null;
  placesRemaining?: number | null;
}): boolean {
  if (offre.kind !== "inscription" || !offre.enabled) return false;
  const now = Date.now();
  if (offre.inscriptionOpensAt) {
    const t = Date.parse(offre.inscriptionOpensAt);
    if (Number.isFinite(t) && now < t) return false;
  }
  if (offre.inscriptionClosesAt) {
    const t = Date.parse(offre.inscriptionClosesAt);
    if (Number.isFinite(t) && now > t) return false;
  }
  if (typeof offre.placesRemaining === "number" && offre.placesRemaining <= 0) return false;
  return true;
}

export function formatTarifLine(t: PartenariatTarif): string {
  if (t.amountLabel?.trim()) return `${t.label} — ${t.amountLabel.trim()}`;
  if (typeof t.amountCents === "number") {
    const euros = (t.amountCents / 100).toLocaleString("fr-FR", {
      style: "currency",
      currency: "EUR",
    });
    return `${t.label} — ${euros}`;
  }
  return t.label;
}
