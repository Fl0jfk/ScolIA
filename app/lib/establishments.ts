import type { ClerkLikeUser } from "@/app/lib/clerk-user-types";
import { loadAppConfig, getEstablishmentByLabel } from "@/app/lib/app-config";
import {
  canSignForEstablishmentLabel,
  userRoleSlugs,
} from "@/app/lib/establishment-sign-permissions";

export async function canSignTravelsDirectionForEtab(
  user: ClerkLikeUser | null | undefined,
  etablissement: string | null | undefined,
): Promise<boolean> {
  const bundle = await loadAppConfig();
  return canSignForEstablishmentLabel(user, bundle.establishments, etablissement);
}

export async function resolveDirectorForEstablishment(
  etablissementLabel: string | null | undefined,
): Promise<{ label: string; directrice: string; email: string }> {
  const bundle = await loadAppConfig();
  const est = getEstablishmentByLabel(bundle, etablissementLabel || "");
  if (est) {
    return {
      label: est.label,
      directrice: est.directorName || est.label,
      email: est.directorEmail || "",
    };
  }
  return {
    label: bundle.identity.shortName || bundle.identity.name,
    directrice: bundle.identity.name,
    email: "",
  };
}
