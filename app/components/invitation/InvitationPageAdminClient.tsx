"use client";

import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import Link from "next/link";
import RequireModuleAccess from "@/app/components/RequireModuleAccess";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import {
  diplomaLabel,
  formatInvitationEleveLabel,
  situationStatusLabel,
  type InvitationAskSituation,
  type InvitationDashboardStats,
  type InvitationDiplomaMode,
  type InvitationDuplicateSuspect,
  type InvitationEligibleRecord,
  type InvitationPageRecord,
  type InvitationRsvpRecord,
  type InvitationTheme,
} from "@/app/lib/invitation-types";

type DashboardPayload = {
  page: InvitationPageRecord;
  rsvps: InvitationRsvpRecord[];
  stats: InvitationDashboardStats;
  duplicates: InvitationDuplicateSuspect[];
  eligible: InvitationEligibleRecord[];
};

function toDatetimeLocal(iso: string | null): string {
  if (!iso) return "";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "";
  const pad = (n: number) => String(n).padStart(2, "0");
  // Affichage Europe/Paris approximatif via toLocaleString parts
  const parts = new Intl.DateTimeFormat("en-GB", {
    timeZone: "Europe/Paris",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).formatToParts(d);
  const get = (type: string) => parts.find((p) => p.type === type)?.value || "00";
  return `${get("year")}-${get("month")}-${get("day")}T${get("hour")}:${get("minute")}`;
}

function fromDatetimeLocal(local: string): string | null {
  const raw = local.trim();
  if (!raw) return null;
  const m = /^(\d{4}-\d{2}-\d{2})T(\d{2}:\d{2})/.exec(raw);
  if (!m) return null;
  // Interpréter comme Europe/Paris → ISO
  const [y, mo, da] = m[1].split("-").map(Number);
  const [h, mi] = m[2].split(":").map(Number);
  const utcGuess = new Date(Date.UTC(y, mo - 1, da, h - 2, mi));
  // Affiner via paris parts
  for (let offset = 0; offset < 3; offset++) {
    const candidate = new Date(Date.UTC(y, mo - 1, da, h - (1 + offset), mi));
    const parts = new Intl.DateTimeFormat("en-GB", {
      timeZone: "Europe/Paris",
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    }).formatToParts(candidate);
    const get = (type: string) => parts.find((p) => p.type === type)?.value || "";
    if (
      get("year") === String(y) &&
      get("month") === String(mo).padStart(2, "0") &&
      get("day") === String(da).padStart(2, "0") &&
      get("hour") === String(h).padStart(2, "0") &&
      get("minute") === String(mi).padStart(2, "0")
    ) {
      return candidate.toISOString();
    }
  }
  return utcGuess.toISOString();
}

export default function InvitationPageAdminClient({ pageId }: { pageId: string }) {
  const [data, setData] = useState<DashboardPayload | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [filter, setFilter] = useState<"all" | "oui" | "non">("all");
  const [search, setSearch] = useState("");

  // Form draft
  const [title, setTitle] = useState("");
  const [slug, setSlug] = useState("");
  const [intro, setIntro] = useState("");
  const [theme, setTheme] = useState<InvitationTheme>("remise_diplome");
  const [enabled, setEnabled] = useState(false);
  const [startsLocal, setStartsLocal] = useState("");
  const [endsLocal, setEndsLocal] = useState("");
  const [location, setLocation] = useState("");
  const [diplomaMode, setDiplomaMode] = useState<InvitationDiplomaMode>("both");
  const [maxTotal, setMaxTotal] = useState(200);
  const [maxPerEleve, setMaxPerEleve] = useState(4);
  const [notifyEmail, setNotifyEmail] = useState("");
  const [requireEligible, setRequireEligible] = useState(false);
  const [askSituation, setAskSituation] = useState<InvitationAskSituation>("off");
  const [rsvpClosesLocal, setRsvpClosesLocal] = useState("");
  const [eligiblePaste, setEligiblePaste] = useState("");
  const [eligibleBusy, setEligibleBusy] = useState(false);
  const [eligibleFileName, setEligibleFileName] = useState<string | null>(null);
  const eligibleFileRef = useRef<HTMLInputElement | null>(null);

  const applyPage = useCallback((page: InvitationPageRecord) => {
    setTitle(page.title);
    setSlug(page.slug);
    setIntro(page.intro);
    setTheme(page.theme);
    setEnabled(page.enabled);
    setStartsLocal(toDatetimeLocal(page.startsAt));
    setEndsLocal(toDatetimeLocal(page.endsAt));
    setLocation(page.location);
    setDiplomaMode(page.diplomaMode);
    setMaxTotal(page.maxTotalPersons);
    setMaxPerEleve(page.maxPersonsPerEleve);
    setNotifyEmail(page.notifyEmail || "");
    setRequireEligible(page.requireEligible);
    setAskSituation(page.askSituation);
    setRsvpClosesLocal(toDatetimeLocal(page.rsvpClosesAt));
  }, []);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch(`/api/invitation/pages/${pageId}/dashboard`, {
        cache: "no-store",
      });
      const json = (await res.json()) as DashboardPayload & { error?: string };
      if (!res.ok) throw new Error(json.error || "Chargement impossible.");
      setData({
        ...json,
        eligible: json.eligible || [],
        stats: {
          ...json.stats,
          eligibleCount: json.stats.eligibleCount ?? (json.eligible || []).length,
        },
      });
      applyPage(json.page);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, [pageId, applyPage]);

  useEffect(() => {
    void load();
  }, [load]);

  async function saveSettings() {
    setSaving(true);
    setError(null);
    setNotice(null);
    try {
      const res = await fetch(`/api/invitation/pages/${pageId}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          title,
          slug,
          intro,
          theme,
          enabled,
          startsAt: fromDatetimeLocal(startsLocal),
          endsAt: fromDatetimeLocal(endsLocal),
          location,
          diplomaMode,
          maxTotalPersons: maxTotal,
          maxPersonsPerEleve: maxPerEleve,
          notifyEmail: notifyEmail.trim() || null,
          requireEligible,
          askSituation,
          rsvpClosesAt: fromDatetimeLocal(rsvpClosesLocal),
        }),
      });
      const json = (await res.json()) as { page?: InvitationPageRecord; error?: string };
      if (!res.ok) throw new Error(json.error || "Enregistrement impossible.");
      setNotice("Paramètres enregistrés.");
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  }

  async function removePage() {
    if (!window.confirm("Supprimer cette page et toutes les réponses ?")) return;
    setSaving(true);
    try {
      const res = await fetch(`/api/invitation/pages/${pageId}`, { method: "DELETE" });
      const json = (await res.json()) as { error?: string };
      if (!res.ok) throw new Error(json.error || "Suppression impossible.");
      window.location.href = "/etablissement/evenements/invitations";
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setSaving(false);
    }
  }

  async function dupAction(action: "link" | "dismiss", rsvpIds: string[]) {
    setError(null);
    setNotice(null);
    try {
      const res = await fetch(`/api/invitation/pages/${pageId}/duplicates`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action, rsvpIds }),
      });
      const json = (await res.json()) as { error?: string };
      if (!res.ok) throw new Error(json.error || "Action impossible.");
      setNotice(action === "link" ? "Réponses reliées." : "Signal doublon ignoré.");
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  async function deleteRsvp(rsvpId: string, label: string) {
    if (!window.confirm(`Supprimer la réponse de ${label} ?`)) return;
    setError(null);
    setNotice(null);
    try {
      const res = await fetch(`/api/invitation/pages/${pageId}/rsvps/${rsvpId}`, {
        method: "DELETE",
      });
      const json = (await res.json()) as { error?: string };
      if (!res.ok) throw new Error(json.error || "Suppression impossible.");
      setNotice("Réponse supprimée.");
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  async function importEligible(mode: "append" | "replace") {
    if (!eligiblePaste.trim()) {
      setError("Collez une liste (Prénom;Nom ou Prénom Nom par ligne).");
      return;
    }
    if (
      mode === "replace" &&
      !window.confirm("Remplacer toute la liste d’élèves autorisés ?")
    ) {
      return;
    }
    setEligibleBusy(true);
    setError(null);
    setNotice(null);
    try {
      const res = await fetch(`/api/invitation/pages/${pageId}/eligible`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ mode, text: eligiblePaste }),
      });
      const json = (await res.json()) as {
        inserted?: number;
        skipped?: number;
        total?: number;
        error?: string;
      };
      if (!res.ok) throw new Error(json.error || "Import impossible.");
      setEligiblePaste("");
      setNotice(
        `Liste mise à jour : ${json.total ?? 0} élève(s) · ${json.inserted ?? 0} traité(s) · ${json.skipped ?? 0} ignoré(s).`,
      );
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setEligibleBusy(false);
    }
  }

  async function importEligibleExcel(mode: "append" | "replace") {
    const input = eligibleFileRef.current;
    const file = input?.files?.[0];
    if (!file) {
      setError("Choisissez un fichier Excel (.xlsx) à importer.");
      return;
    }
    if (
      mode === "replace" &&
      !window.confirm("Remplacer toute la liste d’élèves autorisés par ce fichier ?")
    ) {
      return;
    }
    setEligibleBusy(true);
    setError(null);
    setNotice(null);
    try {
      const form = new FormData();
      form.set("mode", mode);
      form.set("file", file);
      const res = await fetch(`/api/invitation/pages/${pageId}/eligible`, {
        method: "POST",
        body: form,
      });
      const json = (await res.json()) as {
        inserted?: number;
        skipped?: number;
        total?: number;
        error?: string;
      };
      if (!res.ok) throw new Error(json.error || "Import Excel impossible.");
      setNotice(
        `Excel importé : ${json.total ?? 0} élève(s) · ${json.inserted ?? 0} traité(s) · ${json.skipped ?? 0} ignoré(s).`,
      );
      if (input) input.value = "";
      setEligibleFileName(null);
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setEligibleBusy(false);
    }
  }

  async function removeEligible(id: string, label: string) {
    if (!window.confirm(`Retirer ${label} de la liste ?`)) return;
    setError(null);
    try {
      const res = await fetch(`/api/invitation/pages/${pageId}/eligible?id=${encodeURIComponent(id)}`, {
        method: "DELETE",
      });
      const json = (await res.json()) as { error?: string };
      if (!res.ok) throw new Error(json.error || "Suppression impossible.");
      setNotice("Élève retiré de la liste.");
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  const duplicateIdSet = useMemo(() => {
    const s = new Set<string>();
    for (const d of data?.duplicates || []) {
      for (const id of d.rsvpIds) s.add(id);
    }
    return s;
  }, [data?.duplicates]);

  const filteredRsvps = useMemo(() => {
    const q = search.trim().toLowerCase();
    return (data?.rsvps || []).filter((r) => {
      if (filter !== "all" && r.response !== filter) return false;
      if (!q) return true;
      const hay = `${r.eleveFirstName} ${r.eleveLastName} ${r.parentEmail}`.toLowerCase();
      return hay.includes(q);
    });
  }, [data?.rsvps, filter, search]);

  const publicUrl =
    typeof window !== "undefined" && data
      ? `${window.location.origin}/invitation/${data.page.slug}`
      : data
        ? `/invitation/${data.page.slug}`
        : "";

  return (
    <RequireModuleAccess moduleId="evenements">
      <ModulePageShell>
        <ModulePageHeader
          eyebrow="Invitations"
          title={data?.page.title || "Invitation"}
          description="Paramétrage et tableau de bord des réponses"
          actions={
            <Link
              href="/etablissement/evenements/invitations"
              className="rounded-xl border border-slate-200 px-3 py-2 text-xs font-bold text-slate-700 hover:bg-slate-50"
            >
              Liste
            </Link>
          }
        />

        {loading && !data ? (
          <p className="text-sm text-slate-500">Chargement…</p>
        ) : null}

        {error ? (
          <p className="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-800">
            {error}
          </p>
        ) : null}
        {notice ? (
          <p className="rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-900">
            {notice}
          </p>
        ) : null}

        {data ? (
          <>
            <section className="grid gap-3 sm:grid-cols-5">
              <Kpi label="Oui" value={String(data.stats.ouiCount)} />
              <Kpi label="Non" value={String(data.stats.nonCount)} />
              <Kpi label="Personnes" value={String(data.stats.totalPersons)} />
              <Kpi
                label="Places restantes"
                value={
                  data.stats.placesRemaining == null
                    ? "—"
                    : `${data.stats.placesRemaining} / ${data.stats.maxTotalPersons}`
                }
              />
              <Kpi label="Liste invités" value={String(data.stats.eligibleCount)} />
            </section>

            <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-4">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <h2 className="text-sm font-black text-slate-900">Paramètres</h2>
                <div className="flex flex-wrap gap-2">
                  <ModuleButton type="button" onClick={() => void saveSettings()} disabled={saving}>
                    {saving ? "Enregistrement…" : "Enregistrer"}
                  </ModuleButton>
                  <button
                    type="button"
                    onClick={() => void removePage()}
                    className="rounded-xl border border-rose-200 px-3 py-2 text-xs font-bold text-rose-700 hover:bg-rose-50"
                  >
                    Supprimer
                  </button>
                </div>
              </div>

              <label className="flex items-center gap-2 text-sm font-semibold text-slate-800">
                <input
                  type="checkbox"
                  checked={enabled}
                  onChange={(e) => setEnabled(e.target.checked)}
                />
                Publier la page publique
              </label>

              <label className="flex items-center gap-2 text-sm font-semibold text-slate-800">
                <input
                  type="checkbox"
                  checked={requireEligible}
                  onChange={(e) => setRequireEligible(e.target.checked)}
                />
                Restreindre aux élèves de la liste (protection)
              </label>

              {enabled && publicUrl ? (
                <div className="flex flex-wrap items-center gap-2 text-xs">
                  <code className="rounded-lg bg-slate-100 px-2 py-1 text-slate-700">{publicUrl}</code>
                  <button
                    type="button"
                    className="rounded-lg border border-slate-200 px-2 py-1 font-bold text-slate-700 hover:bg-slate-50"
                    onClick={() => void navigator.clipboard.writeText(publicUrl)}
                  >
                    Copier le lien
                  </button>
                  <Link
                    href={`/invitation/${slug || data.page.slug}`}
                    target="_blank"
                    className="rounded-lg border border-slate-200 px-2 py-1 font-bold text-slate-700 hover:bg-slate-50"
                  >
                    Ouvrir
                  </Link>
                </div>
              ) : null}

              <div className="grid gap-3 sm:grid-cols-2">
                <Field label="Titre">
                  <input
                    value={title}
                    onChange={(e) => setTitle(e.target.value)}
                    className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                  />
                </Field>
                <Field label="Slug URL">
                  <input
                    value={slug}
                    onChange={(e) => setSlug(e.target.value)}
                    className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                  />
                </Field>
                <Field label="Thème">
                  <select
                    value={theme}
                    onChange={(e) => setTheme(e.target.value as InvitationTheme)}
                    className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                  >
                    <option value="remise_diplome">Remise de diplôme (festif)</option>
                    <option value="neutre">Neutre</option>
                  </select>
                </Field>
                <Field label="Diplôme dans le formulaire">
                  <select
                    value={diplomaMode}
                    onChange={(e) => setDiplomaMode(e.target.value as InvitationDiplomaMode)}
                    className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                  >
                    <option value="none">Masqué</option>
                    <option value="bac">Bac uniquement</option>
                    <option value="brevet">Brevet uniquement</option>
                    <option value="both">Choix bac / brevet</option>
                  </select>
                </Field>
                <Field label="Début">
                  <input
                    type="datetime-local"
                    value={startsLocal}
                    onChange={(e) => setStartsLocal(e.target.value)}
                    className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                  />
                </Field>
                <Field label="Fin">
                  <input
                    type="datetime-local"
                    value={endsLocal}
                    onChange={(e) => setEndsLocal(e.target.value)}
                    className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                  />
                </Field>
                <Field label="Lieu">
                  <input
                    value={location}
                    onChange={(e) => setLocation(e.target.value)}
                    className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                  />
                </Field>
                <Field label="E-mail notification (optionnel)">
                  <input
                    type="email"
                    value={notifyEmail}
                    onChange={(e) => setNotifyEmail(e.target.value)}
                    className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                  />
                </Field>
                <Field label="Plafond global (personnes)">
                  <input
                    type="number"
                    min={1}
                    max={50000}
                    value={maxTotal}
                    onChange={(e) => setMaxTotal(Number(e.target.value) || 1)}
                    className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                  />
                </Field>
                <Field label="Max personnes par élève">
                  <input
                    type="number"
                    min={1}
                    max={50}
                    value={maxPerEleve}
                    onChange={(e) => setMaxPerEleve(Number(e.target.value) || 1)}
                    className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                  />
                </Field>
                <Field label="Question situation actuelle">
                  <select
                    value={askSituation}
                    onChange={(e) => setAskSituation(e.target.value as InvitationAskSituation)}
                    className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                  >
                    <option value="off">Désactivée</option>
                    <option value="bac_only">Uniquement si bac</option>
                    <option value="always">Toujours (bac et brevet)</option>
                  </select>
                </Field>
                <Field label="Date limite de réponse / modification">
                  <input
                    type="datetime-local"
                    value={rsvpClosesLocal}
                    onChange={(e) => setRsvpClosesLocal(e.target.value)}
                    className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                  />
                </Field>
              </div>
              <Field label="Texte d’introduction">
                <textarea
                  value={intro}
                  onChange={(e) => setIntro(e.target.value)}
                  rows={3}
                  className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                />
              </Field>
            </section>

            <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-4">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <div>
                  <h2 className="text-sm font-black text-slate-900">Liste des élèves autorisés</h2>
                  <p className="mt-1 text-xs text-slate-500">
                    Importez un Excel (.xlsx) avec colonnes <strong>Prénom</strong>,{" "}
                    <strong>Nom</strong>, <strong>Date de naissance</strong> (et Diplôme si besoin).
                    La date sert de filet (match 2/3 avec prénom et nom).
                  </p>
                </div>
                <p className="text-xs font-bold text-slate-600">
                  {data.eligible.length} élève(s)
                </p>
              </div>

              <div className="rounded-xl border border-dashed border-slate-300 bg-slate-50/80 p-4 space-y-3">
                <input
                  ref={eligibleFileRef}
                  type="file"
                  accept=".xlsx,.xls,.csv,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet,application/vnd.ms-excel,text/csv"
                  className="block w-full text-xs text-slate-700 file:mr-3 file:rounded-lg file:border-0 file:bg-slate-900 file:px-3 file:py-2 file:text-xs file:font-bold file:text-white hover:file:bg-slate-800"
                  onChange={(e) => {
                    const f = e.target.files?.[0] || null;
                    setEligibleFileName(f?.name || null);
                  }}
                />
                {eligibleFileName ? (
                  <p className="text-xs font-semibold text-slate-700">Fichier : {eligibleFileName}</p>
                ) : (
                  <p className="text-xs text-slate-500">Aucun fichier sélectionné</p>
                )}
                <div className="flex flex-wrap gap-2">
                  <ModuleButton
                    type="button"
                    onClick={() => void importEligibleExcel("append")}
                    disabled={eligibleBusy}
                  >
                    {eligibleBusy ? "Import…" : "Importer l’Excel (ajouter)"}
                  </ModuleButton>
                  <button
                    type="button"
                    onClick={() => void importEligibleExcel("replace")}
                    disabled={eligibleBusy}
                    className="rounded-xl border border-slate-200 bg-white px-3 py-2 text-xs font-bold text-slate-700 hover:bg-slate-50 disabled:opacity-50"
                  >
                    Remplacer la liste avec l’Excel
                  </button>
                </div>
              </div>

              <details className="rounded-xl border border-slate-100 bg-white px-3 py-2">
                <summary className="cursor-pointer text-xs font-bold text-slate-600">
                  Ou coller une liste texte (secours)
                </summary>
                <div className="mt-3 space-y-3">
                  <textarea
                    value={eligiblePaste}
                    onChange={(e) => setEligiblePaste(e.target.value)}
                    rows={4}
                    placeholder={"Marie;Dupont;12/03/2007;bac\nJean;Martin;01/09/2007"}
                    className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-mono"
                  />
                  <div className="flex flex-wrap gap-2">
                    <button
                      type="button"
                      onClick={() => void importEligible("append")}
                      disabled={eligibleBusy}
                      className="rounded-xl border border-slate-200 px-3 py-2 text-xs font-bold text-slate-700 hover:bg-slate-50 disabled:opacity-50"
                    >
                      Ajouter le texte
                    </button>
                    <button
                      type="button"
                      onClick={() => void importEligible("replace")}
                      disabled={eligibleBusy}
                      className="rounded-xl border border-slate-200 px-3 py-2 text-xs font-bold text-slate-700 hover:bg-slate-50 disabled:opacity-50"
                    >
                      Remplacer avec le texte
                    </button>
                  </div>
                </div>
              </details>
              {data.eligible.length > 0 ? (
                <div className="max-h-56 overflow-auto rounded-xl border border-slate-100">
                  <table className="min-w-full text-left text-sm">
                    <thead className="sticky top-0 bg-slate-50 text-[11px] uppercase tracking-wide text-slate-500">
                      <tr>
                        <th className="px-2 py-2">Élève</th>
                        <th className="px-2 py-2">Né(e) le</th>
                        <th className="px-2 py-2">Diplôme</th>
                        <th className="px-2 py-2" />
                      </tr>
                    </thead>
                    <tbody>
                      {data.eligible.map((e) => (
                        <tr key={e.id} className="border-t border-slate-100">
                          <td className="px-2 py-1.5 font-semibold text-slate-900">
                            {formatInvitationEleveLabel(e.eleveFirstName, e.eleveLastName)}
                          </td>
                          <td className="px-2 py-1.5 text-slate-600">
                            {e.birthDate
                              ? new Date(e.birthDate + "T12:00:00").toLocaleDateString("fr-FR")
                              : "—"}
                          </td>
                          <td className="px-2 py-1.5 text-slate-600">
                            {diplomaLabel(e.diploma) || "—"}
                          </td>
                          <td className="px-2 py-1.5 text-right">
                            <button
                              type="button"
                              onClick={() =>
                                void removeEligible(
                                  e.id,
                                  `${formatInvitationEleveLabel(e.eleveFirstName, e.eleveLastName)}`,
                                )
                              }
                              className="text-[11px] font-bold text-rose-700 hover:underline"
                            >
                              Retirer
                            </button>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              ) : (
                <p className="text-sm text-slate-500">
                  Aucun élève pour l’instant. Sans liste + option protection, le formulaire reste ouvert.
                </p>
              )}
            </section>

            {data.duplicates.length > 0 ? (
              <section className="rounded-2xl border border-amber-300 bg-amber-50/80 p-5 space-y-3">
                <h2 className="text-sm font-black text-amber-950">Doublons suspects</h2>
                <p className="text-xs text-amber-900/80">
                  Même nom d’élève saisi plusieurs fois (ex. papa et maman). Reliez-les ou ignorez le
                  signal.
                </p>
                <ul className="space-y-2">
                  {data.duplicates.map((d) => (
                    <li
                      key={d.eleveNameNorm}
                      className="flex flex-wrap items-center justify-between gap-2 rounded-xl border border-amber-200 bg-white/80 px-3 py-2"
                    >
                      <div>
                        <p className="text-sm font-bold text-slate-900">{d.eleveLabel}</p>
                        <p className="text-[11px] text-slate-500">
                          {d.rsvpIds.length} réponses · ids {d.rsvpIds.join(", ").slice(0, 80)}
                        </p>
                      </div>
                      <div className="flex gap-2">
                        <button
                          type="button"
                          onClick={() => void dupAction("link", d.rsvpIds)}
                          className="rounded-lg bg-slate-900 px-2.5 py-1.5 text-[11px] font-bold text-white"
                        >
                          Relier
                        </button>
                        <button
                          type="button"
                          onClick={() => void dupAction("dismiss", d.rsvpIds)}
                          className="rounded-lg border border-slate-200 px-2.5 py-1.5 text-[11px] font-bold text-slate-700"
                        >
                          Ignorer
                        </button>
                      </div>
                    </li>
                  ))}
                </ul>
              </section>
            ) : null}

            <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-4">
              <div className="flex flex-wrap items-end justify-between gap-3">
                <h2 className="text-sm font-black text-slate-900">Réponses</h2>
                <div className="flex flex-wrap gap-2">
                  <a
                    href={`/api/invitation/pages/${pageId}/export`}
                    className="rounded-xl border border-slate-200 bg-white px-3 py-1.5 text-xs font-bold text-slate-800 hover:bg-slate-50"
                    download
                  >
                    Export CSV
                  </a>
                  <select
                    value={filter}
                    onChange={(e) => setFilter(e.target.value as "all" | "oui" | "non")}
                    className="rounded-xl border border-slate-200 px-2 py-1.5 text-xs font-semibold"
                  >
                    <option value="all">Toutes</option>
                    <option value="oui">Oui</option>
                    <option value="non">Non</option>
                  </select>
                  <input
                    value={search}
                    onChange={(e) => setSearch(e.target.value)}
                    placeholder="Rechercher…"
                    className="rounded-xl border border-slate-200 px-3 py-1.5 text-xs min-w-[160px]"
                  />
                </div>
              </div>

              {filteredRsvps.length === 0 ? (
                <p className="text-sm text-slate-500">Aucune réponse.</p>
              ) : (
                <div className="overflow-x-auto">
                  <table className="min-w-full text-left text-sm">
                    <thead className="text-[11px] uppercase tracking-wide text-slate-500">
                      <tr>
                        <th className="px-2 py-2">Élève</th>
                        <th className="px-2 py-2">Réponse</th>
                        <th className="px-2 py-2">Pers.</th>
                        <th className="px-2 py-2">Diplôme</th>
                        <th className="px-2 py-2">Situation</th>
                        <th className="px-2 py-2">E-mail</th>
                        <th className="px-2 py-2">Date</th>
                        <th className="px-2 py-2" />
                      </tr>
                    </thead>
                    <tbody>
                      {filteredRsvps.map((r) => {
                        const suspect = duplicateIdSet.has(r.id);
                        return (
                          <tr
                            key={r.id}
                            className={`border-t border-slate-100 ${
                              suspect ? "bg-amber-50/90 ring-1 ring-inset ring-amber-200" : ""
                            }`}
                          >
                            <td className="px-2 py-2 font-semibold text-slate-900">
                              {formatInvitationEleveLabel(r.eleveFirstName, r.eleveLastName)}
                              {suspect ? (
                                <span className="ml-1 text-[10px] font-bold text-amber-700">
                                  doublon ?
                                </span>
                              ) : null}
                              {r.duplicateGroupId ? (
                                <span className="ml-1 text-[10px] font-bold text-sky-700">
                                  relié
                                </span>
                              ) : null}
                            </td>
                            <td className="px-2 py-2">
                              <span
                                className={`rounded-full px-2 py-0.5 text-[11px] font-bold ${
                                  r.response === "oui"
                                    ? "bg-emerald-100 text-emerald-800"
                                    : "bg-slate-100 text-slate-600"
                                }`}
                              >
                                {r.response === "oui" ? "Oui" : "Non"}
                              </span>
                            </td>
                            <td className="px-2 py-2">{r.response === "oui" ? r.presentCount : "—"}</td>
                            <td className="px-2 py-2">{diplomaLabel(r.diploma) || "—"}</td>
                            <td className="px-2 py-2 text-xs text-slate-600">
                              {situationStatusLabel(r.situationStatus) || "—"}
                              {r.situationEstablishment
                                ? ` · ${r.situationEstablishment}`
                                : ""}
                            </td>
                            <td className="px-2 py-2 text-slate-600">{r.parentEmail}</td>
                            <td className="px-2 py-2 text-xs text-slate-500">
                              {new Date(r.createdAt).toLocaleString("fr-FR", {
                                timeZone: "Europe/Paris",
                              })}
                            </td>
                            <td className="px-2 py-2 text-right">
                              <button
                                type="button"
                                onClick={() =>
                                  void deleteRsvp(
                                    r.id,
                                    `${formatInvitationEleveLabel(r.eleveFirstName, r.eleveLastName)}`,
                                  )
                                }
                                className="text-[11px] font-bold text-rose-700 hover:underline"
                              >
                                Supprimer
                              </button>
                            </td>
                          </tr>
                        );
                      })}
                    </tbody>
                  </table>
                </div>
              )}
            </section>
          </>
        ) : null}
      </ModulePageShell>
    </RequireModuleAccess>
  );
}

function Kpi({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-2xl border border-slate-200 bg-white px-4 py-3">
      <p className="text-[10px] font-bold uppercase tracking-wide text-slate-500">{label}</p>
      <p className="text-2xl font-black text-slate-900">{value}</p>
    </div>
  );
}

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="flex flex-col gap-1 text-xs font-semibold text-slate-600">
      {label}
      {children}
    </label>
  );
}
