"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
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
    cycle?: "college" | "lycee";
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
type ImportSlot = {
  kind: string;
  label: string;
  filenameHint: string;
  required: boolean;
  order?: number;
  cycleScoped?: boolean;
};
type ImportStatusRow = {
  kind: string;
  cycle: "college" | "lycee";
  imported: boolean;
  lastImport: string | null;
  lastFile: string | null;
  statut: string | null;
  rows: number | null;
};
type DivisionRow = {
  code: string;
  libelle: string;
  pole?: string | null;
};

type ImportCycle = "college" | "lycee";

const CYCLE_LABEL: Record<ImportCycle, string> = {
  college: "Collège",
  lycee: "Lycée",
};

/** Fallback UI si l’API met trop longtemps (évite une page « vide / cassée »). */
const DEFAULT_IMPORT_SLOTS: ImportSlot[] = [
  { kind: "communs", label: "Communs", filenameHint: "Communs.xml", required: true, order: 0, cycleScoped: true },
  { kind: "nomenclature", label: "Nomenclature", filenameHint: "Nomenclature.xml", required: true, order: 1, cycleScoped: true },
  { kind: "geographique", label: "Géographique", filenameHint: "Geographique.xml", required: false, order: 2, cycleScoped: true },
  { kind: "etablissements", label: "Établissements", filenameHint: "Etablissements.xml", required: false, order: 3, cycleScoped: true },
  { kind: "structures", label: "Structures", filenameHint: "Structures.xml", required: true, order: 4, cycleScoped: true },
  { kind: "eleves", label: "Élèves", filenameHint: "ElevesSansAdresses.xml", required: false, order: 5, cycleScoped: true },
  {
    kind: "responsables",
    label: "Responsables (parents + adresses)",
    filenameHint: "ResponsablesAvecAdresses.xml",
    required: false,
    order: 6,
    cycleScoped: true,
  },
];

/** Aligné sur /api/nomenclature/import (MAX_XML_BYTES). */
const MAX_CLIENT_XML_BYTES = 100 * 1024 * 1024;
const MAX_CLIENT_XML_LABEL = "100 Mo";

function formatMo(bytes: number): string {
  return `${(bytes / (1024 * 1024)).toFixed(1)} Mo`;
}

export default function NomenclatureImportPanel() {
  const multiInputRef = useRef<HTMLInputElement>(null);
  const slotInputRefs = useRef<Record<string, HTMLInputElement | null>>({});
  const [counts, setCounts] = useState<CountRow[]>([]);
  const [logs, setLogs] = useState<LogRow[]>([]);
  const [exports, setExports] = useState<ExportRow[]>([]);
  const [refEtabCount, setRefEtabCount] = useState(0);
  const [anomalies, setAnomalies] = useState<AnomalyRow[]>([]);
  const [slots, setSlots] = useState<ImportSlot[]>(DEFAULT_IMPORT_SLOTS);
  const [importStatus, setImportStatus] = useState<ImportStatusRow[]>([]);
  const [omogenConfigured, setOmogenConfigured] = useState(false);
  const [busy, setBusy] = useState(false);
  const [busySlot, setBusySlot] = useState<string | null>(null);
  const [exportBusy, setExportBusy] = useState(false);
  const [omogenBusy, setOmogenBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [officialDivisions, setOfficialDivisions] = useState<DivisionRow[]>([]);
  const [siecleLockedCollègeLycée, setSiecleLockedCollègeLycée] = useState(false);
  const [unmatchedEleveClasses, setUnmatchedEleveClasses] = useState<string[]>([]);
  const [cycle, setCycle] = useState<ImportCycle>("lycee");
  const [lockedByPole, setLockedByPole] = useState<Partial<Record<string, string[]>>>({});

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const settled = await Promise.allSettled([
        fetch("/api/nomenclature/import", { cache: "no-store" }),
        fetch("/api/nomenclature/export-siecle", { cache: "no-store" }),
        fetch("/api/nomenclature/omogen-sync", { cache: "no-store" }),
        fetch("/api/nomenclature/classes", { cache: "no-store" }),
      ]);

      const importRes = settled[0].status === "fulfilled" ? settled[0].value : null;
      const exportRes = settled[1].status === "fulfilled" ? settled[1].value : null;
      const omogenRes = settled[2].status === "fulfilled" ? settled[2].value : null;
      const classesRes = settled[3].status === "fulfilled" ? settled[3].value : null;

      if (!importRes) {
        throw new Error("Chargement Éducation nationale trop long ou indisponible (serveur saturé).");
      }
      const data = await importRes.json();
      if (!importRes.ok) throw new Error(data?.error || "Chargement impossible");
      setCounts(data.counts || []);
      setLogs(data.logs || []);
      setRefEtabCount(data.refEtablissementCount ?? 0);
      setAnomalies(data.anomalies || []);
      setSlots(Array.isArray(data.slots) && data.slots.length ? data.slots : DEFAULT_IMPORT_SLOTS);
      setImportStatus(data.importStatus || []);

      if (exportRes?.ok) {
        const exportData = await exportRes.json();
        setExports(exportData.exports || []);
      }
      if (omogenRes?.ok) {
        const omogenData = await omogenRes.json();
        setOmogenConfigured(Boolean(omogenData?.configured));
      }
      if (classesRes?.ok) {
        const classesData = await classesRes.json();
        const divisions = (classesData.divisions || []).map(
          (d: {
            code: string;
            libelle: string;
            pole?: string | null;
            metadata?: { pole?: string } | null;
          }) => ({
            code: d.code,
            libelle: d.libelle,
            pole: d.pole ?? d.metadata?.pole ?? null,
          }),
        );
        setOfficialDivisions(divisions);
        setSiecleLockedCollègeLycée(Boolean(classesData.siecleLockedCollègeLycée));
        setUnmatchedEleveClasses(classesData.unmatchedEleveClasses || []);
        setLockedByPole(classesData.lockedClassesByPole || {});
      } else if (classesRes && !classesRes.ok) {
        setError((prev) => prev || "Classes Siècle : chargement incomplet (réessayez dans un instant).");
      }
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
      setSlots((prev) => (prev.length ? prev : DEFAULT_IMPORT_SLOTS));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const uploadFiles = async (files: FileList | File[], slotKind?: string) => {
    const list = Array.from(files);
    if (!list.length) return;
    if (slotKind) setBusySlot(slotKind);
    else setBusy(true);
    setMessage(null);
    setError(null);

    const oversized = list.filter((f) => f.size > MAX_CLIENT_XML_BYTES);
    if (oversized.length) {
      setError(
        oversized
          .map((f) => `${f.name} : ${formatMo(f.size)} (max ${MAX_CLIENT_XML_LABEL})`)
          .join(" · "),
      );
      setBusy(false);
      setBusySlot(null);
      if (multiInputRef.current) multiInputRef.current.value = "";
      if (slotKind && slotInputRefs.current[slotKind]) {
        slotInputRefs.current[slotKind]!.value = "";
      }
      return;
    }

    try {
      /** Un fichier à la fois : évite de saturer le body HTTP (multipart) sur les gros XML. */
      const allLines: string[] = [`Import ${CYCLE_LABEL[cycle]}`];
      for (let i = 0; i < list.length; i++) {
        const file = list[i];
        if (list.length > 1) {
          setMessage(`Envoi ${i + 1}/${list.length} : ${file.name} (${formatMo(file.size)})…`);
        }
        const fd = new FormData();
        fd.append("cycle", cycle);
        fd.append("files", file);
        const res = await fetch("/api/nomenclature/import", { method: "POST", body: fd });
        const data = (await res.json().catch(() => ({}))) as {
          error?: string;
          reports?: Array<{ file: string; message?: string; error?: string }>;
        };
        if (!res.ok) {
          throw new Error(
            data?.error ||
              (res.status === 413
                ? `Fichier trop volumineux (${formatMo(file.size)}).`
                : `Import impossible (HTTP ${res.status}).`),
          );
        }
        const lines = (data.reports || []).map((r) =>
          r.error ? `${r.file}: ${r.error}` : r.message || r.file,
        );
        allLines.push(...(lines.length ? lines : [file.name]));
      }
      setMessage(allLines.join("\n"));
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
      setBusySlot(null);
      if (multiInputRef.current) multiInputRef.current.value = "";
      if (slotKind && slotInputRefs.current[slotKind]) {
        slotInputRefs.current[slotKind]!.value = "";
      }
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
    const parts: string[] = [];
    if (r.cycle === "college") parts.push("Collège");
    if (r.cycle === "lycee") parts.push("Lycée");
    if (r.kind === "eleves" && r.total != null) parts.push(`${r.total} élève(s) lus`);
    if (r.kind === "responsables") {
      if (r.linksCreated != null) parts.push(`${r.linksCreated} lien(s) élève`);
      if (r.linkedUsers != null && r.linkedUsers > 0) parts.push(`${r.linkedUsers} compte(s) parent`);
    }
    return parts.length ? parts.join(" · ") : null;
  };

  const statusForKind = (kind: string) =>
    importStatus.find((s) => s.kind === kind && s.cycle === cycle);

  const slotList = useMemo(
    () => [...slots].sort((a, b) => (a.order ?? 0) - (b.order ?? 0)),
    [slots],
  );

  const slotBadge = (slot: ImportSlot) => {
    const st = statusForKind(slot.kind);
    if (st?.imported) {
      return (
        <span className="text-[10px] font-bold uppercase tracking-wide text-emerald-700 bg-emerald-50 px-2 py-0.5 rounded-full">
          OK · {st.rows ?? "—"} entrées
        </span>
      );
    }
    if (slot.required) {
      return (
        <span className="text-[10px] font-bold uppercase tracking-wide text-amber-700 bg-amber-50 px-2 py-0.5 rounded-full">
          Requis
        </span>
      );
    }
    return (
      <span className="text-[10px] font-bold uppercase tracking-wide text-slate-500 bg-slate-100 px-2 py-0.5 rounded-full">
        Optionnel
      </span>
    );
  };

  const renderSlotGrid = (list: ImportSlot[]) => (
    <div className="grid gap-3 sm:grid-cols-2">
      {list.map((slot) => {
        const st = statusForKind(slot.kind);
        const inputKey = `${cycle}:${slot.kind}`;
        const slotBusy = busySlot === inputKey || busy;
        return (
          <div
            key={inputKey}
            className="rounded-xl border border-slate-100 bg-slate-50/50 p-3 flex flex-col gap-2"
          >
            <div className="flex items-start justify-between gap-2">
              <div>
                <div className="font-semibold text-slate-900">{slot.label}</div>
                <code className="text-[11px] text-slate-500">{slot.filenameHint}</code>
              </div>
              {slotBadge(slot)}
            </div>
            {st?.lastImport ? (
              <p className="text-[11px] text-slate-500">
                Dernier import : {new Date(st.lastImport).toLocaleString("fr-FR")}
                {st.lastFile ? ` · ${st.lastFile}` : ""}
              </p>
            ) : (
              <p className="text-[11px] text-slate-400">Pas encore importé</p>
            )}
            <input
              ref={(el) => {
                slotInputRefs.current[inputKey] = el;
              }}
              type="file"
              accept=".xml,application/xml,text/xml"
              className="hidden"
              onChange={(e) => void uploadFiles(e.target.files || [], inputKey)}
            />
            <button
              type="button"
              disabled={slotBusy}
              onClick={() => slotInputRefs.current[inputKey]?.click()}
              className="mt-auto rounded-lg border border-indigo-200 bg-white text-indigo-800 px-3 py-1.5 text-xs font-bold disabled:opacity-50 hover:bg-indigo-50"
            >
              {slotBusy ? "Import…" : `Choisir le fichier (${CYCLE_LABEL[cycle]})`}
            </button>
          </div>
        );
      })}
    </div>
  );

  const collegeCount = lockedByPole.COLLÈGE?.length ?? officialDivisions.filter((d) => d.pole === "COLLÈGE").length;
  const lyceeCount = lockedByPole.LYCÉE?.length ?? officialDivisions.filter((d) => d.pole === "LYCÉE").length;

  const visibleDivisions = useMemo(() => {
    const poleWanted = cycle === "college" ? "COLLÈGE" : "LYCÉE";
    const filtered = officialDivisions.filter((d) => (d.pole || "").toUpperCase() === poleWanted);
    return filtered.length ? filtered : officialDivisions;
  }, [officialDivisions, cycle]);

  return (
    <div className="space-y-6 text-sm">
      <div>
        <h2 className="font-black text-slate-900 text-lg">Éducation nationale — Pont Siècle</h2>
        <p className="text-slate-600 mt-1">
          Siècle exporte le <strong>collège</strong> et le <strong>lycée</strong> séparément (UAJ
          distincts). Importez <strong>tout le jeu XML</strong> pour chaque cycle — y compris
          Nomenclature (matières / MEF), Géographique et Établissements : les référentiels ne sont
          pas partagés et s&apos;ajoutent sans écraser l&apos;autre cycle.
        </p>
        <p className="mt-2">
          <Link href="/parametres?tab=annees" className="text-xs font-bold text-indigo-600 hover:underline">
            Année scolaire courante (exports Siècle)
          </Link>
        </p>
        {loading ? (
          <p className="mt-3 text-xs font-semibold text-amber-800 bg-amber-50 border border-amber-100 rounded-lg px-3 py-2">
            Chargement de l’historique et des classes… (peut prendre un moment si un gros import photos tourne encore).
          </p>
        ) : null}
      </div>

      <div className="flex flex-wrap gap-2 items-center">
        {(["college", "lycee"] as const).map((c) => {
          const active = cycle === c;
          const n = c === "college" ? collegeCount : lyceeCount;
          return (
            <button
              key={c}
              type="button"
              onClick={() => setCycle(c)}
              className={
                active
                  ? "rounded-xl bg-indigo-600 text-white px-4 py-2 text-sm font-bold shadow-sm"
                  : "rounded-xl border border-slate-200 bg-white text-slate-700 px-4 py-2 text-sm font-bold hover:bg-slate-50"
              }
            >
              {CYCLE_LABEL[c]}
              <span className={active ? "ml-2 opacity-90" : "ml-2 text-slate-400"}>
                {n} classe{n === 1 ? "" : "s"}
              </span>
            </button>
          );
        })}
      </div>

      <section className="rounded-2xl border border-slate-200 bg-white p-4">
        <h3 className="font-bold mb-1">Fichiers Siècle — {CYCLE_LABEL[cycle]}</h3>
        <p className="text-xs text-slate-500 mb-4">
          Ordre recommandé : Communs → Nomenclature → Géographique → Structures → Élèves →
          Responsables. Chaque fichier est propre à ce cycle (matières collège ≠ matières lycée).
          Taille max. {MAX_CLIENT_XML_LABEL} par XML — les gros exports (Élèves / Responsables) sont
          envoyés un par un.
        </p>
        {renderSlotGrid(slotList)}
      </section>

      {officialDivisions.length > 0 ? (
        <section className="rounded-2xl border border-indigo-200 bg-indigo-50/40 p-4">
          <h3 className="font-bold mb-1 text-indigo-950">
            Classes {CYCLE_LABEL[cycle]} (Structures Siècle)
          </h3>
          <p className="text-xs text-indigo-900/80 mb-3">
            Collège : {collegeCount} · Lycée : {lyceeCount} — imposées telles quelles par le
            rectorat, sans matching manuel.
          </p>
          {siecleLockedCollègeLycée ? (
            <p className="text-[11px] font-bold text-emerald-800 mb-2">
              {collegeCount > 0 && lyceeCount > 0
                ? "Collège et lycée verrouillés sur Siècle."
                : collegeCount > 0
                  ? "Collège verrouillé — importez encore le Structures.xml du lycée."
                  : "Lycée verrouillé — importez encore le Structures.xml du collège."}{" "}
              École : hors périmètre rectorat pour l&apos;instant.
            </p>
          ) : null}
          {unmatchedEleveClasses.length > 0 ? (
            <div className="rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-950 mb-3">
              <p className="font-bold mb-1">
                Classes hors Structures Siècle ({unmatchedEleveClasses.length}) — ignorées dans les
                filtres
              </p>
              <p className="mb-1">{unmatchedEleveClasses.join(", ")}</p>
              <p className="text-amber-900/80">
                Souvent des libellés N-1 ou d&apos;un autre établissement (élèves arrivés en cours
                d&apos;année). Conservés sur la fiche pour l&apos;historique, exclus des recherches
                par classe année en cours.
              </p>
            </div>
          ) : null}
          <ul className="grid gap-1 sm:grid-cols-2 max-h-56 overflow-y-auto text-xs">
            {visibleDivisions.map((d) => (
              <li
                key={d.code}
                className="flex justify-between gap-2 rounded-lg bg-white border border-indigo-100 px-2 py-1.5"
              >
                <span className="font-mono font-bold text-indigo-900">{d.code}</span>
                <span className="text-slate-600 truncate text-right">{d.libelle}</span>
              </li>
            ))}
          </ul>
        </section>
      ) : null}

      <div className="flex flex-wrap gap-3 items-center">
        <input
          ref={multiInputRef}
          type="file"
          accept=".xml,application/xml,text/xml"
          multiple
          className="hidden"
          onChange={(e) => void uploadFiles(e.target.files || [])}
        />
        <button
          type="button"
          disabled={busy}
          onClick={() => multiInputRef.current?.click()}
          className="rounded-xl bg-indigo-600 text-white px-4 py-2 font-bold disabled:opacity-50"
        >
          {busy ? "Import…" : `Importer plusieurs XML (${CYCLE_LABEL[cycle]})`}
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
        Les divisions Structures.xml deviennent la liste de classes de référence (roster, notes, EDT).
        Les matières Nomenclature.xml alimentent aussi{" "}
        <code className="bg-slate-100 px-1 rounded">note_matiere</code>.
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
