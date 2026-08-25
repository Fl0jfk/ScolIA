"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import Link from "next/link";

type CountRow = { type: string; n: number };
type LogRow = {
  id: string;
  fichier: string;
  statut: string;
  nbInserts: number;
  nbUpdates: number;
  dateImport: string;
  rapportJson?: {
    kind?: string;
    total?: number;
    linkedUsers?: number;
    linksCreated?: number;
    foyersCreated?: number;
  } | null;
};
type ExportRow = {
  id: string;
  fichier: string;
  dateImport: string;
  statut: string;
  rapport?: {
    numEnvoi?: string;
    stats?: { eleves?: number; personnes?: number; sansIne?: number; sansSiecleId?: number };
  };
};
type AnomalyRow = {
  id: string;
  severity: "info" | "warn" | "error";
  label: string;
  detail: string;
  count?: number;
};

export default function NomenclatureImportPanel() {
  const inputRef = useRef<HTMLInputElement>(null);
  const [counts, setCounts] = useState<CountRow[]>([]);
  const [logs, setLogs] = useState<LogRow[]>([]);
  const [exports, setExports] = useState<ExportRow[]>([]);
  const [refEtabCount, setRefEtabCount] = useState(0);
  const [anomalies, setAnomalies] = useState<AnomalyRow[]>([]);
  const [omogenConfigured, setOmogenConfigured] = useState(false);
  const [busy, setBusy] = useState(false);
  const [exportBusy, setExportBusy] = useState(false);
  const [omogenBusy, setOmogenBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const [importRes, exportRes, omogenRes] = await Promise.all([
        fetch("/api/nomenclature/import", { cache: "no-store" }),
        fetch("/api/nomenclature/export-siecle", { cache: "no-store" }),
        fetch("/api/nomenclature/omogen-sync", { cache: "no-store" }),
      ]);
      const data = await importRes.json();
      const exportData = await exportRes.json();
      const omogenData = omogenRes.ok ? await omogenRes.json() : null;
      if (!importRes.ok) throw new Error(data?.error || "Chargement impossible");
      setCounts(data.counts || []);
      setLogs(data.logs || []);
      setRefEtabCount(data.refEtablissementCount ?? 0);
      setAnomalies(data.anomalies || []);
      if (exportRes.ok) setExports(exportData.exports || []);
      setOmogenConfigured(Boolean(omogenData?.configured));
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const onFiles = async (files: FileList | null) => {
    if (!files?.length) return;
    setBusy(true);
    setMessage(null);
    setError(null);
    try {
      const fd = new FormData();
      Array.from(files).forEach((f) => fd.append("files", f));
      const res = await fetch("/api/nomenclature/import", { method: "POST", body: fd });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Import impossible");
      const lines = (data.reports || []).map(
        (r: { file: string; message?: string; error?: string }) =>
          r.error ? `${r.file}: ${r.error}` : r.message || r.file,
      );
      setMessage(lines.join("\n"));
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
      if (inputRef.current) inputRef.current.value = "";
    }
  };

  const onExport = async () => {
    setExportBusy(true);
    setMessage(null);
    setError(null);
    try {
      const res = await fetch("/api/nomenclature/export-siecle", { method: "POST" });
      if (!res.ok) {
        const data = (await res.json().catch(() => ({}))) as { error?: string };
        throw new Error(data.error || "Export impossible");
      }
      const blob = await res.blob();
      const disposition = res.headers.get("Content-Disposition") || "";
      const match = disposition.match(/filename="([^"]+)"/);
      const filename = match?.[1] || "export-siecle.zip";
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = filename;
      a.click();
      URL.revokeObjectURL(url);
      const statsHeader = res.headers.get("X-Scolia-Export-Stats");
      if (statsHeader) {
        const stats = JSON.parse(statsHeader) as {
          eleves?: number;
          personnes?: number;
          sansIne?: number;
          sansSiecleId?: number;
        };
        setMessage(
          `Export généré : ${stats.eleves ?? 0} élève(s), ${stats.personnes ?? 0} responsable(s).` +
            (stats.sansIne ? ` ${stats.sansIne} sans INE.` : "") +
            (stats.sansSiecleId ? ` ${stats.sansSiecleId} sans ID Siècle.` : ""),
        );
      } else {
        setMessage(`Export Siècle téléchargé (${filename}).`);
      }
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setExportBusy(false);
    }
  };

  const onOmogenSync = async () => {
    setOmogenBusy(true);
    setMessage(null);
    setError(null);
    try {
      const res = await fetch("/api/nomenclature/omogen-sync", { method: "POST" });
      const data = (await res.json()) as { message?: string; error?: string };
      if (!res.ok) throw new Error(data.message || data.error || "Sync Omogen impossible");
      setMessage(data.message || "Sync Omogen terminée.");
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setOmogenBusy(false);
    }
  };

  const anomalyStyle = (severity: AnomalyRow["severity"]) => {
    if (severity === "error") return "border-red-200 bg-red-50 text-red-900";
    if (severity === "warn") return "border-amber-200 bg-amber-50 text-amber-900";
    return "border-slate-200 bg-slate-50 text-slate-700";
  };

  const logDetail = (l: LogRow): string | null => {
    const r = l.rapportJson;
    if (!r?.kind) return null;
    if (r.kind === "eleves" && r.total != null) return `${r.total} élève(s) lus`;
    if (r.kind === "responsables") {
      const parts: string[] = [];
      if (r.linksCreated != null) parts.push(`${r.linksCreated} lien(s) élève`);
      if (r.linkedUsers != null && r.linkedUsers > 0) parts.push(`${r.linkedUsers} compte(s) parent`);
      return parts.length ? parts.join(" · ") : null;
    }
    return null;
  };

  return (
    <div className="space-y-6 text-sm">
      <div>
        <h2 className="font-black text-slate-900 text-lg">Éducation nationale — Pont Siècle</h2>
        <p className="text-slate-600 mt-1">
          Importez les 7 XML Siècle (ISO-8859-15) :{" "}
          <code className="text-xs bg-slate-100 px-1 rounded">Communs.xml</code>,{" "}
          <code className="text-xs bg-slate-100 px-1 rounded">Nomenclature.xml</code>,{" "}
          <code className="text-xs bg-slate-100 px-1 rounded">Geographique.xml</code>,{" "}
          <code className="text-xs bg-slate-100 px-1 rounded">Etablissements.xml</code>,{" "}
          <code className="text-xs bg-slate-100 px-1 rounded">Structures.xml</code>,{" "}
          <code className="text-xs bg-slate-100 px-1 rounded">ElevesSansAdresses.xml</code>,{" "}
          <code className="text-xs bg-slate-100 px-1 rounded">ResponsablesAvecAdresses.xml</code>.
          Communs et référentiels d&apos;abord, élèves avant responsables.
        </p>
        <p className="mt-2">
          <Link href="/parametres?tab=annees" className="text-xs font-bold text-indigo-600 hover:underline">
            Année scolaire courante (exports Siècle)
          </Link>
        </p>
      </div>

      <div className="flex flex-wrap gap-3 items-center">
        <input
          ref={inputRef}
          type="file"
          accept=".xml,application/xml,text/xml"
          multiple
          className="hidden"
          onChange={(e) => void onFiles(e.target.files)}
        />
        <button
          type="button"
          disabled={busy}
          onClick={() => inputRef.current?.click()}
          className="rounded-xl bg-indigo-600 text-white px-4 py-2 font-bold disabled:opacity-50"
        >
          {busy ? "Import…" : "Importer XML Siècle"}
        </button>
        <button
          type="button"
          disabled={exportBusy || busy}
          onClick={() => void onExport()}
          className="rounded-xl border border-indigo-200 bg-indigo-50 text-indigo-800 px-4 py-2 font-bold disabled:opacity-50"
        >
          {exportBusy ? "Export…" : "Générer export Siècle (ZIP)"}
        </button>
        <button
          type="button"
          disabled={omogenBusy || busy || !omogenConfigured}
          title={
            omogenConfigured
              ? "Lancer une sync Omogen (certificat académie)"
              : "Omogen non configuré — import XML manuel"
          }
          onClick={() => void onOmogenSync()}
          className="rounded-xl border border-slate-200 bg-white text-slate-700 px-4 py-2 font-bold disabled:opacity-50"
        >
          {omogenBusy ? "Omogen…" : "Sync Omogen"}
        </button>
      </div>

      <p className="text-xs text-slate-500">
        Export <code className="bg-slate-100 px-1 rounded">IMPORT_ELEVES</code> v4.0 — déposez le ZIP
        sur Siècle après import des référentiels et des élèves. Variable{" "}
        <code className="bg-slate-100 px-1 rounded">SIECLE_UAJ</code> pour forcer le code RNE.
        Sync auto :{" "}
        <code className="bg-slate-100 px-1 rounded">POST /api/nomenclature/omogen-sync/cron</code>{" "}
        avec <code className="bg-slate-100 px-1 rounded">OMOGEN_CRON_SECRET</code>.
      </p>

      {message && (
        <pre className="whitespace-pre-wrap rounded-xl bg-emerald-50 border border-emerald-100 p-3 text-emerald-900 text-xs">
          {message}
        </pre>
      )}
      {error && <p className="text-red-600 font-semibold">{error}</p>}

      {anomalies.length > 0 && (
        <section className="rounded-2xl border border-slate-200 bg-white p-4">
          <h3 className="font-bold mb-2">Anomalies import Siècle</h3>
          <ul className="space-y-2">
            {anomalies.map((a) => (
              <li
                key={a.id}
                className={`rounded-lg border px-3 py-2 text-xs ${anomalyStyle(a.severity)}`}
              >
                <div className="font-semibold flex justify-between gap-2">
                  <span>{a.label}</span>
                  {a.count != null && a.count > 0 && (
                    <span className="font-mono">{a.count}</span>
                  )}
                </div>
                <p className="mt-0.5 opacity-90">{a.detail}</p>
              </li>
            ))}
          </ul>
        </section>
      )}

      <div className="grid gap-4 md:grid-cols-2">
        <section className="rounded-2xl border border-slate-200 bg-white p-4">
          <h3 className="font-bold mb-2">Référentiels en base</h3>
          <ul className="space-y-1">
            {counts.map((c) => (
              <li key={c.type} className="flex justify-between">
                <span className="font-mono text-xs">{c.type}</span>
                <span className="font-semibold">{c.n}</span>
              </li>
            ))}
            <li className="flex justify-between border-t border-slate-100 pt-1 mt-1">
              <span className="font-mono text-xs">ref_etablissement (RNE)</span>
              <span className="font-semibold">{refEtabCount}</span>
            </li>
            {!counts.length && !refEtabCount && (
              <li className="text-slate-500">Vide — importez Communs.xml ou Nomenclature.xml</li>
            )}
          </ul>
        </section>
        <section className="rounded-2xl border border-slate-200 bg-white p-4">
          <h3 className="font-bold mb-2">Historique imports</h3>
          <ul className="space-y-2 max-h-48 overflow-y-auto">
            {logs.map((l) => (
              <li key={l.id} className="text-xs border-b border-slate-100 pb-1">
                <div className="font-semibold">{l.fichier}</div>
                <div className="text-slate-500">
                  {l.statut} · +{l.nbInserts} / ~{l.nbUpdates} ·{" "}
                  {l.dateImport ? new Date(l.dateImport).toLocaleString("fr-FR") : ""}
                </div>
                {logDetail(l) ? (
                  <div className="text-indigo-700 mt-0.5">{logDetail(l)}</div>
                ) : null}
              </li>
            ))}
            {!logs.length && <li className="text-slate-500">Aucun import encore</li>}
          </ul>
        </section>
      </div>

      <section className="rounded-2xl border border-slate-200 bg-white p-4">
        <h3 className="font-bold mb-2">Historique exports Siècle</h3>
        <ul className="space-y-2 max-h-40 overflow-y-auto">
          {exports.map((e) => (
            <li key={e.id} className="text-xs border-b border-slate-100 pb-1">
              <div className="font-semibold">{e.fichier}</div>
              <div className="text-slate-500">
                n° envoi {e.rapport?.numEnvoi || "—"} · {e.rapport?.stats?.eleves ?? "—"} élève(s) ·{" "}
                {e.dateImport ? new Date(e.dateImport).toLocaleString("fr-FR") : ""}
              </div>
            </li>
          ))}
          {!exports.length && (
            <li className="text-slate-500">Aucun export — générez un ZIP après import complet.</li>
          )}
        </ul>
      </section>
    </div>
  );
}
