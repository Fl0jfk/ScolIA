"use client";

import type { Dispatch, SetStateAction } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
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
  return (
    <div className="bg-white rounded-2xl border p-6 space-y-4">
      <div className="mb-2">
        <h2 className="text-lg font-black text-slate-900">Établissement</h2>
        <p className="text-sm text-slate-500 mt-1">
          Nom, logo, couleurs, adresse et zone de vacances — identité globale de l&apos;intranet.
        </p>
      </div>
      <label className="block text-sm font-bold text-slate-600">Nom du groupe</label>
      <input
        className="w-full border rounded-xl p-3"
        value={String(identity.name || "")}
        onChange={(e) => setIdentity({ ...identity, name: e.target.value })}
      />
      <label className="block text-sm font-bold text-slate-600">Nom court</label>
      <input
        className="w-full border rounded-xl p-3"
        value={String(identity.shortName || "")}
        onChange={(e) => setIdentity({ ...identity, shortName: e.target.value })}
      />
      <label className="block text-sm font-bold text-slate-600">
        Zone de vacances scolaires
      </label>
      <p className="text-xs text-slate-500 -mt-2">
        Calendrier officiel MEN (zones A, B ou C). Exemple : Normandie = zone B. Requis pour
        afficher les vacances sur les plannings.
      </p>
      <select
        className="w-full border rounded-xl p-3 bg-white"
        value={String(identity.schoolHolidayZone || "")}
        onChange={(e) => {
          const v = e.target.value;
          setIdentity({
            ...identity,
            schoolHolidayZone: v === "A" || v === "B" || v === "C" ? v : undefined,
          });
        }}
      >
        <option value="">— Choisir la zone —</option>
        <option value="A">Zone A (ex. Lyon, Clermont, Montpellier…)</option>
        <option value="B">Zone B (ex. Normandie, Lille, Nantes, Rennes…)</option>
        <option value="C">Zone C (ex. Paris, Versailles, Créteil…)</option>
      </select>
      {!identity.schoolHolidayZone ? (
        <p className="text-xs font-semibold text-amber-800 bg-amber-50 border border-amber-200 rounded-lg px-3 py-2">
          Zone non configurée — les vacances scolaires ne s&apos;afficheront pas sur les
          plannings tant qu&apos;une zone n&apos;est pas enregistrée.
        </p>
      ) : null}

      <div className="rounded-xl border border-sky-200 bg-sky-50/60 p-4 space-y-3">
        <div>
          <p className="text-sm font-black text-sky-950">Site vitrine Scola</p>
          <p className="text-xs text-sky-900/80 mt-1">
            Activez uniquement si un site Next.js packagé a été livré pour cet établissement.
            Cela débloque l&apos;onglet Actus dans Communication.
          </p>
        </div>
        <label className="flex items-center gap-2 text-sm font-semibold text-slate-800 cursor-pointer">
          <input
            type="checkbox"
            className="h-4 w-4"
            checked={Boolean((identity.customWebsite as { enabled?: boolean } | undefined)?.enabled)}
            onChange={(e) =>
              setIdentity({
                ...identity,
                customWebsite: {
                  ...((identity.customWebsite as object) || {}),
                  enabled: e.target.checked,
                  primaryDomain:
                    (identity.customWebsite as { primaryDomain?: string } | undefined)
                      ?.primaryDomain || "",
                },
              })
            }
          />
          Site vitrine Scola activé
        </label>
        <label className="block text-sm font-bold text-slate-600">
          Domaine principal (optionnel)
          <input
            className="mt-1 w-full border rounded-xl p-3 bg-white font-normal"
            placeholder="www.ecole.fr"
            value={String(
              (identity.customWebsite as { primaryDomain?: string } | undefined)
                ?.primaryDomain || "",
            )}
            onChange={(e) =>
              setIdentity({
                ...identity,
                customWebsite: {
                  enabled: Boolean(
                    (identity.customWebsite as { enabled?: boolean } | undefined)?.enabled,
                  ),
                  primaryDomain: e.target.value,
                },
              })
            }
          />
        </label>
      </div>

      <label className="block text-sm font-bold text-slate-600">E-mail assistance technique</label>
      <input
        className="w-full border rounded-xl p-3 bg-slate-50 text-slate-600"
        type="email"
        value={PLATFORM_ASSISTANCE_EMAIL}
        readOnly
      />
      <label className="block text-sm font-bold text-slate-600">Adresse (rue)</label>
      <input
        className="w-full border rounded-xl p-3"
        value={String((identity.address as { street?: string })?.street || "")}
        onChange={(e) =>
          setIdentity({
            ...identity,
            address: { ...(identity.address as object), street: e.target.value },
          })
        }
      />
      <div className="grid grid-cols-2 gap-3">
        <div>
          <label className="block text-sm font-bold text-slate-600">Code postal</label>
          <input
            className="w-full border rounded-xl p-3"
            value={String((identity.address as { zip?: string })?.zip || "")}
            onChange={(e) =>
              setIdentity({
                ...identity,
                address: { ...(identity.address as object), zip: e.target.value },
              })
            }
          />
        </div>
        <div>
          <label className="block text-sm font-bold text-slate-600">Ville</label>
          <input
            className="w-full border rounded-xl p-3"
            value={String((identity.address as { city?: string })?.city || "")}
            onChange={(e) =>
              setIdentity({
                ...identity,
                address: { ...(identity.address as object), city: e.target.value },
              })
            }
          />
        </div>
      </div>

      <p className="text-xs text-slate-500">
        Le widget météo du tableau de bord utilise cette adresse. Les coordonnées GPS sont calculées
        automatiquement à l&apos;enregistrement.
      </p>
      {(identity.address as { latitude?: number; longitude?: number })?.latitude != null &&
      (identity.address as { latitude?: number; longitude?: number })?.longitude != null ? (
        <p className="text-xs font-semibold text-emerald-700 bg-emerald-50 border border-emerald-200 rounded-lg px-3 py-2">
          Coordonnées GPS :{" "}
          {(identity.address as { latitude: number }).latitude.toFixed(4)},{" "}
          {(identity.address as { longitude: number }).longitude.toFixed(4)} — météo active
        </p>
      ) : (
        <p className="text-xs font-semibold text-amber-800 bg-amber-50 border border-amber-200 rounded-lg px-3 py-2">
          Coordonnées GPS manquantes — enregistrez l&apos;identité du site pour activer la météo.
        </p>
      )}

      <div className="pt-2 border-t border-slate-100 space-y-3">
        <label className="block text-sm font-bold text-slate-600">Logo du header (haut gauche)</label>
        <p className="text-xs text-slate-500">
          PNG, JPEG, WebP ou SVG. Le fichier est enregistré sur S3 et affiché sur tout le site.
        </p>
        {identity.headerLogoUrl ? (
          <div className="flex items-center gap-4 p-4 bg-slate-50 rounded-xl border">
            {headerLogoPreviewUrl ? (
              // eslint-disable-next-line @next/next/no-img-element
              <img
                src={headerLogoPreviewUrl}
                alt="Aperçu logo"
                className="max-h-16 w-auto object-contain"
              />
            ) : (
              <p className="text-xs text-amber-700">Aperçu indisponible (clé S3 enregistrée).</p>
            )}
            <button
              type="button"
              onClick={removeHeaderLogo}
              disabled={saving || uploadingLogo}
              className="text-xs font-bold text-red-600 underline disabled:opacity-50"
            >
              Supprimer le logo personnalisé
            </button>
          </div>
        ) : (
          <p className="text-xs text-slate-400 italic">Logo par défaut actuellement utilisé.</p>
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
          className="block w-full text-sm text-slate-600 file:mr-4 file:py-2 file:px-4 file:rounded-xl file:border-0 file:bg-indigo-50 file:text-indigo-700 file:font-bold hover:file:bg-indigo-100 disabled:opacity-50"
        />
        {uploadingLogo && <p className="text-xs text-indigo-600 font-medium">Envoi du logo en cours…</p>}
      </div>

      <div className="pt-4 border-t border-slate-100 space-y-3">
        <label className="block text-sm font-bold text-slate-600">Couleur du tableau de bord</label>
        <p className="text-xs text-slate-500">
          Teinte des boutons, titres et tuiles sur la page d&apos;accueil intranet (le dégradé s&apos;adapte
          automatiquement).
        </p>
        <div className="flex flex-wrap gap-2">
          {DASHBOARD_ACCENT_OPTIONS.map((opt) => {
            const selected = (String(identity.dashboardAccent || "green")) === opt.id;
            return (
              <button
                key={opt.id}
                type="button"
                onClick={() => setIdentity({ ...identity, dashboardAccent: opt.id })}
                className={`flex items-center gap-2 rounded-xl border px-3 py-2 text-sm font-bold transition ${
                  selected
                    ? "border-indigo-600 bg-indigo-50 text-indigo-900 ring-2 ring-indigo-200"
                    : "border-slate-200 bg-white text-slate-700 hover:border-slate-300"
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

      <ModuleButton
        variant="primary"
        disabled={saving}
        onClick={() => saveSection("site", identity)}
      >
        Enregistrer l&apos;identité
      </ModuleButton>
    </div>
  );
}
