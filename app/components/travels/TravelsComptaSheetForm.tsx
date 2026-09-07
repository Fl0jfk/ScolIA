"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  computeComptaSheetDerived,
  COMPTA_MARGE_PRESETS,
  comptaAfficheMargeSecurite,
  emptyComptaSheet,
  formatEuroDisplay,
  formatEuroWhole,
  isUsableComptaAmount,
  isBlankRecetteLine,
  isApelRecetteLineIndex,
  normalizeComptaRecettesLignes,
  perStudentEuroCeilAdjusted,
  suggestComptaMargeFromPreset,
  withoutBlankRecettesLignes,
  type ComptaMargePresetId,
  type TravelsComptaExpenseLine,
  type TravelsComptaRecetteLine,
  type TravelsComptaIndividualAid,
  type TravelsComptaSheet,
} from "@/app/lib/travels-compta-sheet";
import { TripButton } from "@/app/components/travels/TripDetailUI";
import TravelsComptaApelSummaryModal from "@/app/components/travels/TravelsComptaApelSummaryModal";
import type { ComptaApelSummary } from "@/app/lib/travels-compta-apel-summary";

type Props = {
  tripId: string;
  documentsRevision?: string;
  readOnly?: boolean;
  canValidateBudget?: boolean;
  budgetValidated?: boolean;
  variant?: "inline" | "modal";
  onSaved?: (sheet: TravelsComptaSheet) => void;
  onValidateBudget?: (sheet: TravelsComptaSheet) => void | Promise<void>;
};

function parseNumberInput(raw: string): number | null {
  const trimmed = raw.trim().replace(/\s/g, "").replace(",", ".");
  if (!trimmed || trimmed === "." || trimmed === "-" || trimmed === "-.") return null;
  const n = Number(trimmed);
  return Number.isFinite(n) ? Math.round(n * 100) / 100 : null;
}

function formatAmountDraft(value: number | null): string {
  if (value == null || !Number.isFinite(value)) return "";
  return String(value);
}

function EuroAmountInput({
  value,
  onChange,
  className = "",
  readOnly = false,
  onBlur,
  onFocus,
}: {
  value: number | null;
  onChange: (v: number | null) => void;
  className?: string;
  readOnly?: boolean;
  onBlur?: () => void;
  onFocus?: () => void;
}) {
  const [focused, setFocused] = useState(false);
  const [draft, setDraft] = useState(() => formatAmountDraft(value));

  useEffect(() => {
    if (!focused) setDraft(formatAmountDraft(value));
  }, [value, focused]);

  return (
    <input
      type="text"
      inputMode="decimal"
      value={focused ? draft : formatAmountDraft(value)}
      onFocus={() => {
        setFocused(true);
        setDraft(formatAmountDraft(value));
        onFocus?.();
      }}
      onChange={(e) => {
        const raw = e.target.value;
        if (raw !== "" && !/^-?\d*[.,]?\d*$/.test(raw)) return;
        setDraft(raw);
        onChange(parseNumberInput(raw));
      }}
      onBlur={() => {
        setFocused(false);
        const parsed = parseNumberInput(draft);
        onChange(parsed);
        setDraft(formatAmountDraft(parsed));
        onBlur?.();
      }}
      readOnly={readOnly}
      disabled={readOnly}
      className={`w-full rounded-lg border border-slate-200 px-2 py-1.5 text-sm text-right disabled:bg-slate-50 ${className}`}
      placeholder="0"
    />
  );
}

function isBlankDepenseLine(line: TravelsComptaExpenseLine): boolean {
  return !line.label.trim() && line.amount == null;
}

/** Ligne encore en cours de saisie : on ne la coupe pas / on n'écrase pas le focus. */
function isIncompleteDepenseLine(line: TravelsComptaExpenseLine): boolean {
  const hasLabel = Boolean(line.label.trim());
  const hasAmount = line.amount != null;
  return hasLabel !== hasAmount;
}

function withoutBlankDepenses(depenses: TravelsComptaExpenseLine[]): TravelsComptaExpenseLine[] {
  return depenses.filter((line) => !isBlankDepenseLine(line));
}

function newRowKey(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }
  return `row-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
}

/** Réinjecte les brouillons locaux (ligne incomplète / vide) après un save serveur. */
function mergeSavedDepensesWithDrafts(
  saved: TravelsComptaExpenseLine[],
  local: TravelsComptaExpenseLine[],
): TravelsComptaExpenseLine[] {
  const drafts = local.filter((line) => isBlankDepenseLine(line) || isIncompleteDepenseLine(line));
  if (drafts.length === 0) return saved;
  const out = [...saved];
  for (const draft of drafts) {
    const already =
      draft.label.trim() &&
      out.some(
        (line) =>
          line.label.trim().toLowerCase() === draft.label.trim().toLowerCase() &&
          line.amount === draft.amount,
      );
    if (!already) out.push(draft);
  }
  return out;
}

function mergeSavedRecettesWithDrafts(
  saved: TravelsComptaRecetteLine[],
  local: TravelsComptaRecetteLine[],
): TravelsComptaRecetteLine[] {
  const normalizedSaved = normalizeComptaRecettesLignes({ recettesLignes: saved });
  const localNorm = normalizeComptaRecettesLignes({ recettesLignes: local });
  const drafts = localNorm.filter(
    (line, i) =>
      !isApelRecetteLineIndex(i) &&
      (isBlankRecetteLine(line) || Boolean(line.label.trim()) !== (line.amount != null)),
  );
  if (drafts.length === 0) return normalizedSaved;
  return [...normalizedSaved, ...drafts];
}

function depensesAffichees(depenses: TravelsComptaSheet["depenses"]) {
  return depenses.filter((line) => line.label.trim() || line.amount != null);
}

function recettesLignesAffichees(lignes: TravelsComptaRecetteLine[]) {
  return lignes.filter((line) => line.label.trim() || line.amount != null);
}

function aideRowsForDisplay(aides: TravelsComptaIndividualAid[]) {
  const withIndex = aides.map((row, index) => ({ row, index }));
  const filled = withIndex.filter(({ row }) => row.name.trim() || row.amount != null);
  if (filled.length > 0) return filled;
  return [{ row: aides[0] ?? { name: "", amount: null }, index: 0 }];
}

function margeMatchesPreset(
  marge: number | null | undefined,
  depensesTotal: number | null | undefined,
  percent: number,
): boolean {
  if (marge == null || depensesTotal == null) return false;
  const suggested = suggestComptaMargeFromPreset(depensesTotal, percent);
  return suggested != null && marge === suggested;
}

export default function TravelsComptaSheetForm({
  tripId,
  documentsRevision = "",
  readOnly = false,
  canValidateBudget = false,
  budgetValidated = false,
  variant = "inline",
  onSaved,
  onValidateBudget,
}: Props) {
  const [sheet, setSheet] = useState<TravelsComptaSheet>(emptyComptaSheet());
  const [initialLoading, setInitialLoading] = useState(true);
  const [analyzing, setAnalyzing] = useState(false);
  const [saveState, setSaveState] = useState<"idle" | "saving" | "saved" | "error">("idle");
  const [error, setError] = useState<string | null>(null);
  const [ocrInfo, setOcrInfo] = useState<string | null>(null);
  const [pdfBusy, setPdfBusy] = useState(false);
  const [validating, setValidating] = useState(false);
  const [apelSummaryOpen, setApelSummaryOpen] = useState(false);
  const [apelSummary, setApelSummary] = useState<ComptaApelSummary | null>(null);
  const [apelSummaryLoading, setApelSummaryLoading] = useState(false);

  const hydrated = useRef(false);
  const skipSave = useRef(true);
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const onSavedRef = useRef(onSaved);
  const onValidateBudgetRef = useRef(onValidateBudget);
  const loadSeq = useRef(0);
  const sheetRef = useRef(sheet);
  const saveEpoch = useRef(0);
  const lineEditDepth = useRef(0);
  const depenseKeysRef = useRef<string[]>([]);
  const recetteKeysRef = useRef<string[]>([]);

  function syncRowKeys(keysRef: { current: string[] }, length: number, removedIndex?: number) {
    if (removedIndex != null && removedIndex >= 0 && removedIndex < keysRef.current.length) {
      keysRef.current = keysRef.current.filter((_, i) => i !== removedIndex);
    }
    while (keysRef.current.length < length) keysRef.current.push(newRowKey());
    if (keysRef.current.length > length) keysRef.current = keysRef.current.slice(0, length);
  }

  useEffect(() => {
    onSavedRef.current = onSaved;
  }, [onSaved]);

  useEffect(() => {
    onValidateBudgetRef.current = onValidateBudget;
  }, [onValidateBudget]);

  useEffect(() => {
    sheetRef.current = sheet;
    syncRowKeys(depenseKeysRef, sheet.depenses.length);
    syncRowKeys(recetteKeysRef, normalizeComptaRecettesLignes(sheet).length);
  }, [sheet]);

  const derived = useMemo(() => computeComptaSheetDerived(sheet), [sheet]);
  const coutPrevisionnelArrondi = perStudentEuroCeilAdjusted(
    derived.montantCibleFacturation,
    sheet.nbEleves ?? 0,
  );
  const busLine = sheet.depenses.find((d) => d.source === "devis_signe");
  const lignesDepensesInfo = useMemo(() => depensesAffichees(sheet.depenses), [sheet.depenses]);
  const lignesRecettesInfo = useMemo(
    () => recettesLignesAffichees(derived.recettesLignes ?? []),
    [derived.recettesLignes],
  );
  const lignesAides = useMemo(() => aideRowsForDisplay(sheet.aidesIndividuelles), [sheet.aidesIndividuelles]);

  const loadApelSummary = useCallback(async () => {
    setApelSummaryLoading(true);
    try {
      const res = await fetch("/api/travels/compta-apel-summary", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          tripId,
          currentSheet: computeComptaSheetDerived(sheetRef.current),
        }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setApelSummary(data as ComptaApelSummary);
    } catch {
      setApelSummary(null);
    } finally {
      setApelSummaryLoading(false);
    }
  }, [tripId]);

  const patch = useCallback((partial: Partial<TravelsComptaSheet>) => {
    if (readOnly) return;
    setSheet((prev) => computeComptaSheetDerived({ ...prev, ...partial }));
  }, [readOnly]);

  const persistSheet = useCallback(async (finalSheet: TravelsComptaSheet) => {
    const epoch = ++saveEpoch.current;
    const snapshot = JSON.stringify({
      depenses: finalSheet.depenses,
      recettesLignes: finalSheet.recettesLignes,
      compte: finalSheet.compte,
      ligne: finalSheet.ligne,
      classe: finalSheet.classe,
      profs: finalSheet.profs,
      accompagnateurs: finalSheet.accompagnateurs,
      nbEleves: finalSheet.nbEleves,
      margeSecuriteEuro: finalSheet.margeSecuriteEuro,
      prixParEleveAnnonce: finalSheet.prixParEleveAnnonce,
      nbElevesFactures: finalSheet.nbElevesFactures,
      aidesIndividuelles: finalSheet.aidesIndividuelles,
      facturations: finalSheet.facturations,
      recettesElevesFigees: finalSheet.recettesElevesFigees,
      margeFigeeEuro: finalSheet.margeFigeeEuro,
    });
    setSaveState("saving");
    setError(null);
    const cleanedSheet = computeComptaSheetDerived({
      ...finalSheet,
      depenses: withoutBlankDepenses(finalSheet.depenses),
      recettesLignes: withoutBlankRecettesLignes(finalSheet.recettesLignes ?? []),
    });
    try {
      const res = await fetch("/api/travels/compta-sheet", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ tripId, action: "save", sheet: cleanedSheet }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      const saved = computeComptaSheetDerived(data.sheet || cleanedSheet);
      onSavedRef.current?.(saved);

      // Ne jamais écraser une saisie plus récente (libellés / montants qui « sautent »).
      if (epoch !== saveEpoch.current) {
        setSaveState("saved");
        return;
      }
      const local = sheetRef.current;
      const localSnapshot = JSON.stringify({
        depenses: local.depenses,
        recettesLignes: local.recettesLignes,
        compte: local.compte,
        ligne: local.ligne,
        classe: local.classe,
        profs: local.profs,
        accompagnateurs: local.accompagnateurs,
        nbEleves: local.nbEleves,
        margeSecuriteEuro: local.margeSecuriteEuro,
        prixParEleveAnnonce: local.prixParEleveAnnonce,
        nbElevesFactures: local.nbElevesFactures,
        aidesIndividuelles: local.aidesIndividuelles,
        facturations: local.facturations,
        recettesElevesFigees: local.recettesElevesFigees,
        margeFigeeEuro: local.margeFigeeEuro,
      });
      if (localSnapshot !== snapshot) {
        setSaveState("saved");
        return;
      }

      const merged = computeComptaSheetDerived({
        ...saved,
        depenses: mergeSavedDepensesWithDrafts(saved.depenses, local.depenses),
        recettesLignes: mergeSavedRecettesWithDrafts(
          saved.recettesLignes ?? [],
          local.recettesLignes ?? [],
        ),
      });
      skipSave.current = true;
      setSheet(merged);
      setSaveState("saved");
    } catch (e) {
      if (epoch === saveEpoch.current) {
        setSaveState("error");
        setError(e instanceof Error ? e.message : "Erreur d'enregistrement");
      }
    }
  }, [tripId]);

  const loadSheet = useCallback(async () => {
    const seq = ++loadSeq.current;
    setError(null);
    try {
      const resFast = await fetch(
        `/api/travels/compta-sheet?tripId=${encodeURIComponent(tripId)}&skipSync=1`,
        { cache: "no-store" },
      );
      const dataFast = await resFast.json();
      if (!resFast.ok) throw new Error(dataFast?.error || "Erreur");
      if (seq !== loadSeq.current) return;

      skipSave.current = true;
      setSheet(computeComptaSheetDerived(dataFast.sheet || emptyComptaSheet()));
      hydrated.current = true;
      setInitialLoading(false);

      if (readOnly || !dataFast.needSync) {
        setOcrInfo(
          dataFast.sheet?.analysisNotes
            ? String(dataFast.sheet.analysisNotes)
            : readOnly
              ? "Consultation de la fiche budget."
              : "Fiche à jour avec les documents du dossier.",
        );
        return;
      }

      setAnalyzing(true);
      setOcrInfo("Nouveau document détecté — analyse en cours…");

      const resSync = await fetch(`/api/travels/compta-sheet?tripId=${encodeURIComponent(tripId)}`, {
        cache: "no-store",
      });
      const dataSync = await resSync.json();
      if (!resSync.ok) throw new Error(dataSync?.error || "Erreur");
      if (seq !== loadSeq.current) return;

      skipSave.current = true;
      setSheet(computeComptaSheetDerived(dataSync.sheet || emptyComptaSheet()));

      const parts: string[] = [];
      if (dataSync.ocrNewCount > 0) parts.push(`${dataSync.ocrNewCount} document(s) analysé(s)`);
      if (dataSync.removedCount > 0) parts.push(`${dataSync.removedCount} document(s) retiré(s)`);
      if (dataSync.syncNotes) parts.push(String(dataSync.syncNotes));
      setOcrInfo(parts.join(" · ") || "Dossier synchronisé avec les documents.");
    } catch (e) {
      if (seq === loadSeq.current) setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      if (seq === loadSeq.current) {
        setAnalyzing(false);
        setInitialLoading(false);
      }
    }
  }, [tripId, readOnly]);

  async function handleForceBusOcr() {
    if (readOnly) return;
    setAnalyzing(true);
    setError(null);
    try {
      const res = await fetch("/api/travels/compta-sheet", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ tripId, action: "analyze", forceBusOcr: true }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      skipSave.current = true;
      setSheet(computeComptaSheetDerived(data.sheet || emptyComptaSheet()));
      setOcrInfo(data.syncNotes || "Analyse OCR relancée.");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Impossible de relancer l'OCR.");
    } finally {
      setAnalyzing(false);
    }
  }

  useEffect(() => {
    hydrated.current = false;
    skipSave.current = true;
    setInitialLoading(true);
    void loadSheet();
    return () => {
      loadSeq.current += 1;
      if (saveTimer.current) clearTimeout(saveTimer.current);
    };
  }, [tripId, documentsRevision, loadSheet]);

  const schedulePersist = useCallback(
    (delayMs = 1200) => {
      if (readOnly) return;
      if (saveTimer.current) clearTimeout(saveTimer.current);
      saveTimer.current = setTimeout(function tick() {
        if (lineEditDepth.current > 0) {
          saveTimer.current = setTimeout(tick, 500);
          return;
        }
        void persistSheet(sheetRef.current);
      }, delayMs);
    },
    [persistSheet, readOnly],
  );

  useEffect(() => {
    if (readOnly) return;
    if (!hydrated.current || initialLoading || analyzing) return;
    if (skipSave.current) {
      skipSave.current = false;
      return;
    }
    schedulePersist();
    return () => {
      if (saveTimer.current) clearTimeout(saveTimer.current);
    };
  }, [sheet, initialLoading, analyzing, schedulePersist, readOnly]);

  function beginLineEdit() {
    lineEditDepth.current += 1;
    if (saveTimer.current) clearTimeout(saveTimer.current);
  }

  function endLineEdit() {
    lineEditDepth.current = Math.max(0, lineEditDepth.current - 1);
    if (lineEditDepth.current === 0 && hydrated.current && !readOnly) {
      schedulePersist(450);
    }
  }

  async function handleValidateBudget() {
    const finalSheet = computeComptaSheetDerived({
      ...sheet,
      depenses: withoutBlankDepenses(sheet.depenses),
      recettesLignes: withoutBlankRecettesLignes(sheet.recettesLignes ?? []),
    });
    setValidating(true);
    setError(null);
    try {
      skipSave.current = true;
      await persistSheet(finalSheet);
      await onValidateBudgetRef.current?.(finalSheet);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Impossible de valider le budget.");
    } finally {
      setValidating(false);
    }
  }

  async function downloadPdf() {
    setPdfBusy(true);
    setError(null);
    try {
      const finalSheet = computeComptaSheetDerived(sheet);
      const res = await fetch("/api/travels/compta-sheet/pdf", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ tripId, sheet: finalSheet }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      const href = String(data.pdf || "");
      if (!href) throw new Error("PDF vide.");
      const link = document.createElement("a");
      link.href = href;
      link.download = String(data.filename || "Fiche_Compta.pdf");
      link.click();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Impossible de générer le PDF.");
    } finally {
      setPdfBusy(false);
    }
  }

  function updateDepense(index: number, field: "label" | "amount", value: string | number | null) {
    const depenses = sheet.depenses.map((line, i) =>
      i === index ? { ...line, [field]: field === "amount" ? (value as number | null) : String(value) } : line,
    );
    patch({ depenses });
  }

  function removeDepense(index: number) {
    syncRowKeys(depenseKeysRef, sheet.depenses.length - 1, index);
    patch({ depenses: sheet.depenses.filter((_, i) => i !== index) });
  }

  function pruneBlankDepense(index: number) {
    const line = sheet.depenses[index];
    if (!line || !isBlankDepenseLine(line)) return;
    // Garder au moins une ligne vide pour la saisie, sauf suppression explicite.
    if (sheet.depenses.length <= 1) return;
    syncRowKeys(depenseKeysRef, sheet.depenses.length - 1, index);
    patch({ depenses: sheet.depenses.filter((_, i) => i !== index) });
  }

  function addDepense() {
    depenseKeysRef.current = [...depenseKeysRef.current, newRowKey()];
    patch({ depenses: [...sheet.depenses, { label: "", amount: null }] });
  }

  function updateRecette(index: number, field: "label" | "amount", value: string | number | null) {
    const recettesLignes = normalizeComptaRecettesLignes(sheet).map((line, i) =>
      i === index ? { ...line, [field]: field === "amount" ? (value as number | null) : String(value) } : line,
    );
    patch({ recettesLignes });
  }

  function removeRecette(index: number) {
    if (isApelRecetteLineIndex(index)) return;
    syncRowKeys(recetteKeysRef, normalizeComptaRecettesLignes(sheet).length - 1, index);
    patch({ recettesLignes: normalizeComptaRecettesLignes(sheet).filter((_, i) => i !== index) });
  }

  function pruneBlankRecette(index: number) {
    if (isApelRecetteLineIndex(index)) return;
    const line = normalizeComptaRecettesLignes(sheet)[index];
    if (!line || !isBlankRecetteLine(line)) return;
    syncRowKeys(recetteKeysRef, normalizeComptaRecettesLignes(sheet).length - 1, index);
    patch({ recettesLignes: normalizeComptaRecettesLignes(sheet).filter((_, i) => i !== index) });
  }

  function addRecette() {
    recetteKeysRef.current = [...recetteKeysRef.current, newRowKey()];
    patch({
      recettesLignes: [...normalizeComptaRecettesLignes(sheet), { label: "", amount: null }],
    });
  }

  function updateAide(index: number, field: keyof TravelsComptaIndividualAid, value: string | number | null) {
    const aidesIndividuelles = sheet.aidesIndividuelles.map((row, i) =>
      i === index
        ? { ...row, [field]: field === "amount" ? (value as number | null) : String(value) }
        : row,
    );
    patch({ aidesIndividuelles });
  }

  function addAide() {
    patch({ aidesIndividuelles: [...sheet.aidesIndividuelles, { name: "", amount: null }] });
  }

  function applyMargePreset(presetId: ComptaMargePresetId) {
    if (recettesFigees) return;
    const percent = COMPTA_MARGE_PRESETS.find((p) => p.id === presetId)?.percent ?? 0;
    const suggested = suggestComptaMargeFromPreset(derived.depensesTotal, percent);
    patch({ margeSecuriteEuro: suggested ?? 0 });
  }

  function updateFacturationDate(value: string) {
    const row = sheet.facturations[0] ?? { label: "", prixFacture: null, dateFacturation: "", montant: null };
    patch({ facturations: [{ ...row, dateFacturation: value }] });
  }

  function toggleRecettesFigees(enabled: boolean) {
    if (enabled) {
      const marge = derived.margeRisqueMontant ?? 0;
      patch({
        recettesElevesFigees: true,
        prixParEleveAnnonce: derived.prixParEleveAvecSubventions,
        nbElevesFactures: sheet.nbElevesFactures ?? sheet.nbEleves,
        margeFigeeEuro: marge > 0 ? marge : null,
      });
    } else {
      patch({ recettesElevesFigees: false, prixParEleveAnnonce: null, margeFigeeEuro: null });
    }
  }

  function figerPrixAnnonceActuel() {
    const marge = derived.margeRisqueMontant ?? 0;
    patch({
      recettesElevesFigees: true,
      prixParEleveAnnonce: derived.prixParEleveAvecSubventions,
      nbElevesFactures: sheet.nbEleves ?? sheet.nbElevesFactures,
      margeFigeeEuro: marge > 0 ? marge : null,
    });
  }

  const recettesFigees = derived.recettesElevesFigees;
  const afficheMarge = comptaAfficheMargeSecurite({
    recettesElevesFigees: derived.recettesElevesFigees,
    prixParEleveAnnonce: sheet.prixParEleveAnnonce,
    margeFigeeEuro: derived.margeFigeeEuro ?? sheet.margeFigeeEuro,
  });

  const shellClass =
    variant === "inline"
      ? "rounded-2xl border border-slate-200 bg-white shadow-sm overflow-hidden"
      : "flex max-h-[92vh] w-full max-w-4xl flex-col overflow-hidden rounded-2xl bg-white shadow-2xl";

  return (
    <div className={shellClass}>
      <div className="border-b border-slate-100 px-5 py-4 flex flex-wrap items-start justify-between gap-3">
        <div>
          <p className="text-xs font-black uppercase tracking-widest text-indigo-600">Comptabilité</p>
          <h2 className="text-lg font-black text-slate-900">Fiche budget voyage</h2>
          <p className="mt-1 text-sm text-slate-500">
            {readOnly
              ? "Consultation de la fiche budget — modification réservée à la comptabilité."
              : "Saisie et synchronisation automatique avec les documents. Le budget définitif n'est transmis à la direction qu'après validation en bas de page."}
          </p>
        </div>
        <div className="text-right text-xs font-bold flex flex-col items-end gap-2">
          <TripButton
            variant="dark"
            size="sm"
            onClick={() => void downloadPdf()}
            disabled={initialLoading || analyzing || pdfBusy}
          >
            {pdfBusy ? "PDF…" : "Télécharger en PDF"}
          </TripButton>
          {analyzing && <span className="text-indigo-600">Synchronisation des documents…</span>}
          {!analyzing && !readOnly && saveState === "saving" && <span className="text-slate-500">Enregistrement…</span>}
          {!analyzing && !readOnly && saveState === "saved" && <span className="text-emerald-600">Enregistré</span>}
          {!analyzing && !readOnly && saveState === "error" && <span className="text-red-600">Erreur</span>}
        </div>
      </div>

      <div className={`overflow-y-auto px-5 py-4 space-y-6 ${variant === "modal" ? "flex-1 max-h-[70vh]" : ""}`}>
        {error && (
          <div className="rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700">{error}</div>
        )}
        {ocrInfo && (
          <div className="rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-800">
            {ocrInfo}
          </div>
        )}
        {busLine && !isUsableComptaAmount(busLine.amount) && !readOnly && (
          <div className="rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900 flex flex-wrap items-center justify-between gap-2">
            <span>Montant du devis bus non détecté.</span>
            <button
              type="button"
              onClick={() => void handleForceBusOcr()}
              disabled={analyzing}
              className="text-xs font-bold text-amber-800 underline hover:text-amber-950 disabled:opacity-50"
            >
              {analyzing ? "Analyse…" : "Relancer l'OCR bus"}
            </button>
          </div>
        )}

        {initialLoading ? (
          <p className="text-center text-slate-500 py-8">Chargement et synchronisation des documents…</p>
        ) : (
          <>
            <section className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
              {(
                [
                  ["compte", "Compte"],
                  ["ligne", "Ligne"],
                  ["classe", "Classe"],
                  ["profs", "Profs"],
                  ["accompagnateurs", "Accompagnateurs"],
                ] as const
              ).map(([key, label]) => (
                <label key={key} className="space-y-1">
                  <span className="text-[10px] font-black uppercase tracking-widest text-slate-400">{label}</span>
                  <input
                    value={sheet[key]}
                    onChange={(e) => patch({ [key]: e.target.value })}
                    readOnly={readOnly}
                    disabled={readOnly}
                    className="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm disabled:bg-slate-50"
                  />
                </label>
              ))}
            </section>

            <section className="rounded-xl border border-slate-200 overflow-hidden">
              <div className="bg-slate-50 px-4 py-2 text-xs font-black uppercase tracking-widest text-slate-500">
                Dépenses
              </div>
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-slate-100 text-left text-xs text-slate-400">
                    <th className="p-3 font-bold">Libellé</th>
                    <th className="p-3 font-bold w-44 min-w-[11rem]">Montant</th>
                    {!readOnly ? <th className="p-3 w-10" aria-label="Actions" /> : null}
                  </tr>
                </thead>
                <tbody>
                  {sheet.depenses.map((line, i) => (
                    <tr key={depenseKeysRef.current[i] ?? `depense-${i}`} className="border-b border-slate-50">
                      <td className="p-2">
                        <input
                          value={line.label}
                          onChange={(e) => updateDepense(i, "label", e.target.value)}
                          onFocus={beginLineEdit}
                          onBlur={() => {
                            endLineEdit();
                            pruneBlankDepense(i);
                          }}
                          readOnly={readOnly}
                          disabled={readOnly}
                          className="w-full rounded-lg border border-slate-200 px-2 py-1.5 text-sm disabled:bg-slate-50"
                          placeholder="Ex. Transport bus"
                        />
                      </td>
                      <td className="p-2">
                        <EuroAmountInput
                          value={line.amount}
                          onChange={(v) => updateDepense(i, "amount", v)}
                          onFocus={beginLineEdit}
                          onBlur={() => {
                            endLineEdit();
                            pruneBlankDepense(i);
                          }}
                          readOnly={readOnly}
                        />
                      </td>
                      {!readOnly ? (
                        <td className="p-2 text-center">
                          <button
                            type="button"
                            onClick={() => removeDepense(i)}
                            className="inline-flex h-8 w-8 items-center justify-center rounded-lg text-slate-400 hover:bg-red-50 hover:text-red-600"
                            title="Supprimer la ligne"
                            aria-label="Supprimer la ligne"
                          >
                            ✕
                          </button>
                        </td>
                      ) : null}
                    </tr>
                  ))}
                  {!readOnly ? (
                    <tr>
                      <td colSpan={3} className="px-2 py-1">
                        <button
                          type="button"
                          onClick={addDepense}
                          className="text-xs font-bold text-indigo-600 hover:text-indigo-800"
                        >
                          + Ligne de dépense
                        </button>
                      </td>
                    </tr>
                  ) : null}
                </tbody>
                <tfoot>
                  <tr className="bg-slate-50 font-bold">
                    <td className="p-3">
                      Total dépenses
                      <span className="block text-[10px] font-normal text-slate-400 normal-case">
                        {budgetValidated ? "Budget total final validé" : "Devient le budget total final à la validation"}
                      </span>
                    </td>
                    <td className="p-3 text-right font-mono whitespace-nowrap">
                      {derived.depensesTotal != null ? `${formatEuroDisplay(derived.depensesTotal)} €` : "—"}
                    </td>
                    {!readOnly ? <td className="p-3" /> : null}
                  </tr>
                  <tr className="border-t border-slate-100 bg-slate-50/80">
                    <td className="p-3">
                      Nb élèves
                      <span className="block text-[10px] font-normal text-slate-400 normal-case">
                        Joindre une liste
                      </span>
                    </td>
                    <td className="p-2">
                      <input
                        type="number"
                        min={0}
                        value={sheet.nbEleves ?? ""}
                        onChange={(e) =>
                          patch({
                            nbEleves:
                              e.target.value === "" ? null : Math.max(0, Math.floor(Number(e.target.value))),
                          })
                        }
                        readOnly={readOnly}
                        disabled={readOnly}
                        className="w-full rounded-lg border border-slate-200 bg-white px-2 py-1.5 text-sm text-right disabled:bg-slate-50"
                      />
                    </td>
                    {!readOnly ? <td className="p-3" /> : null}
                  </tr>
                  <tr className="border-t border-slate-100 bg-slate-50/80">
                    <td className="p-3">
                      Coût par élève
                      <span className="block text-[10px] font-normal text-slate-400 normal-case">
                        Total dépenses ÷ nombre d&apos;élèves
                      </span>
                    </td>
                    <td className="p-3 text-right font-mono font-semibold text-slate-800 whitespace-nowrap">
                      {derived.prixParEleve != null ? `${formatEuroDisplay(derived.prixParEleve)} €` : "—"}
                    </td>
                    {!readOnly ? <td className="p-3" /> : null}
                  </tr>
                </tfoot>
              </table>
            </section>

            {afficheMarge ? (
              <section className="rounded-xl border border-amber-200 overflow-hidden bg-amber-50/25">
                <div className="bg-amber-50 px-4 py-2 text-xs font-black uppercase tracking-widest text-amber-900 border-b border-amber-100">
                  Marge de sécurité
                  {recettesFigees ? (
                    <span className="ml-2 font-normal normal-case text-amber-800/80">
                      (figée à l&apos;annonce aux parents)
                    </span>
                  ) : null}
                </div>
                <div className="p-4 space-y-4">
                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 text-sm">
                    <div className="rounded-lg border border-amber-100 bg-white px-3 py-2">
                      <p className="text-[10px] font-black uppercase tracking-widest text-amber-800/70">Nb élèves</p>
                      <p className="mt-1 font-mono font-semibold text-slate-800">{sheet.nbEleves ?? "—"}</p>
                    </div>
                    <div className="rounded-lg border border-amber-100 bg-white px-3 py-2">
                      <p className="text-[10px] font-black uppercase tracking-widest text-amber-800/70">
                        Total dépenses
                      </p>
                      <p className="mt-1 font-mono font-semibold text-slate-800 whitespace-nowrap">
                        {derived.depensesTotal != null ? `${formatEuroDisplay(derived.depensesTotal)} €` : "—"}
                      </p>
                    </div>
                  </div>

                  <div className="rounded-lg border border-amber-200 bg-white p-3 space-y-3">
                    <p className="text-xs font-black uppercase tracking-widest text-amber-800">Montant de la marge</p>
                    {!recettesFigees ? (
                      <div className="flex flex-wrap gap-1.5">
                        {COMPTA_MARGE_PRESETS.map((opt) => {
                          const suggested = suggestComptaMargeFromPreset(derived.depensesTotal, opt.percent);
                          const active = margeMatchesPreset(
                            sheet.margeSecuriteEuro,
                            derived.depensesTotal,
                            opt.percent,
                          );
                          return (
                            <button
                              key={opt.id}
                              type="button"
                              onClick={() => applyMargePreset(opt.id)}
                              disabled={readOnly}
                              title={
                                suggested != null && derived.depensesTotal
                                  ? `${formatEuroDisplay(suggested)} €`
                                  : undefined
                              }
                              className={`rounded-lg border px-2 py-1 text-[11px] font-bold transition-colors ${
                                active
                                  ? "border-amber-500 bg-amber-100 text-amber-900"
                                  : "border-amber-200 bg-white text-amber-800 hover:bg-amber-50"
                              }`}
                            >
                              {opt.percent === 0 ? "0 %" : `+${opt.percent} %`}
                              <span className="block font-normal normal-case text-[10px] opacity-80 leading-tight">
                                {opt.label}
                              </span>
                            </button>
                          );
                        })}
                      </div>
                    ) : (
                      <p className="text-sm text-amber-900">Montant prévu avant l&apos;annonce aux familles.</p>
                    )}
                    <div className="max-w-xs ml-auto">
                      <p className="text-[10px] font-bold text-slate-500 mb-1">Montant en €</p>
                      {recettesFigees ? (
                        <div className="w-full rounded-lg border border-amber-200 bg-amber-50/50 px-2 py-1.5 text-sm font-mono text-right text-amber-950">
                          {derived.margeRisqueMontant != null
                            ? `${formatEuroDisplay(derived.margeRisqueMontant)} €`
                            : "—"}
                        </div>
                      ) : (
                        <EuroAmountInput
                          value={sheet.margeSecuriteEuro}
                          onChange={(v) => patch({ margeSecuriteEuro: v })}
                          className="bg-white"
                          readOnly={readOnly}
                        />
                      )}
                    </div>
                  </div>

                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 pt-1 border-t border-amber-100">
                    <div className="flex justify-between items-baseline gap-3 text-sm font-bold text-amber-950">
                      <span>
                        Budget prévisionnel total
                        <span className="block text-[10px] font-normal text-amber-800/80 normal-case">
                          Dépenses + marge de sécurité
                        </span>
                      </span>
                      <span className="font-mono whitespace-nowrap">
                        {derived.montantCibleFacturation != null
                          ? `${formatEuroDisplay(derived.montantCibleFacturation)} €`
                          : "—"}
                      </span>
                    </div>
                    <div className="flex justify-between items-baseline gap-3 text-sm font-bold text-amber-950">
                      <span>
                        Coût prévisionnel par élève
                        <span className="block text-[10px] font-normal text-amber-800/80 normal-case">
                          Budget prévisionnel ÷ nb élèves
                        </span>
                        {coutPrevisionnelArrondi ? (
                          <span className="block text-[10px] font-medium text-amber-700 normal-case mt-1">
                            Attention, ce coût a été arrondi à l&apos;entier supérieur.
                          </span>
                        ) : null}
                      </span>
                      <span className="font-mono whitespace-nowrap">
                        {derived.coutPrevisionnelParEleve != null
                          ? `${formatEuroWhole(derived.coutPrevisionnelParEleve)} €`
                          : "—"}
                      </span>
                    </div>
                  </div>
                </div>
              </section>
            ) : null}

            <section className="rounded-xl border border-slate-200 overflow-hidden">
              <div className="bg-indigo-50 px-4 py-3 flex flex-wrap justify-between items-center gap-3 border-b border-indigo-100">
                <p className="text-xs font-black uppercase tracking-widest text-indigo-800">Recettes</p>
                <div className="text-right">
                  <p className="text-[10px] font-bold uppercase tracking-widest text-slate-500">
                    Total recettes + subventions
                  </p>
                  <p className="font-mono font-black text-indigo-900">
                    {derived.totalRecettes != null ? `${formatEuroDisplay(derived.totalRecettes)} €` : "—"}
                  </p>
                </div>
              </div>

              <div className="border-b border-slate-100 bg-white">
                {!readOnly ? (
                  <div className="px-4 pt-3">
                    <button
                      type="button"
                      onClick={addRecette}
                      className="text-xs font-bold text-indigo-600 hover:text-indigo-800"
                    >
                      + Ligne de recette
                    </button>
                  </div>
                ) : null}
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-slate-100 text-left text-xs text-slate-400">
                      <th className="p-3 font-bold">Libellé</th>
                      <th className="p-3 font-bold w-44 min-w-[11rem] text-right">Montant</th>
                      {!readOnly ? <th className="p-3 w-10" aria-label="Actions" /> : null}
                    </tr>
                  </thead>
                  <tbody>
                    {(derived.recettesLignes ?? []).map((line, i) => (
                      <tr key={recetteKeysRef.current[i] ?? `recette-${i}`} className="border-b border-slate-50">
                        <td className="p-2">
                          {isApelRecetteLineIndex(i) ? (
                            <div className="rounded-lg border border-slate-200 bg-slate-50 px-2 py-1.5 text-sm font-semibold text-slate-700">
                              {line.label}
                            </div>
                          ) : (
                            <input
                              value={line.label}
                              onChange={(e) => updateRecette(i, "label", e.target.value)}
                              onFocus={beginLineEdit}
                              onBlur={() => {
                                endLineEdit();
                                pruneBlankRecette(i);
                              }}
                              readOnly={readOnly}
                              disabled={readOnly}
                              className="w-full rounded-lg border border-slate-200 px-2 py-1.5 text-sm disabled:bg-slate-50"
                              placeholder="Ex. Subvention région…"
                            />
                          )}
                        </td>
                        <td className="p-2">
                          <EuroAmountInput
                            value={line.amount}
                            onChange={(v) => updateRecette(i, "amount", v)}
                            onFocus={beginLineEdit}
                            onBlur={
                              isApelRecetteLineIndex(i)
                                ? () => endLineEdit()
                                : () => {
                                    endLineEdit();
                                    pruneBlankRecette(i);
                                  }
                            }
                            readOnly={readOnly}
                          />
                        </td>
                        {!readOnly ? (
                          <td className="p-2 text-center">
                            {isApelRecetteLineIndex(i) ? null : (
                              <button
                                type="button"
                                onClick={() => removeRecette(i)}
                                className="inline-flex h-8 w-8 items-center justify-center rounded-lg text-slate-400 hover:bg-red-50 hover:text-red-600"
                                title="Supprimer la ligne"
                                aria-label="Supprimer la ligne"
                              >
                                ✕
                              </button>
                            )}
                          </td>
                        ) : null}
                      </tr>
                    ))}
                  </tbody>
                  {lignesRecettesInfo.length > 0 ? (
                    <tfoot>
                      <tr className="bg-emerald-50/60 font-bold text-emerald-900">
                        <td className="p-3">Total recettes complémentaires</td>
                        <td className="p-3 text-right font-mono whitespace-nowrap">
                          {derived.totalSubventions != null && derived.totalSubventions > 0
                            ? `${formatEuroDisplay(derived.totalSubventions)} €`
                            : "—"}
                        </td>
                        {!readOnly ? <td className="p-3" /> : null}
                      </tr>
                    </tfoot>
                  ) : null}
                </table>
              </div>

              <div className="px-4 py-4 border-b border-slate-100 bg-white">
                <div className="flex flex-wrap items-end justify-between gap-4">
                  <div className="min-w-[10rem] flex-1">
                    <p className="text-sm font-semibold text-slate-800">Recettes élèves</p>
                    <p className="text-[10px] text-slate-500 mt-0.5">
                      {recettesFigees
                        ? "Prix annoncé × nb élèves facturés"
                        : "Budget prévisionnel total (dépenses + marge)"}
                    </p>
                  </div>
                  <label className="space-y-1 shrink-0">
                    <span className="text-[10px] font-black uppercase tracking-widest text-slate-400">
                      Date de facturation
                    </span>
                    <input
                      type="date"
                      value={sheet.facturations[0]?.dateFacturation ?? ""}
                      onChange={(e) => updateFacturationDate(e.target.value)}
                      readOnly={readOnly}
                      disabled={readOnly}
                      className="rounded-lg border border-slate-200 px-3 py-2 text-sm disabled:bg-slate-50"
                    />
                  </label>
                  <p className="font-mono font-black text-lg text-indigo-900 whitespace-nowrap">
                    {derived.recettesEleves != null ? `${formatEuroDisplay(derived.recettesEleves)} €` : "—"}
                  </p>
                </div>
              </div>

              <div className="px-4 py-3 space-y-2 border-b border-slate-100 bg-white">
                <div className="flex justify-between text-sm font-bold text-indigo-800">
                  <span>
                    Prix par élève définitif
                    <span className="block text-[10px] font-normal text-slate-500 normal-case">
                      (Budget prévisionnel − subventions) ÷ nb élèves
                    </span>
                  </span>
                  <span className="font-mono whitespace-nowrap">
                    {derived.prixParEleveAvecSubventions != null
                      ? `${formatEuroWhole(derived.prixParEleveAvecSubventions)} €`
                      : sheet.nbEleves
                        ? "#DIV/0!"
                        : "—"}
                  </span>
                </div>
              </div>

              {lignesDepensesInfo.length > 0 ? (
                <>
                  <div className="bg-slate-50 px-4 py-2 text-xs font-black uppercase tracking-widest text-slate-500 border-b border-slate-100">
                    Détail des dépenses
                    <span className="ml-2 font-normal normal-case text-slate-400">(lecture seule)</span>
                  </div>
                  <table className="w-full text-sm">
                    <thead>
                      <tr className="border-b border-slate-100 text-left text-xs text-slate-400">
                        <th className="p-3 font-bold">Libellé</th>
                        <th className="p-3 font-bold w-36 text-right">Montant</th>
                      </tr>
                    </thead>
                    <tbody>
                      {lignesDepensesInfo.map((line, i) => (
                        <tr key={i} className="border-b border-slate-50 text-slate-600">
                          <td className="p-3">{line.label.trim() || "—"}</td>
                          <td className="p-3 font-mono text-right whitespace-nowrap">
                            {line.amount != null ? `${formatEuroDisplay(line.amount)} €` : "—"}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </>
              ) : null}
            </section>

            <section className="rounded-xl border border-violet-200 overflow-hidden bg-violet-50/30">
              <div className="px-4 py-3 border-b border-violet-100 space-y-3">
                <label className={`flex items-start gap-2 ${readOnly ? "" : "cursor-pointer"}`}>
                  <input
                    type="checkbox"
                    checked={recettesFigees}
                    onChange={(e) => toggleRecettesFigees(e.target.checked)}
                    disabled={readOnly}
                    className="mt-0.5 rounded border-violet-300 text-violet-600"
                  />
                  <span className="text-sm text-violet-950">
                    <span className="font-bold">Prix déjà annoncé aux parents</span>
                    <span className="block text-[11px] font-normal text-violet-800/80 mt-0.5">
                      Les recettes restent figées. Toute dépense supplémentaire ou écart d&apos;effectif
                      se traduit en excédent ou déficit ci-dessous.
                    </span>
                  </span>
                </label>
                {recettesFigees ? (
                  <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 pl-6">
                    <label className="space-y-1">
                      <span className="text-[10px] font-black uppercase tracking-widest text-violet-700">
                        Prix annoncé / élève
                      </span>
                      <EuroAmountInput
                        value={sheet.prixParEleveAnnonce}
                        onChange={(v) => patch({ recettesElevesFigees: true, prixParEleveAnnonce: v })}
                        readOnly={readOnly}
                      />
                    </label>
                    <label className="space-y-1">
                      <span className="text-[10px] font-black uppercase tracking-widest text-violet-700">
                        Nb élèves facturés
                      </span>
                      <input
                        type="number"
                        min={0}
                        value={sheet.nbElevesFactures ?? sheet.nbEleves ?? ""}
                        onChange={(e) =>
                          patch({
                            recettesElevesFigees: true,
                            nbElevesFactures:
                              e.target.value === "" ? null : Math.max(0, Math.floor(Number(e.target.value))),
                          })
                        }
                        readOnly={readOnly}
                        disabled={readOnly}
                        className="w-full rounded-lg border border-violet-200 px-3 py-2 text-sm disabled:bg-slate-50"
                      />
                      <p className="text-[10px] text-violet-700/70">
                        Effectif réel : {sheet.nbEleves ?? "—"} élève(s)
                      </p>
                    </label>
                    <div className="flex items-end">
                      {!readOnly && (
                        <button
                          type="button"
                          onClick={figerPrixAnnonceActuel}
                          className="w-full rounded-lg border border-violet-300 bg-white px-3 py-2 text-xs font-bold text-violet-800 hover:bg-violet-50"
                        >
                          Reprendre le prix définitif actuel
                        </button>
                      )}
                    </div>
                  </div>
                ) : null}
              </div>
            </section>

            <section className="rounded-xl border border-dashed border-slate-300 overflow-hidden bg-slate-50/40">
              <div className="bg-slate-100 px-4 py-2 text-xs font-black uppercase tracking-widest text-slate-500">
                APEL — aides individuelles
                <span className="ml-2 font-normal normal-case text-slate-400">
                  (hors calcul global — rattachées à certains élèves)
                </span>
              </div>
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-slate-100 text-left text-xs text-slate-400">
                    <th className="p-3 font-bold">Nom — Prénom</th>
                    <th className="p-3 font-bold w-36">Montant</th>
                  </tr>
                </thead>
                <tbody>
                  {lignesAides.map(({ row, index }) => (
                    <tr key={index} className="border-b border-slate-50">
                      <td className="p-2">
                        <input
                          value={row.name}
                          onChange={(e) => updateAide(index, "name", e.target.value)}
                          readOnly={readOnly}
                          disabled={readOnly}
                          className="w-full rounded-lg border border-slate-200 px-2 py-1.5 text-sm bg-white disabled:bg-slate-50"
                          placeholder="Nom Prénom"
                        />
                      </td>
                      <td className="p-2">
                        <EuroAmountInput
                          value={row.amount}
                          onChange={(v) => updateAide(index, "amount", v)}
                          readOnly={readOnly}
                        />
                      </td>
                    </tr>
                  ))}
                </tbody>
                <tfoot>
                  <tr className="bg-slate-100 font-bold text-slate-600">
                    <td className="p-3">Total aides individuelles (informatif)</td>
                    <td className="p-3 text-right font-mono">
                      {derived.totalAidesIndividuelles != null
                        ? `${formatEuroDisplay(derived.totalAidesIndividuelles)} €`
                        : "—"}
                    </td>
                  </tr>
                </tfoot>
              </table>
              {!readOnly && (
                <button type="button" onClick={addAide} className="m-3 text-xs font-bold text-indigo-600">
                  + Aide individuelle
                </button>
              )}
            </section>

            <div
              className={`rounded-xl border px-4 py-3 flex justify-between items-center font-black ${
                derived.excedentOuDeficit == null
                  ? "border-slate-200 bg-slate-50 text-slate-700"
                  : derived.excedentOuDeficit >= 0
                    ? "border-emerald-200 bg-emerald-50 text-emerald-900"
                    : "border-red-200 bg-red-50 text-red-900"
              }`}
            >
              <div>
                <span>Excédent ou déficit</span>
                <span className="block text-[10px] font-normal normal-case mt-0.5 opacity-80">
                  {recettesFigees
                    ? afficheMarge
                      ? "Recettes (prix annoncé, marge incluse) + subventions − dépenses réelles"
                      : "Recettes figées + subventions − dépenses réelles"
                    : "Total recettes + subventions − total dépenses"}
                </span>
              </div>
              <span className="font-mono text-lg">
                {derived.excedentOuDeficit != null ? `${formatEuroDisplay(derived.excedentOuDeficit)} €` : "—"}
              </span>
            </div>

            {(canValidateBudget || budgetValidated) && (
              <section className="rounded-xl border-2 border-indigo-300 bg-indigo-50/60 p-5 space-y-4">
                <div>
                  <p className="text-xs font-black uppercase tracking-widest text-indigo-700">
                    Validation comptabilité
                  </p>
                  <p className="mt-1 text-sm text-slate-600">
                    Vérifiez la fiche puis validez pour figer le budget et transmettre le dossier à la direction.
                  </p>
                </div>
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 text-sm">
                  <div className="rounded-lg bg-white border border-indigo-100 px-4 py-3">
                    <p className="text-xs text-slate-500">Budget total final</p>
                    <p className="font-mono font-black text-lg text-slate-900">
                      {derived.depensesTotal != null ? `${formatEuroDisplay(derived.depensesTotal)} €` : "—"}
                    </p>
                  </div>
                  <div className="rounded-lg bg-white border border-indigo-100 px-4 py-3">
                    <p className="text-xs text-slate-500">Prix par élève définitif</p>
                    <p className="font-mono font-black text-lg text-slate-900">
                      {derived.prixParEleveAvecSubventions != null
                        ? `${formatEuroWhole(derived.prixParEleveAvecSubventions)} €`
                        : "—"}
                    </p>
                  </div>
                </div>
                {canValidateBudget ? (
                  <TripButton
                    variant="primary"
                    onClick={() => void handleValidateBudget()}
                    disabled={initialLoading || analyzing || validating || pdfBusy}
                  >
                    {validating ? "Validation…" : "Valider le budget et transmettre à la direction"}
                  </TripButton>
                ) : (
                  <p className="text-sm font-semibold text-emerald-800">
                    Budget validé — dossier en attente de validation finale par la direction.
                  </p>
                )}
              </section>
            )}

            <p className="text-center pt-1">
              <button
                type="button"
                onClick={() => setApelSummaryOpen(true)}
                className="text-[11px] text-slate-400 hover:text-slate-600 underline underline-offset-2 decoration-slate-300"
              >
                Total général à facturer à l&apos;APEL
              </button>
            </p>

            <TravelsComptaApelSummaryModal
              tripId={tripId}
              open={apelSummaryOpen}
              onClose={() => setApelSummaryOpen(false)}
              summary={apelSummary}
              loading={apelSummaryLoading}
              onRefresh={loadApelSummary}
              currentSheet={derived}
            />
          </>
        )}
      </div>
    </div>
  );
}
