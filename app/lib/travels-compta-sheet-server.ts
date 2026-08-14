import "server-only";

import { getMistralApiKey } from "@/app/lib/tenant-config";
import { runTextractForS3Key } from "@/app/lib/ocr-textract";
import { ocrS3Key, ocrPdfBytes, extractDevisMetadataWithMistral } from "@/app/lib/travel-devis-ocr";
import { getTenantBucketName } from "@/app/lib/tenant-config";
import { fetchTravelsPdfBytes, resolveTravelsS3ObjectKey } from "@/app/lib/travels-s3";
import type { TravelsTrip } from "@/app/lib/travels-types";
import {
  comptaDocumentsFingerprint,
  comptaSheetFromTrip,
  applyBusQuoteAmountFallback,
  computeComptaSheetDerived,
  depensesFromDocumentSync,
  extractLargestEuroAmountFromText,
  isUsableComptaAmount,
  listComptaDocumentRefs,
  parseEuroAmount,
  resolveComptaDepenses,
  resolveBusQuoteAmountFromTrip,
  resolveSignedBusQuoteAmount,
  resolveFacturationsFromSheet,
  resolveComptaRecettesLignes,
  type TravelsComptaDocumentScan,
  type TravelsComptaExpenseLine,
  type TravelsComptaIndividualAid,
  type TravelsComptaSheet,
  type ComptaOcrLogEntry,
  type ComptaOcrLogLevel,
} from "@/app/lib/travels-compta-sheet";

const LOG_PREFIX = "[compta-bus-ocr]";

type ComptaOcrLogger = {
  entries: ComptaOcrLogEntry[];
  info: (message: string, detail?: unknown) => void;
  warn: (message: string, detail?: unknown) => void;
  error: (message: string, detail?: unknown) => void;
};

function createComptaOcrLogger(): ComptaOcrLogger {
  const entries: ComptaOcrLogEntry[] = [];
  const push = (level: ComptaOcrLogLevel, message: string, detail?: unknown) => {
    entries.push({
      at: new Date().toISOString(),
      level,
      message,
      detail:
        detail !== undefined
          ? typeof detail === "string"
            ? detail.slice(0, 2000)
            : JSON.stringify(detail).slice(0, 2000)
          : undefined,
    });
    const fn = level === "error" ? console.error : level === "warn" ? console.warn : console.info;
    fn(`${LOG_PREFIX} ${message}`, detail ?? "");
  };
  return {
    entries,
    info: (message, detail) => push("info", message, detail),
    warn: (message, detail) => push("warn", message, detail),
    error: (message, detail) => push("error", message, detail),
  };
}

type QuoteOcrChunk = {
  label: string;
  role: "devis_signe" | "devis_activite" | "piece_jointe";
  ocrText: string;
  providerName?: string | null;
  extractedPrice?: string | null;
  fileUrl?: string | null;
};

async function ocrPdfUrl(
  fileUrl: string,
  s3Key: string | null | undefined,
  logger: ComptaOcrLogger,
  context = "",
): Promise<string> {
  const key = await resolveTravelsS3ObjectKey(fileUrl, s3Key);
  if (!key) {
    logger.warn(`Clé S3 introuvable${context ? ` (${context})` : ""}`, {
      fileUrl: fileUrl.slice(0, 160),
      explicitS3Key: s3Key || null,
    });
    return "";
  }
  const bucket = await getTenantBucketName();
  if (!bucket) {
    logger.error(`Bucket tenant absent${context ? ` (${context})` : ""}`);
    return "";
  }

  let pdfBytes: Buffer | null = null;
  try {
    pdfBytes = await fetchTravelsPdfBytes(fileUrl, s3Key);
    logger.info(`PDF téléchargé${context ? ` (${context})` : ""}`, {
      s3Key: key,
      bytes: pdfBytes.length,
    });
  } catch (dlErr) {
    logger.warn(`Téléchargement PDF impossible${context ? ` (${context})` : ""}`, {
      s3Key: key,
      error: dlErr instanceof Error ? dlErr.message : String(dlErr),
    });
  }

  const ctx = context ? ` (${context})` : "";

  const runAsyncOcr = async (): Promise<string> => {
    try {
      const asyncResult = await runTextractForS3Key(key, 90);
      const asyncText = asyncResult.text.replace(/--- Page \d+ ---/g, " ").trim();
      if (asyncText.length > 30) {
        logger.info(`OCR async OK${ctx}`, {
          s3Key: key,
          textLength: asyncText.length,
          pageCount: asyncResult.pageCount,
          preview: asyncText.slice(0, 300).replace(/\s+/g, " "),
        });
        return asyncText;
      }
      logger.warn(`OCR async vide ou trop court${ctx}`, {
        s3Key: key,
        textLength: asyncText.length,
        pageCount: asyncResult.pageCount,
      });
    } catch (err) {
      logger.warn(`OCR async module échoué, repli ocrS3Key${ctx}`, {
        s3Key: key,
        error: err instanceof Error ? err.message : String(err),
      });
      try {
        const legacyText = await ocrS3Key(bucket, key);
        if (legacyText.trim().length > 30) {
          logger.info(`OCR legacy OK${ctx}`, { s3Key: key, textLength: legacyText.length });
          return legacyText;
        }
      } catch {
        /* ignore */
      }
    }
    return "";
  };

  // PDF multi-pages / gros fichiers : OCR Mistral via la façade unifiée
  if (pdfBytes && pdfBytes.length > 200_000) {
    logger.info(`PDF volumineux — OCR async en priorité${ctx}`, { s3Key: key, bytes: pdfBytes.length });
    const asyncFirst = await runAsyncOcr();
    if (asyncFirst) return asyncFirst;
  }

  // OCR synchrone (petits PDF simples)
  if (pdfBytes && pdfBytes.length <= 10 * 1024 * 1024) {
    try {
      const syncText = await ocrPdfBytes(pdfBytes);
      if (syncText.trim().length > 30) {
        logger.info(`OCR sync direct OK${ctx}`, {
          s3Key: key,
          textLength: syncText.length,
          preview: syncText.slice(0, 300).replace(/\s+/g, " "),
        });
        return syncText;
      }
      logger.warn(`OCR sync trop court${ctx}`, { s3Key: key, textLength: syncText.length });
    } catch (syncErr) {
      const msg = syncErr instanceof Error ? syncErr.message : String(syncErr);
      const unsupported = msg.toLowerCase().includes("unsupported document format");
      logger.warn(
        unsupported
          ? `OCR sync non supporté pour ce PDF${ctx} — passage en async`
          : `OCR sync échoué${ctx}`,
        { s3Key: key, error: msg },
      );
    }
  }

  const asyncText = await runAsyncOcr();
  if (asyncText) return asyncText;

  // Dernier recours sync
  if (pdfBytes && pdfBytes.length <= 10 * 1024 * 1024) {
    try {
      const lastText = await ocrPdfBytes(pdfBytes);
      if (lastText.trim()) {
        logger.info(`OCR sync dernier recours${ctx}`, { s3Key: key, textLength: lastText.length });
        return lastText;
      }
    } catch {
      /* déjà loggé */
    }
  }

  logger.error(`Échec OCR total${ctx}`, { s3Key: key, hadPdfBytes: Boolean(pdfBytes) });
  return "";
}

async function collectQuoteOcrForTrip(
  trip: TravelsTrip,
  logger = createComptaOcrLogger(),
): Promise<QuoteOcrChunk[]> {
  const refs = listComptaDocumentRefs(trip).filter(
    (r) => r.role !== "budget_previsionnel" && r.fileUrl,
  );
  const chunks: QuoteOcrChunk[] = [];
  for (const ref of refs) {
    const text = await ocrPdfUrl(ref.fileUrl, ref.s3Key, logger);
    chunks.push({
      label: ref.label,
      role: ref.role === "devis_signe" ? "devis_signe" : ref.role === "devis_activite" ? "devis_activite" : "piece_jointe",
      ocrText: text,
      providerName: ref.role === "devis_signe"
        ? String((trip.data?.selectedBusQuote as Record<string, unknown> | undefined)?.providerName || "") || null
        : null,
      extractedPrice: ref.fallbackAmount != null ? String(ref.fallbackAmount) : null,
      fileUrl: ref.fileUrl,
    });
  }
  return chunks;
}

async function extractDocumentAmount(
  ocrText: string,
  label: string,
  role: string,
  logger: ComptaOcrLogger,
): Promise<number | null> {
  if (!ocrText.trim()) return null;

  if (role === "devis_signe") {
    const fromRegex = extractLargestEuroAmountFromText(ocrText);
    const fromQuoteMistral = await extractQuoteAmountWithMistral(ocrText, label, role);
    const meta = await extractDevisMetadataWithMistral(ocrText);
    const fromDevisMistral = parseEuroAmount(meta.price);

    const candidates = [fromRegex, fromQuoteMistral, fromDevisMistral].filter(isUsableComptaAmount);
    if (!candidates.length) {
      logger.warn("Aucun montant trouvé dans le devis bus", { label, ocrLength: ocrText.length });
      return null;
    }

    const amount = Math.max(...candidates);
    logger.info("Montant devis bus retenu", {
      label,
      amount,
      fromRegex,
      fromQuoteMistral,
      fromDevisMistral,
      rawDevisMistral: meta.price,
    });
    return amount;
  }

  const fromMistral = await extractQuoteAmountWithMistral(ocrText, label, role);
  if (fromMistral != null) return fromMistral;
  return extractLargestEuroAmountFromText(ocrText);
}

async function ocrBusQuoteAmount(
  trip: TravelsTrip,
  ref: {
    fileUrl: string;
    s3Key?: string | null;
    label: string;
    fallbackAmount?: number | null;
  },
  logger: ComptaOcrLogger,
): Promise<{ amount: number | null; ocrText: string }> {
  const tripAmount = resolveBusQuoteAmountFromTrip(trip);
  const fallback = ref.fallbackAmount ?? tripAmount ?? null;

  logger.info("Début extraction montant bus", {
    label: ref.label,
    fileUrl: ref.fileUrl.slice(0, 160),
    s3Key: ref.s3Key || null,
    fallbackAmount: ref.fallbackAmount ?? null,
    tripAmount,
  });

  const selected = trip.data?.selectedBusQuote as Record<string, unknown> | undefined;
  const originalUrl = String(selected?.fileUrl || "");
  const ocrSources: { name: string; text: string }[] = [];

  if (originalUrl && originalUrl !== ref.fileUrl) {
    const ocrOriginal = await ocrPdfUrl(
      originalUrl,
      selected?.s3KeyIncoming ? String(selected.s3KeyIncoming) : null,
      logger,
      "devis original",
    );
    if (ocrOriginal) ocrSources.push({ name: "devis original", text: ocrOriginal });
    else logger.warn("OCR vide sur le devis original", { originalUrl: originalUrl.slice(0, 160) });
  } else {
    logger.info("Pas de devis original distinct", {
      originalUrl: originalUrl.slice(0, 80) || "(vide)",
      signedUrl: ref.fileUrl.slice(0, 80),
    });
  }

  const ocrSigned = await ocrPdfUrl(ref.fileUrl, ref.s3Key, logger, "devis signé");
  if (ocrSigned) ocrSources.push({ name: "devis signé", text: ocrSigned });
  else logger.warn("OCR vide sur le devis signé", { fileUrl: ref.fileUrl.slice(0, 160) });

  const ocrText = ocrSources.map((s) => s.text).filter(Boolean).join("\n\n");

  for (const source of ocrSources) {
    const extracted = await extractDocumentAmount(source.text, ref.label, "devis_signe", logger);
    if (extracted != null) {
      logger.info(`Montant retenu depuis ${source.name}`, { amount: extracted });
      return { amount: extracted, ocrText };
    }
    const regexAmount = extractLargestEuroAmountFromText(source.text);
    if (regexAmount != null) {
      logger.info(`Montant regex depuis ${source.name}`, { amount: regexAmount });
      return { amount: regexAmount, ocrText };
    }
  }

  if (isUsableComptaAmount(fallback)) {
    logger.info("Montant retenu depuis le dossier voyage (sans OCR)", { amount: fallback });
    return { amount: fallback, ocrText };
  }

  logger.error("Échec total — montant bus introuvable", {
    ocrSourcesTried: ocrSources.map((s) => ({ name: s.name, length: s.text.length })),
    selectedBusQuote: selected
      ? {
          providerName: selected.providerName,
          extractedPrice: selected.extractedPrice,
          price: selected.price,
        }
      : null,
  });
  return { amount: null, ocrText };
}

async function extractQuoteAmountWithMistral(
  ocrText: string,
  label: string,
  role: string,
): Promise<number | null> {
  const mistralKey = await getMistralApiKey();
  if (!ocrText.trim() || !mistralKey) return null;
  try {
    const res = await fetch("https://api.mistral.ai/v1/chat/completions", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${mistralKey}`,
      },
      body: JSON.stringify({
        model: "mistral-small-latest",
        temperature: 0.1,
        response_format: { type: "json_object" },
        messages: [
          {
            role: "system",
            content:
              "Tu extrais le montant TTC principal d'un devis scolaire (transport, entrée, activité…). " +
              "Réponds UNIQUEMENT en JSON: { montant_ttc: number|null }. " +
              "montant_ttc = nombre en euros sans symbole. null si introuvable.",
          },
          {
            role: "user",
            content: `Document: ${label} (${role})\n\nOCR:\n${ocrText.slice(0, 4000)}`,
          },
        ],
      }),
    });
    const data = (await res.json()) as Record<string, unknown>;
    if (!res.ok) return null;
    const raw =
      (data.choices as { message?: { content?: string } }[] | undefined)?.[0]?.message?.content?.trim() || "{}";
    const parsed = JSON.parse(raw) as { montant_ttc?: unknown };
    return parseEuroAmount(parsed.montant_ttc);
  } catch {
    return null;
  }
}

type ComptaSyncOptions = {
  forceBusOcr?: boolean;
};

type ComptaSyncResult = {
  sheet: TravelsComptaSheet;
  ocrNewCount: number;
  removedCount: number;
  synced: boolean;
  notes: string;
  debugLog: ComptaOcrLogEntry[];
};

/** Sync auto : structure des dépenses + OCR uniquement sur les documents nouveaux ou modifiés. */
export async function syncComptaSheetWithDocuments(
  trip: TravelsTrip,
  existing: TravelsComptaSheet | null,
  options: ComptaSyncOptions = {},
): Promise<ComptaSyncResult> {
  const logger = createComptaOcrLogger();
  logger.info("Synchronisation fiche compta démarrée", {
    tripId: trip.id,
    forceBusOcr: Boolean(options.forceBusOcr),
    hasSignedQuote: Boolean(trip.data?.signedQuoteUrl),
    provider: (trip.data?.selectedBusQuote as Record<string, unknown> | undefined)?.providerName,
  });

  const base = comptaSheetFromTrip(trip, existing);
  const refs = listComptaDocumentRefs(trip);
  const scannableRefs = refs.filter((r) => r.role !== "budget_previsionnel" && r.fileUrl);
  const isFirstSync = !existing?.analyzedAt;

  const prevScans = existing?.documentScans || [];
  const refKeySet = new Set(refs.map((r) => r.key));
  let keptScans = prevScans.filter((s) => refKeySet.has(s.key));
  if (options.forceBusOcr) {
    keptScans = keptScans.filter((s) => s.key !== "devis_signe");
    logger.info("Relance OCR bus forcée — cache devis_signe effacé");
  }
  const removedCount = prevScans.length - keptScans.length + (options.forceBusOcr ? 1 : 0);
  const scansByKey = new Map(keptScans.map((s) => [s.key, s]));

  let toScan = isFirstSync
    ? scannableRefs
    : scannableRefs.filter((ref) => {
        const prev = scansByKey.get(ref.key);
        if (!prev) return true;
        if (prev.label !== ref.label) return true;
        if (ref.role === "devis_signe") {
          if (options.forceBusOcr) return true;
          if (prev.fileUrl !== ref.fileUrl) return true;
          if (!isUsableComptaAmount(prev.amount)) return true;
          return false;
        }
        if (!isUsableComptaAmount(prev.amount) && ref.fallbackAmount == null) return true;
        return false;
      });

  if (!toScan.length) {
    logger.info("Aucun nouveau document — OCR non relancé (cache conservé)");
  }

  const prevBusScan = existing?.documentScans?.find((s) => s.key === "devis_signe");

  let ocrNewCount = 0;
  const newScans: TravelsComptaDocumentScan[] = [...keptScans];
  const freshOcrChunks: QuoteOcrChunk[] = [];

  for (const ref of toScan) {
    ocrNewCount++;
    let amount = ref.fallbackAmount ?? null;
    let ocrText = "";

    if (ref.role === "devis_signe") {
      const busOcr = await ocrBusQuoteAmount(trip, ref, logger);
      amount = busOcr.amount;
      ocrText = busOcr.ocrText;
      if (!isUsableComptaAmount(amount) && isUsableComptaAmount(prevBusScan?.amount)) {
        amount = prevBusScan!.amount!;
        logger.info("Montant bus conservé depuis le cache (nouvel OCR sans résultat)", { amount });
      }
      logger.info("Fin scan devis_signe", { label: ref.label, amount, ocrTextLength: ocrText.length });
    } else {
      ocrText = await ocrPdfUrl(ref.fileUrl, ref.s3Key, logger);
      if (ocrText) {
        const extracted = await extractDocumentAmount(ocrText, ref.label, ref.role, logger);
        if (extracted != null) amount = extracted;
      }
    }

    if (ocrText) {
      freshOcrChunks.push({
        label: ref.label,
        role:
          ref.role === "devis_signe"
            ? "devis_signe"
            : ref.role === "devis_activite"
              ? "devis_activite"
              : "piece_jointe",
        ocrText,
        providerName:
          ref.role === "devis_signe"
            ? String(
                (trip.data?.selectedBusQuote as Record<string, unknown> | undefined)?.providerName || "",
              ) || null
            : null,
        extractedPrice: amount != null ? String(amount) : ref.fallbackAmount != null ? String(ref.fallbackAmount) : null,
        fileUrl: ref.fileUrl,
      });
    }
    const scan: TravelsComptaDocumentScan = {
      key: ref.key,
      fileUrl: ref.fileUrl,
      label: ref.label,
      role: ref.role,
      amount:
        ref.role === "devis_signe" && !isUsableComptaAmount(amount)
          ? resolveBusQuoteAmountFromTrip(trip)
          : amount,
      scannedAt: new Date().toISOString(),
    };
    const idx = newScans.findIndex((s) => s.key === ref.key);
    if (idx >= 0) newScans[idx] = scan;
    else newScans.push(scan);
  }

  const budgetRef = refs.find((r) => r.role === "budget_previsionnel");
  if (budgetRef) {
    const scan: TravelsComptaDocumentScan = {
      key: "budget_previsionnel",
      fileUrl: "",
      label: budgetRef.label,
      role: "budget_previsionnel",
      amount: budgetRef.fallbackAmount ?? null,
      scannedAt: new Date().toISOString(),
    };
    const idx = newScans.findIndex((s) => s.key === "budget_previsionnel");
    if (idx >= 0) {
      newScans[idx] = { ...newScans[idx], label: scan.label, amount: newScans[idx].amount ?? scan.amount };
    } else {
      newScans.push(scan);
    }
  } else {
    const bi = newScans.findIndex((s) => s.key === "budget_previsionnel");
    if (bi >= 0) newScans.splice(bi, 1);
  }

  const fingerprint = comptaDocumentsFingerprint(trip);
  const tripBusAmount = resolveBusQuoteAmountFromTrip(trip);
  if (tripBusAmount != null) {
    const busIdx = newScans.findIndex((s) => s.key === "devis_signe");
    if (busIdx >= 0 && !isUsableComptaAmount(newScans[busIdx].amount)) {
      newScans[busIdx] = { ...newScans[busIdx], amount: tripBusAmount };
    }
  }

  let sheet: TravelsComptaSheet;

  if (isFirstSync) {
    const ocrChunks =
      freshOcrChunks.length > 0 ? freshOcrChunks : await collectQuoteOcrForTrip(trip, logger);
    sheet = await extractComptaSheetWithAi(trip, ocrChunks, base);
    sheet = computeComptaSheetDerived({
      ...sheet,
      depenses: applyBusQuoteAmountFallback(
        trip,
        depensesFromDocumentSync(trip, newScans, sheet.depenses),
        sheet,
      ),
      documentScans: newScans,
      syncedDocumentsFingerprint: fingerprint,
      syncedAt: new Date().toISOString(),
      ocrDebugLog: logger.entries.slice(-80),
      facteurRisque: sheet.facteurRisque ?? base.facteurRisque,
      margeSecuriteEuro: sheet.margeSecuriteEuro ?? base.margeSecuriteEuro,
    }, trip);
  } else {
    sheet = computeComptaSheetDerived({
      ...base,
      depenses: applyBusQuoteAmountFallback(
        trip,
        depensesFromDocumentSync(trip, newScans, existing?.depenses),
        base,
      ),
      documentScans: newScans,
      syncedDocumentsFingerprint: fingerprint,
      syncedAt: new Date().toISOString(),
      ocrDebugLog: logger.entries.slice(-80),
      margeSecuriteEuro: base.margeSecuriteEuro,
    }, trip);
  }

  const finalBus = sheet.depenses.find((d) => d.source === "devis_signe");
  logger.info("Synchronisation terminée", {
    busMontant: finalBus?.amount ?? null,
    busLabel: finalBus?.label ?? null,
    depensesTotal: sheet.depensesTotal,
    documentsScannes: ocrNewCount,
  });

  const notes: string[] = [];
  if (ocrNewCount > 0) notes.push(`${ocrNewCount} document(s) lu(s)`);
  if (removedCount > 0) notes.push(`${removedCount} document(s) retiré(s)`);
  if (!isUsableComptaAmount(finalBus?.amount)) {
    notes.push("Montant transport non détecté — voir journal OCR ci-dessous");
  }
  if (!notes.length) notes.push("Dossier à jour");

  return {
    sheet: { ...sheet, ocrDebugLog: logger.entries.slice(-80) },
    ocrNewCount,
    removedCount,
    synced: true,
    notes: notes.join(" · "),
    debugLog: logger.entries.slice(-80),
  };
}

async function extractComptaSheetWithAi(
  trip: TravelsTrip,
  ocrChunks: QuoteOcrChunk[],
  baseSheet: TravelsComptaSheet,
): Promise<TravelsComptaSheet> {
  const mistralKey = await getMistralApiKey();
  if (!mistralKey) {
    return {
      ...computeComptaSheetDerived(baseSheet, trip),
      analysisNotes: "Clé Mistral absente — préremplissage depuis le dossier uniquement.",
    };
  }

  const ocrBlock = ocrChunks
    .map(
      (c) =>
        `### ${c.label} (${c.role})\nFournisseur: ${c.providerName || "?"}\nPrix déjà extrait: ${c.extractedPrice || "?"}\nOCR:\n${c.ocrText.slice(0, 2500)}`,
    )
    .join("\n\n");

  const tripContext = {
    titre: trip.data?.title,
    destination: trip.data?.destination,
    etablissement: trip.data?.etablissement,
    classes: trip.data?.classes,
    nb_eleves: trip.data?.nbEleves,
    nb_accompagnateurs: trip.data?.nbAccompagnateurs,
    noms_accompagnateurs: trip.data?.nomsAccompagnateurs,
    budget_previsionnel: trip.data?.coutTotal,
    createur: trip.ownerName,
  };

  try {
    const res = await fetch("https://api.mistral.ai/v1/chat/completions", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${mistralKey}`,
      },
      body: JSON.stringify({
        model: "mistral-small-latest",
        temperature: 0.1,
        response_format: { type: "json_object" },
        messages: [
          {
            role: "system",
            content:
              "Tu aides la comptabilité d'un établissement scolaire à remplir une fiche budget voyage. " +
              "À partir du dossier et des OCR fournis, extrais les montants TTC en euros (nombres, pas de symbole €). " +
              "RÈGLES DÉPENSES IMPORTANTES : " +
              "(1) UNE SEULE ligne transport/bus/autocar : uniquement à partir du devis bus SIGNÉ (rôle devis_signe). " +
              "(2) NE PAS inclure les autres devis transport reçus par mail (devis concurrents non signés). " +
              "(3) Les autres lignes de dépenses viennent des pièces jointes (rôle piece_jointe) et des devis activités (rôle devis_activite) : entrées musée, activités, restauration hors bus, etc. " +
              "Réponds UNIQUEMENT en JSON avec les clés: " +
              "compte (string|null), ligne (string|null), classe (string|null), profs (string|null), accompagnateurs (string|null), " +
              "depenses (array de {label, montant, source}), nb_eleves (number|null), " +
              "apel_aides_collectives (number|null), autres_subventions (number|null), " +
              "aides_individuelles (array de {nom, montant}), " +
              "facturation ({prix_facture = montant du devis bus signé, date_facturation YYYY-MM-DD ou '', montant = montant de facturation reçu, PAS le devis bus}), " +
              "notes (string court). " +
              "Si une info manque, mets null ou chaîne vide. Ne invente pas de subventions APEL.",
          },
          {
            role: "user",
            content: `Dossier voyage:\n${JSON.stringify(tripContext, null, 2)}\n\nDevis OCR:\n${ocrBlock || "(aucun devis OCR — utiliser le budget prévisionnel)"}`,
          },
        ],
      }),
    });

    const data = (await res.json()) as Record<string, unknown>;
    if (!res.ok) {
      return {
        ...computeComptaSheetDerived(baseSheet, trip),
        analysisNotes: "Analyse IA indisponible — vérifiez les champs manuellement.",
      };
    }

    const raw =
      (data.choices as { message?: { content?: string } }[] | undefined)?.[0]?.message?.content?.trim() || "{}";
    const parsed = JSON.parse(raw) as Record<string, unknown>;

    const depensesRaw = Array.isArray(parsed.depenses) ? parsed.depenses : [];
    const depensesMapped: TravelsComptaExpenseLine[] = depensesRaw.map((d) => {
      const row = d as Record<string, unknown>;
      return {
        label: String(row.label || "").trim(),
        amount: parseEuroAmount(row.montant ?? row.amount),
        source: row.source ? String(row.source) : null,
      };
    });
    const depenses = resolveComptaDepenses(trip, depensesMapped);

    const individuellesRaw = Array.isArray(parsed.aides_individuelles) ? parsed.aides_individuelles : [];
    const aidesIndividuelles: TravelsComptaIndividualAid[] = individuellesRaw.map((a) => {
      const row = a as Record<string, unknown>;
      return {
        name: String(row.nom || row.name || "").trim(),
        amount: parseEuroAmount(row.montant ?? row.amount),
      };
    });
    if (aidesIndividuelles.length === 0) aidesIndividuelles.push({ name: "", amount: null });

    const fact = (parsed.facturation || {}) as Record<string, unknown>;
    const baseFacts = resolveFacturationsFromSheet(baseSheet);
    const facturations = [...baseFacts];
    facturations[0] = {
      ...facturations[0],
      prixFacture:
        parseEuroAmount(fact.prix_facture) ??
        facturations[0]?.prixFacture ??
        resolveSignedBusQuoteAmount(trip, baseSheet),
      dateFacturation: String(
        fact.date_facturation ?? facturations[0]?.dateFacturation ?? "",
      ).trim(),
      montant: parseEuroAmount(fact.montant) ?? facturations[0]?.montant,
    };

    const recettesLignes = [...resolveComptaRecettesLignes(baseSheet)];
    const apelAi = parseEuroAmount(parsed.apel_aides_collectives);
    const autresAi = parseEuroAmount(parsed.autres_subventions);
    if (isUsableComptaAmount(apelAi)) {
      recettesLignes[0] = { label: "APEL", amount: apelAi };
    }
    if (isUsableComptaAmount(autresAi)) {
      recettesLignes.push({ label: "Autres subventions", amount: autresAi });
    }

    const sheet: TravelsComptaSheet = {
      ...baseSheet,
      compte: String(parsed.compte ?? baseSheet.compte ?? "").trim(),
      ligne: String(parsed.ligne ?? baseSheet.ligne ?? "").trim(),
      classe: String(parsed.classe ?? baseSheet.classe ?? "").trim(),
      profs: String(parsed.profs ?? baseSheet.profs ?? "").trim(),
      accompagnateurs: String(parsed.accompagnateurs ?? baseSheet.accompagnateurs ?? "").trim(),
      depenses,
      nbEleves: parseEuroAmount(parsed.nb_eleves) ?? baseSheet.nbEleves,
      recettesLignes,
      aidesIndividuelles,
      facturations,
      analyzedAt: new Date().toISOString(),
      analysisNotes: parsed.notes ? String(parsed.notes).slice(0, 500) : null,
    };

    return computeComptaSheetDerived(sheet, trip);
  } catch {
    return {
      ...computeComptaSheetDerived(baseSheet, trip),
      analysisNotes: "Erreur lors de l'analyse IA — champs préremplis depuis le dossier.",
    };
  }
}
