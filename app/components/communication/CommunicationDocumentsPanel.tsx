"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import type {
  DocumentOutputFormat,
  DocumentPlaceholderDef,
  DocumentTemplateMeta,
  InscriptionLevelId,
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

type InscriptionLevelRow = {
  id: InscriptionLevelId;
  label: string;
  cycle: "college" | "lycee";
  hasOverride?: boolean;
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
  const [levels, setLevels] = useState<InscriptionLevelRow[]>([]);
  const [levelId, setLevelId] = useState<InscriptionLevelId | "">("");
  const [establishmentName, setEstablishmentName] = useState("");
  const [accentColor, setAccentColor] = useState("#0f172a");
  const [defaultName, setDefaultName] = useState("");
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [settingsBusy, setSettingsBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);

  const active = useMemo(
    () => templates.find((t) => t.id === templateId) || null,
    [templates, templateId],
  );
  const isInscription = templateId === "fiche-inscription";

  const collegeLevels = useMemo(
    () => levels.filter((l) => l.cycle === "college"),
    [levels],
  );
  const lyceeLevels = useMemo(
    () => levels.filter((l) => l.cycle === "lycee"),
    [levels],
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
      setDefaultName(tj.branding?.name || "");
      if (Array.isArray(tj.inscriptionLevels)) setLevels(tj.inscriptionLevels);
      if (tj.inscriptionSettings) {
        setEstablishmentName(tj.inscriptionSettings.establishmentName || "");
        setAccentColor(tj.inscriptionSettings.accentColor || "#0f172a");
      }
      if (!templateId && tj.templates?.[0]?.id) setTemplateId(tj.templates[0].id);
      if (!levelId && tj.inscriptionLevels?.[0]?.id) {
        setLevelId(tj.inscriptionLevels[0].id);
      }

      const hj = await hRes.json();
      if (hRes.ok) setHistory(Array.isArray(hj.items) ? hj.items : []);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, [templateId, levelId]);

  useEffect(() => {
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const saveSettings = async () => {
    setSettingsBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/document-templates/inscription", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ establishmentName, accentColor }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Enregistrement impossible");
      setMsg("Réglages inscription enregistrés pour l’établissement.");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSettingsBusy(false);
    }
  };

  const uploadOverride = async (file: File | null) => {
    if (!file || !levelId) return;
    setSettingsBusy(true);
    setError(null);
    try {
      const fd = new FormData();
      fd.append("file", file);
      fd.append("levelId", levelId);
      fd.append("action", "upload");
      const res = await fetch("/api/document-templates/inscription", {
        method: "POST",
        body: fd,
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Upload impossible");
      setLevels((prev) =>
        prev.map((l) => (l.id === levelId ? { ...l, hasOverride: true } : l)),
      );
      setMsg("PDF de remplacement enregistré pour ce niveau.");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSettingsBusy(false);
    }
  };

  const clearOverride = async () => {
    if (!levelId) return;
    setSettingsBusy(true);
    setError(null);
    try {
      const fd = new FormData();
      fd.append("levelId", levelId);
      fd.append("action", "clear");
      const res = await fetch("/api/document-templates/inscription", {
        method: "POST",
        body: fd,
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Suppression impossible");
      setLevels((prev) =>
        prev.map((l) => (l.id === levelId ? { ...l, hasOverride: false } : l)),
      );
      setMsg("Retour au modèle standard pour ce niveau.");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSettingsBusy(false);
    }
  };

  const generate = async () => {
    if (!active) return;
    if (isInscription && !levelId) {
      setError("Choisissez un niveau.");
      return;
    }
    setBusy(true);
    setError(null);
    setMsg(null);
    try {
      const res = await fetch("/api/document-templates/generate", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(
          isInscription
            ? {
                templateId: active.id,
                format: "fillable-pdf",
                inscriptionLevelId: levelId,
                establishmentName: establishmentName.trim() || undefined,
                accentColor,
              }
            : {
                templateId: active.id,
                format,
              },
        ),
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

  const activeLevel = levels.find((l) => l.id === levelId) || null;

  if (loading) {
    return <p className="text-sm text-slate-500">Chargement des modèles…</p>;
  }

  const renderLevelGrid = (items: InscriptionLevelRow[], title: string) => (
    <div>
      <p className="text-xs font-bold uppercase text-slate-500">{title}</p>
      <div className="mt-2 grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
        {items.map((l) => (
          <button
            key={l.id}
            type="button"
            onClick={() => setLevelId(l.id)}
            className={`rounded-xl border px-3 py-2.5 text-left text-sm ${
              levelId === l.id
                ? "border-sky-500 bg-sky-50 ring-2 ring-sky-200"
                : "border-slate-200 bg-white hover:border-slate-300"
            }`}
          >
            <span className="font-semibold text-slate-900">{l.label}</span>
            {l.hasOverride ? (
              <span className="mt-0.5 block text-[10px] font-bold text-amber-700">
                PDF personnalisé
              </span>
            ) : null}
          </button>
        ))}
      </div>
    </div>
  );

  return (
    <div className="space-y-6">
      <div className="rounded-2xl border border-sky-200 bg-sky-50/50 p-5">
        <h2 className="text-lg font-black text-sky-950">Documents familles</h2>
        <p className="mt-1 text-sm text-sky-900/80">
          Modèles vierges brandés {branding.name ? `(${branding.name})` : ""} — fiches
          d’inscription par niveau (PDF remplissable), ou Word avec marqueurs{" "}
          <code className="rounded bg-sky-100 px-1">$$…$$</code>.
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

      {!isInscription && placeholders.length > 0 ? (
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
            onClick={() => {
              setTemplateId(t.id);
              if (t.id === "fiche-inscription") setFormat("fillable-pdf");
            }}
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

      {active && isInscription ? (
        <div className="rounded-2xl border border-slate-200 bg-white p-5 space-y-5">
          {renderLevelGrid(collegeLevels, "Collège")}
          {renderLevelGrid(lyceeLevels, "Lycée")}

          <div className="space-y-3 border-t border-slate-100 pt-4">
            <p className="text-xs font-bold uppercase text-slate-500">
              Personnalisation établissement
            </p>
            <label className="block text-sm">
              Nom affiché sur la fiche
              <input
                value={establishmentName}
                onChange={(e) => setEstablishmentName(e.target.value)}
                placeholder={defaultName || "Nom de l’établissement"}
                className="mt-1 w-full rounded-xl border border-slate-300 px-3 py-2"
              />
              <span className="mt-1 block text-[11px] text-slate-500">
                Vide = nom des paramètres ({defaultName || "identité"}). Utile pour les groupes
                scolaires.
              </span>
            </label>
            <label className="inline-flex items-center gap-3 text-sm">
              Couleur bandeau
              <input
                type="color"
                value={accentColor}
                onChange={(e) => setAccentColor(e.target.value)}
                className="h-10 w-14 cursor-pointer rounded border"
              />
              <span className="font-mono text-xs text-slate-500">{accentColor}</span>
            </label>
            <button
              type="button"
              disabled={settingsBusy}
              onClick={() => void saveSettings()}
              className="rounded-xl border border-slate-300 bg-slate-50 px-4 py-2 text-sm font-bold text-slate-800 disabled:opacity-50"
            >
              {settingsBusy ? "…" : "Enregistrer nom & couleur"}
            </button>
          </div>

          {activeLevel ? (
            <div className="space-y-2 rounded-xl border border-amber-100 bg-amber-50/40 p-4">
              <p className="text-xs font-bold uppercase text-amber-900">
                PDF source — {activeLevel.label}
              </p>
              <p className="text-xs text-amber-900/80">
                Si vos options diffèrent, remplacez le PDF de ce niveau (AcroForm recommandé).
                Sinon le modèle standard est utilisé.
              </p>
              <label className="block text-sm">
                Remplacer le PDF
                <input
                  type="file"
                  accept="application/pdf,.pdf"
                  className="mt-1 block w-full text-xs"
                  onChange={(e) => void uploadOverride(e.target.files?.[0] ?? null)}
                />
              </label>
              {activeLevel.hasOverride ? (
                <button
                  type="button"
                  disabled={settingsBusy}
                  onClick={() => void clearOverride()}
                  className="text-xs font-bold text-rose-700 underline disabled:opacity-50"
                >
                  Revenir au modèle standard
                </button>
              ) : (
                <p className="text-[11px] text-slate-500">Modèle standard (Providence) actif.</p>
              )}
            </div>
          ) : null}

          <button
            type="button"
            disabled={busy || !levelId}
            onClick={() => void generate()}
            className="w-full rounded-xl bg-sky-700 px-5 py-3 text-sm font-bold text-white disabled:opacity-50"
          >
            {busy
              ? "Génération…"
              : `Télécharger la fiche ${activeLevel?.label || ""} (PDF remplissable)`}
          </button>
        </div>
      ) : null}

      {active && !isInscription ? (
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
