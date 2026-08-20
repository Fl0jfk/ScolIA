"use client";

import type { SiteIdentity } from "@/app/lib/app-config-schemas";
import { OnboardingField, onboardingInputClass } from "@/app/components/onboarding/OnboardingShell";
import { dash } from "@/app/lib/dashboard-brand";
import { MARKETING } from "@/app/lib/marketing-site";

type Props = {
  identity: Partial<SiteIdentity>;
  onChange: (patch: Partial<SiteIdentity>) => void;
  isHero?: boolean;
};

export default function ChapterWelcome({ identity, onChange, isHero }: Props) {
  if (isHero && !identity.name?.trim()) {
    return (
      <div className="space-y-8 py-4 text-center sm:py-8">
        <div className="mx-auto flex h-16 w-16 items-center justify-center rounded-3xl bg-gradient-to-br from-[var(--dash-primary)] to-[var(--dash-dark)] text-2xl font-black text-white shadow-lg shadow-[color:var(--dash-primary)]/30">
          S
        </div>
        <div>
          <p className={`text-[10px] font-bold uppercase tracking-[0.28em] ${dash.label}`}>
            {MARKETING.productName}
          </p>
          <h2 className={`mt-3 text-2xl font-semibold tracking-tight sm:text-3xl ${dash.ink}`}>
            Bienvenue dans votre intranet
          </h2>
          <p className={`mx-auto mt-3 max-w-md text-sm leading-relaxed ${dash.textMid}`}>
            En quelques minutes, vous allez paramétrer l&apos;identité de votre établissement, les
            directions et les contacts métier. Tout restera modifiable dans les paramètres.
          </p>
        </div>
        <OnboardingField label="Nom affiché de la plateforme">
          <input
            className={`${onboardingInputClass} text-center`}
            value={identity.name || ""}
            onChange={(e) => onChange({ name: e.target.value })}
            placeholder="Ex. Groupe scolaire Notre-Dame"
            autoFocus
          />
        </OnboardingField>
        <OnboardingField label="Type d'organisation">
          <div className="grid gap-3 sm:grid-cols-2">
            {(
              [
                {
                  value: "standalone" as const,
                  title: "Établissement unique",
                  desc: "Un seul niveau (école, collège ou lycée).",
                },
                {
                  value: "groupe" as const,
                  title: "Groupe scolaire",
                  desc: "Plusieurs établissements / niveaux.",
                },
              ] as const
            ).map((opt) => {
              const selected = (identity.organizationKind || "standalone") === opt.value;
              return (
                <button
                  key={opt.value}
                  type="button"
                  onClick={() => onChange({ organizationKind: opt.value })}
                  className={`rounded-2xl border px-4 py-4 text-left transition ${
                    selected
                      ? "border-[color:var(--dash-primary)] bg-[color:var(--dash-soft-muted)] shadow-sm ring-2 ring-[color:var(--dash-bright)]/35"
                      : "border-white/70 bg-white/60 hover:border-[color:var(--dash-primary)]/35"
                  }`}
                >
                  <span className={`block text-sm font-semibold ${dash.ink}`}>{opt.title}</span>
                  <span className={`mt-1 block text-xs ${dash.textMid}`}>{opt.desc}</span>
                </button>
              );
            })}
          </div>
        </OnboardingField>
      </div>
    );
  }

  return (
    <div className="space-y-1">
      <OnboardingField label="Nom affiché de la plateforme">
        <input
          className={onboardingInputClass}
          value={identity.name || ""}
          onChange={(e) => onChange({ name: e.target.value })}
          placeholder="Ex. Groupe scolaire Notre-Dame"
        />
      </OnboardingField>
      <OnboardingField label="Type d'organisation">
        <select
          className={onboardingInputClass}
          value={identity.organizationKind || "standalone"}
          onChange={(e) =>
            onChange({ organizationKind: e.target.value as "standalone" | "groupe" })
          }
        >
          <option value="standalone">Un seul établissement</option>
          <option value="groupe">Groupe scolaire (plusieurs niveaux)</option>
        </select>
      </OnboardingField>
    </div>
  );
}
