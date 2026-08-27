"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import {
  accompagnateursToFormFields,
  escortDisplayName,
  escortMatchesRoleFilter,
  formFieldsToAccompagnateurs,
  hydrateAccompagnateursFromDirectory,
  TRAVELS_ESCORT_ROLE_FILTERS,
  type TravelsAccompagnateur,
  type TravelsEscortDirectoryUser,
  type TravelsEscortRoleFilter,
} from "@/app/lib/travels-accompagnateurs";

export { accompagnateursToFormFields, formFieldsToAccompagnateurs };

type TripAccompagnateursSelectProps = {
  value: TravelsAccompagnateur[];
  onChange: (next: TravelsAccompagnateur[]) => void;
  required?: boolean;
  disabled?: boolean;
  id?: string;
};

function norm(s: string) {
  return s
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "");
}

export default function TripAccompagnateursSelect({
  value,
  onChange,
  required,
  disabled,
  id,
}: TripAccompagnateursSelectProps) {
  const [users, setUsers] = useState<TravelsEscortDirectoryUser[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [roleFilter, setRoleFilter] = useState<TravelsEscortRoleFilter>("all");
  const [autreDraft, setAutreDraft] = useState("");

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      setLoading(true);
      setLoadError(null);
      try {
        const res = await fetch("/api/travels/directory-escorts", { cache: "no-store" });
        const j = (await res.json()) as { users?: TravelsEscortDirectoryUser[]; error?: string };
        if (!res.ok) throw new Error(j.error || "Chargement impossible");
        if (!cancelled) setUsers(j.users || []);
      } catch (e) {
        if (!cancelled) {
          setLoadError(e instanceof Error ? e.message : "Erreur");
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  // Rematch noms libres → annuaire une fois l’annuaire chargé.
  useEffect(() => {
    if (!users.length || value.length === 0) return;
    const hydrated = hydrateAccompagnateursFromDirectory(value, users);
    const changed = hydrated.some(
      (h, i) =>
        h.userId !== value[i]?.userId ||
        h.source !== value[i]?.source ||
        h.name !== value[i]?.name,
    );
    if (changed) onChange(hydrated);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- une seule passe de rematch annuaire
  }, [users]);

  const selectedIds = useMemo(
    () => new Set(value.filter((a) => a.userId).map((a) => a.userId!)),
    [value],
  );

  const filterCounts = useMemo(() => {
    const counts: Record<TravelsEscortRoleFilter, number> = {
      all: users.length,
      administratif: 0,
      direction: 0,
      professeur: 0,
      autre: value.filter((a) => a.source === "autre").length,
    };
    for (const u of users) {
      if (escortMatchesRoleFilter(u.roles, "administratif")) counts.administratif += 1;
      if (escortMatchesRoleFilter(u.roles, "direction")) counts.direction += 1;
      if (escortMatchesRoleFilter(u.roles, "professeur")) counts.professeur += 1;
    }
    return counts;
  }, [users, value]);

  const visibleUsers = useMemo(() => {
    if (roleFilter === "autre") return [];
    const q = norm(search.trim());
    return users
      .filter((u) => escortMatchesRoleFilter(u.roles, roleFilter))
      .filter((u) => {
        if (!q) return true;
        const blob = norm(
          [escortDisplayName(u), u.email, u.firstName, u.lastName, ...(u.roles || [])].join(" "),
        );
        return blob.includes(q);
      })
      .sort((a, b) => {
        const la = (a.lastName || escortDisplayName(a)).localeCompare(
          b.lastName || escortDisplayName(b),
          "fr",
        );
        if (la !== 0) return la;
        return (a.firstName || "").localeCompare(b.firstName || "", "fr");
      });
  }, [users, roleFilter, search]);

  const toggleUser = useCallback(
    (u: TravelsEscortDirectoryUser) => {
      if (selectedIds.has(u.externalUserId)) {
        onChange(value.filter((a) => a.userId !== u.externalUserId));
        return;
      }
      onChange([
        ...value,
        {
          userId: u.externalUserId,
          name: escortDisplayName(u),
          email: u.email,
          source: "directory",
        },
      ]);
    },
    [onChange, selectedIds, value],
  );

  const removeItem = useCallback(
    (index: number) => {
      onChange(value.filter((_, i) => i !== index));
    },
    [onChange, value],
  );

  const addAutre = useCallback(() => {
    const name = autreDraft.trim();
    if (!name) return;
    const already = value.some((a) => a.name.toLowerCase() === name.toLowerCase());
    if (already) {
      setAutreDraft("");
      return;
    }
    onChange([...value, { name, source: "autre" }]);
    setAutreDraft("");
  }, [autreDraft, onChange, value]);

  return (
    <div id={id} className="space-y-3">
      {value.length > 0 ? (
        <div className="flex flex-wrap gap-1.5">
          {value.map((a, idx) => (
            <span
              key={`${a.userId || a.name}-${idx}`}
              className={`inline-flex items-center gap-1.5 rounded-lg border px-2.5 py-1 text-xs font-semibold ${
                a.source === "autre"
                  ? "border-amber-200 bg-amber-50 text-amber-900"
                  : "border-indigo-200 bg-indigo-50 text-indigo-900"
              }`}
            >
              {a.name}
              {a.source === "autre" ? (
                <span className="text-[10px] font-bold uppercase opacity-70">autre</span>
              ) : null}
              <button
                type="button"
                disabled={disabled}
                onClick={() => removeItem(idx)}
                className="ml-0.5 text-slate-500 hover:text-rose-600 disabled:opacity-40"
                aria-label={`Retirer ${a.name}`}
              >
                ×
              </button>
            </span>
          ))}
        </div>
      ) : null}

      <p className="text-[11px] font-semibold text-slate-500">
        {value.length} accompagnateur{value.length === 1 ? "" : "s"} sélectionné
        {value.length === 1 ? "" : "s"}
      </p>

      <div className="flex flex-wrap gap-1.5">
        {TRAVELS_ESCORT_ROLE_FILTERS.map((f) => (
          <button
            key={f.id}
            type="button"
            disabled={disabled}
            onClick={() => setRoleFilter(f.id)}
            className={`rounded-lg px-2.5 py-1.5 text-xs font-bold border transition disabled:opacity-50 ${
              roleFilter === f.id
                ? f.id === "autre"
                  ? "bg-amber-500 text-white border-amber-500"
                  : "bg-indigo-600 text-white border-indigo-600"
                : "bg-white text-slate-700 border-slate-200 hover:border-indigo-300"
            }`}
          >
            {f.label}
            {f.id !== "all" ? (
              <span className="ml-1 opacity-80">({filterCounts[f.id]})</span>
            ) : null}
          </button>
        ))}
      </div>

      {roleFilter === "autre" ? (
        <div className="space-y-2 rounded-xl border border-amber-200 bg-amber-50/70 p-3">
          <p className="text-xs text-amber-900">
            Personnes hors annuaire (parents d’élèves, intervenants externes…).
          </p>
          <div className="flex flex-col sm:flex-row gap-2">
            <input
              type="text"
              disabled={disabled}
              value={autreDraft}
              onChange={(e) => setAutreDraft(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  addAutre();
                }
              }}
              placeholder="Ex. Mme Martin (parent)"
              className="flex-1 rounded-xl border border-amber-200 bg-white px-3 py-2 text-sm outline-indigo-500"
            />
            <button
              type="button"
              disabled={disabled || !autreDraft.trim()}
              onClick={addAutre}
              className="rounded-xl bg-amber-500 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
            >
              Ajouter
            </button>
          </div>
        </div>
      ) : (
        <>
          <input
            type="search"
            disabled={disabled || loading}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Rechercher un accompagnateur…"
            className="w-full rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 text-sm outline-indigo-500"
          />
          <div className="max-h-52 overflow-y-auto rounded-xl border border-slate-200 bg-slate-50 divide-y divide-slate-100">
            {loading ? (
              <p className="p-3 text-sm text-slate-400">Chargement de l’annuaire…</p>
            ) : loadError ? (
              <p className="p-3 text-sm text-rose-600">{loadError}</p>
            ) : visibleUsers.length === 0 ? (
              <p className="p-3 text-sm text-slate-400 italic">
                Aucune personne dans ce filtre. Utilisez « Autre » pour un parent ou un externe.
              </p>
            ) : (
              visibleUsers.map((u) => {
                const on = selectedIds.has(u.externalUserId);
                return (
                  <button
                    key={u.externalUserId}
                    type="button"
                    disabled={disabled}
                    onClick={() => toggleUser(u)}
                    className={`w-full flex items-center justify-between gap-2 px-3 py-2 text-left text-sm transition disabled:opacity-50 ${
                      on ? "bg-indigo-50" : "hover:bg-white"
                    }`}
                  >
                    <span className="min-w-0">
                      <span className="block font-semibold text-slate-900 truncate">
                        {escortDisplayName(u)}
                      </span>
                      <span className="block text-[11px] text-slate-500 truncate">{u.email}</span>
                    </span>
                    <span
                      className={`shrink-0 rounded-md px-2 py-0.5 text-[10px] font-bold ${
                        on ? "bg-indigo-600 text-white" : "bg-white border border-slate-200 text-slate-500"
                      }`}
                    >
                      {on ? "Sélectionné" : "Ajouter"}
                    </span>
                  </button>
                );
              })
            )}
          </div>
        </>
      )}

      {required && value.length === 0 ? (
        <p className="text-[11px] text-rose-600 font-semibold">
          Sélectionnez au moins un accompagnateur (annuaire ou Autre).
        </p>
      ) : null}
    </div>
  );
}
