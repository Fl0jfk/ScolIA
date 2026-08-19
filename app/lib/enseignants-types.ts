import type { Secteur } from "@/app/lib/onedrive-eleves-types";

export type EnseignantConfig = {
  id: string;
  nom: string;
  prenom: string;
  folderName: string;
  secteur: Secteur;
  email?: string;
};
