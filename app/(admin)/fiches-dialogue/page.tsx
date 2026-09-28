"use client";

import Link from "next/link";
import { useCallback, useEffect, useMemo, useState } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModuleCard from "@/app/components/module-chrome/ModuleCard";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import { dash } from "@/app/lib/dashboard-brand";
import {
  FD_NIVEAU_LABELS,
  FD_NIVEAUX,
  FD_OPTIONS_MASTER,
  presetForNiveau,
  type FdNiveau,
} from "@/app/lib/fiches-dialogue-templates";
import type { FdCatalogueChoix, FdEtapeKind, FdStarterMode } from "@/db/schema-fiches-dialogue";

type Campagne = {
  id: string;
  label: string;
  anneeLabel: string;
  calendrierMode: string;
  statut: string;
  templateKey: string | null;
  niveauActuel?: string | null;
  starterMode?: string;
  createdAt: string;
};

type PickerEleve = {
  id: string;
  nom: string;
  prenom: string;
  classe?: string;
  ine?: string | null;
};

type WizardStep = 1 | 2 | 3 | 4;

function slugOptionId(label: string): string {
  return (
    "opt_" +
    label
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "_")
      .replace(/^_|_$/g, "")
      .slice(0, 40)
  );
}

export default function FichesDialogueHubPage() {
  const [campagnes, setCampagnes] = useState<Campagne[]>([]);
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [step, setStep] = useState<WizardStep>(1);

  const [niveau, setNiveau] = useState<FdNiveau | null>(null);
  const [starterMode, setStarterMode] = useState<FdStarterMode>("conseil_dabord");
  const [label, setLabel] = useState("");
  const [anneeLabel, setAnneeLabel] = useState("2025-2026");
  const [contactPpLabel, setContactPpLabel] = useState("via École Directe");
  const [appelDateLimite, setAppelDateLimite] = useState("");
  const [appelProcedure, setAppelProcedure] = useState("");
  const [appelDocuments, setAppelDocuments] = useState(
    "Formulaire d’appel\nDécision du conseil de classe (fiche de dialogue)",
  );

  const [catalogue, setCatalogue] = useState<FdCatalogueChoix | null>(null);
  const [enabledDestIds, setEnabledDestIds] = useState<Set<string>>(new Set());
  const [enabledOptIds, setEnabledOptIds] = useState<Set<string>>(new Set());
  const [newOptionLabel, setNewOptionLabel] = useState("");
  const [etapeDates, setEtapeDates] = useState<
    Array<{
      ordre: number;
      kind: FdEtapeKind | string;
      label: string;
      optionnelle?: boolean;
      opensAt: string;
      closesAt: string;
      conseilDate: string;
    }>
  >([]);

  const [pickerLoading, setPickerLoading] = useState(false);
  const [pickerEleves, setPickerEleves] = useState<PickerEleve[]>([]);
  const [pickerClasses, setPickerClasses] = useState<string[]>([]);
  const [selectedClasses, setSelectedClasses] = useState<Set<string>>(new Set());
  const [selectedEleveIds, setSelectedEleveIds] = useState<Set<string>>(new Set());
  const [browseClass, setBrowseClass] = useState<string>("");

  const reload = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/fiches-dialogue/campagnes", { cache: "no-store" });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Erreur de chargement");
      setCampagnes(json.campagnes || []);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  function applyNiveau(n: FdNiveau) {
    setNiveau(n);
    const preset = presetForNiveau(n, starterMode);
    setCatalogue(preset.catalogue);
    setEnabledDestIds(new Set(preset.catalogue.destinations.map((d) => d.id)));
    setEnabledOptIds(new Set(preset.defaultEnabledOptionIds));
    setLabel(preset.labelSuggest);
    setEtapeDates(
      preset.etapes.map((e, i) => ({
        ordre: i + 1,
        kind: e.kind,
        label: e.label,
        optionnelle: Boolean(e.optionnelle),
        opensAt: "",
        closesAt: "",
        conseilDate: "",
      })),
    );
    setStep(2);
  }

  function applyStarter(mode: FdStarterMode) {
    setStarterMode(mode);
    if (niveau) {
      const preset = presetForNiveau(niveau, mode);
      setCatalogue((prev) => {
        if (!prev) return preset.catalogue;
        return {
          ...preset.catalogue,
          destinations: prev.destinations.length ? prev.destinations : preset.catalogue.destinations,
          options: [...FD_OPTIONS_MASTER],
        };
      });
      setEtapeDates(
        preset.etapes.map((e, i) => ({
          ordre: i + 1,
          kind: e.kind,
          label: e.label,
          optionnelle: Boolean(e.optionnelle),
          opensAt: "",
          closesAt: "",
          conseilDate: "",
        })),
      );
    }
    setStep(3);
  }

  async function loadPicker(n: FdNiveau) {
    setPickerLoading(true);
    setError(null);
    try {
      const res = await fetch(
        `/api/fiches-dialogue/eleves-picker?niveau=${encodeURIComponent(n)}`,
        { cache: "no-store" },
      );
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Chargement élèves impossible");
      const classes: string[] = json.classes || [];
      const eleves: PickerEleve[] = json.eleves || [];
      setPickerClasses(classes);
      setPickerEleves(eleves);
      setSelectedClasses(new Set(classes));
      setSelectedEleveIds(new Set(eleves.map((e) => e.id).filter(Boolean)));
      setBrowseClass(classes[0] || "");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setPickerLoading(false);
    }
  }

  useEffect(() => {
    if (step === 4 && niveau) {
      void loadPicker(niveau);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps -- recharge au passage étape 4 / niveau
  }, [step, niveau]);

  const elevesInBrowse = useMemo(() => {
    if (!browseClass) return pickerEleves;
    return pickerEleves.filter(
      (e) => String(e.classe || "").trim().toLowerCase() === browseClass.trim().toLowerCase(),
    );
  }, [browseClass, pickerEleves]);

  const elevesInSelectedClasses = useMemo(() => {
    if (!selectedClasses.size) return [];
    return pickerEleves.filter((e) => selectedClasses.has(String(e.classe || "").trim()));
  }, [pickerEleves, selectedClasses]);

  function toggleClass(classe: string) {
    setSelectedClasses((prev) => {
      const next = new Set(prev);
      if (next.has(classe)) next.delete(classe);
      else next.add(classe);
      return next;
    });
    setSelectedEleveIds((prev) => {
      const next = new Set(prev);
      const inClass = pickerEleves.filter(
        (e) => String(e.classe || "").trim() === classe,
      );
      const wasOn = selectedClasses.has(classe);
      for (const e of inClass) {
        if (!e.id) continue;
        if (wasOn) next.delete(e.id);
        else next.add(e.id);
      }
      return next;
    });
  }

  function toggleEleve(id: string) {
    setSelectedEleveIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  function selectAllInBrowse(on: boolean) {
    setSelectedEleveIds((prev) => {
      const next = new Set(prev);
      for (const e of elevesInBrowse) {
        if (!e.id) continue;
        if (on) next.add(e.id);
        else next.delete(e.id);
      }
      return next;
    });
  }

  function addCustomOption() {
    const lab = newOptionLabel.trim();
    if (!lab || !catalogue) return;
    const id = slugOptionId(lab);
    if (catalogue.options.some((o) => o.id === id)) {
      setEnabledOptIds((prev) => new Set(prev).add(id));
      setNewOptionLabel("");
      return;
    }
    setCatalogue({
      ...catalogue,
      options: [...catalogue.options, { id, label: lab, kind: "autre" }],
    });
    setEnabledOptIds((prev) => new Set(prev).add(id));
    setNewOptionLabel("");
  }

  async function createCampagne() {
    if (!niveau || !catalogue) {
      setError("Choisissez un niveau.");
      return;
    }
    if (!selectedEleveIds.size && !selectedClasses.size) {
      setError("Sélectionnez au moins une classe ou un élève.");
      return;
    }
    setCreating(true);
    setError(null);
    try {
      const filteredCatalogue: FdCatalogueChoix = {
        ...catalogue,
        destinations: catalogue.destinations.filter((d) => enabledDestIds.has(d.id)),
        options: catalogue.options.filter((o) => enabledOptIds.has(o.id)),
      };
      const allSelectedInClasses =
        elevesInSelectedClasses.length > 0 &&
        elevesInSelectedClasses.every((e) => e.id && selectedEleveIds.has(e.id));
      const eleveIdsCibles =
        allSelectedInClasses && selectedClasses.size > 0
          ? []
          : Array.from(selectedEleveIds);

      const res = await fetch("/api/fiches-dialogue/campagnes", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          niveauActuel: niveau,
          starterMode,
          label: label.trim() || `Fiches de dialogue — ${FD_NIVEAU_LABELS[niveau]}`,
          anneeLabel: anneeLabel.trim(),
          classesCibles: Array.from(selectedClasses),
          eleveIdsCibles,
          contactPpLabel: contactPpLabel.trim() || undefined,
          catalogue: filteredCatalogue,
          etapesDates: etapeDates.map((e) => ({
            ordre: e.ordre,
            opensAt: e.opensAt || null,
            closesAt: e.closesAt || null,
            conseilDate: e.conseilDate || null,
          })),
          appelConfig: {
            enabled: true,
            dateLimite: appelDateLimite || undefined,
            procedureHtml: appelProcedure || undefined,
            contactPpLabel: contactPpLabel.trim() || undefined,
            documentsLabels: appelDocuments
              .split("\n")
              .map((s) => s.trim())
              .filter(Boolean),
          },
        }),
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Création impossible");

      const campagneId = json.campagne.id as string;
      const gen = await fetch(`/api/fiches-dialogue/campagnes/${campagneId}/actions`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "generate" }),
      });
      const genJson = await gen.json().catch(() => ({}));
      if (!gen.ok) {
        console.warn("[fiches-dialogue] generate:", genJson);
      }
      window.location.href = `/fiches-dialogue/${campagneId}`;
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
      setCreating(false);
    }
  }

  if (loading) {
    return (
      <ModulePageShell maxWidthClass="max-w-[1100px]">
        <p className={`text-center text-sm ${dash.textMid}`}>Chargement…</p>
      </ModulePageShell>
    );
  }

  return (
    <ModulePageShell maxWidthClass="max-w-[1100px]">
      <ModulePageHeader
        title="Fiches de dialogue"
        description="Orientation année suivante — un niveau, qui commence, options à cocher, ciblage classes / élèves."
      />

      {error && (
        <p className="rounded-lg border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-800">
          {error}
        </p>
      )}

      <ModuleCard bodyClassName="space-y-5 p-5">
        <div className="flex flex-wrap items-center gap-2 text-xs font-semibold uppercase tracking-wide text-slate-500">
          {([1, 2, 3, 4] as WizardStep[]).map((s) => (
            <button
              key={s}
              type="button"
              onClick={() => {
                if (s === 1) setStep(1);
                else if (s === 2 && niveau) setStep(2);
                else if (s === 3 && niveau) setStep(3);
                else if (s === 4 && niveau && catalogue) setStep(4);
              }}
              className={`rounded-full px-3 py-1 ${
                step === s ? "bg-slate-900 text-white" : "bg-slate-100 text-slate-600"
              }`}
            >
              {s === 1
                ? "1. Niveau"
                : s === 2
                  ? "2. Qui commence"
                  : s === 3
                    ? "3. Formulaire"
                    : "4. Ciblage"}
            </button>
          ))}
        </div>

        {step === 1 ? (
          <div className="space-y-4">
            <h2 className={`text-lg font-semibold ${dash.ink}`}>Choisir un niveau</h2>
            <p className={`text-sm ${dash.textMid}`}>
              Une campagne = un seul niveau (collège ou lycée).
            </p>
            <div className="grid gap-3 sm:grid-cols-2">
              <div>
                <p className="mb-2 text-xs font-bold uppercase text-slate-500">Collège</p>
                <div className="flex flex-wrap gap-2">
                  {(["6e", "5e", "4e", "3e"] as FdNiveau[]).map((n) => (
                    <button
                      key={n}
                      type="button"
                      onClick={() => applyNiveau(n)}
                      className={`rounded-xl border px-4 py-3 text-sm font-bold ${
                        niveau === n
                          ? "border-teal-600 bg-teal-50 text-teal-900"
                          : "border-slate-200 bg-white hover:border-slate-400"
                      }`}
                    >
                      {FD_NIVEAU_LABELS[n]}
                    </button>
                  ))}
                </div>
              </div>
              <div>
                <p className="mb-2 text-xs font-bold uppercase text-slate-500">Lycée</p>
                <div className="flex flex-wrap gap-2">
                  {(["2nde", "1re", "Tle"] as FdNiveau[]).map((n) => (
                    <button
                      key={n}
                      type="button"
                      onClick={() => applyNiveau(n)}
                      className={`rounded-xl border px-4 py-3 text-sm font-bold ${
                        niveau === n
                          ? "border-teal-600 bg-teal-50 text-teal-900"
                          : "border-slate-200 bg-white hover:border-slate-400"
                      }`}
                    >
                      {FD_NIVEAU_LABELS[n]}
                    </button>
                  ))}
                </div>
              </div>
            </div>
          </div>
        ) : null}

        {step === 2 ? (
          <div className="space-y-4">
            <h2 className={`text-lg font-semibold ${dash.ink}`}>Qui commence ?</h2>
            <p className={`text-sm ${dash.textMid}`}>
              Niveau sélectionné : <strong>{niveau ? FD_NIVEAU_LABELS[niveau] : "—"}</strong>
            </p>
            <div className="grid gap-3 sm:grid-cols-2">
              <button
                type="button"
                onClick={() => applyStarter("conseil_dabord")}
                className={`rounded-2xl border p-5 text-left ${
                  starterMode === "conseil_dabord"
                    ? "border-teal-600 bg-teal-50"
                    : "border-slate-200 hover:border-slate-400"
                }`}
              >
                <p className="font-bold text-slate-900">Conseil de classe d’abord</p>
                <p className="mt-1 text-sm text-slate-600">
                  Proposition du conseil, puis réponse de la famille (ex. collège 6ᵉ).
                </p>
              </button>
              <button
                type="button"
                onClick={() => applyStarter("famille_dabord")}
                className={`rounded-2xl border p-5 text-left ${
                  starterMode === "famille_dabord"
                    ? "border-teal-600 bg-teal-50"
                    : "border-slate-200 hover:border-slate-400"
                }`}
              >
                <p className="font-bold text-slate-900">Famille d’abord</p>
                <p className="mt-1 text-sm text-slate-600">
                  La famille formule ses vœux, puis avis du conseil (ex. lycée).
                </p>
              </button>
            </div>
            <ModuleButton type="button" variant="secondary" onClick={() => setStep(1)}>
              Retour
            </ModuleButton>
          </div>
        ) : null}

        {step === 3 && catalogue ? (
          <div className="space-y-5">
            <h2 className={`text-lg font-semibold ${dash.ink}`}>Configurer le formulaire</h2>
            <div className="grid gap-3 sm:grid-cols-2">
              <label className="block text-sm">
                <span className={dash.textMid}>Libellé campagne</span>
                <input
                  className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
                  value={label}
                  onChange={(e) => setLabel(e.target.value)}
                />
              </label>
              <label className="block text-sm">
                <span className={dash.textMid}>Année scolaire</span>
                <input
                  className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
                  value={anneeLabel}
                  onChange={(e) => setAnneeLabel(e.target.value)}
                />
              </label>
              <label className="block text-sm">
                <span className={dash.textMid}>Canal contact PP</span>
                <input
                  className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
                  value={contactPpLabel}
                  onChange={(e) => setContactPpLabel(e.target.value)}
                />
              </label>
            </div>

            <div className="rounded-xl border border-slate-100 bg-slate-50 p-4 space-y-3">
              <p className="text-sm font-semibold text-slate-800">
                Dates par étape (ouverture / clôture famille, conseil, publication)
              </p>
              <p className={`text-xs ${dash.textMid}`}>
                Pour chaque étape famille : dernier jour de réponse. Pour le conseil : date de
                réunion + date de publication des résultats.
              </p>
              {etapeDates.map((e, idx) => {
                const isFamille =
                  e.kind === "saisie_famille" ||
                  e.kind === "choix_definitifs" ||
                  e.kind === "acceptation_famille";
                const isConseil =
                  e.kind === "conseil" || e.kind === "decision_finale_conseil";
                if (!isFamille && !isConseil) return null;
                return (
                  <div
                    key={`${e.ordre}-${e.kind}`}
                    className="grid gap-2 rounded-lg border border-slate-200 bg-white p-3 sm:grid-cols-2"
                  >
                    <p className="sm:col-span-2 text-sm font-medium text-slate-900">
                      {e.ordre}. {e.label}
                      {e.optionnelle ? " (optionnelle)" : ""}
                    </p>
                    {isConseil ? (
                      <>
                        <label className="block text-xs">
                          Date du conseil
                          <input
                            type="date"
                            className="mt-1 w-full rounded border border-slate-200 px-2 py-1.5 text-sm"
                            value={e.conseilDate}
                            onChange={(ev) => {
                              const next = [...etapeDates];
                              next[idx] = { ...e, conseilDate: ev.target.value };
                              setEtapeDates(next);
                            }}
                          />
                        </label>
                        <label className="block text-xs">
                          Publication des résultats
                          <input
                            type="datetime-local"
                            className="mt-1 w-full rounded border border-slate-200 px-2 py-1.5 text-sm"
                            value={e.opensAt}
                            onChange={(ev) => {
                              const next = [...etapeDates];
                              next[idx] = { ...e, opensAt: ev.target.value };
                              setEtapeDates(next);
                            }}
                          />
                        </label>
                      </>
                    ) : (
                      <>
                        <label className="block text-xs">
                          Ouverture saisie
                          <input
                            type="datetime-local"
                            className="mt-1 w-full rounded border border-slate-200 px-2 py-1.5 text-sm"
                            value={e.opensAt}
                            onChange={(ev) => {
                              const next = [...etapeDates];
                              next[idx] = { ...e, opensAt: ev.target.value };
                              setEtapeDates(next);
                            }}
                          />
                        </label>
                        <label className="block text-xs">
                          Dernier jour (clôture)
                          <input
                            type="datetime-local"
                            className="mt-1 w-full rounded border border-slate-200 px-2 py-1.5 text-sm"
                            value={e.closesAt}
                            onChange={(ev) => {
                              const next = [...etapeDates];
                              next[idx] = { ...e, closesAt: ev.target.value };
                              setEtapeDates(next);
                            }}
                          />
                        </label>
                      </>
                    )}
                  </div>
                );
              })}
            </div>

            <div>
              <p className="mb-2 text-sm font-semibold text-slate-800">
                Destinations à faire apparaître
              </p>
              <div className="flex flex-col gap-2">
                {catalogue.destinations.map((d) => (
                  <label key={d.id} className="flex items-center gap-2 text-sm">
                    <input
                      type="checkbox"
                      checked={enabledDestIds.has(d.id)}
                      onChange={() => {
                        setEnabledDestIds((prev) => {
                          const next = new Set(prev);
                          if (next.has(d.id)) next.delete(d.id);
                          else next.add(d.id);
                          return next;
                        });
                      }}
                    />
                    {d.label}
                  </label>
                ))}
              </div>
            </div>

            <div>
              <p className="mb-2 text-sm font-semibold text-slate-800">
                Options / langues / spécialités (cochez selon le niveau)
              </p>
              <div className="flex max-h-64 flex-col gap-2 overflow-y-auto rounded-lg border border-slate-100 p-3">
                {(catalogue.options.length ? catalogue.options : FD_OPTIONS_MASTER).map((o) => (
                  <label key={o.id} className="flex items-center gap-2 text-sm">
                    <input
                      type="checkbox"
                      checked={enabledOptIds.has(o.id)}
                      onChange={() => {
                        setEnabledOptIds((prev) => {
                          const next = new Set(prev);
                          if (next.has(o.id)) next.delete(o.id);
                          else next.add(o.id);
                          return next;
                        });
                      }}
                    />
                    <span>
                      {o.label}{" "}
                      <span className="text-xs text-slate-400">({o.kind})</span>
                    </span>
                  </label>
                ))}
              </div>
              <div className="mt-3 flex flex-wrap gap-2">
                <input
                  className="min-w-[200px] flex-1 rounded-lg border border-slate-200 px-3 py-2 text-sm"
                  placeholder="Ajouter une option…"
                  value={newOptionLabel}
                  onChange={(e) => setNewOptionLabel(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      e.preventDefault();
                      addCustomOption();
                    }
                  }}
                />
                <ModuleButton type="button" variant="secondary" onClick={addCustomOption}>
                  Ajouter
                </ModuleButton>
              </div>
            </div>

            <div className="rounded-xl border border-amber-100 bg-amber-50/80 p-4 space-y-2">
              <p className="text-sm font-semibold text-amber-950">
                Procédure d’appel (obligation légale — toujours active)
              </p>
              <p className="text-xs text-amber-900/80">
                Date limite, documents et texte de procédure sont souvent connus seulement en
                mai–juin. Vous pouvez laisser vide à la création et les compléter plus tard sur
                la fiche campagne.
              </p>
              <input
                className="w-full rounded-lg border border-amber-200 bg-white px-3 py-2 text-sm"
                placeholder="Date limite d’appel (optionnel pour l’instant)"
                value={appelDateLimite}
                onChange={(e) => setAppelDateLimite(e.target.value)}
              />
              <textarea
                className="w-full rounded-lg border border-amber-200 bg-white px-3 py-2 text-sm"
                rows={2}
                placeholder="Procédure (optionnel — à compléter plus tard)"
                value={appelProcedure}
                onChange={(e) => setAppelProcedure(e.target.value)}
              />
              <textarea
                className="w-full rounded-lg border border-amber-200 bg-white px-3 py-2 text-sm"
                rows={2}
                placeholder="Documents requis (un par ligne, optionnel)"
                value={appelDocuments}
                onChange={(e) => setAppelDocuments(e.target.value)}
              />
            </div>

            <div className="flex flex-wrap gap-2">
              <ModuleButton type="button" variant="secondary" onClick={() => setStep(2)}>
                Retour
              </ModuleButton>
              <ModuleButton
                type="button"
                onClick={() => {
                  if (!enabledDestIds.size) {
                    setError("Cochez au moins une destination.");
                    return;
                  }
                  setError(null);
                  setStep(4);
                }}
              >
                Continuer — ciblage
              </ModuleButton>
            </div>
          </div>
        ) : null}

        {step === 4 ? (
          <div className="space-y-4">
            <h2 className={`text-lg font-semibold ${dash.ink}`}>
              Ciblage — {niveau ? FD_NIVEAU_LABELS[niveau] : ""}
            </h2>
            <p className={`text-sm ${dash.textMid}`}>
              Cochez les classes, puis les élèves (utile pour un test sur un seul élève).
            </p>
            {pickerLoading ? (
              <p className="text-sm text-slate-500">Chargement des élèves…</p>
            ) : (
              <>
                <div>
                  <p className="mb-2 text-xs font-bold uppercase text-slate-500">Classes</p>
                  <div className="flex flex-wrap gap-2">
                    {pickerClasses.map((c) => (
                      <button
                        key={c}
                        type="button"
                        onClick={() => toggleClass(c)}
                        className={`rounded-lg border px-3 py-1.5 text-xs font-bold ${
                          selectedClasses.has(c)
                            ? "border-teal-600 bg-teal-50 text-teal-900"
                            : "border-slate-200 bg-white"
                        }`}
                      >
                        {c}
                      </button>
                    ))}
                    {!pickerClasses.length ? (
                      <p className="text-sm text-amber-700">
                        Aucune classe trouvée pour ce niveau.
                      </p>
                    ) : null}
                  </div>
                </div>

                <div className="grid gap-3 sm:grid-cols-[180px_1fr]">
                  <div className="space-y-1">
                    <p className="text-xs font-bold uppercase text-slate-500">Parcourir</p>
                    {pickerClasses
                      .filter((c) => selectedClasses.has(c))
                      .map((c) => (
                        <button
                          key={c}
                          type="button"
                          onClick={() => setBrowseClass(c)}
                          className={`block w-full rounded-lg px-2 py-1.5 text-left text-sm ${
                            browseClass === c ? "bg-slate-900 text-white" : "hover:bg-slate-100"
                          }`}
                        >
                          {c}
                        </button>
                      ))}
                  </div>
                  <div className="rounded-xl border border-slate-200 p-3">
                    <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
                      <p className="text-sm font-semibold">
                        Élèves — {browseClass || "toutes"} (
                        {elevesInBrowse.filter((e) => e.id && selectedEleveIds.has(e.id)).length}/
                        {elevesInBrowse.length})
                      </p>
                      <div className="flex gap-2">
                        <button
                          type="button"
                          className="text-xs font-bold text-teal-700"
                          onClick={() => selectAllInBrowse(true)}
                        >
                          Tout
                        </button>
                        <button
                          type="button"
                          className="text-xs font-bold text-slate-500"
                          onClick={() => selectAllInBrowse(false)}
                        >
                          Aucun
                        </button>
                      </div>
                    </div>
                    <ul className="max-h-64 space-y-1 overflow-y-auto">
                      {elevesInBrowse.map((e) => (
                        <li key={e.id || `${e.nom}-${e.prenom}`}>
                          <label className="flex cursor-pointer items-center gap-2 rounded-lg px-2 py-1 text-sm hover:bg-slate-50">
                            <input
                              type="checkbox"
                              checked={Boolean(e.id && selectedEleveIds.has(e.id))}
                              disabled={!e.id}
                              onChange={() => e.id && toggleEleve(e.id)}
                            />
                            <span>
                              {e.nom} {e.prenom}
                              {e.ine ? (
                                <span className="ml-2 text-xs text-slate-400">INE {e.ine}</span>
                              ) : null}
                            </span>
                          </label>
                        </li>
                      ))}
                    </ul>
                    <p className="mt-2 text-xs text-slate-500">
                      {selectedEleveIds.size} élève(s) sélectionné(s) au total
                    </p>
                  </div>
                </div>
              </>
            )}

            <div className="flex flex-wrap gap-2">
              <ModuleButton type="button" variant="secondary" onClick={() => setStep(3)}>
                Retour
              </ModuleButton>
              <ModuleButton type="button" onClick={() => void createCampagne()} disabled={creating}>
                {creating ? "Création…" : "Créer la campagne et générer les fiches"}
              </ModuleButton>
            </div>
          </div>
        ) : null}
      </ModuleCard>

      <section className="space-y-3">
        <h2 className={`text-lg font-semibold ${dash.ink}`}>Campagnes</h2>
        {!campagnes.length ? (
          <p className={`text-sm ${dash.textMid}`}>Aucune campagne pour le moment.</p>
        ) : (
          <div className="grid gap-3 sm:grid-cols-2">
            {campagnes.map((c) => (
              <div
                key={c.id}
                className="rounded-2xl border border-slate-200 bg-white p-4 hover:border-teal-300"
              >
                <Link href={`/fiches-dialogue/${c.id}`} className="block">
                  <p className="font-bold text-slate-900">{c.label}</p>
                  <p className="text-sm text-slate-600">
                    {c.anneeLabel}
                    {c.niveauActuel ? ` · ${c.niveauActuel}` : ""}
                    {c.starterMode ? ` · ${c.starterMode}` : ""} · {c.statut}
                  </p>
                </Link>
                <button
                  type="button"
                  className="mt-3 text-xs font-bold text-rose-700 hover:underline"
                  onClick={async () => {
                    if (
                      !confirm(
                        `Supprimer définitivement la campagne « ${c.label} » et toutes ses fiches ?`,
                      )
                    ) {
                      return;
                    }
                    setError(null);
                    try {
                      const res = await fetch(`/api/fiches-dialogue/campagnes/${c.id}`, {
                        method: "DELETE",
                      });
                      const json = await res.json().catch(() => ({}));
                      if (!res.ok) throw new Error(json.error || "Suppression impossible");
                      await reload();
                    } catch (e) {
                      setError(e instanceof Error ? e.message : "Erreur");
                    }
                  }}
                >
                  Supprimer
                </button>
              </div>
            ))}
          </div>
        )}
      </section>
    </ModulePageShell>
  );
}
