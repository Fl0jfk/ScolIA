"use client";

import { useCallback, useEffect, useState } from "react";
import PosterPreview from "@/app/components/communication/PosterPreview";
import { defaultPosterDraft, defaultPosterOffsets } from "@/app/lib/posters/catalog";
import type {
  PosterDraft,
  PosterFormat,
  PosterLayoutPreset,
  PosterTemplateMeta,
} from "@/app/lib/posters/types";

type GeneratedRow = {
  id: string;
  templateId: string;
  templateLabel: string;
  title: string;
  createdAt: string;
  downloadUrl: string;
  formatLabel?: string;
};

type FormatOpt = { id: PosterFormat; label: string; hint: string };

export default function CommunicationPostersPanel() {
  const [templates, setTemplates] = useState<PosterTemplateMeta[]>([]);
  const [formats, setFormats] = useState<FormatOpt[]>([]);
  const [schoolName, setSchoolName] = useState("Établissement");
  const [draft, setDraft] = useState<PosterDraft>(() => defaultPosterDraft());
  const [brief, setBrief] = useState("");
  const [partnerLogoUrl, setPartnerLogoUrl] = useState<string | null>(null);
  const [backgroundImageUrl, setBackgroundImageUrl] = useState<string | null>(null);
  const [history, setHistory] = useState<GeneratedRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [suggestBusy, setSuggestBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const [showAdjust, setShowAdjust] = useState(false);

  const patch = useCallback((partial: Partial<PosterDraft>) => {
    setDraft((d) => ({ ...d, ...partial }));
  }, []);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [tRes, hRes] = await Promise.all([
        fetch("/api/posters", { cache: "no-store" }),
        fetch("/api/posters/generated", { cache: "no-store" }),
      ]);
      const tj = await tRes.json();
      if (!tRes.ok) throw new Error(tj.error || "Catalogue indisponible");
      setTemplates(Array.isArray(tj.templates) ? tj.templates : []);
      setFormats(Array.isArray(tj.formats) ? tj.formats : []);
      if (tj.branding?.name) setSchoolName(tj.branding.name);
      if (tj.defaults) setDraft({ ...defaultPosterDraft(), ...tj.defaults, offsets: defaultPosterOffsets() });

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

  const active = templates.find((t) => t.id === draft.templateId) || templates[0] || null;

  const uploadAsset = async (kind: "partner-logo" | "background", file: File) => {
    setError(null);
    const fd = new FormData();
    fd.append("file", file);
    fd.append("kind", kind);
    const res = await fetch("/api/posters/upload", { method: "POST", body: fd });
    const j = await res.json();
    if (!res.ok) throw new Error(j.error || "Upload impossible");
    return j as { key: string; url: string };
  };

  const onPartnerLogo = async (file: File | null) => {
    if (!file) return;
    try {
      const { key, url } = await uploadAsset("partner-logo", file);
      patch({ partnerLogoKey: key });
      setPartnerLogoUrl(url);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Upload logo");
    }
  };

  const onBackground = async (file: File | null) => {
    if (!file) return;
    try {
      const { key, url } = await uploadAsset("background", file);
      patch({ backgroundImageKey: key, backgroundMode: "image" });
      setBackgroundImageUrl(url);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Upload fond");
    }
  };

  const suggestCopy = async () => {
    setSuggestBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/posters/suggest-copy", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          brief: brief || `${draft.title} ${draft.partnerName}`.trim(),
          templateId: draft.templateId,
          partnerName: draft.partnerName,
        }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Suggestion impossible");
      patch({
        title: j.title || draft.title,
        subtitle: j.subtitle || draft.subtitle,
        body: j.body || draft.body,
      });
      setMsg("Textes proposés — vous pouvez les corriger.");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur IA");
    } finally {
      setSuggestBusy(false);
    }
  };

  const generate = async () => {
    setBusy(true);
    setError(null);
    setMsg(null);
    try {
      const res = await fetch("/api/posters/generate", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ draft }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Génération impossible");
      setMsg(`Affiche prête : ${j.poster?.title || "PDF"}`);
      if (j.poster?.downloadUrl) {
        window.open(j.poster.downloadUrl, "_blank", "noopener,noreferrer");
      }
      const hRes = await fetch("/api/posters/generated", { cache: "no-store" });
      const hj = await hRes.json();
      if (hRes.ok) setHistory(Array.isArray(hj.items) ? hj.items : []);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  if (loading) {
    return <p className="text-sm text-slate-500">Chargement du créateur d’affiches…</p>;
  }

  return (
    <div className="space-y-6">
      <div className="rounded-2xl border border-violet-200 bg-violet-50/50 p-5">
        <h2 className="text-lg font-black text-violet-950">Affiches</h2>
        <p className="mt-1 text-sm text-violet-900/80">
          Modèles graphiques brandés (logo établissement auto) — fond, couleurs, logo partenaire,
          textes. Export PDF A4 / A3.
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

      <div className="grid gap-6 lg:grid-cols-2">
        <div className="space-y-4">
          {active ? (
            <div className="rounded-2xl border border-slate-200 bg-white p-4 space-y-3">
              <p className="text-xs font-bold uppercase text-slate-500">Modèle</p>
              <p className="font-bold text-slate-900">{active.label}</p>
              <p className="text-xs text-slate-500">{active.description}</p>

              <p className="pt-2 text-xs font-bold uppercase text-slate-500">Format</p>
              <div className="grid gap-2 sm:grid-cols-2">
                {formats.map((f) => (
                  <button
                    key={f.id}
                    type="button"
                    onClick={() => patch({ format: f.id })}
                    className={`rounded-xl border px-3 py-2 text-left text-sm ${
                      draft.format === f.id
                        ? "border-violet-500 bg-violet-50 ring-2 ring-violet-200"
                        : "border-slate-200"
                    }`}
                  >
                    <span className="font-bold">{f.label}</span>
                    <span className="mt-0.5 block text-[11px] text-slate-500">{f.hint}</span>
                  </button>
                ))}
              </div>

              <p className="pt-2 text-xs font-bold uppercase text-slate-500">Disposition</p>
              <div className="grid gap-2">
                {active.presets.map((p) => (
                  <button
                    key={p.id}
                    type="button"
                    onClick={() => patch({ layoutPreset: p.id as PosterLayoutPreset })}
                    className={`rounded-xl border px-3 py-2 text-left text-sm ${
                      draft.layoutPreset === p.id
                        ? "border-violet-500 bg-violet-50"
                        : "border-slate-200"
                    }`}
                  >
                    <span className="font-semibold">{p.label}</span>
                    <span className="ml-2 text-xs text-slate-500">{p.hint}</span>
                  </button>
                ))}
              </div>
            </div>
          ) : null}

          <div className="rounded-2xl border border-slate-200 bg-white p-4 space-y-3">
            <p className="text-xs font-bold uppercase text-slate-500">Textes</p>
            <label className="block text-sm">
              <span className="text-slate-600">Brief pour l’IA (optionnel)</span>
              <textarea
                value={brief}
                onChange={(e) => setBrief(e.target.value)}
                rows={2}
                className="mt-1 w-full rounded-xl border border-slate-300 px-3 py-2 text-sm"
                placeholder="Ex. partenariat centre équestre, initiation poney CP-CE1, mars-avril…"
              />
            </label>
            <button
              type="button"
              disabled={suggestBusy}
              onClick={() => void suggestCopy()}
              className="rounded-xl border border-violet-300 bg-violet-50 px-4 py-2 text-sm font-bold text-violet-900 disabled:opacity-50"
            >
              {suggestBusy ? "Proposition…" : "Proposer les textes"}
            </button>
            <label className="block text-sm">
              Titre
              <input
                value={draft.title}
                onChange={(e) => patch({ title: e.target.value })}
                className="mt-1 w-full rounded-xl border border-slate-300 px-3 py-2"
              />
            </label>
            <label className="block text-sm">
              Sous-titre
              <input
                value={draft.subtitle}
                onChange={(e) => patch({ subtitle: e.target.value })}
                className="mt-1 w-full rounded-xl border border-slate-300 px-3 py-2"
              />
            </label>
            <label className="block text-sm">
              Corps
              <textarea
                value={draft.body}
                onChange={(e) => patch({ body: e.target.value })}
                rows={3}
                className="mt-1 w-full rounded-xl border border-slate-300 px-3 py-2"
              />
            </label>
            <label className="block text-sm">
              Nom du partenaire
              <input
                value={draft.partnerName}
                onChange={(e) => patch({ partnerName: e.target.value })}
                className="mt-1 w-full rounded-xl border border-slate-300 px-3 py-2"
              />
            </label>
            <div className="grid gap-3 sm:grid-cols-2">
              <label className="block text-sm">
                Date / période
                <input
                  value={draft.dateLabel}
                  onChange={(e) => patch({ dateLabel: e.target.value })}
                  className="mt-1 w-full rounded-xl border border-slate-300 px-3 py-2"
                />
              </label>
              <label className="block text-sm">
                Lieu
                <input
                  value={draft.placeLabel}
                  onChange={(e) => patch({ placeLabel: e.target.value })}
                  className="mt-1 w-full rounded-xl border border-slate-300 px-3 py-2"
                />
              </label>
            </div>
          </div>

          <div className="rounded-2xl border border-slate-200 bg-white p-4 space-y-3">
            <p className="text-xs font-bold uppercase text-slate-500">Graphisme</p>
            <div className="flex flex-wrap gap-2">
              {(["solid", "gradient", "image"] as const).map((mode) => (
                <button
                  key={mode}
                  type="button"
                  onClick={() => patch({ backgroundMode: mode })}
                  className={`rounded-lg border px-3 py-1.5 text-xs font-bold ${
                    draft.backgroundMode === mode
                      ? "border-violet-500 bg-violet-50 text-violet-900"
                      : "border-slate-200 text-slate-600"
                  }`}
                >
                  {mode === "solid" ? "Couleur" : mode === "gradient" ? "Dégradé" : "Image"}
                </button>
              ))}
            </div>
            <div className="flex flex-wrap gap-4">
              <label className="text-sm">
                Fond
                <input
                  type="color"
                  value={draft.backgroundColor}
                  onChange={(e) => patch({ backgroundColor: e.target.value })}
                  className="mt-1 block h-10 w-14 cursor-pointer rounded border"
                />
              </label>
              <label className="text-sm">
                Dégradé
                <input
                  type="color"
                  value={draft.gradientTo}
                  onChange={(e) => patch({ gradientTo: e.target.value })}
                  className="mt-1 block h-10 w-14 cursor-pointer rounded border"
                />
              </label>
              <label className="text-sm">
                Accent
                <input
                  type="color"
                  value={draft.accentColor}
                  onChange={(e) => patch({ accentColor: e.target.value })}
                  className="mt-1 block h-10 w-14 cursor-pointer rounded border"
                />
              </label>
              <label className="text-sm">
                Texte
                <input
                  type="color"
                  value={draft.textColor}
                  onChange={(e) => patch({ textColor: e.target.value })}
                  className="mt-1 block h-10 w-14 cursor-pointer rounded border"
                />
              </label>
            </div>
            {draft.backgroundMode === "image" ? (
              <label className="block text-sm">
                Opacité du voile ({Math.round(draft.overlayOpacity * 100)} %)
                <input
                  type="range"
                  min={0}
                  max={0.85}
                  step={0.05}
                  value={draft.overlayOpacity}
                  onChange={(e) => patch({ overlayOpacity: Number(e.target.value) })}
                  className="mt-1 w-full"
                />
              </label>
            ) : null}
            <label className="block text-sm">
              Image de fond
              <input
                type="file"
                accept="image/png,image/jpeg"
                className="mt-1 block w-full text-xs"
                onChange={(e) => void onBackground(e.target.files?.[0] ?? null)}
              />
            </label>
            <label className="block text-sm">
              Logo partenaire
              <input
                type="file"
                accept="image/png,image/jpeg"
                className="mt-1 block w-full text-xs"
                onChange={(e) => void onPartnerLogo(e.target.files?.[0] ?? null)}
              />
            </label>
            <div className="grid gap-3 sm:grid-cols-3">
              <label className="text-sm">
                Taille titre
                <select
                  value={draft.titleSize}
                  onChange={(e) =>
                    patch({ titleSize: e.target.value as PosterDraft["titleSize"] })
                  }
                  className="mt-1 w-full rounded-xl border border-slate-300 px-2 py-2"
                >
                  <option value="S">S</option>
                  <option value="M">M</option>
                  <option value="L">L</option>
                </select>
              </label>
              <label className="text-sm">
                Logo école
                <select
                  value={draft.logoSchoolSize}
                  onChange={(e) =>
                    patch({ logoSchoolSize: e.target.value as PosterDraft["logoSchoolSize"] })
                  }
                  className="mt-1 w-full rounded-xl border border-slate-300 px-2 py-2"
                >
                  <option value="S">S</option>
                  <option value="M">M</option>
                  <option value="L">L</option>
                </select>
              </label>
              <label className="text-sm">
                Logo partenaire
                <select
                  value={draft.logoPartnerSize}
                  onChange={(e) =>
                    patch({ logoPartnerSize: e.target.value as PosterDraft["logoPartnerSize"] })
                  }
                  className="mt-1 w-full rounded-xl border border-slate-300 px-2 py-2"
                >
                  <option value="S">S</option>
                  <option value="M">M</option>
                  <option value="L">L</option>
                </select>
              </label>
            </div>
            <div className="flex flex-wrap gap-4 text-sm">
              <label className="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={draft.blocks.showDatePlace}
                  onChange={(e) =>
                    patch({ blocks: { ...draft.blocks, showDatePlace: e.target.checked } })
                  }
                />
                Date / lieu
              </label>
              <label className="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={draft.blocks.showSchoolMention}
                  onChange={(e) =>
                    patch({ blocks: { ...draft.blocks, showSchoolMention: e.target.checked } })
                  }
                />
                Mention établissement
              </label>
              <label className="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={draft.blocks.showQr}
                  onChange={(e) =>
                    patch({ blocks: { ...draft.blocks, showQr: e.target.checked } })
                  }
                />
                QR code
              </label>
            </div>
            {draft.blocks.showQr ? (
              <label className="block text-sm">
                URL du QR
                <input
                  value={draft.qrUrl}
                  onChange={(e) => patch({ qrUrl: e.target.value })}
                  className="mt-1 w-full rounded-xl border border-slate-300 px-3 py-2"
                  placeholder="https://…"
                />
              </label>
            ) : null}
          </div>

          <div className="rounded-2xl border border-slate-200 bg-white p-4 space-y-3">
            <button
              type="button"
              onClick={() => setShowAdjust((v) => !v)}
              className="text-sm font-bold text-slate-700 underline"
            >
              {showAdjust ? "Masquer les ajustements" : "Ajuster positions (fin)"}
            </button>
            {showAdjust ? (
              <div className="space-y-3 border-t border-slate-100 pt-3">
                <p className="text-xs text-slate-500">
                  Décalages légers (±8 %) — pas de glisser-déposer libre.
                </p>
                <label className="block text-sm">
                  Titre vertical ({Math.round(draft.offsets.titleOffsetY * 100)} %)
                  <input
                    type="range"
                    min={-0.08}
                    max={0.08}
                    step={0.01}
                    value={draft.offsets.titleOffsetY}
                    onChange={(e) =>
                      patch({
                        offsets: { ...draft.offsets, titleOffsetY: Number(e.target.value) },
                      })
                    }
                    className="mt-1 w-full"
                  />
                </label>
                <label className="block text-sm">
                  Contenu horizontal ({Math.round(draft.offsets.contentShiftX * 100)} %)
                  <input
                    type="range"
                    min={-0.08}
                    max={0.08}
                    step={0.01}
                    value={draft.offsets.contentShiftX}
                    onChange={(e) =>
                      patch({
                        offsets: { ...draft.offsets, contentShiftX: Number(e.target.value) },
                      })
                    }
                    className="mt-1 w-full"
                  />
                </label>
                <label className="block text-sm">
                  Contenu vertical ({Math.round(draft.offsets.contentShiftY * 100)} %)
                  <input
                    type="range"
                    min={-0.08}
                    max={0.08}
                    step={0.01}
                    value={draft.offsets.contentShiftY}
                    onChange={(e) =>
                      patch({
                        offsets: { ...draft.offsets, contentShiftY: Number(e.target.value) },
                      })
                    }
                    className="mt-1 w-full"
                  />
                </label>
                <label className="block text-sm">
                  Échelle logo partenaire ({draft.offsets.logoPartnerScale.toFixed(2)})
                  <input
                    type="range"
                    min={0.6}
                    max={1.6}
                    step={0.05}
                    value={draft.offsets.logoPartnerScale}
                    onChange={(e) =>
                      patch({
                        offsets: { ...draft.offsets, logoPartnerScale: Number(e.target.value) },
                      })
                    }
                    className="mt-1 w-full"
                  />
                </label>
                <button
                  type="button"
                  className="text-xs font-semibold text-slate-500 underline"
                  onClick={() => patch({ offsets: defaultPosterOffsets() })}
                >
                  Réinitialiser les ajustements
                </button>
              </div>
            ) : null}
          </div>

          <button
            type="button"
            disabled={busy}
            onClick={() => void generate()}
            className="w-full rounded-xl bg-violet-700 px-5 py-3 text-sm font-bold text-white disabled:opacity-50"
          >
            {busy ? "Génération PDF…" : "Générer et télécharger le PDF"}
          </button>
        </div>

        <div className="space-y-4">
          <p className="text-xs font-bold uppercase text-slate-500">Aperçu</p>
          <PosterPreview
            draft={draft}
            schoolName={schoolName}
            partnerLogoUrl={partnerLogoUrl}
            backgroundImageUrl={backgroundImageUrl}
          />
        </div>
      </div>

      <div className="rounded-2xl border border-slate-100 bg-slate-50/80 p-5">
        <h3 className="text-sm font-black text-slate-800">Historique des affiches</h3>
        {history.length === 0 ? (
          <p className="mt-2 text-sm text-slate-500">Aucune affiche générée pour l’instant.</p>
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
                    {h.templateLabel} · {h.formatLabel || ""} ·{" "}
                    {new Date(h.createdAt).toLocaleString("fr-FR")}
                  </p>
                </div>
                <a
                  href={h.downloadUrl}
                  className="text-xs font-bold text-violet-700 underline"
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
