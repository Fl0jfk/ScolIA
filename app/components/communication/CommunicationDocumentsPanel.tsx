"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import type {
  DocumentFieldDef,
  DocumentOutputFormat,
  DocumentTemplateMeta,
} from "@/app/lib/document-templates/types";

type GeneratedRow = {
  id: string;
  templateId: string;
  templateLabel: string;
  title: string;
  createdAt: string;
  downloadUrl: string;
  format?: DocumentOutputFormat;
};

type EleveOpt = { ine: string; nom: string; prenom: string; classe?: string };

const FORMAT_OPTIONS: { id: DocumentOutputFormat; label: string; hint: string }[] = [
  { id: "pdf", label: "PDF rempli", hint: "Prêt à envoyer / archiver" },
  { id: "docx", label: "Word (.docx)", hint: "Retouche secrétariat" },
  { id: "fillable-pdf", label: "PDF à trous", hint: "Dépôt ÉcoleDirecte" },
];

function formatBadge(format?: DocumentOutputFormat) {
  if (format === "docx") return "DOCX";
  if (format === "fillable-pdf") return "PDF à trous";
  return "PDF";
}

export default function CommunicationDocumentsPanel() {
  const [templates, setTemplates] = useState<DocumentTemplateMeta[]>([]);
  const [defaults, setDefaults] = useState<Record<string, string>>({});
  const [branding, setBranding] = useState<{ name?: string }>({});
  const [templateId, setTemplateId] = useState<string>("");
  const [values, setValues] = useState<Record<string, string | boolean>>({});
  const [format, setFormat] = useState<DocumentOutputFormat>("pdf");
  const [eleves, setEleves] = useState<EleveOpt[]>([]);
  const [eleveIne, setEleveIne] = useState("");
  const [eleveQ, setEleveQ] = useState("");
  const [history, setHistory] = useState<GeneratedRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [aiBusyKey, setAiBusyKey] = useState<string | null>(null);
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
      const [tRes, hRes, eRes] = await Promise.all([
        fetch("/api/document-templates", { cache: "no-store" }),
        fetch("/api/document-templates/generated", { cache: "no-store" }),
        fetch("/api/eleves", { cache: "no-store" }),
      ]);
      const tj = await tRes.json();
      if (!tRes.ok) throw new Error(tj.error || "Catalogue indisponible");
      setTemplates(Array.isArray(tj.templates) ? tj.templates : []);
      setDefaults(tj.defaults || {});
      setBranding(tj.branding || {});
      if (!templateId && tj.templates?.[0]?.id) setTemplateId(tj.templates[0].id);

      const hj = await hRes.json();
      if (hRes.ok) setHistory(Array.isArray(hj.items) ? hj.items : []);

      const ej = await eRes.json();
      if (eRes.ok && Array.isArray(ej.eleves)) {
        setEleves(
          ej.eleves.map((e: EleveOpt) => ({
            ine: e.ine,
            nom: e.nom,
            prenom: e.prenom,
            classe: e.classe,
          })),
        );
      }
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

  useEffect(() => {
    if (!active) return;
    setValues((prev) => {
      const next: Record<string, string | boolean> = {};
      for (const f of active.fields) {
        if (f.key in prev) next[f.key] = prev[f.key]!;
        else if (f.key === "anneeScolaire" && defaults.anneeScolaire) next[f.key] = defaults.anneeScolaire;
        else if (f.key === "dateDocument" && defaults.dateDocument) next[f.key] = defaults.dateDocument;
        else if (f.key === "dateDebut" && defaults.dateDocument) next[f.key] = defaults.dateDocument;
        else if (f.key === "dateFin" && defaults.dateDocument) next[f.key] = defaults.dateDocument;
        else if (f.key === "ville" && defaults.ville) next[f.key] = defaults.ville;
        else if (f.type === "checkbox") next[f.key] = false;
        else next[f.key] = "";
      }
      return next;
    });
  }, [active, defaults]);

  const filteredEleves = useMemo(() => {
    const q = eleveQ.trim().toLowerCase();
    if (!q) return eleves.slice(0, 40);
    return eleves
      .filter(
        (e) =>
          e.nom.toLowerCase().includes(q) ||
          e.prenom.toLowerCase().includes(q) ||
          (e.classe || "").toLowerCase().includes(q) ||
          e.ine.toLowerCase().includes(q),
      )
      .slice(0, 40);
  }, [eleves, eleveQ]);

  const pickEleve = (e: EleveOpt) => {
    setEleveIne(e.ine);
    if (!active) return;
    setValues((prev) => {
      const next = { ...prev };
      for (const f of active.fields) {
        if (f.fromEleve === "nom") next[f.key] = e.nom;
        if (f.fromEleve === "prenom") next[f.key] = e.prenom;
        if (f.fromEleve === "classe") next[f.key] = e.classe || "";
      }
      return next;
    });
  };

  const setField = (f: DocumentFieldDef, raw: string | boolean) => {
    setValues((v) => ({ ...v, [f.key]: raw }));
  };

  const reformulate = async (f: DocumentFieldDef) => {
    const text = String(values[f.key] || "").trim();
    if (!text || !active) return;
    setAiBusyKey(f.key);
    setError(null);
    try {
      const res = await fetch("/api/document-templates/reformulate", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          text,
          templateId: active.id,
          fieldKey: f.key,
        }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Reformulation impossible");
      setField(f, String(j.text || text));
      setMsg("Texte reformulé par l’assistant IA.");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur IA");
    } finally {
      setAiBusyKey(null);
    }
  };

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
          values,
          eleveIne: eleveIne || undefined,
          format,
        }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Génération impossible");
      const fmtLabel = formatBadge(j.document?.format || format);
      setMsg(`Document prêt (${fmtLabel}) : ${j.document?.title || "fichier"}`);
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
          PDF, Word et PDF à trous brandés {branding.name ? `(${branding.name})` : ""} — parcours
          zéro papier (préinscription, secrétariat, familles, dépôt ED).
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
            <label className="block text-xs font-bold uppercase text-slate-500">
              Préremplir depuis un élève (optionnel)
            </label>
            <input
              className="mt-1 w-full max-w-md rounded-xl border border-slate-200 px-3 py-2 text-sm"
              placeholder="Rechercher nom, prénom, classe…"
              value={eleveQ}
              onChange={(e) => setEleveQ(e.target.value)}
            />
            {eleveQ.trim() || eleveIne ? (
              <ul className="mt-2 max-h-36 overflow-y-auto rounded-xl border border-slate-100 divide-y">
                {filteredEleves.map((e) => (
                  <li key={e.ine}>
                    <button
                      type="button"
                      className={`w-full px-3 py-2 text-left text-sm hover:bg-slate-50 ${
                        eleveIne === e.ine ? "bg-sky-50 font-semibold" : ""
                      }`}
                      onClick={() => pickEleve(e)}
                    >
                      {e.nom} {e.prenom}
                      <span className="ml-2 text-xs text-slate-400">{e.classe}</span>
                    </button>
                  </li>
                ))}
              </ul>
            ) : null}
          </div>

          <div className="grid gap-3 sm:grid-cols-2">
            {active.fields.map((f) => (
              <label
                key={f.key}
                className={`block text-xs font-bold uppercase text-slate-500 ${
                  f.type === "textarea" ? "sm:col-span-2" : ""
                }`}
              >
                <span className="flex items-center justify-between gap-2">
                  <span>{f.label}</span>
                  {f.aiAssist && (f.type === "textarea" || f.type === "text") ? (
                    <button
                      type="button"
                      disabled={aiBusyKey === f.key || !String(values[f.key] || "").trim()}
                      onClick={(e) => {
                        e.preventDefault();
                        void reformulate(f);
                      }}
                      className="rounded-lg bg-violet-50 px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide text-violet-700 ring-1 ring-violet-200 disabled:opacity-40"
                    >
                      {aiBusyKey === f.key ? "IA…" : "Reformuler"}
                    </button>
                  ) : null}
                </span>
                {f.type === "checkbox" ? (
                  <span className="mt-2 flex items-center gap-2 normal-case font-semibold text-slate-800">
                    <input
                      type="checkbox"
                      checked={Boolean(values[f.key])}
                      onChange={(e) => setField(f, e.target.checked)}
                      className="h-4 w-4"
                    />
                    Oui
                  </span>
                ) : f.type === "textarea" ? (
                  <textarea
                    className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal normal-case text-slate-800"
                    rows={f.key === "corps" ? 8 : 3}
                    value={String(values[f.key] || "")}
                    placeholder={f.placeholder}
                    onChange={(e) => setField(f, e.target.value)}
                  />
                ) : (
                  <input
                    type={f.type === "date" ? "date" : "text"}
                    className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal normal-case text-slate-800"
                    value={String(values[f.key] || "")}
                    placeholder={f.placeholder}
                    onChange={(e) => setField(f, e.target.value)}
                  />
                )}
              </label>
            ))}
          </div>

          <div>
            <p className="text-xs font-bold uppercase text-slate-500">Format de sortie</p>
            <div className="mt-2 grid gap-2 sm:grid-cols-3">
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

          <button
            type="button"
            disabled={busy}
            onClick={() => void generate()}
            className="rounded-xl bg-sky-700 px-5 py-2.5 text-sm font-bold text-white disabled:opacity-50"
          >
            {busy ? "Génération…" : `Générer (${formatBadge(format)})`}
          </button>
        </div>
      ) : null}

      <div className="rounded-2xl border border-slate-100 bg-slate-50/80 p-5">
        <h3 className="text-sm font-black text-slate-800">Historique récent</h3>
        {history.length === 0 ? (
          <p className="mt-2 text-sm text-slate-500">Aucun document généré pour l’instant.</p>
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
