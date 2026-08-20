"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import PosterCanvas, {
  PosterPaletteDragItem,
} from "@/app/components/communication/PosterCanvas";
import {
  POSTER_PALETTE,
  clampElementBox,
  createPosterElement,
  defaultPosterDraft,
  elementsForStarter,
} from "@/app/lib/posters/catalog";
import type {
  PosterDraft,
  PosterElement,
  PosterFormat,
  PosterPaletteItem,
  PosterTemplateMeta,
  PosterTextAlign,
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
  const [palette, setPalette] = useState<PosterPaletteItem[]>(POSTER_PALETTE);
  const [schoolName, setSchoolName] = useState("Établissement");
  const [schoolLogoUrl, setSchoolLogoUrl] = useState<string | null>(null);
  const [draft, setDraft] = useState<PosterDraft>(() => defaultPosterDraft());
  const [brief, setBrief] = useState("");
  const [partnerLogoUrl, setPartnerLogoUrl] = useState<string | null>(null);
  const [backgroundImageUrl, setBackgroundImageUrl] = useState<string | null>(null);
  const [imageUrls, setImageUrls] = useState<Record<string, string>>({});
  const [history, setHistory] = useState<GeneratedRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [suggestBusy, setSuggestBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [showSheetPreview, setShowSheetPreview] = useState(false);

  const patch = useCallback((partial: Partial<PosterDraft>) => {
    setDraft((d) => ({ ...d, ...partial }));
  }, []);

  const setElements = useCallback((elements: PosterElement[]) => {
    setDraft((d) => ({ ...d, elements }));
  }, []);

  const selected = useMemo(
    () => draft.elements.find((e) => e.id === selectedId) || null,
    [draft.elements, selectedId],
  );

  const patchSelected = (partial: Partial<PosterElement>) => {
    if (!selected) return;
    setElements(
      draft.elements.map((el) =>
        el.id === selected.id ? clampElementBox({ ...el, ...partial }) : el,
      ),
    );
  };

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
      if (Array.isArray(tj.palette)) setPalette(tj.palette);
      if (tj.branding?.name) setSchoolName(tj.branding.name);
      if (tj.branding?.logoUrl) setSchoolLogoUrl(tj.branding.logoUrl);
      if (tj.defaults?.elements) {
        setDraft({ ...defaultPosterDraft(), ...tj.defaults });
      }

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

  const uploadAsset = async (kind: "partner-logo" | "background" | "image", file: File) => {
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

  const onElementImage = async (file: File | null) => {
    if (!file || !selected) return;
    try {
      const kind = selected.kind === "logo-partner" ? "partner-logo" : "image";
      const { key, url } = await uploadAsset(kind, file);
      if (kind === "partner-logo") {
        patch({ partnerLogoKey: key });
        setPartnerLogoUrl(url);
      } else {
        setImageUrls((m) => ({ ...m, [key]: url }));
        patchSelected({ imageKey: key });
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Upload image");
    }
  };

  const applyStarter = (starterId: string) => {
    setElements(elementsForStarter(starterId));
    setSelectedId(null);
    // Les démarrages partenariat sont pensés pour un fond sombre.
    if (starterId === "partner-sides" || starterId === "logos-band" || starterId === "photo-full") {
      patch({
        backgroundMode: starterId === "photo-full" ? "image" : "gradient",
        backgroundColor: "#0f172a",
        gradientTo: "#1e3a5f",
        textColor: "#ffffff",
        accentColor: "#0ea5e9",
      });
    }
    setMsg("Disposition de départ appliquée — déplacez ou clic droit pour supprimer.");
  };

  const addKind = (kind: PosterElement["kind"]) => {
    const el = createPosterElement(kind);
    setElements([...draft.elements, clampElementBox(el)]);
    setSelectedId(el.id);
  };

  const removeSelected = () => {
    if (!selected) return;
    setElements(draft.elements.filter((e) => e.id !== selected.id));
    setSelectedId(null);
  };

  const suggestCopy = async () => {
    setSuggestBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/posters/suggest-copy", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          brief:
            brief ||
            [draft.partnerName, "partenariat sportif affiche scolaire"].filter(Boolean).join(" — "),
          templateId: draft.templateId,
          partnerName: draft.partnerName,
        }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Suggestion impossible");
      setElements(
        draft.elements.map((el) => {
          if (el.kind === "title" && j.title) return { ...el, text: String(j.title) };
          if (el.kind === "subtitle" && j.subtitle) return { ...el, text: String(j.subtitle) };
          if (el.kind === "body" && j.body) return { ...el, text: String(j.body) };
          return el;
        }),
      );
      setMsg("Textes proposés — vous pouvez les corriger sur le canvas.");
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
          Page blanche : ajoutez logos, textes et images. Clic droit sur un élément pour le
          supprimer. A5 = planche de 4 sur A4.
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

      <div className="grid gap-6 lg:grid-cols-[minmax(0,0.95fr)_minmax(0,1.35fr)]">
        <div className="space-y-4">
          {active ? (
            <div className="rounded-2xl border border-slate-200 bg-white p-4 space-y-3">
              <p className="text-xs font-bold uppercase text-slate-500">Format</p>
              <div className="grid gap-2 sm:grid-cols-3">
                {formats.map((f) => (
                  <button
                    key={f.id}
                    type="button"
                    onClick={() => {
                      patch({ format: f.id });
                      if (f.id !== "a5-portrait") setShowSheetPreview(false);
                    }}
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

              <p className="pt-2 text-xs font-bold uppercase text-slate-500">
                Démarrage rapide
              </p>
              <div className="grid gap-2">
                {(active.starters || []).map((s) => (
                  <button
                    key={s.id}
                    type="button"
                    onClick={() => applyStarter(s.id)}
                    className="rounded-xl border border-slate-200 px-3 py-2 text-left text-sm hover:border-violet-300 hover:bg-violet-50"
                  >
                    <span className="font-semibold">{s.label}</span>
                    <span className="ml-2 text-xs text-slate-500">{s.hint}</span>
                  </button>
                ))}
              </div>
            </div>
          ) : null}

          <div className="rounded-2xl border border-slate-200 bg-white p-4 space-y-3">
            <p className="text-xs font-bold uppercase text-slate-500">
              Éléments (glisser sur l’affiche)
            </p>
            <div className="grid gap-2 sm:grid-cols-2">
              {palette.map((p) => (
                <PosterPaletteDragItem
                  key={p.kind}
                  kind={p.kind}
                  label={p.label}
                  hint={p.hint}
                  onAdd={() => addKind(p.kind)}
                />
              ))}
            </div>
          </div>

          <div className="rounded-2xl border border-slate-200 bg-white p-4 space-y-3">
            <p className="text-xs font-bold uppercase text-slate-500">Bloc sélectionné</p>
            {!selected ? (
              <p className="text-sm text-slate-500">
                Cliquez un élément sur l’aperçu pour le modifier.
              </p>
            ) : (
              <div className="space-y-3">
                <p className="text-sm font-semibold text-slate-800">
                  {palette.find((p) => p.kind === selected.kind)?.label || selected.kind}
                </p>
                {[
                  "title",
                  "subtitle",
                  "body",
                  "date-place",
                  "mention",
                  "qr",
                ].includes(selected.kind) ? (
                  <label className="block text-sm">
                    {selected.kind === "qr" ? "URL du QR" : "Texte"}
                    <textarea
                      value={selected.text || ""}
                      onChange={(e) => patchSelected({ text: e.target.value })}
                      rows={selected.kind === "body" ? 4 : 2}
                      className="mt-1 w-full rounded-xl border border-slate-300 px-3 py-2 text-sm"
                      placeholder={
                        selected.kind === "qr"
                          ? "https://…"
                          : selected.kind === "mention"
                            ? "Vide = nom école × partenaire"
                            : ""
                      }
                    />
                  </label>
                ) : null}
                {["title", "subtitle", "body", "date-place", "mention"].includes(
                  selected.kind,
                ) ? (
                  <>
                    <label className="block text-sm">
                      Alignement
                      <select
                        value={selected.align || "center"}
                        onChange={(e) =>
                          patchSelected({ align: e.target.value as PosterTextAlign })
                        }
                        className="mt-1 w-full rounded-xl border border-slate-300 px-2 py-2"
                      >
                        <option value="left">Gauche</option>
                        <option value="center">Centre</option>
                        <option value="right">Droite</option>
                      </select>
                    </label>
                    <label className="block text-sm">
                      Taille texte ({Math.round((selected.fontScale || 1) * 100)} %)
                      <input
                        type="range"
                        min={0.7}
                        max={1.5}
                        step={0.05}
                        value={selected.fontScale || 1}
                        onChange={(e) =>
                          patchSelected({ fontScale: Number(e.target.value) })
                        }
                        className="mt-1 w-full"
                      />
                    </label>
                  </>
                ) : null}
                {selected.kind === "logo-partner" || selected.kind === "image" ? (
                  <label className="block text-sm">
                    {selected.kind === "logo-partner" ? "Fichier logo partenaire" : "Fichier image"}
                    <input
                      type="file"
                      accept="image/png,image/jpeg,image/svg+xml,.svg"
                      className="mt-1 block w-full text-xs"
                      onChange={(e) => void onElementImage(e.target.files?.[0] ?? null)}
                    />
                  </label>
                ) : null}
                <button
                  type="button"
                  onClick={removeSelected}
                  className="text-sm font-bold text-rose-700 underline"
                >
                  Supprimer ce bloc
                </button>
              </div>
            )}
          </div>

          <div className="rounded-2xl border border-slate-200 bg-white p-4 space-y-3">
            <p className="text-xs font-bold uppercase text-slate-500">Textes IA / partenaire</p>
            <label className="block text-sm">
              Nom du partenaire
              <input
                value={draft.partnerName}
                onChange={(e) => patch({ partnerName: e.target.value })}
                className="mt-1 w-full rounded-xl border border-slate-300 px-3 py-2"
              />
            </label>
            <label className="block text-sm">
              Brief IA (optionnel)
              <textarea
                value={brief}
                onChange={(e) => setBrief(e.target.value)}
                rows={2}
                className="mt-1 w-full rounded-xl border border-slate-300 px-3 py-2 text-sm"
                placeholder="Ex. partenariat centre équestre, initiation poney…"
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
              Logo partenaire (global)
              <input
                type="file"
                accept="image/png,image/jpeg,image/svg+xml,.svg"
                className="mt-1 block w-full text-xs"
                onChange={(e) => void onPartnerLogo(e.target.files?.[0] ?? null)}
              />
            </label>
            <label className="block text-sm">
              URL QR (globale, si bloc QR sans URL)
              <input
                value={draft.qrUrl}
                onChange={(e) => patch({ qrUrl: e.target.value })}
                className="mt-1 w-full rounded-xl border border-slate-300 px-3 py-2"
                placeholder="https://…"
              />
            </label>
          </div>

          <div className="rounded-2xl border border-slate-200 bg-white p-4 space-y-3">
            <p className="text-xs font-bold uppercase text-slate-500">Fond</p>
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
                accept="image/png,image/jpeg,image/svg+xml,.svg"
                className="mt-1 block w-full text-xs"
                onChange={(e) => void onBackground(e.target.files?.[0] ?? null)}
              />
            </label>
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

        <div className="space-y-3 lg:sticky lg:top-4 lg:self-start">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <p className="text-xs font-bold uppercase text-slate-500">Aperçu live</p>
            {draft.format === "a5-portrait" ? (
              <label className="flex items-center gap-2 text-xs text-slate-600">
                <input
                  type="checkbox"
                  checked={showSheetPreview}
                  onChange={(e) => setShowSheetPreview(e.target.checked)}
                />
                Voir planche 4×
              </label>
            ) : null}
          </div>
          <div className="min-h-[60vh]">
            <PosterCanvas
              draft={draft}
              selectedId={selectedId}
              onSelect={setSelectedId}
              onChangeElements={setElements}
              schoolName={schoolName}
              schoolLogoUrl={schoolLogoUrl}
              partnerLogoUrl={partnerLogoUrl}
              backgroundImageUrl={backgroundImageUrl}
              imageUrls={imageUrls}
              showSheetPreview={showSheetPreview}
            />
          </div>
          {!schoolLogoUrl ? (
            <p className="text-xs text-amber-700">
              Logo établissement introuvable — renseignez-le dans Paramètres → identité pour le
              voir ici et sur le PDF.
            </p>
          ) : null}
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
