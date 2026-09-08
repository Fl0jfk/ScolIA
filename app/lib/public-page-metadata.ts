import "server-only";

import type { Metadata } from "next";
import { loadPublicSiteIdentity } from "@/app/lib/site-public";

/** Nom affiché dans l’onglet navigateur (identité tenant). */
export async function publicSiteDisplayName(): Promise<string> {
  const identity = await loadPublicSiteIdentity();
  return identity.shortName?.trim() || identity.name?.trim() || "Établissement";
}

/** Metadata de layout public : défaut « Rentrée » + template `%s — Site`. */
export async function publicRentreeLayoutMetadata(): Promise<Metadata> {
  const site = await publicSiteDisplayName();
  return {
    title: {
      default: `Rentrée — ${site}`,
      template: `%s — ${site}`,
    },
    description:
      "Informations familles : préparation de la rentrée, portes ouvertes, simulateurs de tarifs et de fournitures.",
  };
}
