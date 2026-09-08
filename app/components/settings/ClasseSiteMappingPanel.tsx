"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import {
  SettingsLoading,
  SettingsNotice,
  SettingsSection,
  settingsSelectClass,
} from "@/app/components/settings/SettingsChrome";
import { dash } from "@/app/lib/dashboard-brand";

type SiteRef = { siteId: string; label: string; kind: string | null };
type SiecleDivision = { code: string; libelle: string | null };
type ClasseRow = {
  className: string;
  classKey: string;
  eleveCount: number;
  mappedSiteId: string | null;
  mappedSiecleCode: string | null;
  suggestedSiteId: string | null;
  suggestedSiecleCode: string | null;
};

type DraftRow = {
  classKey: string;
  className: string;
  eleveCount: number;
  siteId: string;
  siecleCode: string;
};

export default function ClasseSiteMappingPanel() {
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const [sites, setSites] = useState<SiteRef[]>([]);
  const [siecleDivisions, setSiecleDivisions] = useState<SiecleDivision[]>([]);
  const [draft, setDraft] = useState<DraftRow[]>([]);
  const [filter, setFilter] = useState<"all" | "unassigned" | string>("all");
  const [q, setQ] = useState("");

  const load = useCallback(async () => {
    setLoading(true);
    setErr(null);
    try {
      const res = await fetch("/api/settings/classe-mapping", { cache: "no-store" });
      const j = (await res.json()) as {
        error?: string;
        sites?: SiteRef[];
        siecleDivisions?: SiecleDivision[];
        classes?: ClasseRow[];
      };
      if (!res.ok) throw new Error(j.error || "Chargement impossible");
      setSites(j.sites || []);
      setSiecleDivisions(j.siecleDivisions || []);
      setDraft(
        (j.classes || []).map((c) => ({
          classKey: c.classKey,
          className: c.className,
          eleveCount: c.eleveCount,
          siteId: c.mappedSiteId || c.suggestedSiteId || "",
          siecleCode: c.mappedSiecleCode || c.suggestedSiecleCode || "",
        })),
      );
    } catch (e) {
      setErr(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const siteLabel = useCallback(
    (siteId: string) => sites.find((s) => s.siteId === siteId)?.label || siteId,
    [sites],
  );

  const filtered = useMemo(() => {
    const needle = q.trim().toLowerCase();
    return draft.filter((row) => {
      if (filter === "unassigned" && row.siteId) return false;
      if (filter !== "all" && filter !== "unassigned" && row.siteId !== filter) return false;
      if (!needle) return true;
      return (
        row.className.toLowerCase().includes(needle) ||
        row.siecleCode.toLowerCase().includes(needle)
      );
    });
  }, [draft, filter, q]);

  const unassignedCount = draft.filter((r) => !r.siteId).length;
  const mappedCount = draft.filter((r) => r.siteId).length;

  function updateRow(classKey: string, patch: Partial<Pick<DraftRow, "siteId" | "siecleCode">>) {
    setDraft((prev) => prev.map((r) => (r.classKey === classKey ? { ...r, ...patch } : r)));
    setMsg(null);
  }

  async function save() {
    setSaving(true);
    setErr(null);
    setMsg(null);
    try {
      const mappings = draft
        .filter((r) => r.siteId)
        .map((r) => ({
          className: r.className,
          siteId: r.siteId,
          siecleCode: r.siecleCode || null,
        }));
      const res = await fetch("/api/settings/classe-mapping", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ mappings }),
      });
      const j = (await res.json().catch(() => ({}))) as { error?: string };
      if (!res.ok) throw new Error(j.error || "Enregistrement impossible");
      setMsg(
        `${mappings.length} classe${mappings.length > 1 ? "s" : ""} enregistrée${
          mappings.length > 1 ? "s" : ""
        }. Les dossiers élèves utilisent ce matching tout de suite.`,
      );
      await load();
    } catch (e) {
      setErr(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  }

  if (loading) return <SettingsLoading />;

  return (
    <SettingsSection
      icon="🏷️"
      title="Classes → établissements"
      description="Toutes les classes qui apparaissent (liste élèves + rectorat). Rangez-les une fois par an vers l’école, le collège ou le lycée, et rattachez le code Siècle quand il existe. Ça pilote les dossiers élèves et le futur rattachement des professeurs."
    >
      {err ? <SettingsNotice tone="warn">{err}</SettingsNotice> : null}
      {msg ? <SettingsNotice tone="ok">{msg}</SettingsNotice> : null}

      {sites.length === 0 ? (
        <SettingsNotice tone="warn">
          Configurez d’abord les sites école / collège / lycée dans l’onglet Sites / directions.
        </SettingsNotice>
      ) : null}

      <div className="flex flex-wrap items-center gap-2">
        <input
          value={q}
          onChange={(e) => setQ(e.target.value)}
          placeholder="Filtrer une classe…"
          className="min-w-[12rem] flex-1 rounded-xl border border-white/70 bg-white/80 px-3 py-2 text-sm"
        />
        <select
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          className={`${settingsSelectClass} !mt-0 max-w-xs`}
          aria-label="Filtrer par établissement"
        >
          <option value="all">Toutes ({draft.length})</option>
          <option value="unassigned">Sans établissement ({unassignedCount})</option>
          {sites.map((s) => (
            <option key={s.siteId} value={s.siteId}>
              {s.label} ({draft.filter((r) => r.siteId === s.siteId).length})
            </option>
          ))}
        </select>
        <button
          type="button"
          disabled={saving || sites.length === 0}
          onClick={() => void save()}
          className="rounded-xl bg-[color:var(--dash-primary)] px-4 py-2 text-sm font-bold text-white shadow-sm hover:opacity-95 disabled:opacity-60"
        >
          {saving ? "Enregistrement…" : "Enregistrer le matching"}
        </button>
      </div>

      <p className={`text-xs ${dash.textMid}`}>
        {mappedCount} classe{mappedCount > 1 ? "s" : ""} rattachée
        {mappedCount > 1 ? "s" : ""}
        {unassignedCount > 0
          ? ` · ${unassignedCount} encore à ranger (pré-remplies si on a pu le deviner)`
          : ""}
        {siecleDivisions.length === 0
          ? " · Aucune division rectorat importée (onglet Éducation nationale)."
          : ` · ${siecleDivisions.length} codes rectorat disponibles.`}
      </p>

      {filtered.length === 0 ? (
        <p className={`text-sm ${dash.textMid}`}>Aucune classe à afficher.</p>
      ) : (
        <div className="overflow-x-auto rounded-2xl border border-white/60 bg-white/50">
          <table className="min-w-full text-left text-sm">
            <thead className="bg-white/70 text-xs font-bold uppercase tracking-wide text-slate-500">
              <tr>
                <th className="px-3 py-2">Classe observée</th>
                <th className="px-3 py-2">Élèves</th>
                <th className="px-3 py-2">Établissement</th>
                <th className="px-3 py-2">Classe rectorat (Siècle)</th>
              </tr>
            </thead>
            <tbody>
              {filtered.map((row) => (
                <tr key={row.classKey} className="border-t border-slate-100">
                  <td className="px-3 py-2 font-semibold text-slate-900">{row.className}</td>
                  <td className="px-3 py-2 text-slate-600">{row.eleveCount || "—"}</td>
                  <td className="px-3 py-2">
                    <select
                      value={row.siteId}
                      onChange={(e) => updateRow(row.classKey, { siteId: e.target.value })}
                      className={`${settingsSelectClass} !mt-0`}
                      aria-label={`Établissement pour ${row.className}`}
                    >
                      <option value="">— Choisir —</option>
                      {sites.map((s) => (
                        <option key={s.siteId} value={s.siteId}>
                          {s.label}
                        </option>
                      ))}
                    </select>
                  </td>
                  <td className="px-3 py-2">
                    <select
                      value={row.siecleCode}
                      onChange={(e) => updateRow(row.classKey, { siecleCode: e.target.value })}
                      className={`${settingsSelectClass} !mt-0`}
                      aria-label={`Code rectorat pour ${row.className}`}
                    >
                      <option value="">— Pas de matching —</option>
                      {siecleDivisions.map((d) => (
                        <option key={d.code} value={d.code}>
                          {d.code}
                          {d.libelle && d.libelle !== d.code ? ` — ${d.libelle}` : ""}
                        </option>
                      ))}
                    </select>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {filter !== "all" && filter !== "unassigned" ? (
        <p className={`text-xs ${dash.textMid}`}>
          Vue {siteLabel(filter)} uniquement. Enregistrer sauve tout le matching, pas seulement ce filtre.
        </p>
      ) : null}
    </SettingsSection>
  );
}
