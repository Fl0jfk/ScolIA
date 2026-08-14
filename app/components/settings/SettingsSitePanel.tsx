"use client";

import type { Dispatch, SetStateAction } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import {
  SettingsField,
  SettingsNotice,
  SettingsSection,
  settingsInputClass,
} from "@/app/components/settings/SettingsChrome";
import { dash } from "@/app/lib/dashboard-brand";
import { DASHBOARD_ACCENT_OPTIONS } from "@/app/lib/dashboard-brand-presets";
import { PLATFORM_ASSISTANCE_EMAIL } from "@/app/lib/platform-assistance-email";

export default function SettingsSitePanel({
  identity,
  setIdentity,
  headerLogoPreviewUrl,
  uploadingLogo,
  saving,
  uploadHeaderLogo,
  removeHeaderLogo,
  saveSection,
}: {
  identity: Record<string, unknown>;
  setIdentity: Dispatch<SetStateAction<Record<string, unknown>>>;
  headerLogoPreviewUrl: string | null;
  uploadingLogo: boolean;
  saving: boolean;
  uploadHeaderLogo: (file: File) => void;
  removeHeaderLogo: () => void;
  saveSection: (section: string, body: unknown) => Promise<void>;
}) {
  const orgKind = String(identity.organizationKind || "standalone") === "groupe" ? "groupe" : "standalone";
  const holidayZone = String(identity.schoolHolidayZone || "");
  const customWebsite = identity.customWebsite as { enabled?: boolean; primaryDomain?: string } | undefined;
  const address = identity.address as { street?: string; zip?: string; city?: string; latitude?: number; longitude?: number } | undefined;
  const hasGps = address?.latitude != null && address?.longitude != null;

  return (
    <div className="space-y-4">
      <SettingsSection
        icon="🏫"
        title="Identité du groupe"
        description="Nom, type d’organisation et coordonnées affichées dans l’intranet. Les établissements (école, collège, lycée…) se créent ensuite dans Sites / directions."
      >
        <div className="grid gap-4 sm:grid-cols-2">
          <SettingsField label="Nom du groupe">
            <input
              className={settingsInputClass}
              value={String(identity.name || "")}
              onChange={(e) => setIdentity({ ...identity, name: e.target.value })}
            />
          </SettingsField>
          <SettingsField label="Nom court">
            <input
              className={settingsInputClass}
              value={String(identity.shortName || "")}
              onChange={(e) => setIdentity({ ...identity, shortName: e.target.value })}
            />
          </SettingsField>
        </div>

        <div>
          <p className={`mb-2 text-[11px] font-semibold uppercase tracking-[0.14em] ${dash.textMid}`}>
            Type d&apos;organisation
          </p>
          <div className="grid gap-3 sm:grid-cols-2">
            {(
              [
                { id: "standalone" as const, emoji: "📍", label: "Établissement unique", hint: "Un seul site" },
                { id: "groupe" as const, emoji: "🗺️", label: "Groupe scolaire", hint: "Plusieurs sites" },
              ] as const
            ).map((opt) => {
              const selected = orgKind === opt.id;
              return (
                <button
                  key={opt.id}
                  type="button"
                  onClick={() => setIdentity({ ...identity, organizationKind: opt.id })}
                  className={`flex items-start gap-3 rounded-2xl border px-4 py-3 text-left transition ${
                    selected
                      ? "border-[color:var(--dash-primary)]/40 bg-white/90 shadow-sm ring-2 ring-[color:var(--dash-soft)]"
                      : "border-white/70 bg-white/40 hover:bg-white/70"
                  }`}
                >
                  <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-2xl bg-white/80 text-lg shadow-sm ring-1 ring-white/80">
                    {opt.emoji}
                  </span>
                  <span>
                    <span className={`block text-sm font-semibold ${dash.ink}`}>{opt.label}</span>
                    <span className={`mt-0.5 block text-xs ${dash.textMid}`}>{opt.hint}</span>
                  </span>
                </button>
              );
            })}
          </div>
        </div>
      </SettingsSection>

      <SettingsSection
        icon="📌"
        title="Adresse et calendrier"
        description="Le widget météo du tableau de bord utilise cette adresse. Les coordonnées GPS sont calculées à l’enregistrement."
      >
        <SettingsField label="Adresse (rue)">
          <input
            className={settingsInputClass}
            value={String(address?.street || "")}
            onChange={(e) =>
              setIdentity({
                ...identity,
                address: { ...(identity.address as object), street: e.target.value },
              })
            }
          />
        </SettingsField>
        <div className="grid grid-cols-2 gap-3">
          <SettingsField label="Code postal">
            <input
              className={settingsInputClass}
              value={String(address?.zip || "")}
              onChange={(e) =>
                setIdentity({
                  ...identity,
                  address: { ...(identity.address as object), zip: e.target.value },
                })
              }
            />
          </SettingsField>
          <SettingsField label="Ville">
            <input
              className={settingsInputClass}
              value={String(address?.city || "")}
              onChange={(e) =>
                setIdentity({
                  ...identity,
                  address: { ...(identity.address as object), city: e.target.value },
                })
              }
            />
          </SettingsField>
        </div>

        {hasGps ? (
          <SettingsNotice tone="ok">
            Coordonnées GPS : {address!.latitude!.toFixed(4)}, {address!.longitude!.toFixed(4)} — météo active
          </SettingsNotice>
        ) : (
          <SettingsNotice tone="warn">
            Coordonnées GPS manquantes — enregistrez l&apos;identité du site pour activer la météo.
          </SettingsNotice>
        )}

        <div>
          <p className={`mb-2 text-[11px] font-semibold uppercase tracking-[0.14em] ${dash.textMid}`}>
            Zone de vacances scolaires
          </p>
          <p className={`mb-3 text-xs ${dash.textMid}`}>
            Calendrier officiel MEN (zones A, B ou C). Exemple : Normandie = zone B.
          </p>
          <div className="grid grid-cols-3 gap-2">
            {(["A", "B", "C"] as const).map((z) => {
              const selected = holidayZone === z;
              const hint =
                z === "A" ? "Lyon, Clermont…" : z === "B" ? "Normandie, Lille…" : "Paris, Créteil…";
              return (
                <button
                  key={z}
                  type="button"
                  onClick={() => setIdentity({ ...identity, schoolHolidayZone: z })}
                  className={`rounded-2xl border px-3 py-3 text-center transition ${
                    selected
                      ? "border-[color:var(--dash-primary)]/40 bg-white/90 shadow-sm ring-2 ring-[color:var(--dash-soft)]"
                      : "border-white/70 bg-white/40 hover:bg-white/70"
                  }`}
                >
                  <span className={`block text-base font-semibold ${dash.ink}`}>Zone {z}</span>
                  <span className={`mt-0.5 block text-[11px] ${dash.textMid}`}>{hint}</span>
                </button>
              );
            })}
          </div>
        </div>
        {!holidayZone ? (
          <SettingsNotice tone="warn">
            Zone non configurée — les vacances scolaires ne s&apos;afficheront pas sur les plannings tant
            qu&apos;une zone n&apos;est pas enregistrée.
          </SettingsNotice>
        ) : null}
      </SettingsSection>

      <SettingsSection
        icon="🎨"
        title="Apparence"
        description="Logo du header et teinte du tableau de bord (boutons, titres, tuiles)."
      >
        <div className="rounded-2xl border border-dashed border-white/80 bg-white/40 p-4">
          <p className={`text-[11px] font-semibold uppercase tracking-[0.14em] ${dash.textMid}`}>
            Logo du header
          </p>
          <p className={`mt-1 text-xs ${dash.textMid}`}>PNG, JPEG, WebP ou SVG — affiché en haut à gauche sur tout le site.</p>
          {identity.headerLogoUrl ? (
            <div className="mt-3 flex flex-wrap items-center gap-4">
              {headerLogoPreviewUrl ? (
                // eslint-disable-next-line @next/next/no-img-element
                <img
                  src={headerLogoPreviewUrl}
                  alt="Aperçu logo"
                  className="max-h-16 w-auto rounded-xl bg-white/80 object-contain p-2 ring-1 ring-white/80"
                />
              ) : (
                <p className="text-xs text-amber-700">Aperçu indisponible (clé S3 enregistrée).</p>
              )}
              <button
                type="button"
                onClick={removeHeaderLogo}
                disabled={saving || uploadingLogo}
                className="text-xs font-semibold text-rose-600 underline disabled:opacity-50"
              >
                Supprimer le logo personnalisé
              </button>
            </div>
          ) : (
            <p className={`mt-2 text-xs italic ${dash.textMid}`}>Logo par défaut actuellement utilisé.</p>
          )}
          <input
            type="file"
            accept="image/png,image/jpeg,image/webp,image/svg+xml"
            disabled={uploadingLogo}
            onChange={(e) => {
              const file = e.target.files?.[0];
              if (file) void uploadHeaderLogo(file);
              e.target.value = "";
            }}
            className="mt-3 block w-full text-sm text-slate-600 file:mr-4 file:rounded-xl file:border-0 file:bg-[color:var(--dash-soft)] file:px-4 file:py-2 file:font-semibold file:text-[var(--dash-primary)] hover:file:brightness-95 disabled:opacity-50"
          />
          {uploadingLogo ? (
            <p className={`mt-2 text-xs font-medium ${dash.textPrimary}`}>Envoi du logo en cours…</p>
          ) : null}
        </div>

        <div>
          <p className={`mb-2 text-[11px] font-semibold uppercase tracking-[0.14em] ${dash.textMid}`}>
            Couleur du tableau de bord
          </p>
          <div className="flex flex-wrap gap-2">
            {DASHBOARD_ACCENT_OPTIONS.map((opt) => {
              const selected = String(identity.dashboardAccent || "green") === opt.id;
              return (
                <button
                  key={opt.id}
                  type="button"
                  onClick={() => setIdentity({ ...identity, dashboardAccent: opt.id })}
                  className={`flex items-center gap-2 rounded-2xl border px-3 py-2 text-sm font-semibold transition ${
                    selected
                      ? "border-[color:var(--dash-primary)]/40 bg-white/90 text-[var(--dash-ink)] shadow-sm ring-2 ring-[color:var(--dash-soft)]"
                      : "border-white/70 bg-white/40 text-slate-700 hover:bg-white/70"
                  }`}
                >
                  <span
                    className="h-5 w-5 shrink-0 rounded-full border border-black/10 shadow-sm"
                    style={{ backgroundColor: opt.swatch }}
                    aria-hidden
                  />
                  {opt.label}
                </button>
              );
            })}
          </div>
        </div>
      </SettingsSection>

      <SettingsSection
        icon="🌐"
        title="Site vitrine et assistance"
        description="Activez uniquement si un site Next.js packagé a été livré pour cet établissement (onglet Actus dans Communication)."
      >
        <label className="flex cursor-pointer items-center gap-3 rounded-2xl border border-white/70 bg-white/40 px-4 py-3">
          <input
            type="checkbox"
            className="h-4 w-4 accent-[var(--dash-primary)]"
            checked={Boolean(customWebsite?.enabled)}
            onChange={(e) =>
              setIdentity({
                ...identity,
                customWebsite: {
                  ...((identity.customWebsite as object) || {}),
                  enabled: e.target.checked,
                  primaryDomain: customWebsite?.primaryDomain || "",
                },
              })
            }
          />
          <span className={`text-sm font-semibold ${dash.ink}`}>Site vitrine Scola activé</span>
        </label>
        <SettingsField label="Domaine principal (optionnel)">
          <input
            className={settingsInputClass}
            placeholder="www.ecole.fr"
            value={String(customWebsite?.primaryDomain || "")}
            onChange={(e) =>
              setIdentity({
                ...identity,
                customWebsite: {
                  enabled: Boolean(customWebsite?.enabled),
                  primaryDomain: e.target.value,
                },
              })
            }
          />
        </SettingsField>
        <SettingsField label="E-mail assistance technique" hint="Adresse plateforme, non modifiable ici.">
          <input className={`${settingsInputClass} bg-white/40 text-slate-500`} type="email" value={PLATFORM_ASSISTANCE_EMAIL} readOnly />
        </SettingsField>
      </SettingsSection>

      <div className="flex justify-end pt-1">
        <ModuleButton
          variant="primary"
          disabled={saving}
          className="rounded-2xl px-5 shadow-[0_12px_28px_-16px_rgba(15,23,42,0.55)]"
          onClick={() => saveSection("site", identity)}
        >
          {saving ? "Enregistrement…" : "Enregistrer l'identité"}
        </ModuleButton>
      </div>
    </div>
  );
}
