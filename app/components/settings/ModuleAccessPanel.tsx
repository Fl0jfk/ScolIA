"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { SettingsNotice } from "@/app/components/settings/SettingsChrome";
import { dash } from "@/app/lib/dashboard-brand";
import { roleHasDefaultPhotocopiesOps } from "@/app/lib/module-access-defaults";

type RoleOpt = { slug: string; label: string };
type PillarGroup = {
  pillarId: string;
  title: string;
  modules: Array<{ id: string; name: string; description?: string }>;
};
type AccessOverride = {
  modules: string[];
  dossierSections?: string[];
  photocopiesOps?: boolean;
  profRoomAdmin?: boolean;
};
type AccessConfig = {
  byRole: Record<string, AccessOverride>;
  byUser: Record<string, AccessOverride>;
};
type MemberRow = {
  userId: string;
  externalUserId: string;
  email: string;
  displayName: string;
  roles: string[];
  baselineModules: string[];
  baselineDossierSections: string[];
};

const DOSSIER_SECTIONS: { id: string; label: string }[] = [
  { id: "identite", label: "Identité" },
  { id: "scolarite", label: "Scolarité" },
  { id: "famille", label: "Famille" },
  { id: "documents", label: "Documents" },
  { id: "notes", label: "Notes" },
  { id: "vie_scolaire", label: "Vie scolaire" },
  { id: "sante", label: "Santé" },
  { id: "facturation", label: "Facturation" },
];

function roleLabel(roles: RoleOpt[], slug: string): string {
  return roles.find((r) => r.slug === slug)?.label || slug;
}

export default function ModuleAccessPanel() {
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [roleOptions, setRoleOptions] = useState<RoleOpt[]>([]);
  const [pillars, setPillars] = useState<PillarGroup[]>([]);
  const [config, setConfig] = useState<AccessConfig>({ byRole: {}, byUser: {} });
  const [members, setMembers] = useState<MemberRow[]>([]);
  const [roleFilter, setRoleFilter] = useState("");
  const [query, setQuery] = useState("");
  const [selectedUserId, setSelectedUserId] = useState("");

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/settings/module-access", { cache: "no-store" });
      const j = (await res.json()) as {
        error?: string;
        config?: AccessConfig;
        roles?: RoleOpt[];
        pillars?: PillarGroup[];
        members?: MemberRow[];
      };
      if (!res.ok) throw new Error(j.error || "Chargement impossible");
      setConfig({
        byRole: j.config?.byRole || {},
        byUser: j.config?.byUser || {},
      });
      setRoleOptions(j.roles || []);
      setPillars(j.pillars || []);
      const list = j.members || [];
      setMembers(list);
      setSelectedUserId((prev) => {
        if (prev && list.some((m) => m.userId === prev)) return prev;
        return list[0]?.userId || "";
      });
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const filteredMembers = useMemo(() => {
    const q = query.trim().toLowerCase();
    const list = members.filter((m) => {
      if (roleFilter && !m.roles.includes(roleFilter)) return false;
      if (!q) return true;
      const blob = `${m.displayName} ${m.email} ${m.roles.join(" ")}`.toLowerCase();
      return blob.includes(q);
    });
    // Tri par nom de famille (dernier mot du displayName), puis prénom.
    return [...list].sort((a, b) => {
      const partsA = a.displayName.trim().split(/\s+/);
      const partsB = b.displayName.trim().split(/\s+/);
      const lastA = partsA.length > 1 ? partsA[partsA.length - 1]! : partsA[0] || a.email;
      const lastB = partsB.length > 1 ? partsB[partsB.length - 1]! : partsB[0] || b.email;
      const byLast = lastA.localeCompare(lastB, "fr", { sensitivity: "base" });
      if (byLast !== 0) return byLast;
      return a.displayName.localeCompare(b.displayName, "fr", { sensitivity: "base" });
    });
  }, [members, roleFilter, query]);

  const selected = useMemo(
    () => members.find((m) => m.userId === selectedUserId) ?? null,
    [members, selectedUserId],
  );

  useEffect(() => {
    if (!selectedUserId) return;
    if (filteredMembers.some((m) => m.userId === selectedUserId)) return;
    setSelectedUserId(filteredMembers[0]?.userId || "");
  }, [filteredMembers, selectedUserId]);

  const effectiveModules = useMemo(() => {
    if (!selected) return new Set<string>();
    const ov = config.byUser[selected.userId];
    if (ov) return new Set(ov.modules);
    // Baseline : overrides de rôle s’ils existent, sinon catalogue par rôles.
    const fromRoleOverrides = new Set<string>();
    let hasRoleOverride = false;
    for (const role of selected.roles) {
      const roleOv = config.byRole[role];
      if (roleOv) {
        hasRoleOverride = true;
        for (const id of roleOv.modules) fromRoleOverrides.add(id);
      }
    }
    if (hasRoleOverride) {
      for (const role of selected.roles) {
        if (config.byRole[role]) continue;
        for (const id of selected.baselineModules) fromRoleOverrides.add(id);
      }
      return fromRoleOverrides;
    }
    return new Set(selected.baselineModules);
  }, [selected, config.byUser, config.byRole]);

  const effectiveSections = useMemo(() => {
    if (!selected) return new Set<string>();
    const ov = config.byUser[selected.userId];
    if (ov?.dossierSections?.length) return new Set(ov.dossierSections);
    return new Set(selected.baselineDossierSections);
  }, [selected, config.byUser]);

  const photocopiesOpsByUser = Boolean(selected && config.byUser[selected.userId]?.photocopiesOps);
  const photocopiesOpsByRole = Boolean(
    selected?.roles.some((r) => roleHasDefaultPhotocopiesOps(r)),
  );
  const photocopiesOps = photocopiesOpsByUser || photocopiesOpsByRole;
  const profRoomAdmin = Boolean(selected && config.byUser[selected.userId]?.profRoomAdmin);

  const isCustomized = Boolean(selected && config.byUser[selected.userId]);

  const patchUserOverride = (
    patch: Partial<AccessOverride> & {
      modules: string[];
      clearPhotocopiesOps?: boolean;
      clearProfRoomAdmin?: boolean;
    },
  ) => {
    if (!selected) return;
    setConfig((prev) => {
      const current = prev.byUser[selected.userId];
      let photocopiesOpsVal =
        patch.photocopiesOps !== undefined ? patch.photocopiesOps : current?.photocopiesOps;
      let profRoomAdminVal =
        patch.profRoomAdmin !== undefined ? patch.profRoomAdmin : current?.profRoomAdmin;
      if (patch.clearPhotocopiesOps) photocopiesOpsVal = undefined;
      if (patch.clearProfRoomAdmin) profRoomAdminVal = undefined;
      const next: AccessOverride = {
        modules: patch.modules,
        dossierSections:
          patch.dossierSections ??
          current?.dossierSections ??
          selected.baselineDossierSections ??
          ["identite", "scolarite"],
      };
      if (photocopiesOpsVal) next.photocopiesOps = true;
      if (profRoomAdminVal) next.profRoomAdmin = true;
      return {
        ...prev,
        byUser: {
          ...prev.byUser,
          [selected.userId]: next,
        },
      };
    });
  };

  const setModulesForUser = (next: Set<string>) => {
    if (!selected) return;
    const modules = [...next];
    const keepPhotoOps =
      config.byUser[selected.userId]?.photocopiesOps === true && modules.includes("photocopies-couleur");
    const keepRoomAdmin =
      config.byUser[selected.userId]?.profRoomAdmin === true && modules.includes("prof-room");
    patchUserOverride({
      modules,
      photocopiesOps: keepPhotoOps || undefined,
      clearPhotocopiesOps: !keepPhotoOps,
      clearProfRoomAdmin: !keepRoomAdmin,
      profRoomAdmin: keepRoomAdmin || undefined,
    });
  };

  const toggleModule = (moduleId: string) => {
    const next = new Set(effectiveModules);
    if (next.has(moduleId)) next.delete(moduleId);
    else next.add(moduleId);
    setModulesForUser(next);
  };

  const toggleSection = (sectionId: string) => {
    if (!selected || !effectiveModules.has("eleve-dossier")) return;
    const next = new Set(effectiveSections);
    if (next.has(sectionId)) next.delete(sectionId);
    else next.add(sectionId);
    if (!next.has("identite")) next.add("identite");
    if (!next.has("scolarite")) next.add("scolarite");
    patchUserOverride({
      modules: [...effectiveModules],
      dossierSections: [...next],
    });
  };

  const togglePhotocopiesOps = () => {
    if (!selected) return;
    const nextModules = new Set(effectiveModules);
    const next = !photocopiesOps;
    if (next) nextModules.add("photocopies-couleur");
    patchUserOverride({
      modules: [...nextModules],
      photocopiesOps: next || undefined,
      clearPhotocopiesOps: !next,
    });
  };

  const toggleProfRoomAdmin = () => {
    if (!selected) return;
    const nextModules = new Set(effectiveModules);
    const next = !profRoomAdmin;
    if (next) nextModules.add("prof-room");
    patchUserOverride({
      modules: [...nextModules],
      profRoomAdmin: next || undefined,
      clearProfRoomAdmin: !next,
    });
  };

  const save = async () => {
    setSaving(true);
    setError(null);
    setMessage(null);
    try {
      const res = await fetch("/api/settings/module-access", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ config }),
      });
      const j = (await res.json()) as { error?: string; config?: AccessConfig };
      if (!res.ok) throw new Error(j.error || "Enregistrement impossible");
      setConfig({
        byRole: j.config?.byRole || {},
        byUser: j.config?.byUser || {},
      });
      setMessage("Droits individuels enregistrés.");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  };

  const resetUser = async () => {
    if (!selected) return;
    setSaving(true);
    setError(null);
    setMessage(null);
    try {
      const res = await fetch("/api/settings/module-access", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ resetUser: selected.userId }),
      });
      const j = (await res.json()) as { error?: string; config?: AccessConfig };
      if (!res.ok) throw new Error(j.error || "Réinitialisation impossible");
      setConfig({
        byRole: j.config?.byRole || {},
        byUser: j.config?.byUser || {},
      });
      setMessage(`Accès de ${selected.displayName} remis sur le profil de rôle.`);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return <p className="text-sm text-slate-500">Chargement des droits par personne…</p>;
  }

  return (
    <div className="space-y-5">
      <div className="rounded-2xl border border-white/70 bg-white/70 p-4 shadow-sm">
        <h2 className={`text-base font-bold ${dash.ink}`}>Droits d’accès aux modules</h2>
        <p className="mt-1 text-sm text-slate-600">
          Sélectionnez une personne, puis cochez ses modules. Deux collègues avec le même rôle
          peuvent avoir des accès différents. Le filtre rôle sert uniquement à retrouver quelqu’un
          plus vite.
        </p>
      </div>

      {error ? <SettingsNotice tone="error">{error}</SettingsNotice> : null}
      {message ? <SettingsNotice tone="ok">{message}</SettingsNotice> : null}

      <div className="grid gap-4 lg:grid-cols-[minmax(16rem,22rem)_1fr]">
        <aside className="space-y-3 rounded-2xl border border-slate-200/80 bg-white/80 p-3 shadow-sm">
          <label className="flex flex-col gap-1 text-xs font-bold uppercase tracking-wide text-slate-500">
            Filtrer par rôle
            <select
              className="rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm font-medium normal-case text-slate-900"
              value={roleFilter}
              onChange={(e) => setRoleFilter(e.target.value)}
            >
              <option value="">Tous les rôles</option>
              {roleOptions.map((r) => (
                <option key={r.slug} value={r.slug}>
                  {r.label}
                </option>
              ))}
            </select>
          </label>
          <label className="flex flex-col gap-1 text-xs font-bold uppercase tracking-wide text-slate-500">
            Rechercher
            <input
              type="search"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Nom ou e-mail"
              className="rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm font-medium normal-case text-slate-900"
            />
          </label>
          <ul className="max-h-[28rem] space-y-1 overflow-y-auto pr-1">
            {filteredMembers.length === 0 ? (
              <li className="px-2 py-4 text-center text-sm text-slate-500">Aucune personne.</li>
            ) : (
              filteredMembers.map((m) => {
                const active = m.userId === selectedUserId;
                const customized = Boolean(config.byUser[m.userId]);
                return (
                  <li key={m.userId}>
                    <button
                      type="button"
                      onClick={() => setSelectedUserId(m.userId)}
                      className={`w-full rounded-xl px-3 py-2 text-left transition ${
                        active
                          ? `${dash.bgPrimary} text-white shadow-sm`
                          : "bg-slate-50 text-slate-800 hover:bg-slate-100"
                      }`}
                    >
                      <span className="block truncate text-sm font-bold">{m.displayName}</span>
                      <span
                        className={`mt-0.5 block truncate text-[11px] ${
                          active ? "text-white/80" : "text-slate-500"
                        }`}
                      >
                        {m.email}
                        {customized ? " · personnalisé" : ""}
                      </span>
                    </button>
                  </li>
                );
              })
            )}
          </ul>
        </aside>

        <div className="space-y-4">
          {selected ? (
            <>
              <div className="flex flex-wrap items-end justify-between gap-3 rounded-2xl border border-slate-200/80 bg-white/80 px-4 py-3 shadow-sm">
                <div className="min-w-0">
                  <p className="text-base font-black text-slate-900">{selected.displayName}</p>
                  <p className="truncate text-sm text-slate-500">{selected.email}</p>
                  <p className="mt-1 text-xs text-slate-500">
                    Rôles :{" "}
                    {selected.roles.map((r) => roleLabel(roleOptions, r)).join(", ") || "—"}
                  </p>
                </div>
                <div className="flex flex-wrap items-center gap-2">
                  <span
                    className={`inline-flex items-center rounded-full px-3 py-1 text-[11px] font-semibold ${
                      isCustomized
                        ? "border border-amber-200 bg-amber-50 text-amber-900"
                        : "border border-slate-200 bg-slate-50 text-slate-600"
                    }`}
                  >
                    {isCustomized ? "Accès personnalisé" : "Profil de rôle"}
                  </span>
                  <button
                    type="button"
                    disabled={saving || !isCustomized}
                    onClick={() => void resetUser()}
                    className="rounded-xl border border-slate-200 bg-white px-3 py-2 text-xs font-bold text-slate-700 hover:bg-slate-50 disabled:opacity-40"
                  >
                    Remettre au profil rôle
                  </button>
                  <button
                    type="button"
                    disabled={saving}
                    onClick={() => void save()}
                    className={`rounded-xl px-4 py-2 text-sm font-bold text-white ${dash.bgPrimary} disabled:opacity-50`}
                  >
                    {saving ? "Enregistrement…" : "Enregistrer"}
                  </button>
                </div>
              </div>

              <div className="space-y-4">
                {pillars.map((pillar) => (
                  <section
                    key={pillar.pillarId}
                    className="overflow-hidden rounded-2xl border border-slate-200/80 bg-white/80 shadow-sm"
                  >
                    <header className="border-b border-slate-100 bg-slate-50/80 px-4 py-2.5">
                      <h3 className="text-sm font-black text-slate-900">{pillar.title}</h3>
                    </header>
                    <ul className="divide-y divide-slate-100">
                      {pillar.modules.map((mod) => {
                        const checked = effectiveModules.has(mod.id);
                        return (
                          <li key={mod.id} className="px-4 py-3">
                            <label className="flex cursor-pointer items-start gap-3">
                              <input
                                type="checkbox"
                                className="mt-1 h-4 w-4 rounded border-slate-300"
                                checked={checked}
                                onChange={() => toggleModule(mod.id)}
                              />
                              <span className="min-w-0">
                                <span className="block text-sm font-bold text-slate-900">
                                  {mod.name}
                                </span>
                                {mod.description ? (
                                  <span className="mt-0.5 block text-xs text-slate-500">
                                    {mod.description}
                                  </span>
                                ) : null}
                              </span>
                            </label>
                            {mod.id === "eleve-dossier" && checked ? (
                              <div className="mt-3 ml-7 grid gap-2 sm:grid-cols-2 lg:grid-cols-4">
                                {DOSSIER_SECTIONS.map((sec) => (
                                  <label
                                    key={sec.id}
                                    className="flex items-center gap-2 rounded-lg border border-slate-100 bg-slate-50/70 px-2.5 py-1.5 text-xs font-semibold text-slate-700"
                                  >
                                    <input
                                      type="checkbox"
                                      className="h-3.5 w-3.5 rounded border-slate-300"
                                      checked={effectiveSections.has(sec.id)}
                                      disabled={sec.id === "identite" || sec.id === "scolarite"}
                                      onChange={() => toggleSection(sec.id)}
                                    />
                                    {sec.label}
                                  </label>
                                ))}
                              </div>
                            ) : null}
                            {mod.id === "photocopies-couleur" ? (
                              <label className="mt-3 ml-7 flex cursor-pointer items-start gap-2 rounded-lg border border-teal-100 bg-teal-50/60 px-2.5 py-2 text-xs font-semibold text-slate-800">
                                <input
                                  type="checkbox"
                                  className="mt-0.5 h-3.5 w-3.5 rounded border-slate-300"
                                  checked={photocopiesOps}
                                  onChange={togglePhotocopiesOps}
                                />
                                <span>
                                  Réceptionnaire impressions
                                  <span className="mt-0.5 block font-medium text-slate-500">
                                    Voit la file d&apos;impression après validation direction et peut
                                    marquer « prête ».
                                    {photocopiesOpsByRole && !photocopiesOpsByUser
                                      ? " Inclus par défaut pour le rôle Accueil."
                                      : ""}
                                  </span>
                                </span>
                              </label>
                            ) : null}
                            {mod.id === "prof-room" ? (
                              <label className="mt-3 ml-7 flex cursor-pointer items-start gap-2 rounded-lg border border-indigo-100 bg-indigo-50/60 px-2.5 py-2 text-xs font-semibold text-slate-800">
                                <input
                                  type="checkbox"
                                  className="mt-0.5 h-3.5 w-3.5 rounded border-slate-300"
                                  checked={profRoomAdmin}
                                  onChange={toggleProfRoomAdmin}
                                />
                                <span>
                                  Administrateur réservation de salles
                                  <span className="mt-0.5 block font-medium text-slate-500">
                                    Paramétrage du module (salles, matières, couleurs).
                                  </span>
                                </span>
                              </label>
                            ) : null}
                          </li>
                        );
                      })}
                    </ul>
                  </section>
                ))}
              </div>
            </>
          ) : (
            <p className="rounded-2xl border border-dashed border-slate-200 bg-white/60 px-4 py-10 text-center text-sm text-slate-500">
              Choisissez une personne dans la liste.
            </p>
          )}
        </div>
      </div>
    </div>
  );
}
