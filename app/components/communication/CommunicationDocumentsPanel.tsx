"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import type {
  DocumentOutputFormat,
  DocumentPlaceholderDef,
  DocumentTemplateMeta,
} from "@/app/lib/document-templates/types";

type GeneratedRow = {
  id: string;
  templateId: string;
  templateLabel: string;
  title: string;
  createdAt: string;
  downloadUrl: string;
  format?: DocumentOutputFormat | "pdf";
};

const FORMAT_OPTIONS: { id: DocumentOutputFormat; label: string; hint: string }[] = [
  { id: "fillable-pdf", label: "PDF à trous", hint: "Vierge, remplissable (ED / Adobe)" },
  { id: "docx", label: "Word (.docx)", hint: "Placeholders $$…$$ pour publipostage" },
];

function formatBadge(format?: DocumentOutputFormat | "pdf") {
  if (format === "docx") return "DOCX";
  if (format === "fillable-pdf") return "PDF à trous";
  return "PDF";
}

export default function CommunicationDocumentsPanel() {
  const [templates, setTemplates] = useState<DocumentTemplateMeta[]>([]);
  const [placeholders, setPlaceholders] = useState<DocumentPlaceholderDef[]>([]);
  const [branding, setBranding] = useState<{ name?: string }>({});
  const [templateId, setTemplateId] = useState<string>("");
  const [format, setFormat] = useState<DocumentOutputFormat>("fillable-pdf");
  const [history, setHistory] = useState<GeneratedRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);

  const active = useMemo(
    () => templates.find((t) => t.id === templateId) || null,
    [templates, templateId],
  );

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [tRes, hRes] = await Promise.all([
        fetch("/api/document-templates", { cache: "no-store" }),
        fetch("/api/document-templates/generated", { cache: "no-store" }),
      ]);
      const tj = await tRes.json();
      if (!tRes.ok) throw new Error(tj.error || "Catalogue indisponible");
      setTemplates(Array.isArray(tj.templates) ? tj.templates : []);
      setPlaceholders(Array.isArray(tj.placeholders) ? tj.placeholders : []);
      setBranding(tj.branding || {});
      if (!templateId && tj.templates?.[0]?.id) setTemplateId(tj.templates[0].id);

      const hj = await hRes.json();
      if (hRes.ok) setHistory(Array.isArray(hj.items) ? hj.items : []);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, [templateId]);

  useEffect(() => {
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const generate = async () => {
    if (!active) return;
    setBusy(true);
    setError(null);
    setMsg(null);
    try {
      const res = await fetch("/api/document-templates/generate", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          templateId: active.id,
          format,
        }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Génération impossible");
      const fmtLabel = formatBadge(j.document?.format || format);
      setMsg(`Modèle prêt (${fmtLabel}) : ${j.document?.title || "fichier"}`);
      if (j.document?.downloadUrl) {
        window.open(j.document.downloadUrl, "_blank", "noopener,noreferrer");
      }
      const hRes = await fetch("/api/document-templates/generated", { cache: "no-store" });
      const hj = await hRes.json();
      if (hRes.ok) setHistory(Array.isArray(hj.items) ? hj.items : []);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  if (loading) {
    return <p className="text-sm text-slate-500">Chargement des modèles…</p>;
  }

  return (
    <div className="space-y-6">
      <div className="rounded-2xl border border-sky-200 bg-sky-50/50 p-5">
        <h2 className="text-lg font-black text-sky-950">Documents familles</h2>
        <p className="mt-1 text-sm text-sky-900/80">
          Modèles vierges brandés {branding.name ? `(${branding.name})` : ""} — PDF à trous ou Word
          avec marqueurs <code className="rounded bg-sky-100 px-1">$$…$$</code> pour publipostage
          (Charlemagne, Pronote, Word).
        </p>
      </div>

      {error ? (
        <p className="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-800">
          {error}
        </p>
      ) : null}
      {msg ? (
        <p className="rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-800">
          {msg}
        </p>
      ) : null}

      {placeholders.length > 0 ? (
        <div className="rounded-2xl border border-slate-200 bg-white p-4">
          <p className="text-xs font-bold uppercase text-slate-500">Placeholders Word</p>
          <p className="mt-1 text-sm text-slate-600">
            Dans le fichier Word, remplacez chaque marqueur par le champ correspondant de votre
            logiciel (rechercher / remplacer).
          </p>
          <ul className="mt-3 grid gap-1.5 sm:grid-cols-2">
            {placeholders.map((ph) => (
              <li key={ph.token} className="text-sm text-slate-700">
                <code className="rounded bg-slate-100 px-1.5 py-0.5 font-mono text-xs text-sky-800">
                  {ph.token}
                </code>
                <span className="ml-2 text-slate-500">{ph.label}</span>
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      <div className="grid gap-3 sm:grid-cols-2">
        {templates.map((t) => (
          <button
            key={t.id}
            type="button"
            onClick={() => setTemplateId(t.id)}
            className={`rounded-2xl border p-4 text-left transition ${
              templateId === t.id
                ? "border-sky-500 bg-sky-50 ring-2 ring-sky-200"
                : "border-slate-200 bg-white hover:border-slate-300"
            }`}
          >
            <p className="font-bold text-slate-900">{t.label}</p>
            <p className="mt-1 text-xs text-slate-500">{t.description}</p>
          </button>
        ))}
      </div>

      {active ? (
        <div className="rounded-2xl border border-slate-200 bg-white p-5 space-y-4">
          <div>
            <p className="text-xs font-bold uppercase text-slate-500">Format de sortie</p>
            <div className="mt-2 grid gap-2 sm:grid-cols-2">
              {FORMAT_OPTIONS.map((opt) => (
                <button
                  key={opt.id}
                  type="button"
                  onClick={() => setFormat(opt.id)}
                  className={`rounded-xl border px-3 py-2.5 text-left transition ${
                    format === opt.id
                      ? "border-sky-500 bg-sky-50 ring-2 ring-sky-200"
                      : "border-slate-200 bg-white hover:border-slate-300"
                  }`}
                >
                  <p className="text-sm font-bold text-slate-900">{opt.label}</p>
                  <p className="text-[11px] text-slate-500">{opt.hint}</p>
                </button>
              ))}
            </div>
          </div>

          {format === "docx" && active.fields.length > 0 ? (
            <div className="rounded-xl bg-slate-50 px-3 py-2 text-xs text-slate-600">
              Ce modèle Word contient notamment :{" "}
              {active.fields
                .filter((f) => f.type !== "checkbox")
                .slice(0, 8)
                .map((f) => `$$${f.key}$$`)
                .join(", ")}
              {active.fields.length > 8 ? "…" : ""}.
            </div>
          ) : null}

          <button
            type="button"
            disabled={busy}
            onClick={() => void generate()}
            className="rounded-xl bg-sky-700 px-5 py-2.5 text-sm font-bold text-white disabled:opacity-50"
          >
            {busy ? "Génération…" : `Télécharger (${formatBadge(format)})`}
          </button>
        </div>
      ) : null}

      <div className="rounded-2xl border border-slate-100 bg-slate-50/80 p-5">
        <h3 className="text-sm font-black text-slate-800">Historique récent</h3>
        {history.length === 0 ? (
          <p className="mt-2 text-sm text-slate-500">Aucun modèle généré pour l’instant.</p>
        ) : (
          <ul className="mt-3 space-y-2">
            {history.slice(0, 20).map((h) => (
              <li
                key={h.id}
                className="flex flex-wrap items-center justify-between gap-2 rounded-xl bg-white px-3 py-2 text-sm ring-1 ring-slate-100"
              >
                <div>
                  <p className="font-semibold text-slate-800">{h.title}</p>
                  <p className="text-xs text-slate-500">
                    {h.templateLabel} · {formatBadge(h.format)} ·{" "}
                    {new Date(h.createdAt).toLocaleString("fr-FR")}
                  </p>
                </div>
                <a
                  href={h.downloadUrl}
                  className="text-xs font-bold text-sky-700 underline"
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  Télécharger
                </a>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
