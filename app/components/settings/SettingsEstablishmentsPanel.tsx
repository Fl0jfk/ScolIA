"use client";

import type { Dispatch, SetStateAction } from "react";
import { motion } from "framer-motion";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ClerkPersonSelect from "@/app/components/settings/ClerkPersonSelect";
import {
  SettingsField,
  SettingsNotice,
  SettingsSection,
  settingsInputClass,
  settingsSelectClass,
} from "@/app/components/settings/SettingsChrome";
import type { ClerkMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";
import type { EstablishmentKind } from "@/app/lib/app-config-schemas";
import { dash } from "@/app/lib/dashboard-brand";
import {
  DEFAULT_ESTABLISHMENT_KIND_COLORS,
  ESTABLISHMENT_COLOR_SWATCHES,
  ESTABLISHMENT_KIND_OPTIONS,
  ESTABLISHMENT_KIND_PRESETS,
  establishmentKindEmoji,
  establishmentVisualFromHex,
  hexToRgba,
  inferEstablishmentKind,
  resolveEstablishmentColorHex,
} from "@/app/lib/establishment-visual";
import { clerkRoleSlugsForEstablishment, directionRoleForKind } from "@/app/lib/establishment-catalog";
import { type SettingsEstablishmentForm } from "@/app/lib/settings-page-model";

function memberDisplayName(m: ClerkMemberOption): string {
  return m.displayName || `${m.firstName ?? ""} ${m.lastName ?? ""}`.trim() || m.email;
}

function formFromPreset(kind: EstablishmentKind, existingIds: Set<string>): SettingsEstablishmentForm {
  const preset = ESTABLISHMENT_KIND_PRESETS.find((p) => p.kind === kind) || ESTABLISHMENT_KIND_PRESETS[3]!;
  let id = preset.id;
  if (existingIds.has(id)) {
    let n = 2;
    while (existingIds.has(`${preset.id}_${n}`)) n += 1;
    id = `${preset.id}_${n}`;
  }
  return {
    id,
    label: preset.label,
    kind: preset.kind,
    directorName: "",
    directorEmail: "",
    directorClerkUserId: "",
    colorHex: DEFAULT_ESTABLISHMENT_KIND_COLORS[preset.kind],
    clerkRoleSlugs: clerkRoleSlugsForEstablishment(preset).join(", "),
    active: true,
    grades: preset.grades,
    signaturePreviewUrl: null,
  };
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

  const availableKinds = establishments
    .filter((e) => e.active)
    .map((e) => inferEstablishmentKind(e));
  const uniqueActiveKinds = ESTABLISHMENT_KIND_OPTIONS.filter((o) => availableKinds.includes(o.value));

  return (
    <div className="space-y-4">
      <SettingsSection icon="🗺️" title="Sites actifs">
        {uniqueActiveKinds.length === 0 ? (
          <div className="rounded-2xl border border-dashed border-white/80 bg-white/35 px-4 py-8 text-center">
            <p className="text-3xl" aria-hidden>
              🏫
            </p>
            <p className={`mt-2 text-sm font-semibold ${dash.ink}`}>Aucun établissement actif</p>
            <p className={`mt-1 text-sm ${dash.textMid}`}>
              Ajoutez une école, un collège ou un lycée ci-dessous — les modules (voyages, absences…)
              s’alignent sur cette liste.
            </p>
          </div>
        ) : (
          <div className="flex flex-wrap gap-2">
            {uniqueActiveKinds.map((k) => {
              const visual = establishmentVisualFromHex(DEFAULT_ESTABLISHMENT_KIND_COLORS[k.value]);
              return (
                <span
                  key={k.value}
                  className="inline-flex items-center gap-1.5 rounded-full border px-3 py-1.5 text-sm font-semibold shadow-sm backdrop-blur"
                  style={{
                    backgroundColor: visual.badgeBg,
                    borderColor: visual.borderColor,
                    color: visual.textColor,
                  }}
                >
                  <span aria-hidden>{establishmentKindEmoji(k.value)}</span>
                  {k.label}
                </span>
              );
            })}
          </div>
        )}
        <p className={`text-[11px] ${dash.textMid}`}>
          La présence d’une école, d’un collège ou d’un lycée se déduit des fiches actives (type), pas
          d’une liste figée.
        </p>
      </SettingsSection>

      {establishments.map((est, idx) => {
        const kind = inferEstablishmentKind(est);
        const color = resolveEstablishmentColorHex(est);
        const visual = establishmentVisualFromHex(color);
        return (
          <motion.article
            key={est.id || idx}
            initial={{ opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.4, delay: 0.04 * idx, ease: [0.22, 1, 0.36, 1] }}
            className="relative overflow-hidden rounded-[1.5rem] border shadow-[0_20px_50px_-32px_rgba(15,23,42,0.45)] backdrop-blur-2xl"
            style={{
              backgroundColor: hexToRgba(color, est.active ? 0.16 : 0.07),
              borderColor: visual.borderColor,
            }}
          >
            <div
              className="pointer-events-none absolute -right-10 -top-12 h-44 w-44 rounded-full blur-3xl"
              style={{ backgroundColor: visual.orbBg }}
              aria-hidden
            />
            <div
              className="pointer-events-none absolute -left-16 bottom-0 h-40 w-40 rounded-full blur-3xl"
              style={{ backgroundColor: hexToRgba(color, 0.14) }}
              aria-hidden
            />
            <div className="relative space-y-4 p-5 sm:p-6">
              <header className="flex flex-wrap items-start justify-between gap-3">
                <div className="flex items-center gap-3">
                  <span
                    className="flex h-12 w-12 items-center justify-center rounded-2xl text-xl shadow-sm ring-1 ring-white/70"
                    style={{ backgroundColor: visual.badgeBg, color: visual.textColor }}
                    aria-hidden
                  >
                    {establishmentKindEmoji(kind)}
                  </span>
                  <div>
                    <p className={`text-lg font-semibold tracking-tight ${dash.ink}`}>
                      {est.label || "Nouvel établissement"}
                    </p>
                    <p className={`text-xs ${dash.textMid}`}>
                      {ESTABLISHMENT_KIND_OPTIONS.find((o) => o.value === kind)?.label || kind}
                    </p>
                  </div>
                </div>
                <div className="flex items-center gap-2.5 rounded-full border border-white/70 bg-white/70 px-3 py-1.5 shadow-sm backdrop-blur">
                  <span className={`text-[11px] font-semibold uppercase tracking-[0.12em] ${dash.textMid}`}>
                    Actif
                  </span>
                  <button
                    type="button"
                    role="switch"
                    aria-checked={est.active}
                    onClick={() => patch(idx, { active: !est.active })}
                    className={`relative h-6 w-11 rounded-full transition ${
                      est.active ? "bg-[var(--dash-primary)]" : "bg-slate-300"
                    }`}
                  >
                    <span
                      className={`absolute top-0.5 h-5 w-5 rounded-full bg-white shadow transition ${
                        est.active ? "left-[1.35rem]" : "left-0.5"
                      }`}
                    />
                  </button>
                </div>
              </header>

              <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
                <SettingsField label="ID">
                  <input
                    className={settingsInputClass}
                    placeholder="ecole, college, lycee…"
                    value={est.id}
                    onChange={(e) => patch(idx, { id: e.target.value })}
                  />
                </SettingsField>
                <SettingsField label="Libellé">
                  <input
                    className={settingsInputClass}
                    placeholder="École, Collège…"
                    value={est.label}
                    onChange={(e) => patch(idx, { label: e.target.value })}
                  />
                </SettingsField>
              </div>

              <SettingsField label="Type de site">
                <select
                  className={settingsSelectClass}
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
                      colorHex: keepCustom ? est.colorHex : DEFAULT_ESTABLISHMENT_KIND_COLORS[nextKind],
                      clerkRoleSlugs: clerkRoleSlugsForEstablishment({ kind: nextKind }).join(", "),
                    });
                  }}
                >
                  {ESTABLISHMENT_KIND_OPTIONS.map((o) => (
                    <option key={o.value} value={o.value}>
                      {establishmentKindEmoji(o.value)} {o.label}
                    </option>
                  ))}
                </select>
              </SettingsField>

              <div>
                <p className={`text-[11px] font-semibold uppercase tracking-[0.14em] ${dash.textMid}`}>
                  Couleur d’affichage
                </p>
                <p className={`mb-2 mt-0.5 text-xs ${dash.textMid}`}>
                  Tuiles voyages, filtres et badges. Par défaut selon le type.
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
                        className={`h-8 w-8 rounded-full border-2 ${
                          selected ? "scale-110 border-slate-900" : "border-white shadow"
                        }`}
                        style={{ backgroundColor: sw.hex }}
                      />
                    );
                  })}
                  <label className={`flex cursor-pointer items-center gap-2 text-xs ${dash.textMid}`}>
                    <input
                      type="color"
                      value={color}
                      onChange={(e) => patch(idx, { colorHex: e.target.value.toUpperCase() })}
                      className="h-8 w-10 cursor-pointer rounded-lg border border-white/80 bg-white p-0"
                    />
                    {color}
                  </label>
                </div>
              </div>

              <div className="space-y-3 rounded-2xl border border-white/70 bg-white/40 p-4">
                <p className={`text-sm font-semibold ${dash.ink}`}>Responsable d’établissement</p>
                <p className={`text-xs ${dash.textMid}`}>
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
                <div className="grid grid-cols-1 gap-3 pt-1 sm:grid-cols-2">
                  <SettingsField label="Nom affiché (PDF)">
                    <input
                      className={settingsInputClass}
                      placeholder="Nom direction"
                      value={est.directorName}
                      onChange={(e) => patch(idx, { directorName: e.target.value })}
                    />
                  </SettingsField>
                  <SettingsField label="E-mail">
                    <input
                      className={settingsInputClass}
                      placeholder="Email direction"
                      type="email"
                      value={est.directorEmail}
                      onChange={(e) => patch(idx, { directorEmail: e.target.value })}
                    />
                  </SettingsField>
                </div>
              </div>

              <div className="space-y-2 rounded-2xl border border-white/70 bg-white/40 p-4">
                <p className={`text-sm font-semibold ${dash.ink}`}>Signature direction</p>
                <p className={`text-xs ${dash.textMid}`}>
                  PDF devis / stages / certificats — stockée dans le bucket privé du tenant.
                </p>
                {est.signaturePreviewUrl ? (
                  // eslint-disable-next-line @next/next/no-img-element
                  <img
                    src={est.signaturePreviewUrl}
                    alt={`Signature ${est.label || est.id}`}
                    className="h-14 max-w-[220px] rounded-xl border border-white/80 bg-white object-contain p-1"
                  />
                ) : est.signatureS3Key ? (
                  <p className="text-xs text-amber-700">Signature enregistrée (aperçu indisponible).</p>
                ) : (
                  <p className={`text-xs italic ${dash.textMid}`}>Aucune signature.</p>
                )}
                <div className="flex flex-wrap items-center gap-3">
                  <label className="cursor-pointer text-xs font-semibold text-[var(--dash-primary)] hover:underline">
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
                      className="text-xs font-semibold text-rose-600"
                      disabled={uploadingSignatureId === est.id}
                      onClick={() => void removeDirectionSignature(est.id)}
                    >
                      Supprimer
                    </button>
                  )}
                </div>
              </div>

              <SettingsNotice tone="info">
                Rôle Clerk assigné automatiquement au responsable :{" "}
                <span className={`font-semibold ${dash.ink}`}>{directionRoleForKind(kind)}</span>
              </SettingsNotice>
            </div>
          </motion.article>
        );
      })}

      <SettingsSection icon="➕" title="Ajouter un établissement">
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
          {ESTABLISHMENT_KIND_PRESETS.map((p) => {
            const visual = establishmentVisualFromHex(DEFAULT_ESTABLISHMENT_KIND_COLORS[p.kind]);
            return (
              <button
                key={p.kind}
                type="button"
                className="flex cursor-pointer flex-col items-center gap-2 rounded-2xl border px-3 py-4 text-center transition hover:-translate-y-0.5"
                style={{
                  backgroundColor: visual.washBg,
                  borderColor: visual.borderColor,
                }}
                onClick={() => {
                  const ids = new Set(establishments.map((e) => e.id));
                  setEstablishments([...establishments, formFromPreset(p.kind, ids)]);
                }}
              >
                <span
                  className="flex h-11 w-11 items-center justify-center rounded-2xl text-xl shadow-sm ring-1 ring-white/70"
                  style={{ backgroundColor: visual.badgeBg, color: visual.textColor }}
                >
                  {establishmentKindEmoji(p.kind)}
                </span>
                <span className="text-sm font-semibold" style={{ color: visual.textColor }}>
                  {p.kind === "custom" ? "Autre" : p.label}
                </span>
              </button>
            );
          })}
        </div>
      </SettingsSection>

      <div className="flex justify-end pt-1">
        <ModuleButton
          variant="primary"
          disabled={saving}
          className="rounded-2xl px-5 shadow-[0_12px_28px_-16px_rgba(15,23,42,0.55)]"
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
                clerkRoleSlugs: clerkRoleSlugsForEstablishment({
                  kind: inferEstablishmentKind(e),
                  id: e.id,
                  label: e.label,
                }),
              })),
            })
          }
        >
          {saving ? "Enregistrement…" : "Enregistrer les établissements"}
        </ModuleButton>
      </div>
    </div>
  );
}
