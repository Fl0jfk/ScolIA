"use client";

import { useCallback, useEffect, useState } from "react";
import type { ExternalQuickLinkConfig } from "@/app/lib/app-config-schemas";
import { QuickLinkIcon } from "@/app/components/Dashboard/ExternalQuickLinks";
import {
  SettingsLoading,
  SettingsNotice,
  SettingsSection,
  settingsInputClass,
} from "@/app/components/settings/SettingsChrome";
import { dash } from "@/app/lib/dashboard-brand";
import { clearDashboardLinksCache } from "@/app/lib/dashboard-links-cache";
import { dashboardQuickLinksFromExternalLinks } from "@/app/lib/dashboard-external-links";
import { DEFAULT_QUICK_LINK_ROLES, newQuickLinkSlot } from "@/app/lib/dashboard-quick-links";
import { INTRANET_ROLE_OPTIONS } from "@/app/lib/intranet-roles";

const ROLE_OPTIONS = INTRANET_ROLE_OPTIONS.filter((r) => r.slug !== "parent" && r.slug !== "eleve");

export default function DashboardQuickLinksPanel() {
  const [links, setLinks] = useState<ExternalQuickLinkConfig[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/settings/external-links", { cache: "no-store" });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Chargement impossible");
      setLinks(Array.isArray(j.links) ? j.links : []);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  function patchLink(index: number, patch: Partial<ExternalQuickLinkConfig>) {
    setLinks((prev) => prev.map((row, i) => (i === index ? { ...row, ...patch } : row)));
  }

  function toggleRole(index: number, slug: string) {
    setLinks((prev) =>
      prev.map((row, i) => {
        if (i !== index) return row;
        const has = row.allowedRoles.includes(slug);
        const allowedRoles = has
          ? row.allowedRoles.filter((r) => r !== slug)
          : [...row.allowedRoles, slug];
        return { ...row, allowedRoles };
      }),
    );
  }

  async function save() {
    setSaving(true);
    setMsg(null);
    setError(null);
    try {
      const payload = links
        .map((l) => ({
          id: l.id.trim() || `link-${Date.now()}`,
          name: l.name.trim(),
          link: l.link.trim(),
          img: l.img?.trim() || "",
          allowedRoles: l.allowedRoles.length ? l.allowedRoles : [...DEFAULT_QUICK_LINK_ROLES],
        }))
        .filter((l) => l.name && l.link);

      const res = await fetch("/api/settings/external-links", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ links: payload }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Enregistrement impossible");
      setLinks(j.links || payload);
      clearDashboardLinksCache();
      setMsg("Raccourcis enregistrés — identiques pour tout le personnel concerné.");
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  }

  if (loading) {
    return <SettingsLoading label="Chargement des raccourcis…" />;
  }

  return (
    <SettingsSection
      icon="🔗"
      title="Raccourcis tableau de bord"
      description={
        <>
          Liens affichés en haut à droite du tableau de bord pour <strong>tout l&apos;établissement</strong>{" "}
          (selon les rôles cochés).
        </>
      }
    >
      <p className={`text-xs ${dash.textMid}`}>
        Source unique : <code className="text-[11px]">settings/external-links.json</code>. Les intégrations
        (Zeendoc, OneDrive) se configurent dans l&apos;onglet <strong>Intégrations</strong>.
      </p>

      {links.length > 0 && (
        <div className="rounded-2xl border border-white/70 bg-white/45 px-4 py-3">
          <p className="text-[10px] font-bold uppercase tracking-wide text-slate-500">Aperçu affiché sur le dashboard</p>
          <ul className="mt-2 space-y-1 text-sm text-slate-700">
            {dashboardQuickLinksFromExternalLinks(links).map((l) => (
              <li key={l.id}>
                <span className="font-semibold">{l.name}</span>
                <span className="text-slate-400"> — </span>
                <span className="text-xs text-slate-500 break-all">{l.link}</span>
              </li>
            ))}
          </ul>
        </div>
      )}

      {error ? <SettingsNotice tone="error">{error}</SettingsNotice> : null}
      {msg ? <SettingsNotice tone="ok">{msg}</SettingsNotice> : null}

      <div className="space-y-4">
        {links.length === 0 ? (
          <p className="text-sm text-slate-500">Aucun raccourci configuré.</p>
        ) : (
          links.map((row, index) => (
            <div key={row.id || index} className="space-y-3 rounded-2xl border border-white/70 bg-white/45 p-4">
              <div className="flex items-center justify-between gap-2">
                <span className="text-xs font-bold uppercase text-slate-500">Raccourci {index + 1}</span>
                <button
                  type="button"
                  onClick={() => setLinks((prev) => prev.filter((_, i) => i !== index))}
                  className="text-xs font-bold text-red-600 hover:text-red-800"
                >
                  Retirer
                </button>
              </div>
              <div className="flex gap-3">
                <QuickLinkIcon src={row.img || ""} name={row.name} />
                <div className="min-w-0 flex-1 space-y-2">
                  <input
                    className={`${settingsInputClass} mt-0`}
                    placeholder="Nom (ex. Arena Ac-Normandie)"
                    value={row.name}
                    onChange={(e) => patchLink(index, { name: e.target.value })}
                  />
                  <input
                    className={`${settingsInputClass} mt-0`}
                    placeholder="https://…"
                    type="url"
                    value={row.link}
                    onChange={(e) => patchLink(index, { link: e.target.value })}
                  />
                  <input
                    className={`${settingsInputClass} mt-0`}
                    placeholder="URL de l'image (optionnel)"
                    type="url"
                    value={row.img || ""}
                    onChange={(e) => patchLink(index, { img: e.target.value })}
                  />
                </div>
              </div>
              <div>
                <p className="text-[10px] font-bold uppercase tracking-wide text-slate-500 mb-2">Rôles autorisés</p>
                <div className="flex flex-wrap gap-2">
                  {ROLE_OPTIONS.map((role) => {
                    const checked = row.allowedRoles.includes(role.slug);
                    return (
                      <label
                        key={role.slug}
                        className={`inline-flex cursor-pointer items-center gap-1.5 rounded-lg border px-2 py-1 text-[11px] font-semibold ${
                          checked
                            ? "border-[color:var(--dash-primary)]/35 bg-[color:var(--dash-soft)] text-[var(--dash-ink)]"
                            : "border-white/70 bg-white/60 text-slate-600"
                        }`}
                      >
                        <input
                          type="checkbox"
                          className="sr-only"
                          checked={checked}
                          onChange={() => toggleRole(index, role.slug)}
                        />
                        {role.label}
                      </label>
                    );
                  })}
                </div>
              </div>
            </div>
          ))
        )}
      </div>

      <div className="flex flex-wrap gap-3">
        <button
          type="button"
          onClick={() => setLinks((prev) => [...prev, newQuickLinkSlot(prev.length)])}
          className="rounded-2xl border border-white/70 bg-white/70 px-4 py-2 text-sm font-semibold text-[var(--dash-primary)] shadow-sm"
        >
          + Ajouter un raccourci
        </button>
        <button
          type="button"
          disabled={saving}
          onClick={() => void save()}
          className="rounded-2xl bg-[var(--dash-primary)] px-4 py-2 text-sm font-semibold text-white shadow-[0_12px_28px_-16px_rgba(15,23,42,0.55)] disabled:opacity-50"
        >
          {saving ? "Enregistrement…" : "Enregistrer"}
        </button>
      </div>
    </SettingsSection>
  );
}
