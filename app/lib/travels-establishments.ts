import type { AppConfigBundle, Establishment } from "@/app/lib/app-config-schemas";
import { shouldShowGroupeScolaire } from "@/app/lib/app-config-establishments";
import { GROUPE_SCOLAIRE_LABEL, matchEstablishment } from "@/app/lib/establishment-catalog";

export { GROUPE_SCOLAIRE_LABEL };

type TravelsEstablishmentOption = {
  id: string;
  label: string;
  isGroupe?: boolean;
};

function getTravelsEstablishmentOptions(config: AppConfigBundle): TravelsEstablishmentOption[] {
  const active = config.establishments.filter((e) => e.active !== false);
  const options: TravelsEstablishmentOption[] = active.map((e) => ({
    id: e.id,
    label: e.label,
  }));
  if (shouldShowGroupeScolaire(active)) {
    options.push({ id: "groupe_scolaire", label: GROUPE_SCOLAIRE_LABEL, isGroupe: true });
  }
  return options;
}

function getTravelsFilterLabels(config: AppConfigBundle): string[] {
  const opts = getTravelsEstablishmentOptions(config);
  return ["Tous", ...opts.map((o) => o.label)];
}

function resolveEstablishmentFromTrip(
  config: AppConfigBundle,
  trip: { establishmentId?: string; etablissement?: string },
): Establishment | null {
  if (trip.establishmentId) {
    const byId = config.establishments.find((e) => e.id === trip.establishmentId);
    if (byId) return byId;
  }
  const label = trip.etablissement?.trim();
  if (!label || label === GROUPE_SCOLAIRE_LABEL) return null;
  return matchEstablishment(config.establishments, label);
}

function tripEstablishmentLabel(
  config: AppConfigBundle,
  trip: { establishmentId?: string; etablissement?: string },
): string {
  const resolved = resolveEstablishmentFromTrip(config, trip);
  if (resolved) return resolved.label;
  const label = trip.etablissement?.trim();
  if (label) return label;
  if (shouldShowGroupeScolaire(config.establishments)) return GROUPE_SCOLAIRE_LABEL;
  return config.establishments[0]?.label || "Établissement";
}

function zeendocButtonLabel(config: AppConfigBundle): string {
  const z = config.integrations.zeendoc;
  if (z?.enabled && z.buttonLabel?.trim()) return z.buttonLabel.trim();
  if (z?.enabled) return "Envoyer sur Zeendoc";
  return z?.buttonLabel?.trim() || "Envoyer par mail";
}

export function zeendocDestinationEmail(config: AppConfigBundle): string | null {
  const z = config.integrations.zeendoc;
  if (z?.destinationEmail?.trim()) return z.destinationEmail.trim();
  if (config.notifications.travelsZeendoc?.trim()) return config.notifications.travelsZeendoc.trim();
  return null;
}
