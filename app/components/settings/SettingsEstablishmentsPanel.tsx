"use client";

import type { Dispatch, SetStateAction } from "react";
import { useMemo } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ClerkPersonSelect from "@/app/components/settings/ClerkPersonSelect";
import type { ClerkMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";
import type { EstablishmentKind } from "@/app/lib/app-config-schemas";
import {
  DEFAULT_ESTABLISHMENT_KIND_COLORS,
  ESTABLISHMENT_COLOR_SWATCHES,
  ESTABLISHMENT_KIND_OPTIONS,
  establishmentKindEmoji,
  inferEstablishmentKind,
  resolveEstablishmentColorHex,
} from "@/app/lib/establishment-visual";
import {
  emptySettingsEstablishmentForm,
  type SettingsEstablishmentForm,
} from "@/app/lib/settings-page-model";

function memberDisplayName(m: ClerkMemberOption): string {
  return m.displayName || `${m.firstName ?? ""} ${m.lastName ?? ""}`.trim() || m.email;
}

export default function SettingsEstablishmentsPanel({
  establishments,
  setEstablishments,
  clerkMembers,
  membersLoading,
  uploadingSignatureId,
  saving,
  uploadDirectionSignature,
  removeDirectionSignature,
  saveSection,
}: {
  establishments: SettingsEstablishmentForm[];
  setEstablishments: Dispatch<SetStateAction<SettingsEstablishmentForm[]>>;
  clerkMembers: ClerkMemberOption[];
  membersLoading: boolean;
  uploadingSignatureId: string | null;
  saving: boolean;
  uploadDirectionSignature: (establishmentId: string, file: File) => void;
  removeDirectionSignature: (establishmentId: string) => void;
  saveSection: (section: string, body: unknown) => Promise<void>;
}) {
  const patch = (idx: number, next: Partial<SettingsEstablishmentForm>) => {
    setEstablishments((prev) => {
      const copy = [...prev];
      copy[idx] = { ...copy[idx]!, ...next };
      return copy;
    });
  };

  const availableKinds = useMemo(() => {
    const kinds = new Set<EstablishmentKind>();
    for (const e of establishments) {
      if (!e.active) continue;
      kinds.add(inferEstablishmentKind(e));
    }
    return ESTABLISHMENT_KIND_OPTIONS.filter((o) => kinds.has(o.value));
  }, [establishments]);

  return (
    <div className="space-y-4">
      <div className="rounded-2xl border border-slate-200 bg-white p-4">
        <p className="text-xs font-bold uppercase tracking-wide text-slate-500">Sites actifs</p>
        <p className="mt-1 text-sm text-slate-700">
          {availableKinds.length === 0
            ? "Aucun établissement actif. Ajoutez une école, un collège ou un lycée ci-dessous — les modules (voyages, absences…) s’alignent sur cette liste."
            : availableKinds
                .map((k) => `${establishmentKindEmoji(k.value)} ${k.label}`)
                .join("  ·  ")}
        </p>
        <p className="mt-2 text-[11px] text-slate-400">
          La présence d’une école, d’un collège ou d’un lycée se déduit des fiches actives (type), pas d’une liste figée.
        </p>
      </div>

      {establishments.map((est, idx) => {
        const kind = inferEstablishmentKind(est);
        const color = resolveEstablishmentColorHex(est);
        return (
          <div key={est.id || idx} className="bg-white rounded-2xl border p-5 space-y-4">
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
              <div>
                <label className="block text-xs font-bold text-slate-500 mb-1">ID</label>
                <input
                  className="w-full border rounded-lg p-2 text-sm"
                  placeholder="ecole, college, lycee…"
                  value={est.id}
                  onChange={(e) => patch(idx, { id: e.target.value })}
                />
              </div>
              <div>
                <label className="block text-xs font-bold text-slate-500 mb-1">Libellé</label>
                <input
                  className="w-full border rounded-lg p-2 text-sm"
                  placeholder="École, Collège…"
                  value={est.label}
                  onChange={(e) => patch(idx, { label: e.target.value })}
                />
              </div>
            </div>

            <div>
              <label className="block text-xs font-bold text-slate-500 mb-1">Type de site</label>
              <select
                className="w-full border rounded-lg p-2 text-sm"
                value={kind}
                onChange={(e) => {
                  const nextKind = e.target.value as EstablishmentKind;
                  const prevDefault = DEFAULT_ESTABLISHMENT_KIND_COLORS[kind];
                  const keepCustom =
                    est.colorHex &&
                    est.colorHex.toUpperCase() !== prevDefault &&
                    est.colorHex.toUpperCase() !== DEFAULT_ESTABLISHMENT_KIND_COLORS[nextKind];
                  patch(idx, {
                    kind: nextKind,
                    colorHex: keepCustom
                      ? est.colorHex
                      : DEFAULT_ESTABLISHMENT_KIND_COLORS[nextKind],
                  });
                }}
              >
                {ESTABLISHMENT_KIND_OPTIONS.map((o) => (
                  <option key={o.value} value={o.value}>
                    {establishmentKindEmoji(o.value)} {o.label}
                  </option>
                ))}
              </select>
            </div>

            <div>
              <label className="block text-xs font-bold text-slate-500 mb-1">
                Couleur d’affichage
              </label>
              <p className="text-[11px] text-slate-400 mb-2">
                Utilisée sur les tuiles voyages, les filtres et les badges. Par défaut selon le type
                (école / collège / lycée).
              </p>
              <div className="flex flex-wrap items-center gap-2">
                {ESTABLISHMENT_COLOR_SWATCHES.map((sw) => {
                  const selected = color.toUpperCase() === sw.hex.toUpperCase();
                  return (
                    <button
                      key={sw.id}
                      type="button"
                      title={sw.label}
                      onClick={() => patch(idx, { colorHex: sw.hex })}
                      className={`h-8 w-8 rounded-full border-2 cursor-pointer ${
                        selected ? "border-slate-900 scale-110" : "border-white shadow"
                      }`}
                      style={{ backgroundColor: sw.hex }}
                    />
                  );
                })}
                <label className="flex items-center gap-2 text-xs text-slate-600 cursor-pointer">
                  <input
                    type="color"
                    value={color}
                    onChange={(e) => patch(idx, { colorHex: e.target.value.toUpperCase() })}
                    className="h-8 w-10 cursor-pointer rounded border border-slate-200 bg-white p-0"
                  />
                  {color}
                </label>
              </div>
            </div>

            <div className="rounded-xl border border-slate-200 bg-slate-50 p-3 space-y-2">
              <p className="text-xs font-bold text-slate-700">Responsable d’établissement</p>
              <p className="text-[11px] text-slate-500">
                Choisissez la personne dans le personnel Clerk. Le nom et l’e-mail servent aux PDF
                (devis, stages, certificats) et aux notifications.
              </p>
              <ClerkPersonSelect
                members={clerkMembers}
                selectedId={est.directorClerkUserId}
                loading={membersLoading}
                onChange={(member) => {
                  if (!member) {
                    patch(idx, { directorClerkUserId: "" });
                    return;
                  }
                  patch(idx, {
                    directorClerkUserId: member.clerkUserId,
                    directorName: memberDisplayName(member),
                    directorEmail: member.email,
                  });
                }}
              />
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 pt-1">
                <div>
                  <label className="block text-xs font-bold text-slate-500 mb-1">
                    Nom affiché (PDF)
                  </label>
                  <input
                    className="w-full border rounded-lg p-2 text-sm bg-white"
                    placeholder="Nom direction"
                    value={est.directorName}
                    onChange={(e) => patch(idx, { directorName: e.target.value })}
                  />
                </div>
                <div>
                  <label className="block text-xs font-bold text-slate-500 mb-1">E-mail</label>
                  <input
                    className="w-full border rounded-lg p-2 text-sm bg-white"
                    placeholder="Email direction"
                    type="email"
                    value={est.directorEmail}
                    onChange={(e) => patch(idx, { directorEmail: e.target.value })}
                  />
                </div>
              </div>
            </div>

            <div className="rounded-xl border border-slate-200 bg-slate-50 p-3 space-y-2">
              <p className="text-xs font-bold text-slate-700">
                Signature direction (PDF devis / stages / certificats)
              </p>
              <p className="text-[11px] text-slate-500">
                Stockée dans le bucket privé du tenant — pas sur scolia-images.
              </p>
              {est.signaturePreviewUrl ? (
                // eslint-disable-next-line @next/next/no-img-element
                <img
                  src={est.signaturePreviewUrl}
                  alt={`Signature ${est.label || est.id}`}
                  className="h-14 max-w-[220px] object-contain bg-white rounded border"
                />
              ) : est.signatureS3Key ? (
                <p className="text-xs text-amber-700">Signature enregistrée (aperçu indisponible).</p>
              ) : (
                <p className="text-xs text-slate-400">Aucune signature.</p>
              )}
              <div className="flex flex-wrap items-center gap-2">
                <label className="cursor-pointer text-xs font-bold text-indigo-600 hover:underline">
                  {uploadingSignatureId === est.id ? "Envoi…" : "Ajouter / remplacer"}
                  <input
                    type="file"
                    accept="image/png,image/jpeg,image/webp"
                    className="hidden"
                    disabled={uploadingSignatureId === est.id || !est.id.trim()}
                    onChange={(e) => {
                      const f = e.target.files?.[0];
                      e.target.value = "";
                      if (f) void uploadDirectionSignature(est.id, f);
                    }}
                  />
                </label>
                {(est.signatureS3Key || est.signaturePreviewUrl) && (
                  <button
                    type="button"
                    className="text-xs font-bold text-rose-600 cursor-pointer"
                    disabled={uploadingSignatureId === est.id}
                    onClick={() => void removeDirectionSignature(est.id)}
                  >
                    Supprimer
                  </button>
                )}
              </div>
            </div>

            <div>
              <label className="block text-xs font-bold text-slate-500 mb-1">
                Rôles Clerk (accès direction)
              </label>
              <p className="text-[11px] text-slate-400 mb-1">
                Toute personne portant l’un de ces rôles peut signer pour ce site, en plus du
                responsable nommé ci-dessus.
              </p>
              <input
                className="w-full border rounded-lg p-2 text-sm"
                placeholder="direction_ecole, direction_college…"
                value={est.clerkRoleSlugs}
                onChange={(e) => patch(idx, { clerkRoleSlugs: e.target.value })}
              />
            </div>

            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={est.active}
                onChange={(e) => patch(idx, { active: e.target.checked })}
              />
              Actif
            </label>
          </div>
        );
      })}
      <button
        type="button"
        className="text-indigo-600 font-bold text-sm cursor-pointer"
        onClick={() => setEstablishments([...establishments, emptySettingsEstablishmentForm()])}
      >
        + Ajouter un établissement
      </button>
      <ModuleButton
        variant="primary"
        disabled={saving}
        onClick={() =>
          saveSection("establishments", {
            establishments: establishments.map((e) => ({
              id: e.id,
              label: e.label,
              kind: inferEstablishmentKind(e),
              directorName: e.directorName,
              directorEmail: e.directorEmail,
              directorClerkUserId: e.directorClerkUserId || undefined,
              colorHex: resolveEstablishmentColorHex(e),
              signatureS3Key: e.signatureS3Key,
              grades: e.grades,
              active: e.active,
              clerkRoleSlugs: e.clerkRoleSlugs.split(",").map((s) => s.trim()).filter(Boolean),
            })),
          })
        }
      >
        Enregistrer les établissements
      </ModuleButton>
    </div>
  );
}
