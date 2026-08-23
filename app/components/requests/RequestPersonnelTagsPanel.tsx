"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import type {
  RequestsRoutingConfig,
  RoutingPersonnelTags,
} from "@/app/lib/app-config-schemas";
import { normalizeIntranetRoles } from "@/app/lib/intranet-roles";

type DirectoryMember = {
  email?: string;
  firstName?: string;
  lastName?: string;
  displayName?: string;
  fullName?: string;
  roles?: string[];
};

/** Exclut les comptes dont le seul rôle est professeur. */
function isProfesseurOnly(roles: string[]): boolean {
  const normalized = normalizeIntranetRoles(roles);
  return normalized.length > 0 && normalized.every((r) => r === "professeur");
}

function normalizeEmail(email: string) {
  return email.trim().toLowerCase();
}

function displayName(m: DirectoryMember, email: string) {
  const full =
    m.displayName?.trim() ||
    m.fullName?.trim() ||
    [m.firstName, m.lastName].filter(Boolean).join(" ").trim();
  return full || email.split("@")[0] || email;
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

/** Onglet admin : catalogue de tags + attribution au personnel (hors professeurs). */
export default function RequestPersonnelTagsPanel() {
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [ok, setOk] = useState<string | null>(null);
  const [config, setConfig] = useState<RequestsRoutingConfig | null>(null);
  const [catalog, setCatalog] = useState<string[]>([]);
  const [newTag, setNewTag] = useState("");
  const [rows, setRows] = useState<RowDraft[]>([]);
  const [filter, setFilter] = useState("");

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [rrRes, memRes] = await Promise.all([
        fetch("/api/settings/requests-routing", { cache: "no-store" }),
        fetch("/api/members", { cache: "no-store" }),
      ]);
      const rr = await rrRes.json();
      const mem = await memRes.json();
      if (!rrRes.ok) throw new Error(rr.error || "Impossible de charger le routage.");
      if (!memRes.ok) throw new Error(mem.error || "Impossible de charger les membres.");

      const cfg = rr.config as RequestsRoutingConfig;
      setConfig(cfg);
      setCatalog([...(cfg.tagCatalog || [])]);

      const members = (mem.users || []) as DirectoryMember[];
      const tagsByEmail = new Map(
        (cfg.personnelTags || []).map((p) => [normalizeEmail(p.email), p]),
      );
      const assignmentEmails = new Set(
        (cfg.assignments || [])
          .filter((a) => a.active)
          .map((a) => normalizeEmail(a.email)),
      );

      // Tout le personnel sauf professeurs (rôle unique)
      const staffMembers = members.filter((m) => {
        const email = normalizeEmail(m.email || "");
        if (!email) return false;
        if (isProfesseurOnly(m.roles || [])) return false;
        return true;
      });

      const byEmail = new Map<string, RowDraft>();

      for (const m of staffMembers) {
        const email = normalizeEmail(m.email || "");
        const existing = tagsByEmail.get(email);
        byEmail.set(email, {
          email,
          personName: existing?.personName || displayName(m, email),
          tags: [...(existing?.tags || [])],
          rolesLabel: (m.roles || []).join(" · ") || "—",
          fromAssignment: assignmentEmails.has(email),
        });
      }

      const next = [...byEmail.values()].sort((a, b) =>
        a.personName.localeCompare(b.personName, "fr", { sensitivity: "base" }),
      );
      setRows(next);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur de chargement");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

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

  const addTagToCatalog = () => {
    const label = normalizeTagLabel(newTag);
    if (!label) return;
    const exists = catalog.some((t) => t.toLowerCase() === label.toLowerCase());
    if (exists) {
      setError(`Le tag « ${label} » existe déjà.`);
      return;
    }
    setCatalog((prev) =>
      [...prev, label].sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" })),
    );
    setNewTag("");
    setError(null);
    setOk(null);
  };

  const removeTagFromCatalog = (tag: string) => {
    setCatalog((prev) => prev.filter((t) => t !== tag));
    setRows((prev) =>
      prev.map((r) => ({
        ...r,
        tags: r.tags.filter((t) => t !== tag),
      })),
    );
    setOk(null);
  };

  const togglePersonTag = (email: string, tag: string) => {
    setRows((prev) =>
      prev.map((r) => {
        if (r.email !== email) return r;
        const has = r.tags.includes(tag);
        return {
          ...r,
          tags: has ? r.tags.filter((t) => t !== tag) : [...r.tags, tag],
        };
      }),
    );
    setOk(null);
  };

  const save = async () => {
    if (!config) return;
    setSaving(true);
    setError(null);
    setOk(null);
    try {
      const catalogSet = new Set(catalog.map((t) => t.toLowerCase()));
      const personnelTags: RoutingPersonnelTags[] = rows
        .map((r) => ({
          email: r.email,
          personName: r.personName.trim() || r.email,
          tags: r.tags.filter((t) => catalogSet.has(t.toLowerCase())),
        }))
        .filter((r) => r.tags.length > 0);

      const next: RequestsRoutingConfig = {
        ...config,
        tagCatalog: catalog,
        personnelTags,
      };

      const res = await fetch("/api/settings/requests-routing", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(next),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Enregistrement impossible");
      const saved = j.config as RequestsRoutingConfig;
      setConfig(saved);
      setCatalog([...(saved.tagCatalog || [])]);
      setOk("Catalogue et attributions enregistrés — l’IA s’en servira pour le routage.");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return <p className="py-8 text-sm text-slate-500">Chargement du personnel…</p>;
  }

  return (
    <div className="space-y-5">
      <div className="rounded-2xl border border-amber-200 bg-amber-50/50 p-5">
        <h2 className="text-lg font-black text-slate-900">Tags du personnel</h2>
        <p className="mt-1 text-sm leading-relaxed text-slate-600">
          Créez vos tags, puis cochez-les sur chaque personne. Pour un groupe scolaire, ajoutez
          des tags de cycle (<em>lycée</em>, <em>collège</em>, <em>école</em>,{" "}
          <em>secrétariat lycée</em>…) : l&apos;IA croise ça avec eleves.json (nom d&apos;élève
          ou e-mail parent) pour rattacher la demande au bon établissement.
        </p>
        <div className="mt-3 flex flex-wrap gap-1.5">
          {["lycée", "collège", "école", "secrétariat lycée", "secrétariat collège"].map((s) => (
            <button
              key={s}
              type="button"
              onClick={() => {
                const label = normalizeTagLabel(s);
                setCatalog((prev) => {
                  if (prev.some((t) => t.toLowerCase() === label.toLowerCase())) return prev;
                  return [...prev, label].sort((a, b) =>
                    a.localeCompare(b, "fr", { sensitivity: "base" }),
                  );
                });
                setOk(null);
              }}
              className="rounded-full bg-white px-2.5 py-1 text-[11px] font-bold text-amber-900 ring-1 ring-amber-200 hover:bg-amber-100"
            >
              + {s}
            </button>
          ))}
        </div>
      </div>

      {/* Catalogue */}
      <section className="space-y-3 rounded-2xl border border-slate-200 bg-white p-5">
        <h3 className="text-sm font-black uppercase tracking-wide text-slate-700">
          1 — Créer des tags
        </h3>
        <div className="flex flex-wrap gap-2">
          <input
            type="text"
            value={newTag}
            onChange={(e) => setNewTag(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                addTagToCatalog();
              }
            }}
            placeholder="ex. plomberie, factures, transport…"
            className="min-w-[200px] flex-1 rounded-xl border border-slate-200 px-3 py-2 text-sm"
          />
          <button
            type="button"
            onClick={addTagToCatalog}
            className="rounded-xl bg-slate-900 px-4 py-2 text-sm font-bold text-white"
          >
            + Ajouter le tag
          </button>
        </div>
        {catalog.length === 0 ? (
          <p className="text-xs text-slate-400">Aucun tag pour l’instant — créez-en au moins un.</p>
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
                  title="Supprimer ce tag du catalogue"
                  onClick={() => removeTagFromCatalog(tag)}
                  className="rounded-full px-1 text-amber-800/70 hover:bg-amber-200 hover:text-amber-950"
                >
                  ×
                </button>
              </span>
            ))}
          </div>
        )}
      </section>

      {/* Attribution */}
      <section className="space-y-3">
        <div className="flex flex-wrap items-center gap-3">
          <h3 className="text-sm font-black uppercase tracking-wide text-slate-700">
            2 — Attribuer aux personnes
          </h3>
          <input
            type="search"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder="Filtrer (nom, e-mail, tag…)"
            className="min-w-[200px] flex-1 rounded-xl border border-slate-200 px-3 py-2 text-sm"
          />
          <button
            type="button"
            disabled={saving}
            onClick={() => void save()}
            className="rounded-xl bg-indigo-600 px-5 py-2.5 text-sm font-black text-white disabled:opacity-50"
          >
            {saving ? "Enregistrement…" : "Enregistrer"}
          </button>
        </div>

        {error ? (
          <p className="rounded-xl bg-rose-50 px-3 py-2 text-sm text-rose-800">{error}</p>
        ) : null}
        {ok ? (
          <p className="rounded-xl bg-emerald-50 px-3 py-2 text-sm text-emerald-800">{ok}</p>
        ) : null}

        {catalog.length === 0 ? (
          <p className="rounded-xl border border-dashed border-slate-200 bg-slate-50 px-4 py-6 text-center text-sm text-slate-500">
            Créez d’abord des tags ci-dessus, puis revenez les cocher sur le personnel.
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
                      Aucun personnel trouvé (hors professeurs).
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
                            Dans le routage
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
