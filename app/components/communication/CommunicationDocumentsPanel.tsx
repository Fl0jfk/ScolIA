"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
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
  codeGenerated?: boolean;
};

type SixiemeConfig = {
  schoolYear: string;
  title?: string;
  subtitle?: string;
  options: { id: string; label: string }[];
};

type BrandingInfo = {
  name?: string;
  logoUrl?: string;
  addressLine?: string;
};

const FORMAT_OPTIONS: { id: DocumentOutputFormat; label: string; hint: string }[] = [
  { id: "fillable-pdf", label: "PDF à trous", hint: "Vierge, remplissable (Adobe / lecteurs PDF)" },
  { id: "docx", label: "Word (.docx)", hint: "Placeholders $$…$$ pour publipostage" },
];

function formatBadge(format?: DocumentOutputFormat | "pdf") {
  if (format === "docx") return "DOCX";
  if (format === "fillable-pdf") return "PDF à trous";
  return "PDF";
}

function formatsForTemplate(t: DocumentTemplateMeta | null): DocumentOutputFormat[] {
  if (!t) return ["fillable-pdf"];
  if (Array.isArray(t.formats) && t.formats.length > 0) return t.formats;
  return ["fillable-pdf", "docx"];
}

export default function CommunicationDocumentsPanel() {
  const [templates, setTemplates] = useState<DocumentTemplateMeta[]>([]);
  const [placeholders, setPlaceholders] = useState<DocumentPlaceholderDef[]>([]);
  const [branding, setBranding] = useState<BrandingInfo>({});
  const [templateId, setTemplateId] = useState<string>("");
  const [modalOpen, setModalOpen] = useState(false);
  const [format, setFormat] = useState<DocumentOutputFormat>("fillable-pdf");
  const [history, setHistory] = useState<GeneratedRow[]>([]);
  const [levels, setLevels] = useState<InscriptionLevelRow[]>([]);
  const [levelId, setLevelId] = useState<InscriptionLevelId | "">("");
  const [establishmentName, setEstablishmentName] = useState("");
  const [accentColor, setAccentColor] = useState("#1E4A32");
  const [defaultName, setDefaultName] = useState("");
  const [sixieme, setSixieme] = useState<SixiemeConfig>({
    schoolYear: "",
    title: "Fiche d'inscription — Sixième",
    subtitle: "",
    options: [],
  });
  const [newOptionLabel, setNewOptionLabel] = useState("");
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [settingsBusy, setSettingsBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const levelConfigRef = useRef<HTMLDivElement | null>(null);

  const active = useMemo(
    () => templates.find((t) => t.id === templateId) || null,
    [templates, templateId],
  );
  const isInscription = templateId === "fiche-inscription";
  const isSixiemeCode = isInscription && levelId === "sixieme";
  const availableFormats = useMemo(() => formatsForTemplate(active), [active]);

  const collegeLevels = useMemo(
    () => levels.filter((l) => l.cycle === "college"),
    [levels],
  );
  const lyceeLevels = useMemo(
    () => levels.filter((l) => l.cycle === "lycee"),
    [levels],
  );

  const openTemplate = (t: DocumentTemplateMeta) => {
    setError(null);
    setMsg(null);
    setTemplateId(t.id);
    const fmts = formatsForTemplate(t);
    setFormat(fmts[0] || "fillable-pdf");
    if (t.id === "fiche-inscription" && !levelId && levels[0]?.id) {
      setLevelId(levels[0].id);
    }
    setModalOpen(true);
  };

  const closeModal = () => {
    setModalOpen(false);
    setError(null);
  };

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
      const nextLevels: InscriptionLevelRow[] = Array.isArray(tj.inscriptionLevels)
        ? tj.inscriptionLevels
        : [];
      setLevels(nextLevels);
      if (tj.inscriptionSettings) {
        setEstablishmentName(tj.inscriptionSettings.establishmentName || "");
        setAccentColor(tj.inscriptionSettings.accentColor || "#1E4A32");
        if (tj.inscriptionSettings.sixieme) {
          setSixieme({
            schoolYear: tj.inscriptionSettings.sixieme.schoolYear || "",
            title: tj.inscriptionSettings.sixieme.title || "",
            subtitle: tj.inscriptionSettings.sixieme.subtitle || "",
            options: Array.isArray(tj.inscriptionSettings.sixieme.options)
              ? tj.inscriptionSettings.sixieme.options
              : [],
          });
        }
      }
      setLevelId((prev) => prev || nextLevels[0]?.id || "");

      const hj = await hRes.json();
      if (hRes.ok) setHistory(Array.isArray(hj.items) ? hj.items : []);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const saveSettings = async () => {
    setSettingsBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/document-templates/inscription", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          establishmentName,
          accentColor,
          ...(isSixiemeCode
            ? {
                sixieme: {
                  schoolYear: sixieme.schoolYear,
                  title: sixieme.title,
                  subtitle: sixieme.subtitle,
                  options: sixieme.options,
                },
              }
            : {}),
        }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Enregistrement impossible");
      if (j.settings?.sixieme) setSixieme(j.settings.sixieme);
      setMsg("Réglages inscription enregistrés.");
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
      setError("Choisissez un niveau dans la liste ci-dessus.");
      return;
    }
    setBusy(true);
    setError(null);
    setMsg(null);
    try {
      const outFormat = isInscription ? "fillable-pdf" : format;
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
                ...(levelId === "sixieme"
                  ? {
                      sixieme: {
                        schoolYear: sixieme.schoolYear,
                        title: sixieme.title,
                        subtitle: sixieme.subtitle,
                        options: sixieme.options,
                      },
                    }
                  : {}),
              }
            : {
                templateId: active.id,
                format: outFormat,
              },
        ),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Génération impossible");
      const fmtLabel = formatBadge(j.document?.format || outFormat);
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
  const displayName = establishmentName.trim() || defaultName || branding.name || "Établissement";

  useEffect(() => {
    if (!levelId || !modalOpen) return;
    const t = window.setTimeout(() => {
      levelConfigRef.current?.scrollIntoView({ behavior: "smooth", block: "nearest" });
    }, 50);
    return () => window.clearTimeout(t);
  }, [levelId, modalOpen]);

  if (loading) {
    return <p className="text-sm text-slate-500">Chargement des modèles…</p>;
  }

  const renderInlineLevelConfig = (level: InscriptionLevelRow) => {
    if (level.id !== levelId) return null;
    if (level.id === "sixieme") {
      return (
        <div
          ref={levelConfigRef}
          className="col-span-full space-y-3 rounded-xl border border-emerald-200 bg-emerald-50/50 p-4 shadow-sm"
        >
          <p className="text-xs font-bold uppercase text-emerald-900">
            Modifier la fiche Sixième
          </p>
          <p className="text-xs text-emerald-900/80">
            Régime (Internat / Demi-pension / Externat) fixe. Options d’enseignements
            éditables ci-dessous — placement automatique en 2 colonnes sur le PDF.
          </p>
          <label className="block text-sm">
            Année scolaire
            <input
              value={sixieme.schoolYear}
              onChange={(e) => setSixieme((s) => ({ ...s, schoolYear: e.target.value }))}
              placeholder="2026-2027"
              className="mt-1 w-full rounded-xl border border-slate-300 px-3 py-2"
            />
          </label>
          <label className="block text-sm">
            Titre
            <input
              value={sixieme.title || ""}
              onChange={(e) => setSixieme((s) => ({ ...s, title: e.target.value }))}
              className="mt-1 w-full rounded-xl border border-slate-300 px-3 py-2"
            />
          </label>
          <label className="block text-sm">
            Sous-titre
            <input
              value={sixieme.subtitle || ""}
              onChange={(e) => setSixieme((s) => ({ ...s, subtitle: e.target.value }))}
              className="mt-1 w-full rounded-xl border border-slate-300 px-3 py-2"
            />
          </label>
          <div>
            <p className="text-sm font-semibold text-slate-800">
              Enseignements / options (cases à cocher)
            </p>
            <p className="text-xs text-slate-500">
              Classique, Bilangue, Théâtre, Foot… Ajoutez ou retirez librement.
            </p>
            <ul className="mt-2 space-y-2">
              {sixieme.options.map((opt, idx) => (
                <li
                  key={`${opt.id}-${idx}`}
                  className="flex items-center gap-2 rounded-lg border border-slate-200 bg-white px-2 py-1.5"
                >
                  <input
                    value={opt.label}
                    onChange={(e) => {
                      const label = e.target.value;
                      setSixieme((s) => ({
                        ...s,
                        options: s.options.map((o, i) => (i === idx ? { ...o, label } : o)),
                      }));
                    }}
                    className="min-w-0 flex-1 rounded border border-slate-200 px-2 py-1 text-sm"
                  />
                  <button
                    type="button"
                    className="text-xs font-bold text-rose-600"
                    onClick={() =>
                      setSixieme((s) => ({
                        ...s,
                        options: s.options.filter((_, i) => i !== idx),
                      }))
                    }
                  >
                    Retirer
                  </button>
                </li>
              ))}
            </ul>
            <div className="mt-2 flex gap-2">
              <input
                value={newOptionLabel}
                onChange={(e) => setNewOptionLabel(e.target.value)}
                placeholder="Nouvelle option"
                className="min-w-0 flex-1 rounded-xl border border-slate-300 px-3 py-2 text-sm"
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    const label = newOptionLabel.trim();
                    if (!label) return;
                    setSixieme((s) => ({
                      ...s,
                      options: [...s.options, { id: `opt-${Date.now()}`, label }],
                    }));
                    setNewOptionLabel("");
                  }
                }}
              />
              <button
                type="button"
                className="rounded-xl bg-slate-800 px-3 py-2 text-xs font-bold text-white"
                onClick={() => {
                  const label = newOptionLabel.trim();
                  if (!label) return;
                  setSixieme((s) => ({
                    ...s,
                    options: [...s.options, { id: `opt-${Date.now()}`, label }],
                  }));
                  setNewOptionLabel("");
                }}
              >
                Ajouter
              </button>
            </div>
          </div>
        </div>
      );
    }
    return (
      <div
        ref={levelConfigRef}
        className="col-span-full space-y-2 rounded-xl border border-amber-100 bg-amber-50/50 p-4"
      >
        <p className="text-xs font-bold uppercase text-amber-900">
          PDF source — {level.label}
        </p>
        <p className="text-xs text-amber-900/80">
          Ce niveau utilise encore le PDF AcroForm d’origine. La reconstruction en code
          arrivera ensuite, comme pour la Sixième.
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
        {level.hasOverride ? (
          <button
            type="button"
            disabled={settingsBusy}
            onClick={() => void clearOverride()}
            className="text-xs font-bold text-rose-700 underline disabled:opacity-50"
          >
            Revenir au modèle standard
          </button>
        ) : (
          <p className="text-[11px] text-slate-500">Modèle standard actif.</p>
        )}
      </div>
    );
  };

  const renderLevelGrid = (items: InscriptionLevelRow[], title: string) => (
    <div>
      <p className="text-xs font-bold uppercase tracking-wide text-slate-500">{title}</p>
      <div className="mt-2 grid gap-2 sm:grid-cols-2">
        {items.map((l) => (
          <div key={l.id} className="contents">
            <button
              type="button"
              onClick={() => {
                setLevelId(l.id);
                setError(null);
              }}
              className={`rounded-xl border px-3 py-2.5 text-left text-sm transition ${
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
              ) : l.codeGenerated ? (
                <span className="mt-0.5 block text-[10px] font-bold text-emerald-700">
                  Générée en code
                </span>
              ) : (
                <span className="mt-0.5 block text-[10px] text-slate-400">Modèle standard</span>
              )}
            </button>
            {renderInlineLevelConfig(l)}
          </div>
        ))}
      </div>
    </div>
  );

  return (
    <div className="space-y-6">
      <div className="rounded-2xl border border-sky-200 bg-sky-50/50 p-5">
        <h2 className="text-lg font-black text-sky-950">Documents familles</h2>
        <p className="mt-1 text-sm text-sky-900/80">
          Cliquez sur un modèle pour ouvrir la fenêtre de configuration et de téléchargement.
          Les formats proposés dépendent du document (PDF, Word, ou les deux).
        </p>
      </div>

      {error && !modalOpen ? (
        <p className="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-800">
          {error}
        </p>
      ) : null}
      {msg && !modalOpen ? (
        <p className="rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-800">
          {msg}
        </p>
      ) : null}

      <div className="grid gap-3 sm:grid-cols-2">
        {templates.map((t) => {
          const fmts = formatsForTemplate(t);
          return (
            <button
              key={t.id}
              type="button"
              onClick={() => openTemplate(t)}
              className="rounded-2xl border border-slate-200 bg-white p-4 text-left transition hover:border-sky-300 hover:shadow-md"
            >
              <p className="font-bold text-slate-900">{t.label}</p>
              <p className="mt-1 text-xs text-slate-500">{t.description}</p>
              <div className="mt-3 flex flex-wrap gap-1.5">
                {fmts.map((f) => (
                  <span
                    key={f}
                    className="rounded-full bg-slate-100 px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide text-slate-600"
                  >
                    {formatBadge(f)}
                  </span>
                ))}
              </div>
            </button>
          );
        })}
      </div>

      {modalOpen && active ? (
        <div
          className="fixed inset-0 z-50 flex items-end justify-center bg-slate-900/45 p-3 sm:items-center sm:p-6"
          role="dialog"
          aria-modal="true"
          aria-labelledby="doc-template-modal-title"
          onClick={(e) => {
            if (e.target === e.currentTarget) closeModal();
          }}
        >
          <div className="flex max-h-[92vh] w-full max-w-2xl flex-col overflow-hidden rounded-2xl border border-slate-200 bg-white shadow-2xl">
            <div className="flex items-start justify-between gap-3 border-b border-slate-100 px-5 py-4">
              <div>
                <p className="text-[10px] font-bold uppercase tracking-[0.2em] text-sky-700">
                  Documents familles
                </p>
                <h3 id="doc-template-modal-title" className="mt-1 text-lg font-black text-slate-900">
                  {active.label}
                </h3>
                <p className="mt-1 text-xs text-slate-500">{active.description}</p>
              </div>
              <button
                type="button"
                onClick={closeModal}
                className="rounded-lg px-2 py-1 text-sm font-bold text-slate-500 hover:bg-slate-100"
                aria-label="Fermer"
              >
                ✕
              </button>
            </div>

            <div className="flex-1 space-y-5 overflow-y-auto px-5 py-4">
              {error ? (
                <p className="rounded-xl border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-800">
                  {error}
                </p>
              ) : null}
              {msg ? (
                <p className="rounded-xl border border-emerald-200 bg-emerald-50 px-3 py-2 text-sm text-emerald-800">
                  {msg}
                </p>
              ) : null}

              {isInscription ? (
                <>
                  <div className="rounded-xl border border-slate-200 bg-slate-50/80 p-4">
                    <p className="text-xs font-bold uppercase tracking-wide text-slate-500">
                      Personnalisation
                    </p>
                    <div className="mt-3 flex flex-wrap items-center gap-4">
                      <div className="flex h-16 w-16 items-center justify-center overflow-hidden rounded-xl border border-slate-200 bg-white">
                        {branding.logoUrl ? (
                          // eslint-disable-next-line @next/next/no-img-element
                          <img
                            src={branding.logoUrl}
                            alt="Logo établissement"
                            className="max-h-full max-w-full object-contain p-1"
                          />
                        ) : (
                          <span className="px-1 text-center text-[10px] text-slate-400">
                            Pas de logo
                          </span>
                        )}
                      </div>
                      <div className="min-w-0 flex-1">
                        <p className="text-sm font-semibold text-slate-900">{displayName}</p>
                        <p className="text-xs text-slate-500">
                          Logo issu des paramètres (identité / tenant).{" "}
                          <a href="/parametres" className="font-semibold text-sky-700 underline">
                            Modifier
                          </a>
                        </p>
                      </div>
                    </div>

                    <label className="mt-4 block text-sm">
                      Nom affiché sur la fiche
                      <input
                        value={establishmentName}
                        onChange={(e) => setEstablishmentName(e.target.value)}
                        placeholder={defaultName || "Nom de l’établissement"}
                        className="mt-1 w-full rounded-xl border border-slate-300 px-3 py-2"
                      />
                    </label>
                    <label className="mt-3 inline-flex items-center gap-3 text-sm">
                      Couleur de la fiche
                      <input
                        type="color"
                        value={accentColor}
                        onChange={(e) => setAccentColor(e.target.value)}
                        className="h-10 w-14 cursor-pointer rounded border"
                      />
                      <span className="font-mono text-xs text-slate-500">{accentColor}</span>
                    </label>
                    <p className="mt-1 text-[11px] text-slate-500">
                      Appliquée au fond d’en-tête, aux encadrés, bordures et lignes (comme sur
                      votre fiche d’origine).
                    </p>
                    <button
                      type="button"
                      disabled={settingsBusy}
                      onClick={() => void saveSettings()}
                      className="mt-3 rounded-xl border border-slate-300 bg-white px-4 py-2 text-sm font-bold text-slate-800 disabled:opacity-50"
                    >
                      {settingsBusy ? "…" : "Enregistrer nom & couleur"}
                    </button>
                  </div>

                  <div>
                    <p className="text-sm font-bold text-slate-900">Fiches disponibles</p>
                    <p className="mt-0.5 text-xs text-slate-500">
                      Cliquez un niveau : les options s’ouvrent juste en dessous.
                    </p>
                    <div className="mt-3 space-y-4">
                      {levels.length === 0 ? (
                        <p className="rounded-xl border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-900">
                          Aucun niveau chargé. Rechargez la page ou contactez le support.
                        </p>
                      ) : (
                        <>
                          {renderLevelGrid(collegeLevels, "Collège")}
                          {renderLevelGrid(lyceeLevels, "Lycée")}
                        </>
                      )}
                    </div>
                  </div>

                  <div className="rounded-xl bg-slate-50 px-3 py-2 text-xs text-slate-600">
                    Format disponible : <strong>PDF à trous</strong> uniquement (pas de Word pour
                    les fiches d’inscription).
                  </div>
                </>
              ) : (
                <>
                  <div>
                    <p className="text-xs font-bold uppercase tracking-wide text-slate-500">
                      Format de sortie
                    </p>
                    <div className="mt-2 grid gap-2 sm:grid-cols-2">
                      {FORMAT_OPTIONS.filter((opt) => availableFormats.includes(opt.id)).map(
                        (opt) => (
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
                        ),
                      )}
                    </div>
                    {availableFormats.length === 1 ? (
                      <p className="mt-2 text-xs text-slate-500">
                        Ce modèle n’est proposé qu’en {formatBadge(availableFormats[0])}.
                      </p>
                    ) : null}
                  </div>

                  {format === "docx" && placeholders.length > 0 ? (
                    <div className="rounded-xl border border-slate-200 bg-slate-50 p-3">
                      <p className="text-xs font-bold text-slate-600">Placeholders Word</p>
                      <ul className="mt-2 grid gap-1 sm:grid-cols-2">
                        {placeholders.slice(0, 8).map((ph) => (
                          <li key={ph.token} className="text-xs text-slate-600">
                            <code className="rounded bg-white px-1 font-mono text-sky-800">
                              {ph.token}
                            </code>{" "}
                            {ph.label}
                          </li>
                        ))}
                      </ul>
                    </div>
                  ) : null}
                </>
              )}
            </div>

            <div className="flex flex-wrap items-center justify-end gap-2 border-t border-slate-100 px-5 py-4">
              <button
                type="button"
                onClick={closeModal}
                className="rounded-xl border border-slate-200 px-4 py-2.5 text-sm font-semibold text-slate-700"
              >
                Fermer
              </button>
              <button
                type="button"
                disabled={busy || (isInscription && !levelId)}
                onClick={() => void generate()}
                className="rounded-xl bg-sky-700 px-5 py-2.5 text-sm font-bold text-white disabled:opacity-50"
              >
                {busy
                  ? "Génération…"
                  : isInscription
                    ? `Télécharger ${activeLevel?.label || "la fiche"} (PDF)`
                    : `Télécharger (${formatBadge(format)})`}
              </button>
            </div>
          </div>
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
