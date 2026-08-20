"use client";

import type { SiteIdentity } from "@/app/lib/app-config-schemas";
import { OnboardingField, onboardingInputClass } from "@/app/components/onboarding/OnboardingShell";
import { DASHBOARD_ACCENT_OPTIONS } from "@/app/lib/dashboard-brand-presets";
import { dash } from "@/app/lib/dashboard-brand";

type Props = {
  identity: Partial<SiteIdentity>;
  onChange: (patch: Partial<SiteIdentity>) => void;
};

export default function ChapterIdentity({ identity, onChange }: Props) {
  const accent = identity.dashboardAccent || "green";

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

      <div className="mb-6">
        <span className={dash.fieldLabel}>Couleur d&apos;accent du tableau de bord</span>
        <div className="mt-2 grid grid-cols-2 gap-2 sm:grid-cols-3">
          {DASHBOARD_ACCENT_OPTIONS.map((o) => {
            const selected = accent === o.id;
            return (
              <button
                key={o.id}
                type="button"
                onClick={() => onChange({ dashboardAccent: o.id })}
                className={`flex items-center gap-2.5 rounded-2xl border px-3 py-3 text-left text-sm font-semibold transition ${
                  selected
                    ? "border-[color:var(--dash-primary)] bg-white ring-2 ring-[color:var(--dash-bright)]/40"
                    : "border-white/70 bg-white/60 hover:border-white"
                }`}
              >
                <span
                  className="h-5 w-5 shrink-0 rounded-full shadow-inner ring-1 ring-black/10"
                  style={{ backgroundColor: o.swatch }}
                />
                <span className={dash.ink}>{o.label}</span>
              </button>
            );
          })}
        </div>
      </div>

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
