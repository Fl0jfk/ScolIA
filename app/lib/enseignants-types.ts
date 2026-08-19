import type { Secteur } from "@/app/lib/onedrive-eleves-types";

export type EnseignantConfig = {
  id: string;
  nom: string;
  prenom: string;
  folderName: string;
  secteur: Secteur;
  /** Email personnel */
  email?: string;
  /** Email professionnel (établissement) */
  emailPro?: string;
};
