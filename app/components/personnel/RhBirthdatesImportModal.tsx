"use client";

import { useState } from "react";
import type { BirthdateImportRow, BirthdateImportStatus } from "@/app/lib/personnel-birthdate-import";

type Counts = Record<BirthdateImportStatus, number>;
type ApiResult = { mode: string; counts: Counts; written: number; failed: number; rows: BirthdateImportRow[] };

const GROUPS: { status: BirthdateImportStatus; label: string; tone: string }[] = [
  { status: "ok", label: "À importer", tone: "text-emerald-700 bg-emerald-50 border-emerald-200" },
  { status: "remplace", label: "Date différente (sera remplacée)", tone: "text-amber-700 bg-amber-50 border-amber-200" },
  { status: "inchange", label: "Déjà à jour", tone: "text-slate-600 bg-slate-50 border-slate-200" },
  { status: "introuvable", label: "Introuvable", tone: "text-rose-700 bg-rose-50 border-rose-200" },
  { status: "ambigu", label: "Ambigu", tone: "text-rose-700 bg-rose-50 border-rose-200" },
  { status: "date_invalide", label: "Date invalide", tone: "text-rose-700 bg-rose-50 border-rose-200" },
];

function fr(d: string | null) {
  if (!d) return "—";
  const [y, m, j] = d.split("-");
  return `${j}/${m}/${y}`;
}

export default function RhBirthdatesImportModal({
  open,
  onClose,
  onDone,
}: {
  open: boolean;
  onClose: () => void;
  onDone: () => void;
}) {
  const [file, setFile] = useState<File | null>(null);
  const [preview, setPreview] = useState<ApiResult | null>(null);
  const [result, setResult] = useState<ApiResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!open) return null;

  const close = () => {
    setFile(null);
    setPreview(null);
    setResult(null);
    setError(null);
    onClose();
  };

  const send = async (f: File, mode: "preview" | "apply") => {
    setBusy(true);
    setError(null);
    try {
      const fd = new FormData();
      fd.set("file", f);
      fd.set("mode", mode);
      const res = await fetch("/api/personnel/birthdates-import", { method: "POST", body: fd });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Import impossible");
      if (mode === "preview") setPreview(j as ApiResult);
      else {
        setResult(j as ApiResult);
        setPreview(null);
        onDone();
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const toImport = preview ? preview.counts.ok + preview.counts.remplace : 0;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/40 backdrop-blur-sm">
      <div className="bg-white rounded-2xl shadow-2xl max-w-3xl w-full max-h-[92vh] overflow-hidden flex flex-col">
        <div className="p-6 border-b border-slate-100 shrink-0">
          <h2 className="text-lg font-black text-slate-900">Importer les dates de naissance</h2>
          <p className="text-xs text-slate-500 mt-1">
            Fichier Excel (.xlsx, .xls) ou CSV, première feuille. Colonnes attendues : <b>Nom</b>, <b>Prénom</b> (ou
            « Nom Prénom »), <b>Email</b> (facultatif, prioritaire pour le rapprochement) et <b>Date de naissance</b>{" "}
            (JJ/MM/AAAA, AAAA-MM-JJ ou date Excel). 2 Mo et 2000 lignes maximum. Rien n&apos;est écrit avant votre
            validation.
          </p>
        </div>

        <div className="p-6 overflow-y-auto flex-1 space-y-4">
          {error && <p className="text-sm text-rose-600 font-medium">{error}</p>}

          {result ? (
            <div className="rounded-xl border border-emerald-200 bg-emerald-50 p-4 text-sm text-emerald-800">
              <p className="font-bold">
                {result.written} date{result.written > 1 ? "s" : ""} enregistrée{result.written > 1 ? "s" : ""}.
              </p>
              {result.failed > 0 && <p className="text-rose-700 mt-1">{result.failed} échec(s) d&apos;enregistrement.</p>}
              <p className="mt-1 text-xs">
                Ignorées : {result.counts.inchange} déjà à jour, {result.counts.introuvable} introuvable(s),{" "}
                {result.counts.ambigu} ambiguë(s), {result.counts.date_invalide} date(s) invalide(s).
              </p>
            </div>
          ) : (
            <>
              <input
                type="file"
                accept=".xlsx,.xls,.csv"
                onChange={(e) => {
                  const f = e.target.files?.[0] ?? null;
                  setFile(f);
                  setPreview(null);
                  setError(null);
                  if (f) void send(f, "preview");
                }}
                className="block w-full text-sm"
              />
              {busy && !preview && <p className="text-xs text-slate-400">Analyse du fichier…</p>}
            </>
          )}

          {preview && (
            <div className="space-y-4">
              <div className="flex flex-wrap gap-2">
                {GROUPS.map((g) => (
                  <span key={g.status} className={`text-xs font-bold border rounded-full px-3 py-1 ${g.tone}`}>
                    {g.label} : {preview.counts[g.status]}
                  </span>
                ))}
              </div>
              {GROUPS.filter((g) => preview.counts[g.status] > 0).map((g) => (
                <section key={g.status}>
                  <h4 className="text-xs font-black uppercase tracking-widest text-slate-500 mb-2">{g.label}</h4>
                  <div className="overflow-x-auto rounded-xl border border-slate-100">
                    <table className="w-full text-xs">
                      <thead className="bg-slate-50 text-slate-500">
                        <tr>
                          <th className="text-left px-3 py-2" title="Numéro de ligne dans le fichier Excel (la ligne 1 est l'en-tête)">Ligne Excel</th>
                          <th className="text-left px-3 py-2">Nom</th>
                          <th className="text-left px-3 py-2">Prénom</th>
                          <th className="text-left px-3 py-2">Actuelle</th>
                          <th className="text-left px-3 py-2">Nouvelle</th>
                          <th className="text-left px-3 py-2">Remarque</th>
                        </tr>
                      </thead>
                      <tbody>
                        {preview.rows
                          .filter((r) => r.status === g.status)
                          .map((r) => (
                            <tr key={r.line} className="border-t border-slate-100">
                              <td className="px-3 py-1.5">{r.line}</td>
                              <td className="px-3 py-1.5">{r.lastName}</td>
                              <td className="px-3 py-1.5">{r.firstName}</td>
                              <td className="px-3 py-1.5">{fr(r.oldDate)}</td>
                              <td className="px-3 py-1.5 font-bold">{fr(r.newDate)}</td>
                              <td className="px-3 py-1.5 text-slate-500">{r.message || ""}</td>
                            </tr>
                          ))}
                      </tbody>
                    </table>
                  </div>
                </section>
              ))}
            </div>
          )}
        </div>

        <div className="p-6 border-t border-slate-100 flex gap-2 shrink-0">
          <button type="button" onClick={close} className="flex-1 py-2.5 rounded-xl border font-bold text-sm text-slate-600">
            {result ? "Fermer" : "Annuler"}
          </button>
          {!result && (
            <button
              type="button"
              disabled={busy || !file || toImport === 0}
              onClick={() => file && void send(file, "apply")}
              className="flex-1 py-2.5 rounded-xl bg-indigo-600 text-white font-bold text-sm disabled:opacity-50"
            >
              {busy && preview ? "Import…" : `Valider l’import (${toImport})`}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
