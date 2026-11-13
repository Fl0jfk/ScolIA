"use client";

import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import Link from "@/app/components/ui/AppLink";
import type {
  RhRegistreAlert,
  RhRegistrePayload,
  RhRegistreRow,
  RhSkillBucket,
} from "@/app/lib/rh/rh-registre-types";

type ViewMode = "registre" | "alertes" | "competences";

const URGENCY_STYLES = {
  high: "border-rose-200 bg-rose-50 text-rose-900",
  medium: "border-amber-200 bg-amber-50 text-amber-900",
  low: "border-slate-200 bg-slate-50 text-slate-700",
};

const KIND_LABELS: Record<RhRegistreAlert["kind"], string> = {
  habilitation: "Habilitation / SST",
  formation: "Formation",
  medecine: "Médecine du travail",
  conformite: "Conformité",
  entretien: "Entretien",
};

function CellMissing({ missing, children }: { missing: boolean; children: ReactNode }) {
  if (!missing) {
    return <span className="text-slate-800">{children}</span>;
  }
  return (
    <span className="inline-flex rounded-md bg-rose-100 px-1.5 py-0.5 text-xs font-bold text-rose-800 ring-1 ring-rose-200">
      {children || "Manquant"}
    </span>
  );
}

function maskNir(nir: string | null): string {
  if (!nir?.trim()) return "—";
  const clean = nir.replace(/\s/g, "");
  if (clean.length < 5) return "••••";
  return `${clean.slice(0, 1)} ${clean.slice(1, 3)} •• •• ••• ••• ${clean.slice(-2)}`;
}

export default function RhRegistrePanel() {
  const [data, setData] = useState<RhRegistrePayload | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [view, setView] = useState<ViewMode>("registre");
  const [filterMissing, setFilterMissing] = useState(false);
  const [query, setQuery] = useState("");
  const [alertKind, setAlertKind] = useState<"all" | RhRegistreAlert["kind"]>("all");

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/rh/registre", { cache: "no-store" });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Registre indisponible");
      setData(json as RhRegistrePayload);
    } catch (e) {
      setData(null);
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const rows = useMemo(() => {
    if (!data) return [] as RhRegistreRow[];
    const q = query.trim().toLowerCase();
    return data.rows.filter((r) => {
      if (filterMissing && !r.hasMissingData) return false;
      if (!q) return true;
      return (
        r.displayName.toLowerCase().includes(q) ||
        r.email.toLowerCase().includes(q) ||
        r.folderName.toLowerCase().includes(q) ||
        (r.jobTitle || "").toLowerCase().includes(q)
      );
    });
  }, [data, filterMissing, query]);

  const alerts = useMemo(() => {
    if (!data) return [] as RhRegistreAlert[];
    if (alertKind === "all") return data.alerts;
    return data.alerts.filter((a) => a.kind === alertKind);
  }, [data, alertKind]);

  if (loading && !data) {
    return <p className="p-8 text-center text-slate-500">Chargement du registre OneDrive…</p>;
  }

  if (error && !data) {
    return (
      <div className="rounded-2xl border border-rose-200 bg-rose-50 p-6 text-sm text-rose-900">
        <p className="font-bold mb-2">Registre indisponible</p>
        <p className="mb-4">{error}</p>
        <p className="text-xs text-rose-800/80 mb-4">
          Vérifiez que le OneDrive RH est connecté (onglet Entrées / sorties).
        </p>
        <button
          type="button"
          onClick={() => void load()}
          className="px-4 py-2 rounded-xl bg-rose-700 text-white text-xs font-bold"
        >
          Réessayer
        </button>
      </div>
    );
  }

  if (!data) return null;

  return (
    <div className="space-y-5">
      <header className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h2 className="text-xl font-black text-slate-900">Registre unique du personnel</h2>
          <p className="text-sm text-slate-500 mt-1">
            Agrégation temps réel des <code className="text-xs">meta-rh.json</code> sur OneDrive (
            {data.basePath}) · médecine : cycle {data.medecineIntervalYears} ans
          </p>
        </div>
        <button
          type="button"
          onClick={() => void load()}
          disabled={loading}
          className="px-4 py-2 rounded-xl border border-slate-200 bg-white text-xs font-bold text-slate-700 hover:border-indigo-300 disabled:opacity-50"
        >
          {loading ? "Actualisation…" : "Actualiser"}
        </button>
      </header>

      <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
        <Stat label="Collaborateurs" value={data.counts.staff} />
        <Stat label="Données manquantes" value={data.counts.withMissingData} tone="rose" />
        <Stat label="Alertes urgentes" value={data.counts.alertsHigh} tone="amber" />
        <Stat label="meta-rh absents" value={data.counts.metaMissing} tone="rose" />
      </div>

      <div className="flex flex-wrap gap-2">
        {(
          [
            ["registre", "Registre"],
            ["alertes", `Alertes (${data.counts.alertsTotal})`],
            ["competences", `Compétences (${data.skills.length})`],
          ] as const
        ).map(([id, label]) => (
          <button
            key={id}
            type="button"
            onClick={() => setView(id)}
            className={`px-4 py-2 rounded-xl text-sm font-bold transition-colors ${
              view === id
                ? "bg-indigo-600 text-white shadow-md"
                : "bg-white border border-slate-200 text-slate-600 hover:border-indigo-300"
            }`}
          >
            {label}
          </button>
        ))}
      </div>

      {view === "registre" && (
        <section className="bg-white rounded-2xl border border-slate-200 shadow-sm overflow-hidden">
          <div className="flex flex-wrap gap-3 items-center p-4 border-b border-slate-100">
            <input
              type="search"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Rechercher nom, e-mail, poste…"
              className="flex-1 min-w-[180px] rounded-xl border border-slate-200 px-3 py-2 text-sm"
            />
            <label className="flex items-center gap-2 text-xs font-bold text-slate-600 cursor-pointer select-none">
              <input
                type="checkbox"
                checked={filterMissing}
                onChange={(e) => setFilterMissing(e.target.checked)}
                className="rounded border-slate-300"
              />
              Uniquement données manquantes
            </label>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full text-sm text-left min-w-[900px]">
              <thead className="bg-slate-50 text-xs uppercase tracking-wide text-slate-500">
                <tr>
                  <th className="px-3 py-3 font-bold">Collaborateur</th>
                  <th className="px-3 py-3 font-bold">Catégorie</th>
                  <th className="px-3 py-3 font-bold">Poste</th>
                  <th className="px-3 py-3 font-bold">N° sécu</th>
                  <th className="px-3 py-3 font-bold">Contrat</th>
                  <th className="px-3 py-3 font-bold">Naissance</th>
                  <th className="px-3 py-3 font-bold">Médecine</th>
                  <th className="px-3 py-3 font-bold">Statut</th>
                </tr>
              </thead>
              <tbody>
                {rows.length === 0 ? (
                  <tr>
                    <td colSpan={8} className="px-4 py-10 text-center text-slate-400 italic">
                      Aucune ligne à afficher.
                    </td>
                  </tr>
                ) : (
                  rows.map((r) => (
                    <tr
                      key={r.id}
                      className={`border-t border-slate-100 ${
                        r.hasMissingData || !r.metaFound ? "bg-rose-50/40" : "hover:bg-slate-50/80"
                      }`}
                    >
                      <td className="px-3 py-3">
                        <Link
                          href={`/rh/${r.id}`}
                          className="font-bold text-indigo-700 hover:underline"
                        >
                          {r.displayName}
                        </Link>
                        <p className="text-xs text-slate-500 mt-0.5">{r.email || "—"}</p>
                        {!r.metaFound && (
                          <p className="text-[11px] font-bold text-rose-700 mt-1">
                            meta-rh.json manquant
                          </p>
                        )}
                      </td>
                      <td className="px-3 py-3 text-slate-700">{r.categoryLabel}</td>
                      <td className="px-3 py-3 text-slate-700">{r.jobTitle || "—"}</td>
                      <td className="px-3 py-3">
                        <CellMissing missing={r.missingSocialSecurity}>
                          {r.missingSocialSecurity ? "Manquant" : maskNir(r.socialSecurityNumber)}
                        </CellMissing>
                      </td>
                      <td className="px-3 py-3">
                        <CellMissing missing={r.missingContractType}>
                          {r.missingContractType
                            ? "Manquant"
                            : (r.contractType || "").toUpperCase()}
                        </CellMissing>
                      </td>
                      <td className="px-3 py-3">
                        <CellMissing missing={r.missingBirthDate}>
                          {r.missingBirthDate
                            ? "Manquant"
                            : r.birthDate
                              ? new Date(r.birthDate).toLocaleDateString("fr-FR")
                              : "—"}
                        </CellMissing>
                      </td>
                      <td className="px-3 py-3 text-slate-700">
                        {r.medecineNextVisitAt
                          ? new Date(r.medecineNextVisitAt).toLocaleDateString("fr-FR")
                          : "—"}
                      </td>
                      <td className="px-3 py-3">
                        <span className="text-xs font-bold capitalize text-slate-600">
                          {r.accountStatus}
                          {!r.active ? " · inactif" : ""}
                        </span>
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        </section>
      )}

      {view === "alertes" && (
        <section className="space-y-4">
          <div className="flex flex-wrap gap-2">
            <FilterChip
              active={alertKind === "all"}
              onClick={() => setAlertKind("all")}
              label={`Toutes (${data.alerts.length})`}
            />
            {(Object.keys(KIND_LABELS) as RhRegistreAlert["kind"][]).map((k) => {
              const n = data.alerts.filter((a) => a.kind === k).length;
              return (
                <FilterChip
                  key={k}
                  active={alertKind === k}
                  onClick={() => setAlertKind(k)}
                  label={`${KIND_LABELS[k]} (${n})`}
                />
              );
            })}
          </div>
          {alerts.length === 0 ? (
            <p className="rounded-2xl border border-slate-200 bg-white p-8 text-center text-sm text-slate-400 italic">
              Aucune alerte sur ce filtre.
            </p>
          ) : (
            <ul className="space-y-2">
              {alerts.map((a) => (
                <li key={a.id}>
                  <Link
                    href={`/rh/${a.personnelId}`}
                    className={`block rounded-xl border p-4 transition hover:shadow-md ${URGENCY_STYLES[a.urgency]}`}
                  >
                    <div className="flex flex-wrap items-start justify-between gap-2">
                      <div>
                        <p className="font-black text-sm">{a.displayName}</p>
                        <p className="text-xs font-bold mt-1 opacity-80">{KIND_LABELS[a.kind]}</p>
                        <p className="text-sm font-bold mt-2">{a.title}</p>
                        <p className="text-xs mt-1 opacity-90">{a.detail}</p>
                      </div>
                      {a.dueDate && (
                        <span className="text-[11px] font-bold uppercase tracking-wide opacity-70">
                          {new Date(a.dueDate).toLocaleDateString("fr-FR")}
                        </span>
                      )}
                    </div>
                  </Link>
                </li>
              ))}
            </ul>
          )}
        </section>
      )}

      {view === "competences" && <SkillsView skills={data.skills} />}
    </div>
  );
}

function Stat({
  label,
  value,
  tone,
}: {
  label: string;
  value: number;
  tone?: "rose" | "amber";
}) {
  const toneClass =
    tone === "rose"
      ? "border-rose-200 bg-rose-50 text-rose-900"
      : tone === "amber"
        ? "border-amber-200 bg-amber-50 text-amber-900"
        : "border-slate-200 bg-white text-slate-900";
  return (
    <div className={`rounded-2xl border p-4 ${toneClass}`}>
      <p className="text-[11px] font-bold uppercase tracking-wide opacity-70">{label}</p>
      <p className="text-2xl font-black mt-1">{value}</p>
    </div>
  );
}

function FilterChip({
  active,
  onClick,
  label,
}: {
  active: boolean;
  onClick: () => void;
  label: string;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`px-3 py-1.5 rounded-lg text-xs font-bold transition-colors ${
        active
          ? "bg-slate-800 text-white"
          : "bg-white border border-slate-200 text-slate-600 hover:border-slate-400"
      }`}
    >
      {label}
    </button>
  );
}

function SkillsView({ skills }: { skills: RhSkillBucket[] }) {
  const [open, setOpen] = useState<string | null>(null);
  if (skills.length === 0) {
    return (
      <p className="rounded-2xl border border-slate-200 bg-white p-8 text-center text-sm text-slate-400 italic">
        Aucune habilitation ni formation réalisée renseignée dans les meta-rh.
      </p>
    );
  }
  return (
    <div className="grid sm:grid-cols-2 gap-3">
      {skills.map((s) => {
        const key = `${s.kind}:${s.label}`;
        const isOpen = open === key;
        return (
          <div key={key} className="rounded-2xl border border-slate-200 bg-white p-4 shadow-sm">
            <button
              type="button"
              className="w-full text-left"
              onClick={() => setOpen(isOpen ? null : key)}
            >
              <div className="flex items-start justify-between gap-2">
                <div>
                  <p className="text-[10px] font-bold uppercase tracking-wide text-indigo-600">
                    {s.kind === "habilitation" ? "Habilitation" : "Formation réalisée"}
                  </p>
                  <p className="font-black text-slate-900 mt-1">{s.label}</p>
                </div>
                <span className="shrink-0 rounded-full bg-indigo-50 px-2.5 py-1 text-xs font-black text-indigo-800">
                  {s.count}
                </span>
              </div>
            </button>
            {isOpen && (
              <ul className="mt-3 space-y-1 border-t border-slate-100 pt-3">
                {s.people.map((p) => (
                  <li key={p.id}>
                    <Link
                      href={`/rh/${p.id}`}
                      className="text-sm font-bold text-indigo-700 hover:underline"
                    >
                      {p.displayName}
                    </Link>
                  </li>
                ))}
              </ul>
            )}
          </div>
        );
      })}
    </div>
  );
}
