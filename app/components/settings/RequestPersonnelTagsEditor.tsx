"use client";

import { useMemo, useState } from "react";
import type {
  RequestsOrgConfig,
  RequestsRoutingConfig,
  RoutingPersonnelTags,
} from "@/app/lib/app-config-schemas";
import type { DirectoryMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";
import { normalizeIntranetRoles } from "@/app/lib/intranet-roles";
import { normalizeRequestEmail } from "@/app/lib/requests-board";

function isProfesseurOnly(roles: string[]): boolean {
  const normalized = normalizeIntranetRoles(roles);
  return normalized.length > 0 && normalized.every((r) => r === "professeur");
}

function normalizeTagLabel(raw: string): string {
  return raw.trim().replace(/\s+/g, " ");
}

type RowDraft = {
  email: string;
  personName: string;
  tags: string[];
  rolesLabel: string;
  serviceLabels: string[];
};

type Props = {
  config: RequestsRoutingConfig;
  org: RequestsOrgConfig;
  onChange: (next: RequestsRoutingConfig) => void;
  members: Array<
    DirectoryMemberOption & { roles?: string[]; firstName?: string; lastName?: string }
  >;
  membersLoading: boolean;
};

/** Affinage par personne : qui fait quoi (paye, facturation…) au sein d'un service. */
export default function RequestPersonnelTagsEditor({
  config,
  org,
  onChange,
  members,
  membersLoading,
}: Props) {
  const [newTag, setNewTag] = useState("");
  const [filter, setFilter] = useState("");
  const [localError, setLocalError] = useState<string | null>(null);

  const serviceTags = useMemo(
    () =>
      [...new Set(org.units.flatMap((u) => u.tags))].sort((a, b) =>
        a.localeCompare(b, "fr", { sensitivity: "base" }),
      ),
    [org.units],
  );

  const catalog = useMemo(() => {
    const merged = [...new Set([...(config.tagCatalog ?? []), ...serviceTags])];
    return merged.sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" }));
  }, [config.tagCatalog, serviceTags]);

  const assignedEmails = useMemo(() => {
    const emails = new Set<string>();
    for (const unit of org.units) {
      if (!unit.active) continue;
      for (const email of [...unit.managerEmails, ...unit.memberEmails]) {
        const n = normalizeRequestEmail(email);
        if (n) emails.add(n);
      }
    }
    return emails;
  }, [org.units]);

  const serviceLabelsByEmail = useMemo(() => {
    const map = new Map<string, string[]>();
    for (const unit of org.units) {
      if (!unit.active) continue;
      for (const email of [...unit.managerEmails, ...unit.memberEmails]) {
        const n = normalizeRequestEmail(email);
        if (!n) continue;
        const list = map.get(n) ?? [];
        if (!list.includes(unit.label)) list.push(unit.label);
        map.set(n, list);
      }
    }
    return map;
  }, [org.units]);

  const rows = useMemo(() => {
    const tagsByEmail = new Map(
      (config.personnelTags ?? []).map((p) => [normalizeRequestEmail(p.email), p]),
    );

    const staffMembers = members.filter((m) => {
      const email = normalizeRequestEmail(m.email || "");
      if (!email) return false;
      if (assignedEmails.has(email)) return true;
      if (!m.roles?.length) return true;
      return !isProfesseurOnly(m.roles);
    });

    const byEmail = new Map<string, RowDraft>();
    for (const m of staffMembers) {
      const email = normalizeRequestEmail(m.email || "");
      const existing = tagsByEmail.get(email);
      const personName =
        existing?.personName ||
        m.displayName?.trim() ||
        [m.firstName, m.lastName].filter(Boolean).join(" ").trim() ||
        email.split("@")[0] ||
        email;
      byEmail.set(email, {
        email,
        personName,
        tags: [...(existing?.tags ?? [])],
        rolesLabel: (m.roles || []).join(" · ") || "—",
        serviceLabels: serviceLabelsByEmail.get(email) ?? [],
      });
    }

    return [...byEmail.values()].sort((a, b) => {
      const aAssigned = a.serviceLabels.length > 0 ? 0 : 1;
      const bAssigned = b.serviceLabels.length > 0 ? 0 : 1;
      if (aAssigned !== bAssigned) return aAssigned - bAssigned;
      return a.personName.localeCompare(b.personName, "fr", { sensitivity: "base" });
    });
  }, [assignedEmails, config.personnelTags, members, serviceLabelsByEmail]);

  const filtered = useMemo(() => {
    const q = filter.trim().toLowerCase();
    if (!q) return rows;
    return rows.filter(
      (r) =>
        r.personName.toLowerCase().includes(q) ||
        r.email.includes(q) ||
        r.tags.some((t) => t.toLowerCase().includes(q)) ||
        r.serviceLabels.some((s) => s.toLowerCase().includes(q)) ||
        r.rolesLabel.toLowerCase().includes(q),
    );
  }, [filter, rows]);

  const persistTags = (nextCatalog: string[], nextRows: RowDraft[]) => {
    const catalogSet = new Set(nextCatalog.map((t) => t.toLowerCase()));
    const personnelTags: RoutingPersonnelTags[] = nextRows
      .map((r) => ({
        email: r.email,
        personName: r.personName.trim() || r.email,
        tags: r.tags.filter((t) => catalogSet.has(t.toLowerCase())),
      }))
      .filter((r) => r.tags.length > 0);

    onChange({
      ...config,
      tagCatalog: nextCatalog,
      personnelTags,
    });
  };

  const addTagToCatalog = (labelRaw: string) => {
    const label = normalizeTagLabel(labelRaw);
    if (!label) return;
    if (catalog.some((t) => t.toLowerCase() === label.toLowerCase())) {
      setLocalError(`Le tag « ${label} » existe déjà.`);
      return;
    }
    persistTags(
      [...catalog, label].sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" })),
      rows,
    );
    setNewTag("");
    setLocalError(null);
  };

  const removeTagFromCatalog = (tag: string) => {
    if (serviceTags.some((t) => t.toLowerCase() === tag.toLowerCase())) {
      setLocalError(`« ${tag} » est défini sur un service — retirez-le d'abord dans l'onglet Services.`);
      return;
    }
    const nextCatalog = catalog.filter((t) => t !== tag);
    const nextRows = rows.map((r) => ({ ...r, tags: r.tags.filter((t) => t !== tag) }));
    persistTags(nextCatalog, nextRows);
    setLocalError(null);
  };

  const togglePersonTag = (email: string, tag: string) => {
    const nextRows = rows.map((r) => {
      if (r.email !== email) return r;
      const has = r.tags.includes(tag);
      return { ...r, tags: has ? r.tags.filter((t) => t !== tag) : [...r.tags, tag] };
    });
    persistTags(catalog, nextRows);
  };

  return (
    <div className="space-y-5">
      <section className="rounded-2xl border border-amber-200 bg-amber-50/50 p-5">
        <h3 className="text-base font-black text-slate-900">Compétences par personne</h3>
        <p className="mt-1 text-sm leading-relaxed text-slate-600">
          Les tags des services (onglet 1) décrivent le périmètre global. Ici, vous précisez{" "}
          <strong>qui fait quoi</strong> : par exemple dans Comptabilité, Marie → paye, Paul →
          facturation. L&apos;IA s&apos;en sert pour cibler la bonne personne quand le service est déjà
          identifié.
        </p>
        {serviceTags.length > 0 ? (
          <p className="mt-2 text-xs text-amber-900">
            Tags disponibles depuis vos services : {serviceTags.join(", ")}
          </p>
        ) : null}
      </section>

      <section className="space-y-3 rounded-2xl border border-slate-200 bg-white p-5">
        <h4 className="text-sm font-black uppercase tracking-wide text-slate-700">
          Tags personnels (hors services)
        </h4>
        <p className="text-xs text-slate-500">
          Pour des compétences transverses (cycles école/collège/lycée, secrétariat…) non liées à un
          service.
        </p>
        <div className="flex flex-wrap gap-2">
          <input
            type="text"
            value={newTag}
            onChange={(e) => setNewTag(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                addTagToCatalog(newTag);
              }
            }}
            placeholder="ex. secrétariat lycée, transport…"
            className="min-w-[200px] flex-1 rounded-xl border border-slate-200 px-3 py-2 text-sm"
          />
          <button
            type="button"
            onClick={() => addTagToCatalog(newTag)}
            className="rounded-xl bg-slate-900 px-4 py-2 text-sm font-bold text-white"
          >
            + Ajouter
          </button>
        </div>
        {localError ? (
          <p className="rounded-xl bg-rose-50 px-3 py-2 text-sm text-rose-800">{localError}</p>
        ) : null}
        {catalog.length === 0 ? (
          <p className="text-xs text-slate-400">
            Ajoutez des tags dans Services ou ici pour commencer.
          </p>
        ) : (
          <div className="flex flex-wrap gap-2">
            {catalog.map((tag) => {
              const fromService = serviceTags.some((t) => t.toLowerCase() === tag.toLowerCase());
              return (
                <span
                  key={tag}
                  className={`inline-flex items-center gap-1.5 rounded-full px-3 py-1 text-xs font-bold ring-1 ${
                    fromService
                      ? "bg-emerald-50 text-emerald-900 ring-emerald-200"
                      : "bg-amber-100 text-amber-950 ring-amber-200"
                  }`}
                >
                  {tag}
                  {fromService ? (
                    <span className="text-[10px] font-normal opacity-70">service</span>
                  ) : (
                    <button
                      type="button"
                      title="Supprimer ce tag"
                      onClick={() => removeTagFromCatalog(tag)}
                      className="rounded-full px-1 hover:bg-amber-200"
                    >
                      ×
                    </button>
                  )}
                </span>
              );
            })}
          </div>
        )}
      </section>

      <section className="space-y-3">
        <div className="flex flex-wrap items-center gap-3">
          <h4 className="text-sm font-black uppercase tracking-wide text-slate-700">Personnes</h4>
          <input
            type="search"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder="Filtrer…"
            className="min-w-[200px] flex-1 rounded-xl border border-slate-200 px-3 py-2 text-sm"
          />
        </div>

        {membersLoading ? (
          <p className="text-xs text-slate-400">Chargement annuaire…</p>
        ) : catalog.length === 0 ? (
          <p className="rounded-xl border border-dashed border-slate-200 bg-slate-50 px-4 py-6 text-center text-sm text-slate-500">
            Créez d&apos;abord des tags (Services ou ci-dessus), puis cochez-les sur le personnel.
          </p>
        ) : (
          <div className="overflow-hidden rounded-2xl border border-slate-200 bg-white">
            <table className="w-full text-left text-sm">
              <thead className="bg-slate-50 text-[11px] uppercase tracking-wide text-slate-500">
                <tr>
                  <th className="px-4 py-3 font-bold">Personne</th>
                  <th className="px-4 py-3 font-bold">Service(s)</th>
                  <th className="px-4 py-3 font-bold">Tags</th>
                </tr>
              </thead>
              <tbody>
                {filtered.length === 0 ? (
                  <tr>
                    <td colSpan={3} className="px-4 py-8 text-center text-slate-400">
                      Aucun personnel trouvé.
                    </td>
                  </tr>
                ) : (
                  filtered.map((r) => (
                    <tr key={r.email} className="border-t border-slate-100 align-top">
                      <td className="px-4 py-3">
                        <p className="font-bold text-slate-900">{r.personName}</p>
                        <p className="break-all text-xs text-slate-500">{r.email}</p>
                      </td>
                      <td className="px-4 py-3 text-xs text-slate-600">
                        {r.serviceLabels.length > 0 ? r.serviceLabels.join(", ") : "—"}
                      </td>
                      <td className="px-4 py-3">
                        <div className="flex flex-wrap gap-1.5">
                          {catalog.map((tag) => {
                            const on = r.tags.includes(tag);
                            return (
                              <button
                                key={tag}
                                type="button"
                                onClick={() => togglePersonTag(r.email, tag)}
                                className={`rounded-full px-2.5 py-1 text-[11px] font-bold transition ${
                                  on
                                    ? "bg-emerald-600 text-white ring-1 ring-emerald-700"
                                    : "bg-slate-100 text-slate-600 ring-1 ring-slate-200 hover:bg-slate-200"
                                }`}
                              >
                                {tag}
                              </button>
                            );
                          })}
                        </div>
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        )}
      </section>
    </div>
  );
}
