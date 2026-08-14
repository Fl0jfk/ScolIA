import { scolaImageUrl } from "@/app/lib/scola-image";
import type { RentreeEstablishmentPage, RentreeLinksByLevel, RentreeSection } from "@/app/lib/rentree-types";

const RENTREE_SECTION_CALENDRIER = "Calendrier";
const RENTREE_SECTION_INFOS = "Infos pratiques";
export const RENTREE_SECTION_INTERNAT = "Internat";

export const DEFAULT_RENTREE_SECTIONS: RentreeSection[] = [
  { title: RENTREE_SECTION_CALENDRIER, items: [] },
  { title: RENTREE_SECTION_INFOS, items: [] },
  { title: RENTREE_SECTION_INTERNAT, items: [] },
];

export function isInternatRentreeSection(section: RentreeSection): boolean {
  return section.title.trim().toLowerCase() === "internat";
}

/** Aligne titres legacy et garantit la rubrique Internat si absente (sans réordonner). */
export function normalizeRentreeSections(sections: RentreeSection[]): RentreeSection[] {
  const normalized = sections.map((s) => ({
    title: s.title.trim().toLowerCase() === "documents" ? RENTREE_SECTION_INFOS : s.title.trim(),
    items: s.items.map((it) => ({ ...it })),
  }));

  if (!normalized.some((s) => isInternatRentreeSection(s))) {
    normalized.push({ title: RENTREE_SECTION_INTERNAT, items: [] });
  }

  return normalized;
}

/**
 * Documents publics par défaut (La Providence — migration legacy).
 * Les nouveaux tenants démarrent avec des sections vides synchronisées aux établissements.
 */
export const RENTREE_LINKS: RentreeLinksByLevel[] = [
  {
    level: "ecole",
    label: "École",
    accent: "yellow",
    sections: [
      {
        title: "Calendrier",
        items: [
          {
            title: "Circulaire",
            description: "Calendrier scolaire, jours fériés et fermetures spécifiques à l'établissement.",
            href: "/documents/rentree/ecole/calendrier-annee.pdf",
            kind: "pdf",
          },
        ],
      },
      {
        title: RENTREE_SECTION_INFOS,
        items: [
          {
            title: "Organisation de la rentrée",
            description: "Horaires, accueil, contacts, informations clés.",
            href: "/documents/rentree/ecole/rentree-infos.pdf",
            kind: "pdf",
          },
          {
            title: "Fournitures (simulateur)",
            description: "Simuler / imprimer la liste selon la classe.",
            href: "/simulateurFournitures",
            kind: "link",
          },
          {
            title: "Librairie Colbert – Commander les fournitures",
            description: "Commande groupée en partenariat avec l'APEL de l'établissement.",
            href: scolaImageUrl("rentree/Colbert.pdf"),
            kind: "pdf",
          },
        ],
      },
      {
        title: RENTREE_SECTION_INTERNAT,
        items: [],
      },
    ],
  },
  {
    level: "college",
    label: "Collège",
    accent: "sky",
    sections: [
      {
        title: "Calendrier",
        items: [
          {
            title: "Circulaire",
            description: "Calendrier scolaire, jours fériés et fermetures spécifiques à l'établissement.",
            href: "/documents/rentree/college/calendrier-annee.pdf",
            kind: "pdf",
          },
        ],
      },
      {
        title: RENTREE_SECTION_INFOS,
        items: [
          {
            title: "Organisation de la rentrée",
            description: "Horaires, remise des manuels, consignes.",
            href: "/documents/rentree/college/rentree-infos.pdf",
            kind: "pdf",
          },
          {
            title: "Fournitures (simulateur)",
            description: "Simuler / imprimer la liste selon la classe.",
            href: "/simulateurFournitures",
            kind: "link",
          },
          {
            title: "Librairie Colbert – Commander les fournitures",
            description: "Commande groupée en partenariat avec l'APEL de l'établissement.",
            href: scolaImageUrl("rentree/Colbert.pdf"),
            kind: "pdf",
          },
        ],
      },
      {
        title: RENTREE_SECTION_INTERNAT,
        items: [],
      },
    ],
  },
  {
    level: "lycee",
    label: "Lycée",
    accent: "pink",
    sections: [
      {
        title: "Calendrier",
        items: [
          {
            title: "Circulaire",
            description: "Calendrier scolaire, jours fériés et fermetures spécifiques à l'établissement.",
            href: "/documents/rentree/lycee/calendrier-annee.pdf",
            kind: "pdf",
          },
        ],
      },
      {
        title: RENTREE_SECTION_INFOS,
        items: [
          {
            title: "Organisation de la rentrée",
            description: "Horaires, informations élèves et familles.",
            href: "/documents/rentree/lycee/rentree-infos.pdf",
            kind: "pdf",
          },
          {
            title: "Fournitures (simulateur)",
            description: "Simuler / imprimer la liste selon la classe.",
            href: "/simulateurFournitures",
            kind: "link",
          },
          {
            title: "ARBS – Location de manuels scolaires",
            description: "Système de location de livres pour les lycéens.",
            href: scolaImageUrl("rentree/Flyer-ARBS.pdf"),
            kind: "pdf",
          },
          {
            title: "Atout Normandie – Subvention sur les manuels",
            description: "Dispositif de la Région Normandie.",
            href: scolaImageUrl("rentree/Atouts+normandie.pdf"),
            kind: "pdf",
          },
        ],
      },
      {
        title: RENTREE_SECTION_INTERNAT,
        items: [],
      },
    ],
  },
];

export function emptyRentreePage(establishmentId: string, label: string): RentreeEstablishmentPage {
  return {
    establishmentId,
    label,
    accent: "violet",
    sections: DEFAULT_RENTREE_SECTIONS.map((s) => ({ ...s, items: [...s.items] })),
  };
}

export { rentreeAccentClasses } from "@/app/lib/rentree-accent-styles";
