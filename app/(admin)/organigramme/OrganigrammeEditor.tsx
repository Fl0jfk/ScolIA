"use client";

import { useCallback, useEffect, useState } from "react";
import type { OrganigramConfig, OrganigramSlot } from "@/app/lib/organigramme-config";

const SECTION_LABELS: Record<string, string> = {
  direction: "Direction",
  admin: "Administration",
  accounting: "Comptabilité",
  poles: "Pôles / vie scolaire",
  reception: "Accueil",
  health: "Santé",
  maintenance: "Maintenance",
  pastoral: "Pastorale",
  ogec: "OGEC",
  tutelle: "Tutelle",
};

export function OrganigrammeEditor({ onSaved }: { onSaved: () => void }) {
  const [config, setConfig] = useState<OrganigramConfig | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setError(null);
    const res = await fetch("/api/organigramme/config", { cache: "no-store" });
    const data = await res.json();
    if (!res.ok) {
      setError(data.error || "Impossible de charger la config");
      return;
    }
    setConfig(data.config as OrganigramConfig);
    if (!selectedId && data.config?.slots?.[0]?.id) {
      setSelectedId(data.config.slots[0].id);
    }
  }, [selectedId]);

  useEffect(() => {
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const selected = config?.slots.find((s) => s.id === selectedId) || null;

  const updateSelected = (patch: Partial<OrganigramSlot>) => {
    if (!config || !selectedId) return;
    setConfig({
      ...config,
      slots: config.slots.map((s) => (s.id === selectedId ? { ...s, ...patch } : s)),
    });
  };

  const save = async () => {
    if (!config) return;
    setSaving(true);
    setMessage(null);
    setError(null);
    try {
      const res = await fetch("/api/organigramme/config", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ config }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Échec enregistrement");
      setConfig(data.config);
      setMessage("Organigramme enregistré.");
      onSaved();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  };

  const applySuggestions = async () => {
    setSaving(true);
    setMessage(null);
    setError(null);
    try {
      const res = await fetch("/api/organigramme/config", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "applySuggestions" }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Échec suggestions");
      setConfig(data.config);
      setMessage(
        data.added
          ? `${data.added} personne(s) ajoutée(s) depuis tickets / RH.`
          : "Aucune nouvelle suggestion.",
      );
      onSaved();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  };

  const addBlank = () => {
    if (!config) return;
    const id = `slot-${Date.now()}`;
    const slot: OrganigramSlot = {
      id,
      sectionId: "admin",
      order: config.slots.length,
      firstName: "",
      lastName: "",
      role: "À préciser",
      missions: [],
      photoUrl: "",
    };
    setConfig({ ...config, slots: [...config.slots, slot] });
    setSelectedId(id);
  };

  const removeSelected = () => {
    if (!config || !selectedId) return;
    const next = config.slots.filter((s) => s.id !== selectedId);
    setConfig({ ...config, slots: next });
    setSelectedId(next[0]?.id || null);
  };

  if (error && !config) {
    return (
      <div className="mb-8 rounded-2xl border border-red-200 bg-red-50 p-4 text-sm text-red-700">
        {error}
      </div>
    );
  }

  if (!config) {
    return (
      <div className="mb-8 rounded-2xl border border-slate-200 bg-white/80 p-4 text-sm text-slate-500">
        Chargement de l&apos;éditeur…
      </div>
    );
  }

  const missionsText = (selected?.missions || []).join("\n");

  return (
    <div className="mb-8 rounded-2xl border border-slate-200 bg-white/90 shadow-sm overflow-hidden">
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-slate-100 px-4 py-3 bg-slate-50/80">
        <div>
          <p className="text-sm font-bold text-slate-900">Édition de l&apos;organigramme</p>
          <p className="text-[11px] text-slate-500">
            Modifiez les fiches, puis enregistrez. Les suggestions RH / tickets n&apos;écrasent pas
            l&apos;existant.
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            disabled={saving}
            onClick={() => void applySuggestions()}
            className="rounded-lg border border-indigo-200 bg-indigo-50 px-3 py-1.5 text-xs font-semibold text-indigo-800 hover:bg-indigo-100 disabled:opacity-50"
          >
            Proposer depuis tickets / RH
          </button>
          <button
            type="button"
            onClick={addBlank}
            className="rounded-lg border border-slate-200 bg-white px-3 py-1.5 text-xs font-semibold text-slate-700 hover:bg-slate-50"
          >
            + Personne
          </button>
          <button
            type="button"
            disabled={saving}
            onClick={() => void save()}
            className="rounded-lg bg-slate-900 px-3 py-1.5 text-xs font-bold text-white hover:bg-slate-800 disabled:opacity-50"
          >
            {saving ? "…" : "Enregistrer"}
          </button>
        </div>
      </div>

      {(message || error) && (
        <p className={`px-4 py-2 text-xs ${error ? "text-red-600 bg-red-50" : "text-emerald-700 bg-emerald-50"}`}>
          {error || message}
        </p>
      )}

      <div className="grid grid-cols-1 md:grid-cols-[14rem_1fr] min-h-[280px]">
        <aside className="border-b md:border-b-0 md:border-r border-slate-100 max-h-72 md:max-h-[420px] overflow-y-auto">
          {config.slots
            .slice()
            .sort((a, b) => a.sectionId.localeCompare(b.sectionId) || a.order - b.order)
            .map((s) => {
              const label =
                [s.firstName, s.lastName].filter(Boolean).join(" ") || s.role || s.id;
              return (
                <button
                  key={s.id}
                  type="button"
                  onClick={() => setSelectedId(s.id)}
                  className={`w-full text-left px-3 py-2 text-xs border-b border-slate-50 hover:bg-sky-50 ${
                    s.id === selectedId ? "bg-sky-100 font-semibold text-sky-900" : "text-slate-700"
                  }`}
                >
                  <span className="block truncate">{label}</span>
                  <span className="block text-[10px] text-slate-400">
                    {SECTION_LABELS[s.sectionId] || s.sectionId}
                    {s.hidden ? " · masqué" : ""}
                  </span>
                </button>
              );
            })}
        </aside>

        <div className="p-4 space-y-3">
          {!selected ? (
            <p className="text-sm text-slate-500">Sélectionnez une personne.</p>
          ) : (
            <>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                <label className="text-xs font-semibold text-slate-600">
                  Prénom
                  <input
                    className="mt-1 w-full rounded-lg border border-slate-200 px-2.5 py-1.5 text-sm"
                    value={selected.firstName || ""}
                    onChange={(e) => updateSelected({ firstName: e.target.value })}
                  />
                </label>
                <label className="text-xs font-semibold text-slate-600">
                  Nom
                  <input
                    className="mt-1 w-full rounded-lg border border-slate-200 px-2.5 py-1.5 text-sm"
                    value={selected.lastName || ""}
                    onChange={(e) => updateSelected({ lastName: e.target.value })}
                  />
                </label>
                <label className="text-xs font-semibold text-slate-600 sm:col-span-2">
                  Rôle affiché
                  <input
                    className="mt-1 w-full rounded-lg border border-slate-200 px-2.5 py-1.5 text-sm"
                    value={selected.role || ""}
                    onChange={(e) => updateSelected({ role: e.target.value })}
                  />
                </label>
                <label className="text-xs font-semibold text-slate-600">
                  E-mail
                  <input
                    className="mt-1 w-full rounded-lg border border-slate-200 px-2.5 py-1.5 text-sm"
                    value={selected.email || ""}
                    onChange={(e) => updateSelected({ email: e.target.value })}
                  />
                </label>
                <label className="text-xs font-semibold text-slate-600">
                  Section
                  <select
                    className="mt-1 w-full rounded-lg border border-slate-200 px-2.5 py-1.5 text-sm"
                    value={selected.sectionId}
                    onChange={(e) =>
                      updateSelected({
                        sectionId: e.target.value as OrganigramSlot["sectionId"],
                      })
                    }
                  >
                    {Object.entries(SECTION_LABELS).map(([id, label]) => (
                      <option key={id} value={id}>
                        {label}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="text-xs font-semibold text-slate-600 sm:col-span-2">
                  URL photo
                  <input
                    className="mt-1 w-full rounded-lg border border-slate-200 px-2.5 py-1.5 text-sm"
                    value={selected.photoUrl || ""}
                    onChange={(e) => updateSelected({ photoUrl: e.target.value })}
                    placeholder="https://… ou laisser vide"
                  />
                </label>
                <label className="text-xs font-semibold text-slate-600 sm:col-span-2">
                  Missions (une par ligne)
                  <textarea
                    className="mt-1 w-full rounded-lg border border-slate-200 px-2.5 py-1.5 text-sm min-h-[100px]"
                    value={missionsText}
                    onChange={(e) =>
                      updateSelected({
                        missions: e.target.value
                          .split("\n")
                          .map((l) => l.trim())
                          .filter(Boolean),
                      })
                    }
                  />
                </label>
              </div>
              <div className="flex flex-wrap items-center gap-3 pt-1">
                <label className="inline-flex items-center gap-2 text-xs text-slate-600">
                  <input
                    type="checkbox"
                    checked={Boolean(selected.hidden)}
                    onChange={(e) => updateSelected({ hidden: e.target.checked })}
                  />
                  Masquer sur l&apos;organigramme
                </label>
                <button
                  type="button"
                  onClick={removeSelected}
                  className="ml-auto text-xs font-semibold text-red-600 hover:underline"
                >
                  Retirer cette fiche
                </button>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
