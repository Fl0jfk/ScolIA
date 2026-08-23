"use client";

import type { Establishment, SiteIdentity } from "@/app/lib/app-config-schemas";
import { OnboardingField, onboardingInputClass } from "@/app/components/onboarding/OnboardingShell";
import { ESTABLISHMENT_KIND_PRESETS } from "@/app/lib/establishment-visual";
import { roleSlugsForEstablishment, directionRoleForKind } from "@/app/lib/establishment-catalog";
import DirectoryPersonSelect from "@/app/components/settings/DirectoryPersonSelect";
import type { DirectoryMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";
import { dash } from "@/app/lib/dashboard-brand";

function memberDisplayName(m: DirectoryMemberOption): string {
  return m.displayName || `${m.firstName ?? ""} ${m.lastName ?? ""}`.trim() || m.email;
}

type Props = {
  identity: Partial<SiteIdentity>;
  establishments: Establishment[];
  onChange: (list: Establishment[]) => void;
  directoryMembers: DirectoryMemberOption[];
  membersLoading: boolean;
};

export default function ChapterStructure({
  identity,
  establishments,
  onChange,
  directoryMembers,
  membersLoading,
}: Props) {
  const addPreset = (preset: (typeof ESTABLISHMENT_KIND_PRESETS)[number]) => {
    if (establishments.some((e) => e.id === preset.id)) return;
    if (identity.organizationKind === "standalone" && establishments.length >= 1) return;
    onChange([
      ...establishments,
      {
        id: preset.id,
        label: preset.label,
        kind: preset.kind,
        grades: preset.grades,
        directorName: "",
        directorEmail: "",
        directorExternalUserId: "",
        roleSlugs: roleSlugsForEstablishment(preset),
        active: true,
      },
    ]);
  };

  return (
    <div>
      <p className={`mb-4 text-sm leading-relaxed ${dash.textMid}`}>
        Ajoutez les établissements actifs. Vous pouvez indiquer une direction par e-mail même si
        la personne n&apos;a pas encore de compte — le rôle sera synchronisé à son arrivée.
      </p>

      <div className="mb-4 flex flex-wrap gap-2">
        {ESTABLISHMENT_KIND_PRESETS.filter((p) => !establishments.some((e) => e.id === p.id)).map(
          (p) => (
            <button
              key={p.id}
              type="button"
              disabled={identity.organizationKind === "standalone" && establishments.length >= 1}
              className="rounded-xl border border-[color:var(--dash-primary)]/30 bg-white/70 px-3 py-1.5 text-sm font-semibold text-[var(--dash-primary)] transition hover:bg-[color:var(--dash-soft-muted)] disabled:opacity-40"
              onClick={() => addPreset(p)}
            >
              + {p.label}
            </button>
          ),
        )}
      </div>

      {identity.organizationKind === "standalone" ? (
        <p className={`mb-4 text-xs ${dash.textMid}`}>
          Établissement unique : un seul niveau (école, collège, lycée ou autre).
        </p>
      ) : null}

      {establishments.length === 0 ? (
        <p className={`mb-4 rounded-2xl border border-dashed border-white/80 bg-white/40 px-4 py-6 text-center text-sm ${dash.textMid}`}>
          Aucun établissement pour l&apos;instant. Utilisez les boutons ci-dessus.
        </p>
      ) : null}

      {establishments.map((e, idx) => (
        <div
          key={e.id}
          className="mb-4 rounded-2xl border border-white/70 bg-white/55 p-4 shadow-sm"
        >
          <div className="mb-2 flex items-center justify-between gap-2">
            <strong className={dash.ink}>{e.label}</strong>
            <button
              type="button"
              className="text-xs font-semibold text-red-600"
              onClick={() => onChange(establishments.filter((_, i) => i !== idx))}
            >
              Retirer
            </button>
          </div>
          <p className={`mb-3 text-xs ${dash.textMid}`}>
            Rôle intranet : {directionRoleForKind(e.kind || e.id)}
          </p>

          {directoryMembers.length > 0 ? (
            <OnboardingField label="Responsable (personnel déjà invité)">
              <DirectoryPersonSelect
                members={directoryMembers}
                selectedId={e.directorExternalUserId || ""}
                loading={membersLoading}
                onChange={(member) => {
                  const copy = [...establishments];
                  if (!member) {
                    copy[idx] = { ...copy[idx], directorExternalUserId: "" };
                  } else {
                    copy[idx] = {
                      ...copy[idx],
                      directorExternalUserId: member.externalUserId,
                      directorName: memberDisplayName(member),
                      directorEmail: member.email,
                    };
                  }
                  onChange(copy);
                }}
              />
            </OnboardingField>
          ) : null}

          <OnboardingField label="Nom affiché (PDF / documents)">
            <input
              className={onboardingInputClass}
              value={e.directorName || ""}
              onChange={(ev) => {
                const copy = [...establishments];
                copy[idx] = { ...copy[idx], directorName: ev.target.value };
                onChange(copy);
              }}
              placeholder="Prénom Nom"
            />
          </OnboardingField>
          <OnboardingField
            label="E-mail direction"
            hint="Peut être renseigné avant que la personne ait un compte."
          >
            <input
              className={onboardingInputClass}
              type="email"
              value={e.directorEmail || ""}
              onChange={(ev) => {
                const copy = [...establishments];
                copy[idx] = { ...copy[idx], directorEmail: ev.target.value };
                onChange(copy);
              }}
              placeholder="direction@etablissement.fr"
            />
          </OnboardingField>
        </div>
      ))}
    </div>
  );
}
