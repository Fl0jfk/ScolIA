"use client";

import type { Dispatch, SetStateAction } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import type { SettingsEstablishmentForm } from "@/app/lib/settings-page-model";

export default function SettingsEstablishmentsPanel({
  establishments,
  setEstablishments,
  uploadingSignatureId,
  saving,
  uploadDirectionSignature,
  removeDirectionSignature,
  saveSection,
}: {
  establishments: SettingsEstablishmentForm[];
  setEstablishments: Dispatch<SetStateAction<SettingsEstablishmentForm[]>>;
  uploadingSignatureId: string | null;
  saving: boolean;
  uploadDirectionSignature: (establishmentId: string, file: File) => void;
  removeDirectionSignature: (establishmentId: string) => void;
  saveSection: (section: string, body: unknown) => Promise<void>;
}) {
  return (
    <div className="space-y-4">
      {establishments.map((est, idx) => (
        <div key={est.id || idx} className="bg-white rounded-2xl border p-5 space-y-3">
          <div className="grid grid-cols-2 gap-3">
            <input
              className="border rounded-lg p-2 text-sm"
              placeholder="ID (ecole, college…)"
              value={est.id}
              onChange={(e) => {
                const next = [...establishments];
                next[idx] = { ...est, id: e.target.value };
                setEstablishments(next);
              }}
            />
            <input
              className="border rounded-lg p-2 text-sm"
              placeholder="Libellé"
              value={est.label}
              onChange={(e) => {
                const next = [...establishments];
                next[idx] = { ...est, label: e.target.value };
                setEstablishments(next);
              }}
            />
          </div>
          <input
            className="w-full border rounded-lg p-2 text-sm"
            placeholder="Nom direction"
            value={est.directorName}
            onChange={(e) => {
              const next = [...establishments];
              next[idx] = { ...est, directorName: e.target.value };
              setEstablishments(next);
            }}
          />
          <input
            className="w-full border rounded-lg p-2 text-sm"
            placeholder="Email direction"
            value={est.directorEmail}
            onChange={(e) => {
              const next = [...establishments];
              next[idx] = { ...est, directorEmail: e.target.value };
              setEstablishments(next);
            }}
          />
          <div className="rounded-xl border border-slate-200 bg-slate-50 p-3 space-y-2">
            <p className="text-xs font-bold text-slate-700">Signature direction (PDF devis / stages / certificats)</p>
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
                  className="text-xs font-bold text-rose-600"
                  disabled={uploadingSignatureId === est.id}
                  onClick={() => void removeDirectionSignature(est.id)}
                >
                  Supprimer
                </button>
              )}
            </div>
          </div>
          <input
            className="w-full border rounded-lg p-2 text-sm"
            placeholder="Rôles Clerk (séparés par des virgules)"
            value={est.clerkRoleSlugs}
            onChange={(e) => {
              const next = [...establishments];
              next[idx] = { ...est, clerkRoleSlugs: e.target.value };
              setEstablishments(next);
            }}
          />
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={est.active}
              onChange={(e) => {
                const next = [...establishments];
                next[idx] = { ...est, active: e.target.checked };
                setEstablishments(next);
              }}
            />
            Actif
          </label>
        </div>
      ))}
      <button
        type="button"
        className="text-indigo-600 font-bold text-sm"
        onClick={() =>
          setEstablishments([
            ...establishments,
            { id: "", label: "", directorName: "", directorEmail: "", clerkRoleSlugs: "", active: true, signaturePreviewUrl: null },
          ])
        }
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
              kind: e.kind,
              directorName: e.directorName,
              directorEmail: e.directorEmail,
              signatureS3Key: e.signatureS3Key,
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
