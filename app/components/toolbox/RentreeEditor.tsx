"use client";

import { useState } from "react";
import type { Establishment } from "@/app/lib/app-config-schemas";
import {
  isInternatRentreeSection,
  RENTREE_SECTION_INTERNAT,
  rentreeAccentClasses,
} from "@/app/lib/rentree-defaults";
import {
  parseRentreeRecipientEmails,
  RENTREE_ACCENT_OPTIONS,
  newRentreeItemId,
  type RentreeEstablishmentPage,
  type RentreeLinkItem,
  type RentreeLinkKind,
  type RentreeSection,
} from "@/app/lib/rentree-types";
import type { RentreeToolConfig } from "@/app/lib/toolbox-types";

function emptyItem(): RentreeLinkItem {
  return { id: newRentreeItemId(), title: "", description: "", href: "", kind: "pdf" };
}

function emptySection(): RentreeSection {
  return { title: "Nouvelle rubrique", items: [] };
}

function MoveArrows({
  canUp,
  canDown,
  onUp,
  onDown,
  label,
}: {
  canUp: boolean;
  canDown: boolean;
  onUp: () => void;
  onDown: () => void;
  label: string;
}) {
  if (!canUp && !canDown) return null;
  const btnClass =
    "flex h-7 w-7 items-center justify-center rounded-md border border-slate-200 bg-white text-sm font-bold text-slate-600 hover:bg-slate-50 hover:border-slate-300";
  return (
    <div className="flex shrink-0 flex-col gap-0.5" role="group" aria-label={label}>
      {canUp ? (
        <button type="button" onClick={onUp} className={btnClass} title="Monter" aria-label={`${label} — monter`}>
          ↑
        </button>
      ) : null}
      {canDown ? (
        <button type="button" onClick={onDown} className={btnClass} title="Descendre" aria-label={`${label} — descendre`}>
          ↓
        </button>
      ) : null}
    </div>
  );
}

function swapInArray<T>(arr: T[], from: number, to: number): T[] {
  if (to < 0 || to >= arr.length || from === to) return arr;
  const next = [...arr];
  [next[from], next[to]] = [next[to], next[from]];
  return next;
}

type Props = {
  rentree: RentreeToolConfig;
  establishments: Establishment[];
  onChange: (patch: Partial<RentreeToolConfig>) => void;
  onPagesChange: (pages: RentreeEstablishmentPage[]) => void;
};

export default function RentreeEditor({ rentree, establishments, onChange, onPagesChange }: Props) {
  const pages = rentree.pages;
  const [selectedId, setSelectedId] = useState<string | null>(pages[0]?.establishmentId ?? null);
  const [uploading, setUploading] = useState<string | null>(null);

  const activePage = pages.find((p) => p.establishmentId === selectedId) ?? pages[0];

  function updatePage(pageId: string, patch: Partial<RentreeEstablishmentPage>) {
    onPagesChange(pages.map((p) => (p.establishmentId === pageId ? { ...p, ...patch } : p)));
  }

  function updateSection(pageId: string, sectionIdx: number, patch: Partial<RentreeSection>) {
    const page = pages.find((p) => p.establishmentId === pageId);
    if (!page) return;
    const sections = page.sections.map((s, i) => (i === sectionIdx ? { ...s, ...patch } : s));
    updatePage(pageId, { sections });
  }

  function updateItem(pageId: string, sectionIdx: number, itemIdx: number, patch: Partial<RentreeLinkItem>) {
    const page = pages.find((p) => p.establishmentId === pageId);
    if (!page) return;
    const sections = page.sections.map((s, si) => {
      if (si !== sectionIdx) return s;
      return {
        ...s,
        items: s.items.map((it, ii) => (ii === itemIdx ? { ...it, ...patch } : it)),
      };
    });
    updatePage(pageId, { sections });
  }

  function addSection(pageId: string) {
    const page = pages.find((p) => p.establishmentId === pageId);
    if (!page) return;
    updatePage(pageId, { sections: [...page.sections, emptySection()] });
  }

  function removeSection(pageId: string, sectionIdx: number) {
    const page = pages.find((p) => p.establishmentId === pageId);
    if (!page) return;
    updatePage(pageId, { sections: page.sections.filter((_, i) => i !== sectionIdx) });
  }

  function addItem(pageId: string, sectionIdx: number) {
    const page = pages.find((p) => p.establishmentId === pageId);
    if (!page) return;
    const sections = page.sections.map((s, i) =>
      i === sectionIdx ? { ...s, items: [...s.items, emptyItem()] } : s,
    );
    updatePage(pageId, { sections });
  }

  function removeItem(pageId: string, sectionIdx: number, itemIdx: number) {
    const page = pages.find((p) => p.establishmentId === pageId);
    if (!page) return;
    const sections = page.sections.map((s, si) =>
      si === sectionIdx ? { ...s, items: s.items.filter((_, ii) => ii !== itemIdx) } : s,
    );
    updatePage(pageId, { sections });
  }

  function moveSection(pageId: string, sectionIdx: number, direction: -1 | 1) {
    const page = pages.find((p) => p.establishmentId === pageId);
    if (!page) return;
    const target = sectionIdx + direction;
    if (target < 0 || target >= page.sections.length) return;
    updatePage(pageId, { sections: swapInArray(page.sections, sectionIdx, target) });
  }

  function moveItem(pageId: string, sectionIdx: number, itemIdx: number, direction: -1 | 1) {
    const page = pages.find((p) => p.establishmentId === pageId);
    if (!page) return;
    const section = page.sections[sectionIdx];
    if (!section) return;
    const target = itemIdx + direction;
    if (target < 0 || target >= section.items.length) return;
    const sections = page.sections.map((s, si) =>
      si === sectionIdx ? { ...s, items: swapInArray(s.items, itemIdx, target) } : s,
    );
    updatePage(pageId, { sections });
  }

  async function uploadDocument(pageId: string, sectionIdx: number, itemIdx: number, file: File) {
    const key = `${pageId}-${sectionIdx}-${itemIdx}`;
    setUploading(key);
    try {
      const res = await fetch("/api/toolbox/rentree/upload", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          fileName: file.name,
          fileType: file.type,
          establishmentId: pageId,
        }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Upload impossible");

      const put = await fetch(data.uploadUrl, {
        method: "PUT",
        headers: { "Content-Type": file.type },
        body: file,
      });
      if (!put.ok) throw new Error("Envoi du fichier échoué");

      const isPdf = file.type === "application/pdf" || file.name.toLowerCase().endsWith(".pdf");
      updateItem(pageId, sectionIdx, itemIdx, {
        href: data.fileUrl,
        kind: isPdf ? "pdf" : "link",
        title: pages
          .find((p) => p.establishmentId === pageId)
          ?.sections[sectionIdx]?.items[itemIdx]?.title || file.name.replace(/\.[^.]+$/, ""),
      });
    } catch (e: unknown) {
      alert(e instanceof Error ? e.message : "Erreur upload");
    } finally {
      setUploading(null);
    }
  }

  if (pages.length === 0) {
    return (
      <div className="rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900">
        Aucun établissement actif dans les paramètres généraux. Ajoutez au moins un établissement dans{" "}
        <a href="/parametres" className="font-bold underline">
          Paramètres → Établissements
        </a>
        , puis enregistrez cette page.
      </div>
    );
  }

  const previewStyle = activePage ? rentreeAccentClasses(activePage.accent) : rentreeAccentClasses("violet");

  return (
    <div className="space-y-6">
      <div className="grid gap-4 sm:grid-cols-2">
        <label className="block">
          <span className="text-xs font-bold text-slate-500 uppercase">Titre page publique</span>
          <input
            className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
            value={rentree.title}
            onChange={(e) => onChange({ title: e.target.value })}
          />
        </label>
        <label className="block">
          <span className="text-xs font-bold text-slate-500 uppercase">Année scolaire</span>
          <input
            className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
            value={rentree.schoolYear}
            onChange={(e) => onChange({ schoolYear: e.target.value })}
          />
        </label>
      </div>

      <div className="flex flex-wrap gap-2">
        {pages.map((page) => {
          const est = establishments.find((e) => e.id === page.establishmentId);
          const isActive = page.establishmentId === (activePage?.establishmentId ?? null);
          return (
            <button
              key={page.establishmentId}
              type="button"
              onClick={() => setSelectedId(page.establishmentId)}
              className={`rounded-xl px-4 py-2 text-sm font-bold border transition ${
                isActive ? "bg-slate-900 text-white border-slate-900" : "bg-white text-slate-700 border-slate-200 hover:border-slate-300"
              }`}
            >
              {page.label || est?.label || page.establishmentId}
            </button>
          );
        })}
      </div>

      {activePage && (
        <div className="grid gap-6 lg:grid-cols-[1fr_280px]">
          <div className="space-y-5 rounded-2xl border border-slate-200 bg-white p-5">
            <div className="grid gap-4 sm:grid-cols-2">
              <label className="block">
                <span className="text-xs font-bold text-slate-500 uppercase">Libellé affiché</span>
                <input
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                  value={activePage.label}
                  onChange={(e) => updatePage(activePage.establishmentId, { label: e.target.value })}
                  placeholder={establishments.find((e) => e.id === activePage.establishmentId)?.label}
                />
              </label>
              <label className="block">
                <span className="text-xs font-bold text-slate-500 uppercase">Couleur d&apos;accent</span>
                <select
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                  value={activePage.accent}
                  onChange={(e) =>
                    updatePage(activePage.establishmentId, {
                      accent: e.target.value as RentreeEstablishmentPage["accent"],
                    })
                  }
                >
                  {RENTREE_ACCENT_OPTIONS.map((opt) => (
                    <option key={opt.id} value={opt.id}>
                      {opt.label}
                    </option>
                  ))}
                </select>
              </label>
            </div>

            <div className="flex items-center justify-between gap-3">
              <div>
                <h3 className="font-black text-slate-900">Rubriques et liens</h3>
                <p className="text-xs text-slate-500 mt-1">
                  Calendrier, Infos pratiques et {RENTREE_SECTION_INTERNAT} — documents, liens ou dépôts familles (ex. assurances).
                </p>
              </div>
              <button
                type="button"
                onClick={() => addSection(activePage.establishmentId)}
                className="rounded-lg bg-indigo-600 px-3 py-1.5 text-xs font-bold text-white"
              >
                + Rubrique
              </button>
            </div>

            {activePage.sections.map((section, sIdx) => (
              <div
                key={`${activePage.establishmentId}-sec-${sIdx}`}
                className={`rounded-xl border p-4 space-y-3 ${
                  isInternatRentreeSection(section)
                    ? "border-amber-300 bg-amber-50/60 ring-1 ring-amber-200"
                    : "border-slate-100 bg-slate-50"
                }`}
              >
                <div className="flex items-center gap-2">
                  <MoveArrows
                    label={`Rubrique « ${section.title || "sans titre"} »`}
                    canUp={sIdx > 0}
                    canDown={sIdx < activePage.sections.length - 1}
                    onUp={() => moveSection(activePage.establishmentId, sIdx, -1)}
                    onDown={() => moveSection(activePage.establishmentId, sIdx, 1)}
                  />
                  <input
                    className="flex-1 rounded-lg border border-slate-200 px-3 py-2 text-sm font-bold"
                    value={section.title}
                    onChange={(e) => updateSection(activePage.establishmentId, sIdx, { title: e.target.value })}
                  />
                  <button
                    type="button"
                    onClick={() => removeSection(activePage.establishmentId, sIdx)}
                    className="text-xs font-bold text-rose-600"
                  >
                    Suppr.
                  </button>
                </div>

                {section.items.map((item, iIdx) => {
                  const uploadKey = `${activePage.establishmentId}-${sIdx}-${iIdx}`;
                  return (
                    <div key={uploadKey} className="flex gap-2">
                      <MoveArrows
                        label={`Lien « ${item.title || "sans titre"} »`}
                        canUp={iIdx > 0}
                        canDown={iIdx < section.items.length - 1}
                        onUp={() => moveItem(activePage.establishmentId, sIdx, iIdx, -1)}
                        onDown={() => moveItem(activePage.establishmentId, sIdx, iIdx, 1)}
                      />
                      <div className="min-w-0 flex-1 rounded-lg border border-white bg-white p-3 space-y-2">
                      <div className="grid gap-2 sm:grid-cols-2">
                        <input
                          className="rounded-lg border px-2 py-1.5 text-sm"
                          placeholder="Titre (ex. Assurance scolaire)"
                          value={item.title}
                          onChange={(e) =>
                            updateItem(activePage.establishmentId, sIdx, iIdx, { title: e.target.value })
                          }
                        />
                        <select
                          className="rounded-lg border px-2 py-1.5 text-sm"
                          value={item.kind || "link"}
                          onChange={(e) => {
                            const kind = e.target.value as RentreeLinkKind;
                            updateItem(activePage.establishmentId, sIdx, iIdx, {
                              kind,
                              id: item.id || newRentreeItemId(),
                              ...(kind === "submission"
                                ? {
                                    href: "",
                                    submission: item.submission ?? { recipientEmails: [] },
                                  }
                                : {}),
                            });
                          }}
                        >
                          <option value="pdf">PDF / document</option>
                          <option value="link">Lien web</option>
                          <option value="submission">Dépôt famille (e-mail)</option>
                        </select>
                      </div>
                      <input
                        className="w-full rounded-lg border px-2 py-1.5 text-sm"
                        placeholder="Description courte (optionnel)"
                        value={item.description || ""}
                        onChange={(e) =>
                          updateItem(activePage.establishmentId, sIdx, iIdx, { description: e.target.value })
                        }
                      />
                      {item.kind === "submission" ? (
                        <div className="space-y-2">
                          <input
                            className="w-full rounded-lg border px-2 py-1.5 text-sm"
                            placeholder="Destinataire(s) — e-mails séparés par une virgule"
                            value={(item.submission?.recipientEmails ?? []).join(", ")}
                            onChange={(e) =>
                              updateItem(activePage.establishmentId, sIdx, iIdx, {
                                submission: {
                                  recipientEmails: e.target.value ? [e.target.value] : [],
                                },
                              })
                            }
                          />
                          <p className="text-[11px] text-slate-500 leading-relaxed">
                            Les familles déposent un fichier et indiquent leur e-mail. Rien n’est transmis tant
                            qu’elles n’ont pas cliqué le lien de confirmation. Ex. assurances → compta@…
                          </p>
                          {parseRentreeRecipientEmails(item.submission?.recipientEmails).length === 0 ? (
                            <p className="text-[11px] font-bold text-amber-700">
                              Ajoutez au moins un e-mail destinataire pour publier ce dépôt.
                            </p>
                          ) : null}
                          <button
                            type="button"
                            onClick={() => removeItem(activePage.establishmentId, sIdx, iIdx)}
                            className="text-xs font-bold text-rose-600"
                          >
                            ✕
                          </button>
                        </div>
                      ) : (
                      <div className="flex flex-wrap gap-2 items-center">
                        <input
                          className="flex-1 min-w-[12rem] rounded-lg border px-2 py-1.5 text-sm font-mono text-xs"
                          placeholder="URL (S3, /documents/… ou https://…)"
                          value={item.href}
                          onChange={(e) =>
                            updateItem(activePage.establishmentId, sIdx, iIdx, { href: e.target.value })
                          }
                        />
                        <label className="cursor-pointer rounded-lg border border-slate-200 px-3 py-1.5 text-xs font-bold text-slate-700 hover:bg-slate-50">
                          {uploading === uploadKey ? "Envoi…" : "📎 Fichier S3"}
                          <input
                            type="file"
                            className="hidden"
                            accept=".pdf,.doc,.docx,.xls,.xlsx,.png,.jpg,.jpeg,.webp"
                            disabled={uploading === uploadKey}
                            onChange={(e) => {
                              const file = e.target.files?.[0];
                              if (file) void uploadDocument(activePage.establishmentId, sIdx, iIdx, file);
                              e.target.value = "";
                            }}
                          />
                        </label>
                        <button
                          type="button"
                          onClick={() => removeItem(activePage.establishmentId, sIdx, iIdx)}
                          className="text-xs font-bold text-rose-600"
                        >
                          ✕
                        </button>
                      </div>
                      )}
                      </div>
                    </div>
                  );
                })}

                <button
                  type="button"
                  onClick={() => addItem(activePage.establishmentId, sIdx)}
                  className="text-xs font-bold text-indigo-600"
                >
                  + Ajouter un élément
                </button>
              </div>
            ))}
          </div>

          <aside className="space-y-3">
            <p className="text-xs font-bold uppercase text-slate-500">Aperçu couleur</p>
            <div className={`rounded-2xl bg-gradient-to-b ${previewStyle.hero} p-4 min-h-[120px]`}>
              <p className="text-white font-black text-lg">{activePage.label}</p>
              <span className={`inline-block mt-2 text-[10px] font-bold px-2 py-0.5 rounded-full border ${previewStyle.badge}`}>
                Exemple
              </span>
            </div>
            <a
              href={`/rentree?establishment=${encodeURIComponent(activePage.establishmentId)}`}
              target="_blank"
              rel="noopener noreferrer"
              className="block text-center rounded-xl border border-slate-200 px-3 py-2 text-xs font-bold text-indigo-600 hover:bg-indigo-50"
            >
              Prévisualiser cette page →
            </a>
            <p className="text-[11px] text-slate-500 leading-relaxed">
              Les modifications sont publiées après <strong>Enregistrer</strong> en haut de page. Les fichiers sont stockés sur S3 du tenant.
            </p>
          </aside>
        </div>
      )}
    </div>
  );
}
