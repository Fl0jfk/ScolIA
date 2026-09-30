"use client";

import type { SiteIdentity } from "@/app/lib/app-config-schemas";
import { OnboardingField, onboardingInputClass } from "@/app/components/onboarding/OnboardingShell";
import { dash } from "@/app/lib/dashboard-brand";

type Props = {
  identity: Partial<SiteIdentity>;
  onChange: (patch: Partial<SiteIdentity>) => void;
};

export default function ChapterIdentity({ identity, onChange }: Props) {
  return (
    <div className="space-y-1">
      <OnboardingField label="Nom court" hint="Affiché dans l’en-tête et les e-mails courts.">
        <input
          className={onboardingInputClass}
          value={identity.shortName || ""}
          onChange={(e) => onChange({ shortName: e.target.value })}
          placeholder={identity.name || "Nom court"}
        />
      </OnboardingField>

      <p className={`mb-3 text-sm ${dash.textMid}`}>
        Adresse de l&apos;établissement — elle alimente le widget météo du tableau de bord.
      </p>
      <OnboardingField label="Rue">
        <input
          className={onboardingInputClass}
          value={identity.address?.street || ""}
          onChange={(e) =>
            onChange({ address: { ...identity.address, street: e.target.value } })
          }
        />
      </OnboardingField>
      <div className="grid grid-cols-2 gap-3">
        <OnboardingField label="Code postal">
          <input
            className={onboardingInputClass}
            value={identity.address?.zip || ""}
            onChange={(e) =>
              onChange({ address: { ...identity.address, zip: e.target.value } })
            }
          />
        </OnboardingField>
        <OnboardingField label="Ville">
          <input
            className={onboardingInputClass}
            value={identity.address?.city || ""}
            onChange={(e) =>
              onChange({ address: { ...identity.address, city: e.target.value } })
            }
          />
        </OnboardingField>
      </div>
      {identity.address?.latitude != null && identity.address?.longitude != null ? (
        <p className={`text-xs ${dash.textPrimary}`}>
          Coordonnées : {identity.address.latitude.toFixed(4)},{" "}
          {identity.address.longitude.toFixed(4)}
        </p>
      ) : null}
    </div>
  );
}
