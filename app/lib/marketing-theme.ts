import { scolaImageUrl } from "@/app/lib/scola-image";

/** Tokens visuels du site commercial Scola */
export const SCOLA = {
  green: "#2F6B4A",
  greenDark: "#1E4A32",
  greenMid: "#3D8A5C",
  greenBright: "#4ADE80",
  greenSoft: "#D4F0E0",
  cream: "#FAFAF7",
  warm: "#FFF8F0",
  amber: "#F59E0B",
  amberSoft: "#FEF3C7",
  ink: "#14231A",
  muted: "#4B6358",
} as const;

/** Dégradé texte signature (hero, logo, titres) */
export const SCOLA_GRADIENT_TEXT =
  "bg-gradient-to-r from-[#2F6B4A] via-[#3D8A5C] to-[#4ADE80] bg-clip-text text-transparent";

/** Barre de navigation marketing / auth */
export const SCOLA_HEADER_SHELL =
  "sticky top-0 z-50 border-b border-emerald-300/40 bg-gradient-to-r from-white/90 via-emerald-50/60 to-white/90 shadow-[0_4px_24px_-4px_rgba(47,107,74,0.12)] backdrop-blur-xl relative";

export const SCOLA_HEADER_ACCENT =
  "pointer-events-none absolute inset-x-0 bottom-0 h-px bg-gradient-to-r from-transparent via-[#4ADE80]/50 to-transparent";

export const WORKFLOW_IMAGES: Record<string, string> = {
  "Documents élèves":
    scolaImageUrl("categories/add Docs.png"),
  "Sorties scolaires":
    scolaImageUrl("categories/transport.avif"),
  "Réservation de salles":
    scolaImageUrl("categories/reservationsalle.jpg"),
  Absences:
    scolaImageUrl("categories/planning abs.jpg"),
  "RH & personnel":
    scolaImageUrl("categories/Organigramme.jpg"),
  Internat:
    scolaImageUrl("categories/Internat.jpg"),
};

/** Hauteur fixe des cartes animation — évite le saut de layout entre les phases. */
export const WORKFLOW_ANIMATION_SHELL =
  "relative flex h-[24rem] flex-col overflow-hidden rounded-3xl p-1 shadow-2xl sm:h-[25rem]";

export const WORKFLOW_ANIMATION_INNER =
  "relative flex min-h-0 flex-1 flex-col rounded-[1.35rem] p-4 backdrop-blur-sm sm:p-5";

/** Zone de contenu variable (phases) — hauteur réservée */
export const WORKFLOW_ANIMATION_BODY = "min-h-0 flex-1 overflow-hidden";

export const TILE_IMAGES: Record<string, string> = {
  documents:
    scolaImageUrl("categories/classeur.jpg"),
  "docs-eleves":
    scolaImageUrl("categories/add Docs.png"),
  sorties:
    scolaImageUrl("categories/transport.avif"),
  absences:
    scolaImageUrl("categories/planning abs.jpg"),
  salles:
    scolaImageUrl("categories/reservationsalle.jpg"),
  rh: scolaImageUrl("categories/Organigramme.jpg"),
  internat:
    scolaImageUrl("categories/Internat.jpg"),
  demandes:
    scolaImageUrl("categories/demandes.jpg"),
  familles:
    scolaImageUrl("categories/classeur.jpg"),
};
