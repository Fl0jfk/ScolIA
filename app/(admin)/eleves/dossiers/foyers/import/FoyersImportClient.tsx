"use client";

import Link from "next/link";
import { useMemo, useState } from "react";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import type { Plan } from "@/app/lib/foyers-import/plan";

type Orphan = {
  foyerId: string;
  label: string;
  facturation: boolean;
  filsMessagerie: number;
  comptes: number;
  autresEleves: number;
};

type Preview = { plan: Plan; fingerprint: string; orphans: Orphan[] };

type Tab = "foyers" | "eleves" | "anomalies" | "orphelins" | "verifier";

const ACTION_LABEL: Record<string, string> = {
  create: "Créé",
  update: "Mis à jour",
  unchanged: "Inchangé",
};

const SHOW_STEP = 100;

export default function FoyersImportClient() {
  const [file, setFile] = useState<File | null>(null);
  const [keepOldLinks, setKeepOldLinks] = useState(false);
  const [linkAccounts, setLinkAccounts] = useState(false);
  const [removeProtected, setRemoveProtected] = useState(false);
  const [writeSuspect, setWriteSuspect] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [preview, setPreview] = useState<Preview | null>(null);
  const [confirm, setConfirm] = useState(false);
  const [done, setDone] = useState<{ applied: Record<string, number>; comptesRattaches: number; csv: string } | null>(
    null,
  );
  const [tab, setTab] = useState<Tab>("foyers");
  const [search, setSearch] = useState("");
  const [actionFilter, setActionFilter] = useState<"all" | "create" | "update" | "unchanged">("all");
  const [shown, setShown] = useState(SHOW_STEP);

  async function send(action: "preview" | "apply") {
    if (!file) return;
    setBusy(true);
    setError(null);
    try {
      const fd = new FormData();
      fd.set("file", file);
      fd.set("action", action);
      fd.set("keepOldLinks", keepOldLinks ? "1" : "0");
      fd.set("removeProtectedLinks", removeProtected && !keepOldLinks ? "1" : "0");
      fd.set("writeSuspectEmails", writeSuspect ? "1" : "0");
      if (action === "apply" && preview) {
        fd.set("confirm", "oui");
        fd.set("fingerprint", preview.fingerprint);
        fd.set("linkAccounts", linkAccounts ? "1" : "0");
      }
      const res = await fetch("/api/eleves/foyers/import", { method: "POST", body: fd });
      const j = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(j.error || "Échec.");
      if (action === "preview") {
        setPreview(j as Preview);
        setDone(null);
        setConfirm(false);
        setShown(SHOW_STEP);
      } else {
        setDone({ applied: j.applied, comptesRattaches: j.comptesRattaches, csv: j.csv });
        setPreview(null);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur.");
    } finally {
      setBusy(false);
    }
  }

  function downloadCsv(csv: string) {
    const url = URL.createObjectURL(new Blob([csv], { type: "text/csv;charset=utf-8" }));
    const a = document.createElement("a");
    a.href = url;
    a.download = `import-foyers-${new Date().toISOString().slice(0, 10)}.csv`;
    a.click();
    URL.revokeObjectURL(url);
  }

  const plan = preview?.plan;
  const eleveName = useMemo(() => {
    const m = new Map<string, string>();
    for (const s of plan?.students ?? []) m.set(s.eleveId, `${s.prenom} ${s.nom}`);
    return m;
  }, [plan]);
  const q = search.trim().toLowerCase();
  const foyersFiltered = useMemo(() => {
    if (!plan) return [];
    return plan.foyers.filter((f) => {
      if (actionFilter !== "all" && f.action !== actionFilter) return false;
      if (!q) return true;
      const hay = [
        f.label,
        ...f.responsables.flatMap((r) => [r.nom, r.prenom, r.email ?? "", r.telephone ?? ""]),
        ...f.eleveIds.map((id) => eleveName.get(id) ?? ""),
      ]
        .join(" ")
        .toLowerCase();
      return hay.includes(q);
    });
  }, [plan, actionFilter, q, eleveName]);
  const elevesFiltered = useMemo(
    () => (plan?.students ?? []).filter((s) => !q || `${s.prenom} ${s.nom}`.toLowerCase().includes(q)),
    [plan, q],
  );
  const anomaliesFiltered = useMemo(
    () => (plan?.anomalies ?? []).filter((a) => !q || `${a.eleve} ${a.message} ${a.kind}`.toLowerCase().includes(q)),
    [plan, q],
  );

  const c = plan?.counts;

  return (
    <ModulePageShell maxWidthClass="max-w-6xl">
      <ModulePageHeader
        title="Importer les foyers (Excel)"
        description="Met à jour les responsables légaux et les liens élève ↔ foyer. La fiche élève n'est jamais modifiée."
        actions={
          <Link href="/eleves/dossiers/foyers" className="text-sm font-bold text-indigo-600 hover:underline">
            Voir les foyers
          </Link>
        }
      />

      <section className="mb-4 space-y-3 rounded-2xl border border-slate-200 bg-white p-4">
        <input
          type="file"
          accept=".xlsx,.xls"
          onChange={(e) => {
            setFile(e.target.files?.[0] ?? null);
            setPreview(null);
            setDone(null);
          }}
          className="block text-sm"
        />
        <label className="flex items-center gap-2 text-sm">
          <input type="checkbox" checked={keepOldLinks} onChange={(e) => setKeepOldLinks(e.target.checked)} />
          Ne pas retirer les anciens liens (sinon, les liens vers des foyers absents du fichier sont retirés)
        </label>
        <label className="flex items-start gap-2 text-sm">
          <input
            type="checkbox"
            checked={removeProtected}
            disabled={keepOldLinks}
            onChange={(e) => {
              setRemoveProtected(e.target.checked);
              setPreview(null);
            }}
          />
          <span>
            Retirer aussi les liens des foyers avec facturation / messagerie / compte famille
            <span className="block text-xs font-semibold text-rose-600">
              Attention : par défaut ces liens sont conservés et listés « à vérifier ». Cocher cette case peut
              couper l'accès d'une famille à la facturation ou à la messagerie d'un enfant.
            </span>
          </span>
        </label>
        <label className="flex items-start gap-2 text-sm">
          <input
            type="checkbox"
            checked={writeSuspect}
            onChange={(e) => {
              setWriteSuspect(e.target.checked);
              setPreview(null);
            }}
          />
          <span>
            Écrire quand même les e-mails génériques, du personnel ou déjà présents sur un autre foyer
            <span className="block text-xs text-slate-500">
              Par défaut ces adresses ne sont pas écrites (une adresse ajoutée à un foyer donne accès à ses enfants).
            </span>
          </span>
        </label>
        <label className="flex items-center gap-2 text-sm">
          <input type="checkbox" checked={linkAccounts} onChange={(e) => setLinkAccounts(e.target.checked)} />
          Rattacher les responsables aux comptes existants (par e-mail) à l'application
        </label>
        <button
          type="button"
          disabled={!file || busy}
          onClick={() => void send("preview")}
          className="rounded-xl bg-indigo-600 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
        >
          {busy && !preview ? "Analyse…" : "Analyser (aperçu, rien n'est écrit)"}
        </button>
        <p className="text-xs text-slate-500">
          Le fichier est lu en mémoire sur le serveur et n'est pas conservé. 5 Mo maximum.
        </p>
        {error ? <p className="text-sm font-semibold text-rose-600">{error}</p> : null}
      </section>

      {done ? (
        <section className="mb-4 space-y-2 rounded-2xl border border-emerald-200 bg-emerald-50 p-4 text-sm">
          <h3 className="font-bold text-emerald-900">Import appliqué</h3>
          <p>
            Foyers créés : {done.applied.foyersCrees} · mis à jour : {done.applied.foyersMisAJour} · responsables
            ajoutés : {done.applied.responsablesAjoutes} · modifiés : {done.applied.responsablesModifies} · liens
            ajoutés : {done.applied.liensAjoutes} · retirés : {done.applied.liensRetires} · comptes rattachés :{" "}
            {done.comptesRattaches}
          </p>
          <button
            type="button"
            onClick={() => downloadCsv(done.csv)}
            className="rounded-xl bg-emerald-700 px-3 py-1.5 text-xs font-bold text-white"
          >
            Télécharger le rapport (CSV)
          </button>
        </section>
      ) : null}

      {plan && c ? (
        <>
          <section className="mb-4 grid grid-cols-2 gap-2 sm:grid-cols-4">
            {[
              ["Élèves reconnus", c.elevesReconnus],
              ["Non trouvés", c.elevesNonTrouves],
              ["Ambigus", c.elevesAmbigus],
              ["Foyers créés", c.foyersCrees],
              ["Foyers mis à jour", c.foyersMisAJour],
              ["Foyers inchangés", c.foyersInchanges],
              ["Responsables ajoutés", c.responsablesAjoutes],
              ["Responsables modifiés", c.responsablesModifies],
              ["Liens ajoutés", c.liensAjoutes],
              ["Liens retirés", c.liensRetires],
              ["Liens à vérifier", c.liensAVerifier],
              ["Foyers orphelins", c.foyersOrphelins],
              ["Anomalies", plan.anomalies.length],
            ].map(([label, n]) => (
              <div key={String(label)} className="rounded-xl border border-slate-200 bg-white px-3 py-2">
                <div className="text-xl font-bold text-slate-900">{n}</div>
                <div className="text-xs text-slate-500">{label}</div>
              </div>
            ))}
          </section>

          <div className="mb-3 flex flex-wrap items-center gap-2">
            {(
              [
                ["foyers", "Foyers"],
                ["eleves", "Élèves"],
                ["anomalies", "Anomalies"],
                ["orphelins", "Orphelins"],
                ["verifier", "À vérifier"],
              ] as Array<[Tab, string]>
            ).map(([id, label]) => (
              <button
                key={id}
                type="button"
                onClick={() => {
                  setTab(id);
                  setShown(SHOW_STEP);
                }}
                className={`rounded-full px-3 py-1 text-xs font-bold ${
                  tab === id ? "bg-indigo-600 text-white" : "bg-slate-100 text-slate-700"
                }`}
              >
                {label}
              </button>
            ))}
            <input
              value={search}
              onChange={(e) => {
                setSearch(e.target.value);
                setShown(SHOW_STEP);
              }}
              placeholder="Filtrer…"
              className="ml-auto rounded-xl border border-slate-200 px-3 py-1.5 text-sm"
            />
            {tab === "foyers" ? (
              <select
                value={actionFilter}
                onChange={(e) => setActionFilter(e.target.value as typeof actionFilter)}
                className="rounded-xl border border-slate-200 px-2 py-1.5 text-sm"
              >
                <option value="all">Tous</option>
                <option value="create">Créés</option>
                <option value="update">Mis à jour</option>
                <option value="unchanged">Inchangés</option>
              </select>
            ) : null}
          </div>

          {tab === "foyers" ? (
            <ul className="space-y-2">
              {foyersFiltered.slice(0, shown).map((f) => (
                <li key={f.ref} className="rounded-2xl border border-slate-200 bg-white p-3 text-sm">
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="font-bold">{f.label}</span>
                    <span className="rounded-full bg-slate-100 px-2 py-0.5 text-[10px] font-bold uppercase">
                      {ACTION_LABEL[f.action]}
                    </span>
                    <span className="text-xs text-slate-500">
                      {f.eleveIds.map((id) => eleveName.get(id) ?? id).join(", ")}
                    </span>
                  </div>
                  <ul className="mt-2 space-y-1">
                    {f.responsables.map((r, i) => (
                      <li key={i} className="text-xs">
                        <span className="font-semibold">
                          {r.prenom} {r.nom}
                        </span>{" "}
                        · {r.email || "—"} · {r.telephone || "—"}{" "}
                        {r.action === "create" ? <em className="text-emerald-700">(ajouté)</em> : null}
                        {r.differences.length ? (
                          <em className="text-rose-700"> (compte famille : différence à vérifier — {r.differences.join(", ")})</em>
                        ) : null}
                        {r.action === "update" && r.before ? (
                          <em className="text-amber-700">
                            (modifié : {r.changes.join(", ")} — avant : {r.before.prenom} {r.before.nom},{" "}
                            {r.before.email || "—"}, {r.before.telephone || "—"})
                          </em>
                        ) : null}
                      </li>
                    ))}
                    {f.conserves.map((r) => (
                      <li key={r.id} className="text-xs text-slate-500">
                        {r.prenom} {r.nom} — conservé (absent du fichier
                        {r.payeur ? ", payeur" : ""}
                        {r.hasUser ? ", compte famille" : ""})
                      </li>
                    ))}
                  </ul>
                </li>
              ))}
            </ul>
          ) : null}

          {tab === "eleves" ? (
            <ul className="space-y-1 text-sm">
              {elevesFiltered.slice(0, shown).map((s) => (
                <li key={s.eleveId} className="rounded-xl bg-white px-3 py-2 border border-slate-200">
                  <span className="font-semibold">
                    {s.prenom} {s.nom}
                  </span>{" "}
                  <span className="text-xs text-slate-500">
                    {s.classe || "—"} (fichier : {s.codeClasse || "—"}) · {s.foyerRefs.length} foyer(s) · liens +
                    {s.liensAjoutes} / −{s.liensRetires}
                  </span>
                </li>
              ))}
            </ul>
          ) : null}

          {tab === "anomalies" ? (
            <ul className="space-y-1 text-sm">
              {anomaliesFiltered.slice(0, shown).map((a, i) => (
                <li key={i} className="rounded-xl border border-amber-200 bg-amber-50 px-3 py-2">
                  <span className="text-[10px] font-bold uppercase text-amber-800">{a.kind}</span>{" "}
                  <span className="font-semibold">{a.eleve}</span> — {a.message}{" "}
                  <span className="text-xs text-slate-500">(ligne {a.lines.join(", ")})</span>
                </li>
              ))}
            </ul>
          ) : null}

          {tab === "verifier" ? (
            <ul className="space-y-1 text-sm">
              {plan.liensAVerifier.length === 0 ? (
                <li className="text-slate-500">Aucun lien à vérifier.</li>
              ) : null}
              {plan.liensAVerifier.map((l, i) => (
                <li key={i} className="rounded-xl border border-amber-200 bg-amber-50 px-3 py-2">
                  <span className="font-semibold">{eleveName.get(l.eleveId) ?? l.eleveId}</span> — lien conservé vers «{" "}
                  {l.label} » ({l.raisons.join(", ")})
                </li>
              ))}
              {plan.foyers
                .flatMap((f) =>
                  f.responsables
                    .filter((r) => r.differences.length)
                    .map((r) => ({ f, r })),
                )
                .map(({ f, r }, i) => (
                  <li key={`d${i}`} className="rounded-xl border border-amber-200 bg-amber-50 px-3 py-2">
                    <span className="font-semibold">
                      {r.prenom} {r.nom}
                    </span>{" "}
                    ({f.label}) a un compte famille : différence fichier / base non appliquée ({r.differences.join(", ")})
                  </li>
                ))}
            </ul>
          ) : null}

          {tab === "orphelins" ? (
            <ul className="space-y-1 text-sm">
              {(preview?.orphans ?? []).length === 0 ? (
                <li className="text-slate-500">Aucun foyer orphelin.</li>
              ) : null}
              {(preview?.orphans ?? []).map((o) => (
                <li key={o.foyerId} className="rounded-xl border border-slate-200 bg-white px-3 py-2">
                  <span className="font-semibold">{o.label}</span>{" "}
                  <span className="text-xs text-slate-500">
                    {o.facturation ? "facturation · " : ""}
                    {o.filsMessagerie ? `${o.filsMessagerie} fil(s) messagerie · ` : ""}
                    {o.comptes ? `${o.comptes} compte(s) · ` : ""}
                    {o.autresEleves} autre(s) élève(s) lié(s) — conservé, jamais supprimé
                  </span>
                </li>
              ))}
            </ul>
          ) : null}

          <button
            type="button"
            onClick={() => setShown(shown + SHOW_STEP)}
            className="mt-3 text-xs font-bold text-indigo-600"
          >
            Afficher plus
          </button>

          <section className="mt-6 space-y-3 rounded-2xl border border-rose-200 bg-rose-50 p-4">
            <label className="flex items-start gap-2 text-sm">
              <input type="checkbox" checked={confirm} onChange={(e) => setConfirm(e.target.checked)} />
              <span>
                Je confirme l'application de cet aperçu : {c.foyersCrees} foyer(s) créé(s), {c.foyersMisAJour} mis à
                jour, {c.liensAjoutes} lien(s) ajouté(s), {c.liensRetires} retiré(s). Les élèves non trouvés ou
                ambigus ne sont pas touchés.
              </span>
            </label>
            <button
              type="button"
              disabled={!confirm || busy}
              onClick={() => void send("apply")}
              className="rounded-xl bg-rose-600 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
            >
              {busy ? "Application…" : "Appliquer l'import"}
            </button>
          </section>
        </>
      ) : null}
    </ModulePageShell>
  );
}
