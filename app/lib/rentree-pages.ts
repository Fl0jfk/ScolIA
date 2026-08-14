import type { Establishment, EstablishmentKind } from "@/app/lib/app-config-schemas";
import { getActiveEstablishments } from "@/app/lib/app-config-establishments";
import { DEFAULT_RENTREE_SECTIONS, emptyRentreePage, normalizeRentreeSections, RENTREE_LINKS } from "@/app/lib/rentree-defaults";
import type { RentreeAccent, RentreeEstablishmentPage, RentreeLevel, RentreeLinkItem, RentreeLinksByLevel, RentreeSection } from "@/app/lib/rentree-types";

function defaultAccentForKind(kind?: EstablishmentKind): RentreeAccent {
  if (kind === "ecole") return "yellow";
  if (kind === "college") return "sky";
  if (kind === "lycee") return "pink";
  return "violet";
}

function cloneSections(sections: RentreeSection[]): RentreeSection[] {
  return sections.map((s) => ({
    title: s.title,
    items: s.items.map((it) => ({ ...it })),
  }));
}

function legacyForKind(kind: EstablishmentKind | undefined, legacy: RentreeLinksByLevel[]): RentreeLinksByLevel | undefined {
  if (!kind || kind === "custom") return undefined;
  return legacy.find((l) => l.level === kind);
}

function pageFromLegacy(est: Establishment, legacy: RentreeLinksByLevel): RentreeEstablishmentPage {
  return {
    establishmentId: est.id,
    label: legacy.label || est.label,
    accent: legacy.accent || defaultAccentForKind(est.kind),
    sections: normalizeRentreeSections(cloneSections(legacy.sections)),
  };
}

function rentreePageHasItems(sections: RentreeSection[]): boolean {
  return sections.some((s) => s.items.length > 0);
}

function pageFromExisting(
  est: Establishment,
  page: RentreeEstablishmentPage,
  legacyLinks: RentreeLinksByLevel[],
): RentreeEstablishmentPage {
  let sections =
    Array.isArray(page.sections) && page.sections.length > 0
      ? normalizeRentreeSections(cloneSections(page.sections))
      : DEFAULT_RENTREE_SECTIONS.map((s) => ({ ...s, items: [] }));

  if (!rentreePageHasItems(sections)) {
    const legacy = legacyForKind(est.kind, legacyLinks);
    if (legacy && rentreePageHasItems(legacy.sections)) {
      sections = normalizeRentreeSections(cloneSections(legacy.sections));
    }
  }

  return {
    establishmentId: est.id,
    label: String(page.label || est.label).trim() || est.label,
    accent: page.accent || defaultAccentForKind(est.kind),
    sections,
  };
}

/**
 * Aligne les pages rentrée sur les établissements actifs des paramètres généraux.
 * Conserve le contenu existant ; migre l'ancien format par niveau si besoin.
 */
export function syncRentreePages(
  establishments: Establishment[],
  pages: RentreeEstablishmentPage[] = [],
  legacyLinks: RentreeLinksByLevel[] = RENTREE_LINKS,
): RentreeEstablishmentPage[] {
  const active = getActiveEstablishments(establishments);
  if (active.length === 0) {
    return pages.length > 0
      ? pages.map((p) => pageFromExisting({ id: p.establishmentId, label: p.label }, p, legacyLinks))
      : [];
  }

  return active.map((est) => {
    const existing = pages.find((p) => p.establishmentId === est.id);
    if (existing) return pageFromExisting(est, existing, legacyLinks);

    const sameKindWithContent = pages.find((p) => {
      if (p.establishmentId === est.id || !rentreePageHasItems(p.sections)) return false;
      const linked = establishments.find((e) => e.id === p.establishmentId);
      return linked?.kind === est.kind;
    });
    if (sameKindWithContent) return pageFromExisting(est, sameKindWithContent, legacyLinks);

    const legacy = legacyForKind(est.kind, legacyLinks);
    if (legacy) return pageFromLegacy(est, legacy);
    const blank = emptyRentreePage(est.id, est.label);
    blank.accent = defaultAccentForKind(est.kind);
    return blank;
  });
}

export function parseRentreeAccent(raw: unknown, fallback: RentreeAccent = "violet"): RentreeAccent {
  const accents: RentreeAccent[] = ["yellow", "sky", "pink", "green", "blue", "rose", "violet", "amber", "teal"];
  return accents.includes(raw as RentreeAccent) ? (raw as RentreeAccent) : fallback;
}

export function parseRentreeSections(raw: unknown): RentreeSection[] {
  if (!Array.isArray(raw)) return [];
  const sections: RentreeSection[] = [];
  for (const s of raw) {
    const sec = s && typeof s === "object" ? (s as Record<string, unknown>) : {};
    const items: RentreeLinkItem[] = Array.isArray(sec.items)
      ? sec.items.flatMap((it) => {
          const row = it && typeof it === "object" ? (it as Record<string, unknown>) : {};
          const href = String(row.href || "").trim();
          const title = String(row.title || "").trim();
          if (!title || !href) return [];
          const item: RentreeLinkItem = {
            title,
            href,
            description: String(row.description || "").trim() || undefined,
            kind: row.kind === "pdf" ? "pdf" : row.kind === "link" ? "link" : undefined,
          };
          return [item];
        })
      : [];
    const title = String(sec.title || "").trim();
    if (!title) continue;
    sections.push({ title, items });
  }
  return sections;
}

export function parseRentreePages(raw: unknown): RentreeEstablishmentPage[] {
  if (!Array.isArray(raw)) return [];
  return raw
    .map((item) => {
      const o = item && typeof item === "object" ? (item as Record<string, unknown>) : {};
      const establishmentId = String(o.establishmentId || "").trim();
      if (!establishmentId) return null;
      const sections = parseRentreeSections(o.sections);
      return {
        establishmentId,
        label: String(o.label || "").trim(),
        accent: parseRentreeAccent(o.accent),
        sections,
      };
    })
    .filter((x): x is RentreeEstablishmentPage => Boolean(x));
}

export function parseRentreeLinks(raw: unknown): RentreeLinksByLevel[] {
  if (!Array.isArray(raw) || raw.length === 0) return RENTREE_LINKS;
  return raw
    .map((item) => {
      const o = item && typeof item === "object" ? (item as Record<string, unknown>) : {};
      const level: RentreeLevel = o.level === "college" || o.level === "lycee" ? o.level : "ecole";
      return {
        level,
        label: String(o.label || level).trim(),
        accent: parseRentreeAccent(o.accent, defaultAccentForKind(level)),
        sections: parseRentreeSections(o.sections),
      };
    })
    .filter((x) => x.sections.length > 0);
}

/** Résout l'établissement depuis ?establishment= ou l'ancien ?level=. */
export function resolveRentreePageId(
  pages: RentreeEstablishmentPage[],
  establishments: Establishment[],
  params: { establishment?: string | null; level?: string | null },
): string | null {
  if (pages.length === 0) return null;
  const byId = params.establishment && pages.find((p) => p.establishmentId === params.establishment);
  if (byId) return byId.establishmentId;

  const level = params.level;
  if (level === "ecole" || level === "college" || level === "lycee") {
    const est = getActiveEstablishments(establishments).find((e) => e.kind === level);
    if (est) {
      const page = pages.find((p) => p.establishmentId === est.id);
      if (page) return page.establishmentId;
    }
    const legacyPage = pages.find((p) => {
      const est = establishments.find((e) => e.id === p.establishmentId);
      return est?.kind === level;
    });
    if (legacyPage) return legacyPage.establishmentId;
  }

  return pages[0]?.establishmentId ?? null;
}
