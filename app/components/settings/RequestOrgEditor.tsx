"use client";

import { useMemo, useState } from "react";
import type { DirectoryMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";
import type { RequestsOrgConfig, RequestsRoutingConfig, RequestServiceUnit } from "@/app/lib/app-config-schemas";
import { newRequestServiceUnit } from "@/app/lib/requests-org-shared";

type Props = {
  org: RequestsOrgConfig;
  routing: RequestsRoutingConfig;
  onChange: (next: RequestsOrgConfig) => void;
  members: DirectoryMemberOption[];
  membersLoading: boolean;
};

function normalizeTag(raw: string): string {
  return raw.trim().replace(/\s+/g, " ");
}

function EmailMultiPicker({
  label,
  values,
  onChange,
  members,
  membersLoading,
}: {
  label: string;
  values: string[];
  onChange: (emails: string[]) => void;
  members: DirectoryMemberOption[];
  membersLoading: boolean;
}) {
  const options = useMemo(() => {
    const fromMembers = members.map((m) => m.email?.trim()).filter(Boolean) as string[];
    return [...new Set([...fromMembers, ...values])].sort((a, b) => a.localeCompare(b, "fr"));
  }, [members, values]);

  const toggle = (email: string) => {
    const e = email.trim().toLowerCase();
    if (values.includes(e)) onChange(values.filter((x) => x !== e));
    else onChange([...values, e]);
  };

  return (
    <div>
      <p className="text-xs font-bold text-slate-500 mb-1">{label}</p>
      {membersLoading ? (
        <p className="text-xs text-slate-400">Chargement annuaire…</p>
      ) : (
        <div className="flex flex-wrap gap-1.5 max-h-28 overflow-y-auto rounded-lg border border-slate-200 bg-slate-50/80 p-2">
          {options.length === 0 ? (
            <span className="text-xs text-slate-400 italic">Aucun collaborateur</span>
          ) : (
            options.map((email) => {
              const checked = values.includes(email.toLowerCase());
              const name = members.find((m) => m.email?.toLowerCase() === email.toLowerCase())?.displayName;
              return (
                <button
                  key={email}
                  type="button"
                  onClick={() => toggle(email)}
                  className={`rounded-lg px-2 py-1 text-[11px] font-semibold border ${
                    checked
                      ? "bg-indigo-100 border-indigo-300 text-indigo-900"
                      : "bg-white border-slate-200 text-slate-600 hover:bg-slate-100"
                  }`}
                >
                  {name || email.split("@")[0]}
                </button>
              );
            })
          )}
        </div>
      )}
    </div>
  );
}

function ServiceTagsEditor({
  tags,
  onChange,
  suggestions,
}: {
  tags: string[];
  onChange: (tags: string[]) => void;
  suggestions: string[];
}) {
  const [draft, setDraft] = useState("");

  const addTag = (raw: string) => {
    const label = normalizeTag(raw);
    if (!label) return;
    if (tags.some((t) => t.toLowerCase() === label.toLowerCase())) return;
    onChange([...tags, label].sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" })));
    setDraft("");
  };

  const removeTag = (tag: string) => {
    onChange(tags.filter((t) => t !== tag));
  };

  const unusedSuggestions = suggestions.filter(
    (s) => !tags.some((t) => t.toLowerCase() === s.toLowerCase()),
  );

  return (
    <div>
      <p className="text-xs font-bold text-slate-500 mb-1">Tags du service</p>
      <p className="text-[11px] text-slate-400 mb-2">
        Ex. comptabilité → paye, facturation · maintenance → plomberie, électricité
      </p>
      <div className="flex flex-wrap gap-1.5 mb-2">
        {tags.length === 0 ? (
          <span className="text-xs text-slate-400 italic">Aucun tag — ajoutez-en pour le routage IA</span>
        ) : (
          tags.map((tag) => (
            <span
              key={tag}
              className="inline-flex items-center gap-1 rounded-full bg-emerald-100 px-2.5 py-1 text-[11px] font-bold text-emerald-900 ring-1 ring-emerald-200"
            >
              {tag}
              <button
                type="button"
                onClick={() => removeTag(tag)}
                className="rounded-full px-1 hover:bg-emerald-200"
                title="Retirer"
              >
                ×
              </button>
            </span>
          ))
        )}
      </div>
      <div className="flex flex-wrap gap-2">
        <input
          type="text"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              addTag(draft);
            }
          }}
          placeholder="Nouveau tag…"
          className="min-w-[140px] flex-1 rounded-lg border border-slate-200 px-2 py-1.5 text-sm"
        />
        <button
          type="button"
          onClick={() => addTag(draft)}
          className="rounded-lg bg-slate-800 px-3 py-1.5 text-xs font-bold text-white"
        >
          + Tag
        </button>
      </div>
      {unusedSuggestions.length > 0 ? (
        <div className="mt-2 flex flex-wrap gap-1">
          {unusedSuggestions.slice(0, 8).map((s) => (
            <button
              key={s}
              type="button"
              onClick={() => addTag(s)}
              className="rounded-full bg-slate-100 px-2 py-0.5 text-[10px] font-semibold text-slate-600 hover:bg-slate-200"
            >
              + {s}
            </button>
          ))}
        </div>
      ) : null}
    </div>
  );
}

function UnitCard({
  unit,
  org,
  tagSuggestions,
  members,
  membersLoading,
  onUpdate,
  onRemove,
}: {
  unit: RequestServiceUnit;
  org: RequestsOrgConfig;
  tagSuggestions: string[];
  members: DirectoryMemberOption[];
  membersLoading: boolean;
  onUpdate: (patch: Partial<RequestServiceUnit>) => void;
  onRemove: () => void;
}) {
  const parentOptions = org.units.filter((u) => u.id !== unit.id);

  const depth = unit.parentUnitId
    ? 1 + (org.units.find((u) => u.id === unit.parentUnitId)?.parentUnitId ? 1 : 0)
    : 0;

  return (
    <div
      className={`rounded-xl border p-4 space-y-3 ${unit.active ? "border-slate-200 bg-white" : "border-amber-200 bg-amber-50/40"}`}
      style={{ marginLeft: depth * 12 }}
    >
      <div className="flex flex-wrap items-center gap-2">
        <label className="flex items-center gap-2 text-sm font-bold text-slate-700">
          <input
            type="checkbox"
            checked={unit.active}
            onChange={(e) => onUpdate({ active: e.target.checked })}
          />
          Actif
        </label>
        {org.globalOversightUnitIds.includes(unit.id) ? (
          <span className="text-[10px] font-black uppercase tracking-wide text-violet-700 bg-violet-50 px-2 py-0.5 rounded">
            Supervision globale
          </span>
        ) : null}
        <button type="button" onClick={onRemove} className="ml-auto text-xs font-bold text-rose-600 underline">
          Supprimer
        </button>
      </div>

      <div className="grid gap-3 sm:grid-cols-2">
        <div>
          <label className="block text-xs font-bold text-slate-500 mb-1">Nom du service</label>
          <input
            className="w-full border rounded-lg p-2 text-sm"
            value={unit.label}
            onChange={(e) => onUpdate({ label: e.target.value })}
          />
        </div>
        <div>
          <label className="block text-xs font-bold text-slate-500 mb-1">Rattaché à (parent)</label>
          <select
            className="w-full border rounded-lg p-2 text-sm bg-white"
            value={unit.parentUnitId || ""}
            onChange={(e) => onUpdate({ parentUnitId: e.target.value || null })}
          >
            <option value="">— Racine —</option>
            {parentOptions.map((p) => (
              <option key={p.id} value={p.id}>
                {p.label}
              </option>
            ))}
          </select>
        </div>
      </div>

      <ServiceTagsEditor
        tags={unit.tags ?? []}
        onChange={(tags) => onUpdate({ tags })}
        suggestions={tagSuggestions}
      />

      <EmailMultiPicker
        label="Managers (peuvent confier des tâches)"
        values={unit.managerEmails}
        onChange={(managerEmails) => onUpdate({ managerEmails })}
        members={members}
        membersLoading={membersLoading}
      />

      <EmailMultiPicker
        label="Membres / exécutants"
        values={unit.memberEmails}
        onChange={(memberEmails) => onUpdate({ memberEmails })}
        members={members}
        membersLoading={membersLoading}
      />

      <label className="flex items-center gap-2 text-xs font-bold text-slate-700">
        <input
          type="checkbox"
          checked={unit.canDelegateToChildUnits}
          onChange={(e) => onUpdate({ canDelegateToChildUnits: e.target.checked })}
        />
        Peut confier aux sous-services (ex. CPE → surveillants)
      </label>
    </div>
  );
}

export default function RequestOrgEditor({ org, routing, onChange, members, membersLoading }: Props) {
  const tagSuggestions = useMemo(() => {
    const fromCatalog = routing.tagCatalog ?? [];
    const fromUnits = org.units.flatMap((u) => u.tags);
    return [...new Set([...fromCatalog, ...fromUnits])].sort((a, b) =>
      a.localeCompare(b, "fr", { sensitivity: "base" }),
    );
  }, [org.units, routing.tagCatalog]);

  const updateUnit = (idx: number, patch: Partial<RequestServiceUnit>) => {
    const units = [...org.units];
    units[idx] = { ...units[idx]!, ...patch };
    onChange({ ...org, units });
  };

  const removeUnit = (idx: number) => {
    const removed = org.units[idx]?.id;
    if (!removed) return;
    onChange({
      ...org,
      globalOversightUnitIds: org.globalOversightUnitIds.filter((id) => id !== removed),
      metierOversightUnitIds: (org.metierOversightUnitIds ?? []).filter((id) => id !== removed),
      units: org.units
        .filter((_, i) => i !== idx)
        .map((u) => (u.parentUnitId === removed ? { ...u, parentUnitId: null } : u)),
    });
  };

  const addUnit = () => {
    onChange({ ...org, units: [...org.units, newRequestServiceUnit()] });
  };

  const toggleGlobalOversight = (unitId: string) => {
    const has = org.globalOversightUnitIds.includes(unitId);
    onChange({
      ...org,
      globalOversightUnitIds: has
        ? org.globalOversightUnitIds.filter((id) => id !== unitId)
        : [...org.globalOversightUnitIds, unitId],
    });
  };

  const toggleMetierOversight = (unitId: string) => {
    const current = org.metierOversightUnitIds ?? [];
    const has = current.includes(unitId);
    onChange({
      ...org,
      metierOversightUnitIds: has ? current.filter((id) => id !== unitId) : [...current, unitId],
    });
  };

  const sortedUnits = useMemo(() => {
    const roots = org.units.filter((u) => !u.parentUnitId);
    const out: RequestServiceUnit[] = [];
    const walk = (parentId: string | null) => {
      const list = org.units.filter((u) => u.parentUnitId === parentId);
      for (const u of list) {
        out.push(u);
        walk(u.id);
      }
    };
    for (const r of roots) {
      out.push(r);
      walk(r.id);
    }
    const listed = new Set(out.map((u) => u.id));
    for (const u of org.units) {
      if (!listed.has(u.id)) out.push(u);
    }
    return out;
  }, [org.units]);

  return (
    <div className="space-y-6">
      <section className="rounded-2xl border border-indigo-100 bg-indigo-50/40 p-5 space-y-3">
        <h2 className="text-lg font-bold text-slate-900">Services</h2>
        <p className="text-sm text-slate-600 leading-relaxed">
          Créez un service, définissez ses <strong>tags métier</strong>, puis affectez un ou plusieurs{" "}
          <strong>managers</strong> et des <strong>membres</strong>. Les demandes arrivent en pile chez
          les managers — ils peuvent les prendre ou les confier.
        </p>
      </section>

      <section className="space-y-4">
        <div className="flex items-center justify-between gap-3">
          <h3 className="font-black text-slate-800">Liste des services</h3>
          <button
            type="button"
            onClick={addUnit}
            className="px-3 py-1.5 rounded-xl bg-indigo-600 text-white text-xs font-bold"
          >
            + Service
          </button>
        </div>

        {sortedUnits.map((unit) => {
          const idx = org.units.findIndex((u) => u.id === unit.id);
          if (idx < 0) return null;
          return (
            <div key={unit.id} className="space-y-2">
              <UnitCard
                unit={unit}
                org={org}
                tagSuggestions={tagSuggestions}
                members={members}
                membersLoading={membersLoading}
                onUpdate={(patch) => updateUnit(idx, patch)}
                onRemove={() => removeUnit(idx)}
              />
              <label className="flex items-center gap-2 text-xs font-bold text-violet-800 ml-2">
                <input
                  type="checkbox"
                  checked={org.globalOversightUnitIds.includes(unit.id)}
                  onChange={() => toggleGlobalOversight(unit.id)}
                />
                Supervision globale (direction générale — voit tout)
              </label>
              <label className="flex items-center gap-2 text-xs font-bold text-amber-900 ml-2">
                <input
                  type="checkbox"
                  checked={(org.metierOversightUnitIds ?? []).includes(unit.id)}
                  onChange={() => toggleMetierOversight(unit.id)}
                />
                Direction métier (voit les demandes de ce service)
              </label>
            </div>
          );
        })}
      </section>
    </div>
  );
}
