"use client";

import { useMemo, useState } from "react";
import type { RequestsRoutingConfig, RoutingPersonnelTags } from "@/app/lib/app-config-schemas";
import type { DirectoryMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";
import { normalizeIntranetRoles } from "@/app/lib/intranet-roles";

function isProfesseurOnly(roles: string[]): boolean {
  const normalized = normalizeIntranetRoles(roles);
  return normalized.length > 0 && normalized.every((r) => r === "professeur");
}

function normalizeEmail(email: string) {
  return email.trim().toLowerCase();
}

function normalizeTagLabel(raw: string): string {
  return raw.trim().replace(/\s+/g, " ");
}

type RowDraft = {
  email: string;
  personName: string;
  tags: string[];
  rolesLabel: string;
  fromAssignment: boolean;
};

type Props = {
  config: RequestsRoutingConfig;
  onChange: (next: RequestsRoutingConfig) => void;
  members: Array<
    DirectoryMemberOption & { roles?: string[]; firstName?: string; lastName?: string }
  >;
  membersLoading: boolean;
};

/** Éditeur tags (sans fetch) — intégré au panneau réglages demandes. */
export default function RequestPersonnelTagsEditor({ config, onChange, members, membersLoading }: Props) {
  const [newTag, setNewTag] = useState("");
  const [filter, setFilter] = useState("");
  const [localError, setLocalError] = useState<string | null>(null);

  const catalog = config.tagCatalog ?? [];

  const rows = useMemo(() => {
    const tagsByEmail = new Map(
      (config.personnelTags ?? []).map((p) => [normalizeEmail(p.email), p]),
    );
    const assignmentEmails = new Set(
      (config.assignments ?? [])
        .filter((a) => a.active)
        .map((a) => normalizeEmail(a.email)),
    );

    const staffMembers = members.filter((m) => {
      const email = normalizeEmail(m.email || "");
      if (!email) return false;
      if (!m.roles?.length) return true;
      return !isProfesseurOnly(m.roles);
    });

    const byEmail = new Map<string, RowDraft>();
    for (const m of staffMembers) {
      const email = normalizeEmail(m.email || "");
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
        fromAssignment: assignmentEmails.has(email),
      });
    }

    return [...byEmail.values()].sort((a, b) =>
      a.personName.localeCompare(b.personName, "fr", { sensitivity: "base" }),
    );
  }, [config.assignments, config.personnelTags, members]);

  const filtered = useMemo(() => {
    const q = filter.trim().toLowerCase();
    if (!q) return rows;
    return rows.filter(
      (r) =>
        r.personName.toLowerCase().includes(q) ||
        r.email.includes(q) ||
        r.tags.some((t) => t.toLowerCase().includes(q)) ||
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
    const nextCatalog = catalog.filter((t) => t !== tag);
    const nextRows = rows.map((r) => ({ ...r, tags: r.tags.filter((t) => t !== tag) }));
    persistTags(nextCatalog, nextRows);
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
        <h3 className="text-base font-black text-slate-900">Tags du personnel</h3>
        <p className="mt-1 text-sm leading-relaxed text-slate-600">
          Affinent le routage IA (cycles école / collège / lycée, compétences métier). En cas de doute
          sur la <em>personne</em> mais service identifié, la demande va quand même vers la pile du
          service — les tags aident surtout à choisir le bon service ou cycle.
        </p>
        <div className="mt-3 flex flex-wrap gap-1.5">
          {["lycée", "collège", "école", "secrétariat lycée", "secrétariat collège", "plomberie"].map(
            (s) => (
              <button
                key={s}
                type="button"
                onClick={() => addTagToCatalog(s)}
                className="rounded-full bg-white px-2.5 py-1 text-[11px] font-bold text-amber-900 ring-1 ring-amber-200 hover:bg-amber-100"
              >
                + {s}
              </button>
            ),
          )}
        </div>
      </section>

      <section className="space-y-3 rounded-2xl border border-slate-200 bg-white p-5">
        <h4 className="text-sm font-black uppercase tracking-wide text-slate-700">Catalogue de tags</h4>
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
            placeholder="ex. factures, transport, infirmerie…"
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
          <p className="text-xs text-slate-400">Aucun tag — créez-en pour commencer.</p>
        ) : (
          <div className="flex flex-wrap gap-2">
            {catalog.map((tag) => (
              <span
                key={tag}
                className="inline-flex items-center gap-1.5 rounded-full bg-amber-100 px-3 py-1 text-xs font-bold text-amber-950 ring-1 ring-amber-200"
              >
                {tag}
                <button
                  type="button"
                  title="Supprimer ce tag"
                  onClick={() => removeTagFromCatalog(tag)}
                  className="rounded-full px-1 text-amber-800/70 hover:bg-amber-200"
                >
                  ×
                </button>
              </span>
            ))}
          </div>
        )}
      </section>

      <section className="space-y-3">
        <div className="flex flex-wrap items-center gap-3">
          <h4 className="text-sm font-black uppercase tracking-wide text-slate-700">
            Attribution aux personnes
          </h4>
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
            Créez d&apos;abord des tags, puis cochez-les sur le personnel.
          </p>
        ) : (
          <div className="overflow-hidden rounded-2xl border border-slate-200 bg-white">
            <table className="w-full text-left text-sm">
              <thead className="bg-slate-50 text-[11px] uppercase tracking-wide text-slate-500">
                <tr>
                  <th className="px-4 py-3 font-bold">Personne</th>
                  <th className="px-4 py-3 font-bold">Rôles</th>
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
                        {r.fromAssignment ? (
                          <span className="mt-1 inline-block rounded-full bg-indigo-50 px-2 py-0.5 text-[10px] font-bold text-indigo-700">
                            Affectation legacy
                          </span>
                        ) : null}
                      </td>
                      <td className="px-4 py-3 text-xs text-slate-600">{r.rolesLabel}</td>
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
