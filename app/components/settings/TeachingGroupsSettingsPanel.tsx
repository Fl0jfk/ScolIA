"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import {
  defaultTeachingGroupsConfig,
  emptyTeachingGroup,
  parseTeachingGroupsConfig,
  type TeachingGroup,
  type TeachingGroupsConfig,
} from "@/app/lib/rh/teaching-groups";

export default function TeachingGroupsSettingsPanel({
  establishments = [],
  classCatalog = [],
}: {
  establishments?: Array<{ id: string; label: string; kind?: string }>;
  classCatalog?: string[];
}) {
  const [cfg, setCfg] = useState<TeachingGroupsConfig>(defaultTeachingGroupsConfig());
  const [activeId, setActiveId] = useState("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [classDraft, setClassDraft] = useState("");
  const [rosterClasses, setRosterClasses] = useState<string[]>([]);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [settingsRes, rosterRes] = await Promise.all([
        fetch("/api/settings", { cache: "no-store" }),
        fetch("/api/settings/roster", { cache: "no-store" }),
      ]);
      const j = await settingsRes.json();
      if (!settingsRes.ok) throw new Error(j.error || "Chargement impossible");
      const parsed = parseTeachingGroupsConfig(j.config?.teachingGroups);
      setCfg(parsed);
      setActiveId(parsed.groups[0]?.id || "");
      if (rosterRes.ok) {
        const rosterJson = await rosterRes.json();
        const fromRoster = Array.isArray(rosterJson?.classes)
          ? rosterJson.classes.map((c: unknown) => String(c || "").trim()).filter(Boolean)
          : Array.isArray(rosterJson?.roster?.classAssignments)
            ? rosterJson.roster.classAssignments
                .map((a: { className?: string }) => String(a.className || "").trim())
                .filter(Boolean)
            : [];
        if (fromRoster.length) setRosterClasses(fromRoster);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur de chargement");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const active = cfg.groups.find((g) => g.id === activeId) || cfg.groups[0] || null;

  const catalogOptions = useMemo(() => {
    const set = new Set(
      [...classCatalog, ...rosterClasses].map((c) => c.trim()).filter(Boolean),
    );
    for (const g of cfg.groups) {
      for (const c of g.classNames) set.add(c);
    }
    return [...set].sort((a, b) =>
      a.localeCompare(b, "fr", { sensitivity: "base", numeric: true }),
    );
  }, [classCatalog, rosterClasses, cfg.groups]);

  const updateGroup = (next: TeachingGroup) => {
    setCfg((prev) => ({
      groups: prev.groups.map((g) => (g.id === next.id ? next : g)),
    }));
  };

  const save = async () => {
    setSaving(true);
    setMessage(null);
    setError(null);
    try {
      const body = parseTeachingGroupsConfig(cfg);
      const res = await fetch("/api/settings/teaching-groups", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Enregistrement impossible");
      const saved = parseTeachingGroupsConfig(j.config?.teachingGroups ?? body);
      setCfg(saved);
      setMessage("Groupes enregistrés.");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur d’enregistrement");
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return <p className="text-sm text-slate-500">Chargement des groupes…</p>;
  }

  return (
    <div className="space-y-4">
      <div>
        <h2 className="text-lg font-black text-slate-900">Groupes EDT</h2>
        <p className="text-sm text-slate-600 mt-1 max-w-2xl">
          Composition multi-classes (latinistes, LV, options…). Sur le calendrier EDT, le
          libellé du groupe s’affiche à la place de la liste des classes.
        </p>
      </div>

      {error ? (
        <p className="text-sm text-rose-700 bg-rose-50 border border-rose-100 rounded-xl px-3 py-2">
          {error}
        </p>
      ) : null}
      {message ? (
        <p className="text-sm text-emerald-800 bg-emerald-50 border border-emerald-100 rounded-xl px-3 py-2">
          {message}
        </p>
      ) : null}

      <div className="flex flex-wrap gap-2 items-center">
        {cfg.groups.map((g) => (
          <button
            key={g.id}
            type="button"
            onClick={() => setActiveId(g.id)}
            className={`px-3 py-1.5 rounded-xl text-xs font-bold border ${
              active?.id === g.id
                ? "bg-slate-900 text-white border-slate-900"
                : "bg-white text-slate-700 border-slate-200 hover:bg-slate-50"
            }`}
          >
            {g.label}
          </button>
        ))}
        <button
          type="button"
          className="px-3 py-1.5 rounded-xl text-xs font-bold border border-dashed border-indigo-300 text-indigo-700 hover:bg-indigo-50"
          onClick={() => {
            const g = emptyTeachingGroup({
              label: `Groupe ${cfg.groups.length + 1}`,
            });
            setCfg((prev) => ({ groups: [...prev.groups, g] }));
            setActiveId(g.id);
          }}
        >
          + Ajouter un groupe
        </button>
      </div>

      {active ? (
        <div className="rounded-2xl border border-slate-200 bg-white p-4 space-y-4">
          <div className="grid sm:grid-cols-2 lg:grid-cols-3 gap-3">
            <label className="text-xs font-bold text-slate-600 space-y-1 block">
              Libellé (affiché sur le calendrier)
              <input
                className="w-full border rounded-lg px-2 py-1.5 text-sm"
                value={active.label}
                onChange={(e) => updateGroup({ ...active, label: e.target.value })}
              />
            </label>
            <label className="text-xs font-bold text-slate-600 space-y-1 block">
              Site lié (optionnel)
              <select
                className="w-full border rounded-lg px-2 py-1.5 text-sm bg-white"
                value={active.establishmentId || ""}
                onChange={(e) =>
                  updateGroup({
                    ...active,
                    establishmentId: e.target.value || null,
                  })
                }
              >
                <option value="">— Aucun —</option>
                {establishments.map((e) => (
                  <option key={e.id} value={e.id}>
                    {e.label}
                  </option>
                ))}
              </select>
            </label>
            <div className="flex items-end">
              <button
                type="button"
                className="px-3 py-1.5 rounded-xl text-xs font-bold border border-rose-200 text-rose-700 hover:bg-rose-50"
                onClick={() => {
                  setCfg((prev) => ({
                    groups: prev.groups.filter((g) => g.id !== active.id),
                  }));
                  setActiveId((id) =>
                    id === active.id
                      ? cfg.groups.find((g) => g.id !== id)?.id || ""
                      : id,
                  );
                }}
              >
                Supprimer le groupe
              </button>
            </div>
          </div>

          <div>
            <p className="text-[11px] font-black uppercase tracking-wide text-slate-500 mb-1.5">
              Classes du groupe
            </p>
            <div className="flex flex-wrap gap-1.5 max-h-48 overflow-y-auto">
              {catalogOptions.map((className) => {
                const checked = active.classNames.includes(className);
                return (
                  <label
                    key={className}
                    className={`inline-flex items-center gap-1 rounded-lg border px-2 py-1 text-[11px] font-semibold cursor-pointer ${
                      checked
                        ? "border-indigo-300 bg-indigo-100 text-indigo-900"
                        : "border-slate-200 bg-white text-slate-600 hover:bg-slate-50"
                    }`}
                  >
                    <input
                      type="checkbox"
                      className="sr-only"
                      checked={checked}
                      onChange={() => {
                        const set = new Set(active.classNames);
                        if (checked) set.delete(className);
                        else set.add(className);
                        updateGroup({ ...active, classNames: [...set] });
                      }}
                    />
                    {className}
                  </label>
                );
              })}
            </div>
            <div className="mt-2 flex flex-wrap gap-2">
              <input
                className="border rounded-lg px-2 py-1.5 text-sm flex-1 min-w-[10rem]"
                placeholder="Ajouter une classe (ex. 3A)"
                value={classDraft}
                onChange={(e) => setClassDraft(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key !== "Enter") return;
                  e.preventDefault();
                  const name = classDraft.trim();
                  if (!name) return;
                  if (!active.classNames.includes(name)) {
                    updateGroup({
                      ...active,
                      classNames: [...active.classNames, name],
                    });
                  }
                  setClassDraft("");
                }}
              />
              <button
                type="button"
                className="px-3 py-1.5 rounded-xl text-xs font-bold border border-slate-200 hover:bg-slate-50"
                onClick={() => {
                  const name = classDraft.trim();
                  if (!name) return;
                  if (!active.classNames.includes(name)) {
                    updateGroup({
                      ...active,
                      classNames: [...active.classNames, name],
                    });
                  }
                  setClassDraft("");
                }}
              >
                Ajouter
              </button>
            </div>
          </div>
        </div>
      ) : (
        <p className="text-sm text-slate-500">
          Aucun groupe pour l’instant. Ajoutez-en un pour commencer.
        </p>
      )}

      <div className="flex justify-end">
        <button
          type="button"
          disabled={saving}
          onClick={() => void save()}
          className="px-4 py-2 rounded-xl text-sm font-bold bg-emerald-600 text-white hover:bg-emerald-700 disabled:opacity-60"
        >
          {saving ? "Enregistrement…" : "Enregistrer les groupes"}
        </button>
      </div>
    </div>
  );
}
