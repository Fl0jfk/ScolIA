'use client';
import { Suspense, useState, useEffect, useRef, useCallback, useMemo } from "react";
import { useSearchParams } from "next/navigation";
import { useUser } from "@clerk/nextjs";
import * as msal from "@azure/msal-browser";
import { consumeDashboardUpload } from "@/app/lib/dashboard-upload-bridge";
import { getOneDriveProfileForClerkUser } from "@/app/lib/onedrive-user-profiles";
import ReplayModuleTourButton from "@/app/components/module-tour/ReplayModuleTourButton";
import {
  ONEDRIVE_MSAL_SCOPES,
  buildOneDriveMsalConfig,
  fetchMicrosoftOneDrivePublicConfig,
  storeMsalReturnPath,
} from "@/app/lib/msal-onedrive-client";
import {
  obtainValidOneDriveAccessToken,
  pickCachedAccessToken,
  tryRestoreOneDriveAccessToken,
} from "@/app/lib/onedrive-msal-session";
import { graphDriveRootItemUrl } from "@/app/lib/graph-onedrive-path";

const ONEDRIVE_SCOPES = [...ONEDRIVE_MSAL_SCOPES];

let msalInstance: msal.PublicClientApplication | null = null;

function getMsalInstance() {
  if (!msalInstance) throw new Error("MSAL non initialisé");
  return msalInstance;
}

function getMsalActiveAccount(): msal.AccountInfo | null {
  const instance = getMsalInstance();
  return instance.getActiveAccount() ?? instance.getAllAccounts()[0] ?? null;
}

function setMsalActiveAccount(account: msal.AccountInfo | null) {
  if (account) getMsalInstance().setActiveAccount(account);
}

async function obtainValidOneDriveToken(account: msal.AccountInfo): Promise<string> {
  return obtainValidOneDriveAccessToken(getMsalInstance(), account);
}

async function restoreOneDriveToken(account: msal.AccountInfo): Promise<string | null> {
  return tryRestoreOneDriveAccessToken(getMsalInstance(), account);
}

type ProcessResult = {
  success: boolean;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  result?: any;
  error?: string;
  fileName: string;
  /** Chemin OneDrive (ex. Temp/monfichier.pdf) si le fichier est resté dans Temp */
  tempOneDrivePath?: string;
};

const INITIAL_OCR_PROCESSING_STATUS = {
  percent: 0,
  total: 0,
  done: 0,
  completed: 0,
  failed: 0,
  totalKnown: false,
  label: "",
};

const BATCH_JOB_STORAGE_KEY = "agentIAOCR-active-batch-job";
/** Dernier lot terminé — conservé jusqu'au prochain dépôt de fichiers. */
const BATCH_JOB_LAST_RESULTS_KEY = "agentIAOCR-last-batch-job";

type OcrProgressDetail = {
  percent: number;
  label: string;
  phase: string;
  phaseLabel: string;
  fileName: string | null;
  fileIndex: number;
  fileTotal: number;
  pageCount: number | null;
  pdfPageCount: number | null;
  ocrPagesRead: number | null;
  segmentIndex: number | null;
  segmentTotal: number | null;
  segmentationEngine: "identity" | "mistral" | "mistral_chunked" | "heuristic" | null;
  documentsProcessed: number;
  documentsSucceeded: number;
  documentsFailed: number;
  updatedAt: string;
  idleSeconds: number;
};

type OcrServerTraceEntry = {
  t: string;
  scope: string;
  phase: string;
  level: string;
  message: string;
  data?: Record<string, unknown>;
};

/** Ne jamais faire régresser la liste de résultats affichée (poll S3 parfois en retard). */
function mergeOcrResultsForUi(prev: ProcessResult[], incoming: ProcessResult[]): ProcessResult[] {
  if (incoming.length === 0 && prev.length > 0) return prev;
  const byName = new Map<string, ProcessResult>();
  for (const r of prev) byName.set(r.fileName, r);
  for (const r of incoming) {
    const ex = byName.get(r.fileName);
    if (!ex) byName.set(r.fileName, r);
    else if (r.success && !ex.success) byName.set(r.fileName, r);
    else if (incoming.length >= prev.length) byName.set(r.fileName, r);
  }
  if (incoming.length >= prev.length) {
    return incoming.map((r) => byName.get(r.fileName) ?? r);
  }
  const order = [...prev.map((r) => r.fileName), ...incoming.map((r) => r.fileName)];
  const seen = new Set<string>();
  const merged: ProcessResult[] = [];
  for (const name of order) {
    if (seen.has(name)) continue;
    const row = byName.get(name);
    if (row) {
      merged.push(row);
      seen.add(name);
    }
  }
  return merged;
}

type BatchJobStatusPayload = {
  jobId?: string;
  status?: string;
  label?: string;
  percent?: number;
  currentItemIndex?: number;
  totalItems?: number;
  completed?: number;
  failed?: number;
  results?: ProcessResult[];
  error?: string | null;
  serverManaged?: boolean;
  serverSelfRelays?: boolean;
  traceLog?: OcrServerTraceEntry[];
  progress?: OcrProgressDetail;
};

function OneDriveUpDocsOCRAIContent() {
  const searchParams = useSearchParams();
  const { user: clerkUser } = useUser();
  const oneDriveProfile = useMemo(
    () => (clerkUser ? getOneDriveProfileForClerkUser(clerkUser) : null),
    [clerkUser],
  );
  const [account, setAccount] = useState<msal.AccountInfo | null>(null);
  const [accessToken, setAccessToken] = useState<string | null>(null);
  const [error, setError] = useState("");
  const [msalReady, setMsalReady] = useState(false);
  const [ocrProcessing, setOcrProcessing] = useState(false);
  const [ocrResults, setOcrResults] = useState<ProcessResult[]>([]);
  const [processingStatus, setProcessingStatus] = useState(INITIAL_OCR_PROCESSING_STATUS);
  const [ocrResultsSessionId, setOcrResultsSessionId] = useState(0);
  const [pendingClassFiles, setPendingClassFiles] = useState<File[]>([]);
  const [isDraggingClass, setIsDraggingClass] = useState(false);

  const [elevesCount, setElevesCount] = useState<number | null>(null);
  const [syncingFolders, setSyncingFolders] = useState(false);
  const [syncReport, setSyncReport] = useState<{
    message?: string;
    secteurLabel?: string;
    basePath?: string;
    jsonForYourSecteur?: number;
    created?: number;
    alreadyThere?: number;
    createdFolders?: string[];
    extraFoldersCount?: number;
    extraFoldersOnOneDrive?: string[];
    ambiguousCount?: number;
    ambiguous?: Array<{ folderName: string; mef?: string; reason?: string }>;
    errors?: Array<{ folderName: string; error: string }>;
    otherSecteurCounts?: Record<string, number>;
    mefTableConfigured?: boolean;
  } | null>(null);
  const [mefCounts, setMefCounts] = useState<{ total: number; lycee: number; college: number; ecole: number } | null>(
    null,
  );
  const [mefUploading, setMefUploading] = useState(false);
  const [mefMessage, setMefMessage] = useState("");
  const mefInputRef = useRef<HTMLInputElement | null>(null);
  const classInputRef = useRef<HTMLInputElement | null>(null);
  const oneDriveTokenRef = useRef<string | null>(null);
  const ocrSessionIdRef = useRef(0);
  const ocrAbortRef = useRef<AbortController | null>(null);
  const ocrProcessingRef = useRef(false);
  const processingLockRef = useRef(false);
  const activeBatchJobIdRef = useRef<string | null>(null);
  /** Pics monotones par session : empêchent les compteurs UI de régresser (poll S3 en retard). */
  const progressPeakRef = useRef<{ percent: number; totalDocs: number }>({ percent: 0, totalDocs: 0 });
  const [checkingOneDrive, setCheckingOneDrive] = useState(false);
  const [oneDriveVerified, setOneDriveVerified] = useState(false);
  const [activeBatchJobId, setActiveBatchJobId] = useState<string | null>(null);
  const [batchJobNeedsToken, setBatchJobNeedsToken] = useState(false);
  const [batchPollIssue, setBatchPollIssue] = useState<"offline" | "auth" | null>(null);
  const [batchServerSelfRelays, setBatchServerSelfRelays] = useState(false);
  const [progressDetail, setProgressDetail] = useState<OcrProgressDetail | null>(null);
  const [openingOneDrivePath, setOpeningOneDrivePath] = useState<string | null>(null);

  const applyOneDriveSession = useCallback((activeAccount: msal.AccountInfo | null, token: string | null) => {
    setAccount(activeAccount);
    setAccessToken(token);
    oneDriveTokenRef.current = token;
    setOneDriveVerified(Boolean(token));
  }, []);

  const ensureOneDriveConnection = useCallback(async (): Promise<string | null> => {
    if (!msalReady) {
      setError("Initialisation Microsoft en cours… Réessayez dans quelques secondes.");
      return null;
    }
    setCheckingOneDrive(true);
    try {
      const activeAccount = getMsalActiveAccount();
      if (!activeAccount) {
        applyOneDriveSession(null, null);
        setError("Connectez-vous à OneDrive avant de déposer des fichiers (bouton en haut à droite).");
        return null;
      }
      setMsalActiveAccount(activeAccount);

      const cachedToken = pickCachedAccessToken(oneDriveTokenRef.current);
      if (cachedToken) {
        applyOneDriveSession(activeAccount, cachedToken);
        setError("");
        return cachedToken;
      }

      const token = await obtainValidOneDriveToken(activeAccount);
      applyOneDriveSession(activeAccount, token);
      setError("");
      return token;
    } catch (err: unknown) {
      const account = getMsalActiveAccount();
      applyOneDriveSession(account, null);
      const msg = err instanceof Error ? err.message : String(err);
      setError(`OneDrive indisponible : ${msg}`);
      return null;
    } finally {
      setCheckingOneDrive(false);
    }
  }, [applyOneDriveSession, msalReady]);

  const loadElevesCount = useCallback(async () => {
    try {
      const res = await fetch("/api/eleves");
      if (!res.ok) return;
      const data = await res.json();
      setElevesCount(data.count ?? (Array.isArray(data.eleves) ? data.eleves.length : null));
    } catch {
      /* ignore */
    }
  }, []);

  const loadMefCounts = useCallback(async () => {
    try {
      const res = await fetch("/api/mef-secteurs");
      if (!res.ok) return;
      const data = await res.json();
      if (data.counts) setMefCounts(data.counts);
    } catch {
      /* ignore */
    }
  }, []);

  useEffect(() => {
    const init = async () => {
      try {
        const ms = await fetchMicrosoftOneDrivePublicConfig();
        if (!ms) {
          setError("OneDrive n'est pas activé pour cet établissement. Activez-le dans Paramètres → Intégrations.");
          setMsalReady(true);
          return;
        }
        msalInstance = new msal.PublicClientApplication(buildOneDriveMsalConfig(ms));
        await getMsalInstance().initialize();

        const redirectResult = await getMsalInstance().handleRedirectPromise();
        if (redirectResult?.account) {
          setMsalActiveAccount(redirectResult.account);
          try {
            const token = await restoreOneDriveToken(redirectResult.account);
            applyOneDriveSession(redirectResult.account, token);
          } catch {
            applyOneDriveSession(redirectResult.account, null);
          }
        }

        setMsalReady(true);
        const accounts = getMsalInstance().getAllAccounts();
        if (!redirectResult?.account && accounts.length > 0) {
          setMsalActiveAccount(accounts[0]);
          try {
            const token = await restoreOneDriveToken(accounts[0]);
            applyOneDriveSession(accounts[0], token);
          } catch {
            applyOneDriveSession(accounts[0], null);
          }
        }
        await loadElevesCount();
        await loadMefCounts();
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
      } catch (err: any) {
        setError("Erreur init MSAL: " + err.message);
      }
    };
    init();
  }, [applyOneDriveSession, loadElevesCount, loadMefCounts]);

  useEffect(() => {
    if (searchParams.get("upload") !== "1") return;
    requestAnimationFrame(() => {
      const el = document.getElementById("ocr-drop-standard");
      el?.scrollIntoView({ behavior: "smooth", block: "center" });
      el?.classList.add("ring-4", "ring-violet-400");
      window.setTimeout(() => el?.classList.remove("ring-4", "ring-violet-400"), 2500);
    });
  }, [searchParams, msalReady]);

  useEffect(() => {
    if (!msalReady) return;
    const staged = consumeDashboardUpload();
    if (!staged) return;
    (async () => {
      const token = await ensureOneDriveConnection();
      if (!token) return;
      setPendingClassFiles((prev) => [...prev, ...staged.files]);
    })();
  }, [ensureOneDriveConnection, msalReady]);

  useEffect(() => {
    ocrProcessingRef.current = ocrProcessing;
  }, [ocrProcessing]);

  useEffect(() => {
    activeBatchJobIdRef.current = activeBatchJobId;
  }, [activeBatchJobId]);

  const abortOcrInFlight = useCallback(() => {
    ocrAbortRef.current?.abort();
    ocrAbortRef.current = new AbortController();
  }, []);

  const isActiveOcrSession = useCallback((sessionId: number) => sessionId === ocrSessionIdRef.current, []);

  const resetOcrSessionUi = useCallback((clearResults: boolean) => {
    if (clearResults) setOcrResults([]);
    setError("");
    setProgressDetail(null);
    setProcessingStatus(INITIAL_OCR_PROCESSING_STATUS);
    if (classInputRef.current) classInputRef.current.value = "";
  }, []);

  // Nouveau lot → on repart de zéro pour les pics monotones (sinon le total d'un lot précédent reste).
  useEffect(() => {
    progressPeakRef.current = { percent: 0, totalDocs: 0 };
  }, [activeBatchJobId]);

  const prepareOcrSessionForNewBatch = useCallback(() => {
    abortOcrInFlight();
    resetOcrSessionUi(true);
    setPendingClassFiles([]);
    setActiveBatchJobId(null);
    activeBatchJobIdRef.current = null;
    localStorage.removeItem(BATCH_JOB_STORAGE_KEY);
    localStorage.removeItem(BATCH_JOB_LAST_RESULTS_KEY);
  }, [abortOcrInFlight, resetOcrSessionUi]);

  const applyBatchJobStatusToUi = useCallback((st: BatchJobStatusPayload, jobId?: string | null) => {
    const incomingJobId = jobId ?? st.jobId ?? null;
    if (incomingJobId && activeBatchJobIdRef.current !== incomingJobId) return;

    const pct = st.progress?.percent ?? st.percent;
    const succeeded =
      st.progress?.documentsSucceeded ??
      (typeof st.completed === "number" ? st.completed : undefined);
    const failedCount =
      st.progress?.documentsFailed ?? (typeof st.failed === "number" ? st.failed : undefined);

    setProcessingStatus((prev) => ({
      percent: typeof pct === "number" ? pct : 0,
      label: st.progress?.label || st.label || "",
      done: typeof st.currentItemIndex === "number" ? st.currentItemIndex : 0,
      total: typeof st.totalItems === "number" ? st.totalItems : 0,
      totalKnown: true,
      completed:
        typeof succeeded === "number"
          ? Math.max(prev.completed, succeeded)
          : prev.completed,
      failed:
        typeof failedCount === "number" ? Math.max(prev.failed, failedCount) : prev.failed,
    }));
    setProgressDetail(st.progress ?? null);
    if (Array.isArray(st.results)) {
      setOcrResults((prev) => mergeOcrResultsForUi(prev, st.results!));
    }
  }, []);

  const persistFinishedBatchJob = useCallback((jobId: string) => {
    localStorage.removeItem(BATCH_JOB_STORAGE_KEY);
    localStorage.setItem(BATCH_JOB_LAST_RESULTS_KEY, jobId);
  }, []);

  const canAcceptNewOcrFiles = useCallback(() => {
    if (ocrProcessingRef.current || processingLockRef.current || activeBatchJobId) {
      setError(
        "Un traitement est encore en cours (sur le serveur). Attendez la fin ou consultez les échecs ci-dessous avant de déposer de nouveaux fichiers.",
      );
      return false;
    }
    return true;
  }, [activeBatchJobId]);

  const login = async () => {
    if (!msalReady) {
      setError("MSAL n'est pas encore initialisé.");
      return;
    }
    setError("");
    try {
      storeMsalReturnPath();
      await getMsalInstance().loginRedirect({
        scopes: ONEDRIVE_SCOPES,
        prompt: "select_account",
      });
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
    } catch (err: any) {
      if (err instanceof msal.BrowserAuthError && err.errorCode === "interaction_in_progress") {
        setError("Une connexion Microsoft est déjà en cours. Patientez quelques secondes puis réessayez.");
        return;
      }
      setError(
        `Erreur connexion OneDrive : ${err?.message || "échec inconnu"}. Choisissez votre compte professionnel (@ac-normandie.fr ou @laprovidence-nicolasbarre.fr).`,
      );
    }
  };

  const reconnectOneDrive = async () => {
    if (!msalReady) return;
    setError("");
    try {
      storeMsalReturnPath();
      const activeAccount = getMsalActiveAccount();
      await getMsalInstance().loginRedirect({
        scopes: ONEDRIVE_SCOPES,
        prompt: "consent",
        ...(activeAccount ? { account: activeAccount } : {}),
      });
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : "Échec de la reconnexion OneDrive.");
    }
  };

  const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

  const parseRetryAfterMs = (res: Response, attempt: number): number => {
    const raw = res.headers.get("Retry-After");
    if (raw) {
      const sec = Number(raw);
      if (!Number.isNaN(sec) && sec > 0) return sec * 1000;
    }
    return Math.min(60_000, 4000 * 2 ** attempt);
  };

  const uploadToS3AndOneDrive = async (
    file: File,
    token: string,
    signal: AbortSignal,
  ): Promise<{ key: string; tempPath: string }> => {
    const r1 = await fetch("/api/agentIAOCR/upload-url", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ filename: file.name, contentType: file.type || "application/pdf" }),
      signal,
    });
    if (!r1.ok) throw new Error(await r1.text());
    const { url, key } = await r1.json();
    const upload = await fetch(url, {
      method: "PUT",
      headers: { "Content-Type": file.type || "application/pdf" },
      body: file,
      signal,
    });
    if (!upload.ok) throw new Error("Échec upload S3 : " + (await upload.text()));

    const tempPath = `Temp/${file.name}`;
    const putOneDrive = (accessToken: string) =>
      fetch(graphDriveRootItemUrl(tempPath, "/content"), {
        method: "PUT",
        headers: {
          Authorization: `Bearer ${accessToken}`,
          "Content-Type": file.type || "application/pdf",
        },
        body: file,
        signal,
      });

    let activeToken = token;
    let odRes: Response | null = null;
    for (let attempt = 0; attempt < 4; attempt++) {
      odRes = await putOneDrive(activeToken);
      if (odRes.status === 401) {
        const refreshed = await ensureOneDriveConnection();
        if (!refreshed) {
          throw new Error(
            "Session OneDrive expirée (401 Graph). Reconnectez-vous à Microsoft puis relancez les fichiers restants.",
          );
        }
        activeToken = refreshed;
        continue;
      }
      if (odRes.status === 429) {
        const waitMs = parseRetryAfterMs(odRes, attempt);
        await sleep(waitMs);
        continue;
      }
      break;
    }
    if (!odRes || !odRes.ok) {
      const detail = odRes ? await odRes.text() : "réponse vide";
      const status = odRes?.status ?? 0;
      const hint =
        status === 401
          ? "Session OneDrive expirée — reconnectez Microsoft."
          : status === 429
            ? "Limite de requêtes OneDrive atteinte — relancez les fichiers restants dans quelques minutes."
            : status >= 500
              ? "Service Microsoft Graph temporairement indisponible."
              : "";
      throw new Error(
        `Échec upload OneDrive Temp (${status})${hint ? ` — ${hint}` : ""} : ${detail.slice(0, 300)}`,
      );
    }
    return { key, tempPath };
  };

  const applyProcessingProgress = (
    patch: {
      label: string;
      percent: number;
      done: number;
      total: number;
      totalKnown: boolean;
      completed: number;
      failed: number;
    },
    sessionId?: number,
  ) => {
    if (sessionId != null && !isActiveOcrSession(sessionId)) return;
    setProcessingStatus(patch);
  };

  const cancelOcrProcessing = useCallback(async () => {
    abortOcrInFlight();
    ocrSessionIdRef.current += 1;

    if (activeBatchJobId) {
      try {
        const res = await fetch("/api/agentIAOCR/batch-job/cancel", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ jobId: activeBatchJobId }),
        });
        const data = await res.json().catch(() => ({}));
        if (!res.ok) {
          setError(typeof data.error === "string" ? data.error : "Impossible d'annuler le traitement.");
          return;
        }
        if (Array.isArray(data.results)) {
          setOcrResults(data.results);
          setOcrResultsSessionId((id) => id + 1);
        }
        persistFinishedBatchJob(activeBatchJobId);
      } catch (e: unknown) {
        setError(e instanceof Error ? e.message : "Impossible d'annuler le traitement.");
        return;
      }
    }

    setOcrProcessing(false);
    activeBatchJobIdRef.current = null;
    setActiveBatchJobId(null);
    setBatchJobNeedsToken(false);
    setProgressDetail(null);
    localStorage.removeItem(BATCH_JOB_STORAGE_KEY);
    setProcessingStatus({
      ...INITIAL_OCR_PROCESSING_STATUS,
      label: "Traitement annulé",
    });
    setError("");
  }, [abortOcrInFlight, activeBatchJobId, persistFinishedBatchJob]);

  const resumeBatchWithOneDrive = useCallback(async () => {
    if (!activeBatchJobId) return;
    const token = await ensureOneDriveConnection();
    if (!token) return;
    const res = await fetch("/api/agentIAOCR/batch-job/token", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ jobId: activeBatchJobId, accessToken: token }),
    });
    if (!res.ok) {
      setError(await res.text());
      return;
    }
    setBatchJobNeedsToken(false);
    setError("");
  }, [activeBatchJobId, ensureOneDriveConnection]);

  const triggerBatchWorker = useCallback(async (jobId: string) => {
    try {
      await fetch("/api/agentIAOCR/batch-job/process", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ jobId }),
      });
    } catch {
      /* le statut reste la source de vérité */
    }
  }, []);

  const resumeBatchTracking = useCallback(async () => {
    const jobId = activeBatchJobIdRef.current;
    if (!jobId) return;
    setBatchPollIssue(null);
    setError("");
    try {
      const stRes = await fetch(
        `/api/agentIAOCR/batch-job/status?jobId=${encodeURIComponent(jobId)}`,
      );
      if (stRes.status === 401) {
        setBatchPollIssue("auth");
        setError(
          "Session expirée — reconnectez-vous à l'intranet (Clerk), puis recliquez sur « Reprendre le suivi ».",
        );
        return;
      }
      if (!stRes.ok) {
        setBatchPollIssue("offline");
        setError("Impossible de joindre le serveur. Vérifiez votre connexion internet.");
        return;
      }
      const st = (await stRes.json()) as BatchJobStatusPayload;
      applyBatchJobStatusToUi(st, jobId);
      if (st.status === "completed" || st.status === "failed") {
        setOcrProcessing(false);
        activeBatchJobIdRef.current = null;
        setActiveBatchJobId(null);
        persistFinishedBatchJob(jobId);
        return;
      }
      if (st.status === "needs_token") {
        setBatchJobNeedsToken(true);
        return;
      }
      // /process exécute un chunk synchrone (jusqu'à ~55s) : on ne bloque pas le bouton.
      void triggerBatchWorker(jobId);
    } catch {
      setBatchPollIssue("offline");
      setError("Connexion interrompue. Le traitement peut continuer côté serveur — réessayez dans un instant.");
    }
  }, [applyBatchJobStatusToUi, persistFinishedBatchJob, triggerBatchWorker]);

  useEffect(() => {
    if (!clerkUser?.id) return;
    void (async () => {
      try {
        const listRes = await fetch("/api/agentIAOCR/batch-job/list");
        if (!listRes.ok) return;
        const listData = await listRes.json();
        const jobs = (listData.jobs as Array<{ jobId: string; status: string }> | undefined) ?? [];
        const active = jobs.find(
          (j) => j.status === "pending" || j.status === "processing" || j.status === "needs_token",
        );
        const storedActive = localStorage.getItem(BATCH_JOB_STORAGE_KEY);
        const storedLast = localStorage.getItem(BATCH_JOB_LAST_RESULTS_KEY);
        const recentFinished = jobs.find((j) => j.status === "completed" || j.status === "failed");
        const jobId =
          active?.jobId || storedActive || storedLast || recentFinished?.jobId;
        if (!jobId) return;

        const stRes = await fetch(
          `/api/agentIAOCR/batch-job/status?jobId=${encodeURIComponent(jobId)}`,
        );
        if (!stRes.ok) return;
        const st = (await stRes.json()) as BatchJobStatusPayload & { jobId?: string };

        if (st.status === "pending" || st.status === "processing" || st.status === "needs_token") {
          activeBatchJobIdRef.current = jobId;
          setActiveBatchJobId(jobId);
          setOcrProcessing(true);
          applyBatchJobStatusToUi(st, jobId);
          if (st.status === "needs_token") setBatchJobNeedsToken(true);
          return;
        }

        if (st.status === "completed" || st.status === "failed") {
          persistFinishedBatchJob(jobId);
          if (Array.isArray(st.results)) {
            setOcrResults(st.results);
            setOcrResultsSessionId((id) => id + 1);
          }
          if (st.status === "failed" && st.error) setError(String(st.error));
          return;
        }
      } catch {
        /* ignore */
      }
    })();
  }, [clerkUser?.id, applyBatchJobStatusToUi, persistFinishedBatchJob]);

  useEffect(() => {
    if (!activeBatchJobId || batchJobNeedsToken) return;
    let cancelled = false;
    let polls = 0;
    let serverManaged = false;
    let consecutiveFailures = 0;
    let pendingTerminal = 0;
    // Garde « un seul chunk en vol » : /process exécute désormais un chunk SYNCHRONE (jusqu'à ~55s).
    // On le lance sans bloquer la boucle de poll (l'UI continue de se rafraîchir via /status),
    // et on enchaîne le chunk suivant dès que le précédent rend la main.
    let workerInFlight = false;
    const driveWorker = (id: string) => {
      if (workerInFlight || cancelled) return;
      workerInFlight = true;
      void triggerBatchWorker(id).finally(() => {
        workerInFlight = false;
      });
    };

    const tick = async () => {
      if (cancelled) return;
      try {
        if (polls > 0) {
          const delay =
            consecutiveFailures > 0
              ? Math.min(30_000, 2000 * 2 ** Math.min(consecutiveFailures, 4))
              : 2000;
          await sleep(delay);
        }
        polls += 1;

        const stRes = await fetch(
          `/api/agentIAOCR/batch-job/status?jobId=${encodeURIComponent(activeBatchJobId)}`,
        );
        if (cancelled) return;

        if (!stRes.ok) {
          consecutiveFailures += 1;
          if (stRes.status === 401) {
            setBatchPollIssue("auth");
            setError(
              "Session intranet expirée (veille / réseau coupé). Reconnectez-vous puis utilisez « Reprendre le suivi » — le lot peut avoir continué sur le serveur.",
            );
          } else {
            setBatchPollIssue("offline");
          }
          void tick();
          return;
        }

        consecutiveFailures = 0;
        setBatchPollIssue(null);

        const st = (await stRes.json()) as BatchJobStatusPayload;

        if (typeof st.serverManaged === "boolean") {
          serverManaged = st.serverManaged;
        }
        if (typeof st.serverSelfRelays === "boolean") {
          setBatchServerSelfRelays(st.serverSelfRelays);
        }

        applyBatchJobStatusToUi(st, activeBatchJobId);

        if (st.status === "needs_token") {
          const fresh = await ensureOneDriveConnection();
          if (fresh) {
            const resumeRes = await fetch("/api/agentIAOCR/batch-job/token", {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({ jobId: activeBatchJobId, accessToken: fresh }),
            });
            if (resumeRes.ok) {
              setBatchJobNeedsToken(false);
              setError("");
              driveWorker(activeBatchJobId);
              void tick();
              return;
            }
          }
          setBatchJobNeedsToken(true);
          setOcrProcessing(true);
          setError(
            "Session OneDrive expirée (401 Microsoft Graph). Cliquez sur « Reconnecter OneDrive » puis reprenez.",
          );
          return;
        }

        if (st.status === "completed" || st.status === "failed") {
          // Un état terminal peut être TRANSITOIRE (un worker concurrent côté serveur relance le
          // lot juste après). On confirme sur 2 sondages consécutifs avant d'arrêter le suivi —
          // sinon l'interface se figeait et proposait à tort de redéposer un fichier.
          pendingTerminal += 1;
          if (pendingTerminal < 2) {
            void tick();
            return;
          }
          setOcrProcessing(false);
          activeBatchJobIdRef.current = null;
          setActiveBatchJobId(null);
          persistFinishedBatchJob(activeBatchJobId);
          if (st.status === "failed" && st.error) setError(String(st.error));
          return;
        }
        pendingTerminal = 0;

        // Rafraîchissement token OneDrive (~toutes les 8 s) — utile même en mode serveur.
        if (polls % 4 === 0) {
          const cached = pickCachedAccessToken(oneDriveTokenRef.current);
          const fresh = cached ?? (await ensureOneDriveConnection());
          if (fresh) {
            await fetch("/api/agentIAOCR/batch-job/token", {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                jobId: activeBatchJobId,
                accessToken: fresh,
                refreshOnly: true,
              }),
            });
          }
        }

        // Moteur d'avancement : en mode client (pas d'auto-relance serveur), on pilote le worker
        // à chaque poll (le garde workerInFlight enchaîne les chunks sans les empiler). En mode
        // serveur, la chaîne tourne en arrière-plan : on se contente d'un secours périodique.
        if (!serverManaged || polls === 1 || polls % 5 === 0) {
          driveWorker(activeBatchJobId);
        }

        void tick();
      } catch (e) {
        if (!cancelled) {
          consecutiveFailures += 1;
          setBatchPollIssue("offline");
          console.warn("[agentIAOCR] poll batch:", e);
        }
        void tick();
      }
    };

    void tick();
    return () => {
      cancelled = true;
    };
  }, [
    activeBatchJobId,
    batchJobNeedsToken,
    ensureOneDriveConnection,
    applyBatchJobStatusToUi,
    persistFinishedBatchJob,
    triggerBatchWorker,
  ]);

  const runProcessing = async (files: File[]) => {
    const token = await ensureOneDriveConnection();
    if (!token) return;

    abortOcrInFlight();
    const sessionId = ocrSessionIdRef.current + 1;
    ocrSessionIdRef.current = sessionId;
    const signal = ocrAbortRef.current!.signal;

    setOcrProcessing(true);
    setError("");
    setOcrResults([]);
    setBatchJobNeedsToken(false);
    activeBatchJobIdRef.current = null;
    setActiveBatchJobId(null);
    setProgressDetail(null);
    localStorage.removeItem(BATCH_JOB_STORAGE_KEY);
    setProcessingStatus({
      ...INITIAL_OCR_PROCESSING_STATUS,
      percent: 1,
      label: "Préparation de l'envoi…",
    });

    const allEntries: { file: File; mode: "standard" | "class" }[] = files.map(
      (file) => ({ file, mode: "class" as const }),
    );

    applyProcessingProgress(
      {
        percent: 1,
        label: "Envoi des fichiers vers S3 et OneDrive…",
        done: 0,
        total: allEntries.length,
        totalKnown: true,
        completed: 0,
        failed: 0,
      },
      sessionId,
    );

    try {
      const items: Array<{
        fileName: string;
        mode: "standard" | "class";
        s3Key: string;
        tempPath: string;
      }> = [];

      let uploadToken = token;

      for (let i = 0; i < allEntries.length; i++) {
        if (!isActiveOcrSession(sessionId)) return;
        if (i > 0 && i % 25 === 0) {
          const refreshed = await ensureOneDriveConnection();
          if (refreshed) uploadToken = refreshed;
        }
        const { file, mode } = allEntries[i];
        applyProcessingProgress(
          {
            percent: Math.max(2, Math.round((32 * (i + 1)) / allEntries.length)),
            label: `Upload ${i + 1} / ${allEntries.length} — ${file.name}`,
            done: i,
            total: allEntries.length,
            totalKnown: true,
            completed: 0,
            failed: 0,
          },
          sessionId,
        );
        const { key, tempPath } = await uploadToS3AndOneDrive(file, uploadToken, signal);
        items.push({ fileName: file.name, mode, s3Key: key, tempPath });
      }

      if (!isActiveOcrSession(sessionId)) return;

      const freshToken = (await ensureOneDriveConnection()) ?? uploadToken;

      applyProcessingProgress(
        {
          percent: 36,
          label: "Lancement du traitement sur le serveur…",
          done: allEntries.length,
          total: allEntries.length,
          totalKnown: true,
          completed: 0,
          failed: 0,
        },
        sessionId,
      );

      const createRes = await fetch("/api/agentIAOCR/batch-job", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ accessToken: freshToken, items }),
        signal,
      });
      if (!createRes.ok) throw new Error(await createRes.text());
      const created = await createRes.json();
      const jobId = created.jobId as string | undefined;
      if (!jobId) throw new Error("Impossible de créer le traitement serveur");

      const serverRelays = Boolean(created.serverSelfRelays);
      setBatchServerSelfRelays(serverRelays);

      localStorage.setItem(BATCH_JOB_STORAGE_KEY, jobId);
      activeBatchJobIdRef.current = jobId;
      setActiveBatchJobId(jobId);
      applyProcessingProgress(
        {
          percent: 40,
          label: serverRelays
            ? `Traitement serveur lancé (${items.length} PDF) — vous pouvez quitter cette page`
            : `Traitement lancé (${items.length} PDF) — gardez cet onglet ouvert`,
          done: 0,
          total: items.length,
          totalKnown: true,
          completed: 0,
          failed: 0,
        },
        sessionId,
      );
    } catch (err: unknown) {
      if (err instanceof DOMException && err.name === "AbortError") return;
      if (!isActiveOcrSession(sessionId)) return;
      setOcrProcessing(false);
      setError(
        "Erreur lors de l'envoi ou du lancement : " +
          (err instanceof Error ? err.message : String(err)),
      );
    } finally {
      if (isActiveOcrSession(sessionId)) {
        if (classInputRef.current) classInputRef.current.value = "";
      }
    }
  };

  useEffect(() => {
    const hasWork = pendingClassFiles.length > 0;
    if (!hasWork || ocrProcessing || processingLockRef.current || !msalReady) return;

    processingLockRef.current = true;
    const files = [...pendingClassFiles];
    setPendingClassFiles([]);

    void runProcessing(files).finally(() => {
      processingLockRef.current = false;
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pendingClassFiles, ocrProcessing, msalReady]);

  const enqueueOcrFiles = (fileList: FileList | File[]) => {
    const all = Array.from(fileList);
    const pdfs = all.filter(
      (f) => f.type === "application/pdf" || f.name.endsWith(".pdf"),
    );
    if (pdfs.length === 0) {
      setError("Seuls les fichiers PDF sont acceptés.");
      return;
    }
    if (pdfs.length !== all.length) {
      setError("Seuls les fichiers PDF sont acceptés.");
    }
    if (!canAcceptNewOcrFiles()) return;

    setProgressDetail(null);

    const hasPending = pendingClassFiles.length > 0;
    const hasPriorSession =
      !hasPending &&
      (ocrResults.length > 0 ||
        processingStatus.done > 0 ||
        processingStatus.percent >= 100);

    if (hasPriorSession) {
      prepareOcrSessionForNewBatch();
    }

    setPendingClassFiles((prev) => [...prev, ...pdfs]);
  };

  const enqueueAuto = async (fileList: FileList | File[]) => {
    if (!oneDriveVerified || !oneDriveTokenRef.current) {
      const token = await ensureOneDriveConnection();
      if (!token) return;
    }
    enqueueOcrFiles(fileList);
  };

  const handleStartFreshOcrSession = () => {
    if (ocrProcessingRef.current || processingLockRef.current) return;
    abortOcrInFlight();
    ocrSessionIdRef.current += 1;
    resetOcrSessionUi(true);
    setPendingClassFiles([]);
  };

  const handleSyncOneDriveFolders = async () => {
    const token = await ensureOneDriveConnection();
    if (!token) return;
    setSyncingFolders(true);
    setSyncReport(null);
    try {
      const res = await fetch("/api/agentIAOCR/sync-folders", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ accessToken: token }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Échec synchronisation");
      setSyncReport(data);
    } catch (e: unknown) {
      setError("Erreur synchronisation OneDrive : " + (e instanceof Error ? e.message : String(e)));
    } finally {
      setSyncingFolders(false);
    }
  };

  const handleMefUpload = async (file: File) => {
    setMefUploading(true);
    setMefMessage("");
    try {
      const text = await file.text();
      const parsed = JSON.parse(text);
      const res = await fetch("/api/mef-secteurs", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(parsed),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Échec enregistrement MEF");
      if (data.counts) setMefCounts(data.counts);
      setMefMessage(data.message || "Table MEF enregistrée.");
    } catch (e: unknown) {
      setMefMessage("Erreur : " + (e instanceof Error ? e.message : String(e)));
    } finally {
      setMefUploading(false);
      if (mefInputRef.current) mefInputRef.current.value = "";
    }
  };

  const openOneDrivePath = useCallback(
    async (itemPath: string) => {
      const cleanPath = String(itemPath || "").replace(/^\/+/, "");
      if (!cleanPath) return;
      setOpeningOneDrivePath(cleanPath);
      try {
        const token = pickCachedAccessToken(oneDriveTokenRef.current) ?? (await ensureOneDriveConnection());
        if (!token) return;
        const res = await fetch(graphDriveRootItemUrl(cleanPath, "?$select=webUrl"), {
          headers: { Authorization: `Bearer ${token}` },
        });
        if (!res.ok) {
          throw new Error(`Graph ${res.status}`);
        }
        const data = (await res.json()) as { webUrl?: string };
        if (!data.webUrl) throw new Error("Lien OneDrive introuvable");
        window.open(data.webUrl, "_blank", "noopener,noreferrer");
      } catch (e: unknown) {
        setError(
          `Impossible d'ouvrir le document OneDrive (${cleanPath}) : ${
            e instanceof Error ? e.message : String(e)
          }`,
        );
      } finally {
        setOpeningOneDrivePath(null);
      }
    },
    [ensureOneDriveConnection],
  );

  if (!msalReady) {
    return (
      <div className="flex items-center justify-center min-h-[60vh]">
        <div className="flex flex-col items-center gap-4">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600"></div>
          <p className="text-gray-500 font-medium">Initialisation de MSAL...</p>
        </div>
      </div>
    );
  }

  const dropsAvailable = oneDriveVerified && Boolean(accessToken);
  const dropDisabled = !dropsAvailable || ocrProcessing || checkingOneDrive || processingLockRef.current;
  const canStartFreshSession =
    !ocrProcessing &&
    !processingLockRef.current &&
    (ocrResults.length > 0 || processingStatus.done > 0 || processingStatus.percent >= 100);
  const isUploadPhase = ocrProcessing && !activeBatchJobId;
  const isServerPhase = ocrProcessing && Boolean(activeBatchJobId) && !batchJobNeedsToken;
  // Compteurs de session : même source que la liste de résultats (ocrResults, fusionnée et
  // monotone). On ne s'appuie PAS sur progressDetail brut, qui peut régresser (poll S3 en retard
  // ou écriture concurrente côté serveur) et provoquait les sauts 8→7→5 / le sous-comptage.
  const sessionDocSucceeded = ocrResults.filter((r) => r.success).length;
  const sessionDocFailed = ocrResults.filter((r) => !r.success).length;
  const sessionDocProcessed = sessionDocSucceeded + sessionDocFailed;
  const rawDocTotal =
    progressDetail?.phase === "segments" && progressDetail.segmentTotal
      ? progressDetail.segmentTotal
      : 0;
  // Pic monotone du total de documents (ne redescend jamais pendant un lot).
  const displayDocTotal = Math.max(progressPeakRef.current.totalDocs, rawDocTotal, sessionDocProcessed);
  const sessionDocTotal = displayDocTotal > 0 ? displayDocTotal : null;
  // Pendant le classement, le % suit EXACTEMENT le ratio "documents traités / total"
  // (9/12 → 75 %) pour ne plus afficher un % incohérent avec le compteur. Sinon, poids serveur.
  const rawPercent = progressDetail?.percent ?? processingStatus.percent ?? 0;
  const ratioPercent =
    progressDetail?.phase === "segments" && sessionDocTotal
      ? Math.round((sessionDocProcessed / sessionDocTotal) * 100)
      : rawPercent;
  const displayPercent = Math.min(100, Math.max(progressPeakRef.current.percent, ratioPercent));
  progressPeakRef.current = { percent: displayPercent, totalDocs: displayDocTotal };
  const progressPercent = displayPercent;
  const progressCaption = isUploadPhase
    ? processingStatus.totalKnown && processingStatus.total > 1
      ? `Fichier ${Math.min(processingStatus.done + 1, processingStatus.total)} / ${processingStatus.total}`
      : "Envoi en cours…"
    : progressDetail
    ? progressDetail.phase === "segments" && sessionDocTotal
      ? `Document ${sessionDocProcessed} / ${sessionDocTotal}`
      : progressDetail.phase === "ocr"
        ? progressDetail.pdfPageCount
          ? progressDetail.ocrPagesRead && progressDetail.ocrPagesRead > 0
            ? `Page ${progressDetail.ocrPagesRead} / ${progressDetail.pdfPageCount}`
            : `0 / ${progressDetail.pdfPageCount} page(s)`
          : "Mistral lit le document…"
        : progressDetail.phase === "segmenting"
          ? progressDetail.pageCount
            ? progressDetail.segmentationEngine === "identity"
              ? `Repérage élèves · ${progressDetail.pageCount} p.`
              : progressDetail.segmentationEngine === "heuristic"
                ? `Découpage auto · ${progressDetail.pageCount} p.`
                : progressDetail.segmentationEngine === "mistral_chunked"
                  ? `Mistral découpe · ${progressDetail.pageCount} p.`
                  : progressDetail.segmentationEngine === "mistral"
                    ? `Mistral découpe · ${progressDetail.pageCount} p.`
                    : `Découpage · ${progressDetail.pageCount} p.`
            : "Mistral en déduit le découpage…"
          : progressDetail.fileTotal > 1
            ? `Fichier ${progressDetail.fileIndex} / ${progressDetail.fileTotal}`
            : progressDetail.pageCount
              ? `${progressDetail.pageCount} page${progressDetail.pageCount > 1 ? "s" : ""}`
              : ""
    : processingStatus.totalKnown
      ? `${processingStatus.done} / ${processingStatus.total} document${processingStatus.total > 1 ? "s" : ""}`
      : "";

  const formatIdleHint = (seconds: number): string | null => {
    if (seconds < 90) return null;
    if (seconds < 3600) {
      const min = Math.floor(seconds / 60);
      return `Dernière activité il y a ${min} min — sur un gros PDF, Mistral peut analyser 10–20 min sans mise à jour visible.`;
    }
    const h = Math.floor(seconds / 3600);
    return `Dernière activité il y a ${h} h — si rien ne bouge, vérifiez les résultats ou relancez.`;
  };

  const phaseSteps: { id: string; label: string }[] = [
    { id: "ocr", label: "1. Lecture Mistral" },
    { id: "segmenting", label: "2. Découpage" },
    { id: "segments", label: "3. Nom & rangement" },
  ];

  const activePhaseIndex = progressDetail
    ? progressDetail.phase === "ocr"
      ? 0
      : progressDetail.phase === "segmenting"
        ? 1
        : progressDetail.phase === "segments" || progressDetail.phase === "analyze"
          ? 2
          : -1
    : -1;

  const dropZoneClass = (active: boolean, variant: "blue" | "violet") => {
    if (!dropsAvailable) {
      return "relative overflow-hidden border-2 border-dashed rounded-3xl p-10 text-center border-slate-200 bg-slate-50/90 cursor-not-allowed opacity-80";
    }
    return `relative overflow-hidden border-2 border-dashed rounded-3xl p-10 text-center transition-all duration-300 group
    ${active ? (variant === "violet" ? "border-violet-600 bg-violet-50 scale-[1.01]" : "border-blue-600 bg-blue-50 scale-[1.01]") : "border-gray-300 bg-white hover:border-blue-400 hover:bg-gray-50"}
    ${dropDisabled ? "opacity-60 cursor-not-allowed shadow-none" : "cursor-pointer shadow-lg hover:shadow-xl"}
    ${ocrProcessing ? "ring-4 ring-blue-400/40 border-blue-500 bg-blue-50/80" : ""}`;
  };

  const failureHint = (result: ProcessResult): string => {
    const err = (result.error || "").toLowerCase();
    if (err.includes("élève") || err.includes("eleve") || err.includes("identifi")) {
      return "Le nom ou prénom de l'élève n'a pas été reconnu clairement dans le document.";
    }
    if (err.includes("incomplet") || err.includes("filename")) {
      return "Le type de document ou les informations attendues (classe, date…) n'ont pas pu être lues.";
    }
    if (err.includes("ocr") || err.includes("texte")) {
      return "Le texte du PDF est illisible ou trop pauvre pour une analyse fiable.";
    }
    if (err.includes("déplacé") || err.includes("deplace") || err.includes("folder") || err.includes("graph")) {
      return "Le dossier de destination n'a pas pu être créé ou atteint sur OneDrive.";
    }
    return "Le rangement automatique n'a pas abouti pour ce fichier.";
  };

  /** Distingue un échec « métier » (à classer à la main) d'une vraie erreur technique. */
  const failureCategory = (
    result: ProcessResult,
  ): { label: string; technical: boolean } => {
    const err = (result.error || "").toLowerCase();
    if (err.includes("élève") || err.includes("eleve") || err.includes("identifi")) {
      return { label: "Élève non trouvé", technical: false };
    }
    if (err.includes("incomplet") || err.includes("filename")) {
      return { label: "Lecture incomplète", technical: false };
    }
    if (err.includes("ocr") || err.includes("mistral") || err.includes("textract")) {
      return { label: "Lecture Mistral échouée", technical: true };
    }
    if (
      err.includes("token") ||
      err.includes("onedrive") ||
      err.includes("graph") ||
      err.includes("déplac") ||
      err.includes("deplac") ||
      err.includes("upload") ||
      err.includes("401") ||
      err.includes("429") ||
      err.includes("500")
    ) {
      return { label: "Erreur technique", technical: true };
    }
    return { label: "Échec", technical: true };
  };

  const failedResults = ocrResults.filter((r) => !r.success);

  const ProcessingSpinner = ({ size = "text-7xl" }: { size?: string }) => (
    <span
      className={`${size} inline-block animate-spin`}
      role="status"
      aria-label="Analyse en cours"
    >
      ⚙️
    </span>
  );

  return (
    <div className="p-6 max-w-[1200px] mx-auto mt-[1vh]">
      <div className="flex flex-col md:flex-row justify-between items-start md:items-center mb-8 gap-4">
        <div>
          <h1 className="text-3xl font-extrabold text-gray-900 tracking-tight">
            Ajout de documents IA
          </h1>
          <p className="text-gray-500 mt-1">
            Numérisez et rangez vos PDF dans les dossiers élèves sur OneDrive.
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-3">
          {oneDriveVerified && accessToken ? (
            <span className="px-4 py-2 bg-green-50 text-green-800 text-sm font-bold rounded-xl border border-green-200">
              OneDrive connecté
            </span>
          ) : null}
        </div>
      </div>

      {error && (
        <div className="mb-6 p-4 bg-red-50 border-l-4 border-red-500 text-red-700 rounded-r-xl">
          <p className="font-bold">Attention</p>
          <p className="text-sm">{error}</p>
        </div>
      )}

      {batchPollIssue && activeBatchJobId && !batchJobNeedsToken && (
        <div className="mb-6 p-4 bg-amber-50 border-l-4 border-amber-500 text-amber-900 rounded-r-xl flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3">
          <div>
            <p className="font-bold">
              {batchPollIssue === "auth" ? "Suivi interrompu (session)" : "Suivi interrompu (réseau)"}
            </p>
            <p className="text-sm">
              {batchPollIssue === "auth"
                ? "La connexion intranet a expiré (veille, onglet en arrière-plan, Wi‑Fi coupé). Le traitement peut avoir continué sur le serveur."
                : "Connexion internet coupée ou ordinateur en veille. Le traitement peut avoir continué sur le serveur."}
            </p>
          </div>
          <button
            type="button"
            onClick={() => void resumeBatchTracking()}
            className="shrink-0 px-4 py-2 bg-amber-600 hover:bg-amber-700 text-white font-bold rounded-xl"
          >
            Reprendre le suivi
          </button>
        </div>
      )}

      {batchJobNeedsToken && activeBatchJobId && (
        <div className="mb-6 p-4 bg-amber-50 border-l-4 border-amber-500 text-amber-900 rounded-r-xl flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3">
          <div>
            <p className="font-bold">Session OneDrive expirée</p>
            <p className="text-sm">
              Le traitement serveur est en pause. Reconnectez Microsoft pour reprendre le rangement des fichiers restants.
            </p>
          </div>
          <div className="flex flex-wrap gap-2 shrink-0">
            <button
              type="button"
              onClick={() => void resumeBatchWithOneDrive()}
              className="px-4 py-2 bg-amber-600 hover:bg-amber-700 text-white font-bold rounded-xl"
            >
              Reconnecter et reprendre
            </button>
            <button
              type="button"
              onClick={() => void cancelOcrProcessing()}
              className="px-4 py-2 bg-white border border-amber-400 text-amber-900 font-bold rounded-xl hover:bg-amber-100"
            >
              Annuler le traitement
            </button>
          </div>
        </div>
      )}

      {(isServerPhase || (ocrProcessing && activeBatchJobId)) && !batchJobNeedsToken && batchServerSelfRelays && (
        <div className="mb-6 p-5 bg-emerald-50 border-2 border-emerald-400 rounded-2xl flex gap-4 items-start justify-between shadow-sm">
          <div className="flex gap-4 items-start min-w-0">
            <span
              className="flex h-11 w-11 shrink-0 items-center justify-center rounded-full bg-emerald-600 text-white text-xl font-bold"
              aria-hidden
            >
              ✓
            </span>
            <div>
              <p className="text-lg font-extrabold text-emerald-950">Vous pouvez quitter cette page</p>
              <p className="text-sm text-emerald-900 mt-1 leading-relaxed">
                Le reste du traitement tourne sur le <strong>serveur</strong> (Mistral, rangement OneDrive).
                Revenez sur cette page à tout moment pour voir où en est le lot et consulter les résultats.
              </p>
            </div>
          </div>
          <button
            type="button"
            onClick={() => void cancelOcrProcessing()}
            className="shrink-0 px-4 py-2 bg-white border border-emerald-500 text-emerald-900 font-bold rounded-xl hover:bg-emerald-100"
          >
            Annuler
          </button>
        </div>
      )}

      {(isServerPhase || (ocrProcessing && activeBatchJobId)) && !batchJobNeedsToken && !batchServerSelfRelays && (
        <div className="mb-6 p-5 bg-amber-50 border-2 border-amber-400 rounded-2xl flex gap-4 items-start justify-between shadow-sm">
          <div className="flex gap-4 items-start min-w-0">
            <span
              className="flex h-11 w-11 shrink-0 items-center justify-center rounded-full bg-amber-600 text-white text-xl font-bold"
              aria-hidden
            >
              !
            </span>
            <div>
              <p className="text-lg font-extrabold text-amber-950">Gardez cet onglet ouvert</p>
              <p className="text-sm text-amber-900 mt-1 leading-relaxed">
                L&apos;auto-relance serveur n&apos;est pas active sur cet environnement. Le traitement avance
                tant que cette page reste ouverte (veille ou fermeture = arrêt). Contactez l&apos;administrateur
                pour activer <code className="text-xs">OCR_WORKER_SECRET</code> en production.
              </p>
            </div>
          </div>
          <button
            type="button"
            onClick={() => void cancelOcrProcessing()}
            className="shrink-0 px-4 py-2 bg-white border border-amber-500 text-amber-900 font-bold rounded-xl hover:bg-amber-100"
          >
            Annuler
          </button>
        </div>
      )}

      {isUploadPhase && (
        <div
          className="fixed inset-0 z-[100] flex items-center justify-center bg-slate-900/60 backdrop-blur-sm p-4"
          role="alertdialog"
          aria-modal="true"
          aria-labelledby="ocr-upload-phase-title"
          aria-describedby="ocr-upload-phase-desc"
        >
          <div className="w-full max-w-xl rounded-3xl border-4 border-amber-500 bg-amber-50 p-8 md:p-10 shadow-2xl text-center">
            <p className="text-[11px] font-black uppercase tracking-[0.2em] text-amber-800 mb-3">
              Phase d&apos;upload
            </p>
            <h2
              id="ocr-upload-phase-title"
              className="text-2xl md:text-3xl font-black text-amber-950 leading-tight mb-4"
            >
              Ne quittez pas cette page
            </h2>
            <p id="ocr-upload-phase-desc" className="text-base md:text-lg font-semibold text-amber-900 mb-6 leading-relaxed">
              Vos PDF sont envoyés vers le cloud et OneDrive. Sur un gros lot, cela peut prendre plusieurs
              minutes — laissez cet onglet ouvert jusqu&apos;au message de confirmation serveur.
            </p>
            <div className="rounded-2xl bg-white/90 border border-amber-300 px-4 py-3 text-sm font-bold text-amber-950">
              {processingStatus.label || "Préparation de l'envoi…"}
            </div>
            <div className="mt-5 w-full bg-amber-200/80 rounded-full h-3 overflow-hidden">
              <div
                className="bg-amber-600 h-full transition-all duration-500"
                style={{ width: `${Math.min(100, progressPercent)}%` }}
              />
            </div>
            <p className="mt-3 text-xs font-bold uppercase tracking-wide text-amber-800">
              {progressCaption ? `${progressCaption} · ` : ""}
              {Math.round(progressPercent)}%
            </p>
            <button
              type="button"
              onClick={() => void cancelOcrProcessing()}
              className="mt-6 px-5 py-2.5 bg-white border-2 border-amber-600 text-amber-950 font-bold rounded-xl hover:bg-amber-100"
            >
              Annuler l&apos;envoi
            </button>
          </div>
        </div>
      )}

      {clerkUser && !oneDriveProfile && (
        <div className="mb-6 p-4 bg-amber-50 border-l-4 border-amber-500 text-amber-900 rounded-r-xl">
          <p className="font-bold">Profil OneDrive non reconnu</p>
          <p className="text-sm">
            Votre compte Clerk ({clerkUser.lastName || "nom absent"} —{" "}
            {clerkUser.primaryEmailAddress?.emailAddress || "e-mail absent"}) n&apos;est pas encore associé au
            dossier collège / lycée / école. Contactez l&apos;administrateur pour l&apos;ajouter.
          </p>
        </div>
      )}

      {oneDriveProfile && (
        <div className="mb-6 p-4 bg-slate-50 border border-slate-200 text-slate-700 rounded-xl text-sm">
          Dossier OneDrive configuré : <strong>{oneDriveProfile.label}</strong> —{" "}
          <span className="font-mono text-xs">{oneDriveProfile.basePath}</span>
        </div>
      )}

      <div
        data-tour="onedrive-connect"
        className={`mb-8 rounded-3xl border p-5 md:p-6 ${
          dropsAvailable
            ? "border-green-200 bg-green-50/60"
            : "border-amber-200 bg-amber-50/80"
        }`}
      >
        <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
          <div>
            <p className="text-[10px] font-black uppercase tracking-wider text-slate-500 mb-1">
              Étape 1 — Connexion
            </p>
            <h2 className="text-lg font-bold text-slate-900">
              {dropsAvailable ? "OneDrive est connecté" : "Connectez-vous à OneDrive"}
            </h2>
            <p className="text-sm text-slate-600 mt-1 max-w-xl">
              {dropsAvailable
                ? account?.name
                  ? `Compte : ${account.name}. Vous pouvez déposer vos PDF ci-dessous.`
                  : "Vous pouvez déposer vos PDF ci-dessous."
                : "La connexion Microsoft est obligatoire avant tout dépôt. Sans OneDrive, l'analyse et le rangement ne sont pas possibles."}
            </p>
          </div>
          {!dropsAvailable && (
            <div className="flex flex-wrap gap-2 shrink-0">
              <button
                type="button"
                onClick={login}
                className="px-6 py-3 bg-blue-600 hover:bg-blue-700 text-white font-bold rounded-xl shadow-lg transition-all"
              >
                Se connecter à OneDrive
              </button>
              {account ? (
                <button
                  type="button"
                  onClick={() => void reconnectOneDrive()}
                  className="px-6 py-3 bg-white border border-blue-300 text-blue-800 font-bold rounded-xl hover:bg-blue-50 transition-all"
                >
                  Reconnecter (consentement)
                </button>
              ) : null}
            </div>
          )}
        </div>
      </div>

      <>
          {checkingOneDrive && (
            <div className="mb-6 p-4 bg-blue-50 border border-blue-200 rounded-2xl text-blue-800 text-sm font-medium flex items-center gap-3">
              <div className="animate-spin rounded-full h-5 w-5 border-2 border-blue-600 border-t-transparent" />
              Vérification de la connexion OneDrive…
            </div>
          )}
          {ocrProcessing && (
            <div
              className={`mb-8 p-8 rounded-3xl shadow-xl flex flex-col items-center gap-4 border-2 ${
                isServerPhase
                  ? "bg-gradient-to-br from-emerald-50 to-indigo-50 border-emerald-300"
                  : "bg-gradient-to-br from-blue-50 to-indigo-50 border-blue-300"
              }`}
            >
              <ProcessingSpinner size="text-8xl" />
              <p className="text-2xl font-extrabold text-blue-900 tracking-tight">
                {isServerPhase ? "Traitement serveur en cours…" : "Envoi des fichiers…"}
              </p>
              {isServerPhase && batchServerSelfRelays && (
                <div className="flex items-center gap-3 rounded-2xl bg-white/80 border border-emerald-300 px-5 py-3 max-w-lg">
                  <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-emerald-600 text-white font-bold">
                    ✓
                  </span>
                  <p className="text-sm font-semibold text-emerald-950 text-left">
                    Vous pouvez fermer cet onglet — revenez plus tard pour suivre la progression.
                  </p>
                </div>
              )}
              {isServerPhase && !batchServerSelfRelays && (
                <div className="flex items-center gap-3 rounded-2xl bg-white/80 border border-amber-300 px-5 py-3 max-w-lg">
                  <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-amber-600 text-white font-bold">
                    !
                  </span>
                  <p className="text-sm font-semibold text-amber-950 text-left">
                    Gardez cet onglet ouvert — sans auto-relance serveur, le traitement s&apos;arrête si vous partez.
                  </p>
                </div>
              )}
              <div className="w-full max-w-lg">
                <div className="flex justify-between text-xs font-bold text-blue-700 mb-2 uppercase">
                  <span>Progression</span>
                  <span>
                    {progressCaption ? `${progressCaption} · ` : ""}
                    {Math.round(progressPercent)}%
                  </span>
                </div>
                <div className="w-full bg-white/80 rounded-full h-4 overflow-hidden border border-blue-200">
                  <div
                    className="bg-blue-600 h-full transition-all duration-500 ease-out"
                    style={{ width: `${Math.min(100, progressPercent)}%` }}
                  />
                </div>
                {progressDetail && progressDetail.phase !== "done" && progressDetail.phase !== "idle" ? (
                  <div className="mt-4 rounded-2xl border border-blue-200 bg-white/90 p-4 text-left space-y-3">
                    <div className="flex flex-wrap gap-2">
                      {phaseSteps.map((step, idx) => {
                        const isActive = idx === activePhaseIndex;
                        const isDone = activePhaseIndex > idx;
                        return (
                          <span
                            key={step.id}
                            className={`text-[10px] font-black uppercase tracking-wide px-2.5 py-1 rounded-full border ${
                              isActive
                                ? "bg-blue-600 text-white border-blue-600"
                                : isDone
                                  ? "bg-emerald-50 text-emerald-800 border-emerald-200"
                                  : "bg-slate-50 text-slate-400 border-slate-200"
                            }`}
                          >
                            {isDone ? "✓ " : ""}
                            {step.label}
                          </span>
                        );
                      })}
                    </div>
                    <p className="text-sm font-bold text-slate-900">{progressDetail.phaseLabel}</p>
                    {progressDetail.fileName ? (
                      <p className="text-xs text-slate-600 font-mono truncate">{progressDetail.fileName}</p>
                    ) : null}
                    <div className="grid grid-cols-2 sm:grid-cols-3 gap-2 text-xs">
                      {progressDetail.phase === "ocr" && progressDetail.pdfPageCount ? (
                        <div className="rounded-xl bg-slate-50 border border-slate-100 px-3 py-2">
                          <p className="text-[10px] font-bold uppercase text-slate-400">Lecture Mistral</p>
                          <p className="font-black text-slate-800">
                            {progressDetail.ocrPagesRead && progressDetail.ocrPagesRead > 0
                              ? `${progressDetail.ocrPagesRead} / ${progressDetail.pdfPageCount}`
                              : `0 / ${progressDetail.pdfPageCount}`}
                          </p>
                        </div>
                      ) : progressDetail.pageCount ? (
                        <div className="rounded-xl bg-slate-50 border border-slate-100 px-3 py-2">
                          <p className="text-[10px] font-bold uppercase text-slate-400">Pages PDF</p>
                          <p className="font-black text-slate-800">{progressDetail.pageCount}</p>
                        </div>
                      ) : null}
                      {progressDetail.phase === "segments" && sessionDocTotal ? (
                        <div className="rounded-xl bg-slate-50 border border-slate-100 px-3 py-2">
                          <p className="text-[10px] font-bold uppercase text-slate-400">Documents</p>
                          <p className="font-black text-slate-800">
                            {sessionDocProcessed} / {sessionDocTotal}
                          </p>
                        </div>
                      ) : progressDetail.phase === "segmenting" ? (
                        <div className="rounded-xl bg-slate-50 border border-slate-100 px-3 py-2">
                          <p className="text-[10px] font-bold uppercase text-slate-400">Documents</p>
                          <p className="font-black text-slate-500">
                            {progressDetail.segmentationEngine === "identity"
                              ? "Repérage élèves…"
                              : progressDetail.segmentationEngine === "heuristic"
                                ? "Découpage auto…"
                                : progressDetail.segmentationEngine === "mistral_chunked"
                                  ? "Mistral découpe…"
                                  : progressDetail.segmentationEngine === "mistral"
                                    ? "Mistral découpe…"
                                    : "En cours…"}
                          </p>
                        </div>
                      ) : null}
                      <div className="rounded-xl bg-slate-50 border border-slate-100 px-3 py-2">
                        <p className="text-[10px] font-bold uppercase text-slate-400">Traités</p>
                        <p className="font-black text-slate-800">
                          <span className="text-emerald-700">{progressDetail.documentsSucceeded}</span>
                          {progressDetail.documentsFailed > 0 ? (
                            <span className="text-red-600"> · {progressDetail.documentsFailed} échec(s)</span>
                          ) : null}
                        </p>
                      </div>
                    </div>
                    {progressDetail.phase === "ocr" && progressDetail.pdfPageCount ? (
                      <p className="text-[11px] text-slate-500 leading-relaxed">
                        {progressDetail.ocrPagesRead && progressDetail.ocrPagesRead > 0
                          ? "Mistral lit les pages au fur et à mesure."
                          : `PDF de ${progressDetail.pdfPageCount} page${progressDetail.pdfPageCount > 1 ? "s" : ""} — le compteur peut rester à 0 quelques minutes pendant que Mistral analyse le document.`}
                      </p>
                    ) : null}
                    {progressDetail.phase === "segmenting" ? (
                      <p className="text-[11px] text-slate-500 leading-relaxed">
                        {progressDetail.segmentationEngine === "identity" ? (
                          <>
                            <strong className="text-slate-700">Mistral a terminé la lecture.</strong> Les
                            pages sont regroupées par élève (INE + noms de votre liste) : chaque bulletin
                            multi-pages reste entier, aucun découpage page par page.
                          </>
                        ) : progressDetail.segmentationEngine === "heuristic" ? (
                          <>
                            <strong className="text-slate-700">Mistral a terminé la lecture.</strong> Repli
                            automatique (règles locales) — utilisé seulement si Mistral échoue.
                          </>
                        ) : progressDetail.segmentationEngine === "mistral_chunked" ? (
                          <>
                            <strong className="text-slate-700">Mistral en déduit le découpage</strong>{" "}
                            par blocs (~30 pages max), en ne coupant qu&apos;entre deux documents (pas
                            au milieu d&apos;un bulletin sur 2 pages).
                          </>
                        ) : progressDetail.segmentationEngine === "mistral" ? (
                          <>
                            <strong className="text-slate-700">Mistral en déduit le découpage</strong>{" "}
                            en lisant tout le PDF pour repérer chaque document (environ 15–30 s).
                          </>
                        ) : (
                          <>
                            <strong className="text-slate-700">Mistral prépare le découpage</strong> pour
                            séparer les documents du PDF…
                          </>
                        )}
                      </p>
                    ) : null}
                    {progressDetail.phase === "segments" ? (
                      <p className="text-[11px] text-slate-500 leading-relaxed">
                        <strong className="text-slate-700">Découpage terminé.</strong> Pour chaque
                        document, Mistral déduit le nom du fichier et où le ranger sur OneDrive —
                        c&apos;est l&apos;étape la plus longue sur un gros lot.
                      </p>
                    ) : null}
                    {progressDetail.phase === "segments" && sessionDocTotal ? (
                      <div className="w-full bg-slate-100 rounded-full h-2 overflow-hidden">
                        <div
                          className="bg-indigo-500 h-full transition-all duration-500"
                          style={{
                            width: `${Math.min(100, Math.round((sessionDocProcessed / sessionDocTotal) * 100))}%`,
                          }}
                        />
                      </div>
                    ) : null}
                    {formatIdleHint(progressDetail.idleSeconds) ? (
                      <div className="space-y-2">
                        <p className="text-[11px] text-amber-800 bg-amber-50 border border-amber-200 rounded-xl px-3 py-2 leading-relaxed">
                          {formatIdleHint(progressDetail.idleSeconds)}
                        </p>
                        {progressDetail.idleSeconds >= 300 && activeBatchJobId ? (
                          <button
                            type="button"
                            onClick={() => void resumeBatchTracking()}
                            className="w-full px-3 py-2 text-xs font-bold rounded-xl bg-amber-600 hover:bg-amber-700 text-white"
                          >
                            Reprendre le suivi et relancer le worker
                          </button>
                        ) : null}
                      </div>
                    ) : null}
                  </div>
                ) : null}
                {processingStatus.label ? (
                  <p className="mt-3 text-center text-sm font-semibold text-blue-900/90">
                    {processingStatus.label}
                  </p>
                ) : null}
              </div>
            </div>
          )}

          {!ocrProcessing ? (
          <>
          <div className="mb-4">
            <p className="text-[10px] font-black uppercase tracking-wider text-slate-500 mb-1">
              Étape 2 — Dépôt des PDF
            </p>
            <p className="text-sm text-slate-600">
              {dropsAvailable
                ? "Déposez vos PDF : l'outil détecte automatiquement s'il s'agit d'un document par élève ou d'un export de classe entière à découper."
                : "La zone ci-dessous reste désactivée tant que OneDrive n'est pas connecté."}
            </p>
          </div>

          <div className="mb-8">
            <div
              id="ocr-drop-standard"
              data-tour="drop-standard"
              onDragOver={
                dropDisabled
                  ? undefined
                  : (e) => {
                      e.preventDefault();
                      setIsDraggingClass(true);
                    }
              }
              onDragLeave={() => setIsDraggingClass(false)}
              onDrop={
                dropDisabled
                  ? undefined
                  : (e) => {
                      e.preventDefault();
                      setIsDraggingClass(false);
                      if (e.dataTransfer.files?.length) {
                        enqueueAuto(e.dataTransfer.files);
                      }
                    }
              }
              onClick={() => !dropDisabled && classInputRef.current?.click()}
              className={dropZoneClass(isDraggingClass, "blue")}
            >
              <div className="mb-3 min-h-[4rem] flex items-center justify-center">
                {ocrProcessing ? (
                  <ProcessingSpinner size="text-6xl" />
                ) : (
                  <span className="text-5xl">📄</span>
                )}
              </div>
              <h3 className="text-lg font-bold text-gray-800 mb-2">
                {!dropsAvailable
                  ? "Connexion OneDrive requise"
                  : checkingOneDrive
                    ? "Vérification OneDrive…"
                    : ocrProcessing
                      ? "Analyse en cours…"
                      : "Déposez vos PDF — détection automatique"}
              </h3>
              <p className="text-sm text-gray-500">
                {!dropsAvailable
                  ? "Connectez-vous à l'étape 1 pour débloquer le dépôt de fichiers."
                  : checkingOneDrive
                    ? "Connexion Microsoft vérifiée avant tout traitement."
                    : ocrProcessing
                      ? "Traitement en cours — patientez."
                      : "Un document par élève OU un export de classe entière : l'outil reconnaît, découpe si besoin et range automatiquement. Glissez-déposez (plusieurs fichiers possibles) ou cliquez."}
              </p>
              <input
                ref={classInputRef}
                type="file"
                className="hidden"
                multiple
                accept="application/pdf,.pdf"
                onChange={(e) => {
                  if (e.target.files) {
                    enqueueAuto(e.target.files);
                    e.target.value = "";
                  }
                }}
              />
            </div>
          </div>
          </>
          ) : null}

          <div className="bg-white p-6 rounded-3xl shadow-lg border border-gray-100 mb-8">
            <div className="flex flex-wrap items-center justify-between gap-3 mb-4">
              <h4 className="font-bold text-gray-800">📊 Session actuelle</h4>
              {canStartFreshSession ? (
                <button
                  type="button"
                  onClick={handleStartFreshOcrSession}
                  className="px-4 py-2 rounded-xl border border-slate-200 bg-slate-50 text-slate-700 text-sm font-bold hover:bg-slate-100"
                >
                  Nouvelle session
                </button>
              ) : null}
            </div>
            {canStartFreshSession ? (
              <p className="text-xs text-slate-500 mb-4">
                Les résultats ci-dessous restent visibles jusqu&apos;au prochain dépôt. Utilisez « Nouvelle session » pour
                effacer l&apos;écran sans recharger la page.
              </p>
            ) : null}
            <div className="grid grid-cols-3 gap-3">
              <div className="p-3 bg-gray-50 rounded-2xl text-center">
                <span className="text-xs text-gray-600 block">
                  {sessionDocTotal ? "Documents" : "Fichiers"}
                </span>
                <span className="font-black text-lg">
                  {sessionDocTotal ? (
                    <>
                      {sessionDocProcessed}
                      <span className="text-sm font-bold text-gray-400">
                        {" "}
                        / {sessionDocTotal}
                      </span>
                    </>
                  ) : processingStatus.totalKnown ? (
                    <>
                      {processingStatus.done}
                      <span className="text-sm font-bold text-gray-400">
                        {" "}
                        / {processingStatus.total}
                      </span>
                    </>
                  ) : (
                    <span className="text-sm font-bold text-gray-400">—</span>
                  )}
                </span>
              </div>
              <div className="p-3 bg-green-50 rounded-2xl text-center">
                <span className="text-xs text-green-700 block">Succès</span>
                <span className="font-black text-lg text-green-600">{sessionDocSucceeded}</span>
              </div>
              <div className="p-3 bg-red-50 rounded-2xl text-center">
                <span className="text-xs text-red-700 block">Échecs</span>
                <span className="font-black text-lg text-red-600">{sessionDocFailed}</span>
              </div>
            </div>
            {sessionDocTotal && progressDetail?.phase === "segments" ? (
              <p className="text-[11px] text-slate-500 mt-3 text-center">
                Classement document par document — chaque bulletin passe par Mistral puis OneDrive (~15–30 s
                / document).
              </p>
            ) : null}
          </div>

          {failedResults.length > 0 && (
            <div className="mb-8 p-6 bg-amber-50 border-2 border-amber-300 rounded-3xl shadow-lg">
              <h3 className="text-lg font-black text-amber-950 mb-2 flex items-center gap-2">
                <span>📁</span>
                {failedResults.length} document
                {failedResults.length > 1 ? "s" : ""} à traiter manuellement
              </h3>
              <div className="text-sm text-amber-950 mb-4 leading-relaxed space-y-3">
                <p>
                  Ces fichiers n&apos;ont <strong>pas pu être rangés automatiquement</strong> dans le dossier
                  d&apos;un élève. Ils se trouvent dans le dossier{" "}
                  <strong>Temp</strong>, à la <strong>racine de votre OneDrive</strong> (même niveau que
                  « Documents », « Images », etc.).
                </p>
                <p>
                  <strong>Pourquoi ?</strong> Le plus souvent : le nom de l&apos;élève n&apos;a pas été reconnu,
                  le type de document est ambigu, ou le texte du PDF est illisible.
                </p>
                <p>
                  <strong>Que faire ?</strong>
                </p>
                <ul className="list-disc pl-5 space-y-1">
                  <li>
                    Ouvrez OneDrive → dossier <strong>Temp</strong> → déplacez le PDF dans le bon dossier élève ;
                  </li>
                  <li>
                    ou, si le document n&apos;a pas été reconnu, vérifiez qu&apos;il est lisible et que l&apos;élève
                    figure bien dans la liste, puis redéposez-le.
                  </li>
                </ul>
              </div>
              <ul className="space-y-3">
                {failedResults.map((r, index) => (
                  <li
                    key={`${ocrResultsSessionId}-fail-${r.fileName}-${index}`}
                    className="p-4 bg-white rounded-xl border border-amber-200"
                  >
                    <p className="font-bold text-slate-900">{r.fileName}</p>
                    <p className="text-sm text-slate-600 mt-1">{failureHint(r)}</p>
                    {r.tempOneDrivePath && (
                      <p className="text-sm text-slate-700 mt-2">
                        Emplacement OneDrive :{" "}
                        <span className="font-semibold">Temp / {r.tempOneDrivePath.replace(/^Temp\//, "")}</span>
                      </p>
                    )}
                  </li>
                ))}
              </ul>
            </div>
          )}

          {ocrResults.length > 0 && (
            <div>
              <h3 className="text-xl font-black text-gray-900 mb-4">
                Journal d&apos;analyse
              </h3>
              <div className="grid grid-cols-1 gap-3">
                {[...ocrResults]
                  .sort((a, b) =>
                    a.success === b.success ? 0 : a.success ? 1 : -1
                  )
                  .map((result, index) => (
                    <div
                      key={`${ocrResultsSessionId}-${result.fileName}-${index}`}
                      className={`p-4 rounded-2xl border ${
                        result.success
                          ? "bg-white border-gray-100"
                          : "bg-red-50 border-red-100 ring-2 ring-red-400/20"
                      }`}
                    >
                      <div className="flex items-center gap-2 flex-wrap">
                        <p
                          className={`font-bold ${result.success ? "text-gray-800" : "text-red-700"}`}
                        >
                          {result.fileName}
                        </p>
                        {!result.success && (
                          <span
                            className={`text-[10px] px-2 py-0.5 rounded-full font-black uppercase text-white ${
                              failureCategory(result).technical ? "bg-orange-600" : "bg-red-600"
                            }`}
                          >
                            {failureCategory(result).label}
                          </span>
                        )}
                      </div>
                      {result.success ? (
                        <p className="text-xs text-gray-500 mt-1">
                          Classé : {result.result?.fileName || "—"}
                        </p>
                      ) : (
                        <>
                          <p className="text-sm text-red-600 mt-1 font-medium">
                            {failureHint(result)}
                          </p>
                          {result.error ? (
                            <p className="text-[11px] text-slate-500 mt-1 font-mono break-words">
                              Détail : {result.error}
                            </p>
                          ) : null}
                          {result.tempOneDrivePath ? (
                            <p className="text-xs text-slate-600 mt-2 bg-slate-50 p-2 rounded-lg">
                              Fichier dans OneDrive → dossier{" "}
                              <strong>Temp</strong> ({result.tempOneDrivePath.replace(/^Temp\//, "")})
                            </p>
                          ) : null}
                        </>
                      )}
                      {result.result?.oneDriveItemPath || result.tempOneDrivePath ? (
                        <button
                          type="button"
                          onClick={() =>
                            void openOneDrivePath(
                              String(result.result?.oneDriveItemPath || result.tempOneDrivePath),
                            )
                          }
                          disabled={
                            openingOneDrivePath ===
                            String(result.result?.oneDriveItemPath || result.tempOneDrivePath)
                          }
                          className="mt-2 inline-flex items-center rounded-lg border border-indigo-200 bg-indigo-50 px-2.5 py-1 text-xs font-bold text-indigo-800 hover:bg-indigo-100 disabled:opacity-60"
                        >
                          {openingOneDrivePath ===
                          String(result.result?.oneDriveItemPath || result.tempOneDrivePath)
                            ? "Ouverture…"
                            : "Ouvrir le document source dans OneDrive"}
                        </button>
                      ) : null}
                    </div>
                  ))}
              </div>
            </div>
          )}

          <details
            data-tour="eleves-import"
            className="mt-10 rounded-3xl border border-slate-200 bg-white shadow-sm overflow-hidden group"
          >
            <summary className="cursor-pointer list-none px-6 py-4 font-bold text-slate-800 hover:bg-slate-50 flex items-center justify-between gap-2">
              <span>Configuration — liste élèves & dossiers OneDrive</span>
              <span className="text-slate-400 text-sm font-normal group-open:rotate-180 transition-transform">
                ▼
              </span>
            </summary>
            <div className="px-6 pb-6 pt-2 border-t border-slate-100 space-y-6">
              <p className="text-sm text-slate-600">
                Trois actions dans l&apos;ordre : importer la liste élèves, configurer la table MEF, puis créer les
                dossiers OneDrive manquants.
                {elevesCount != null && (
                  <span className="block mt-1 font-medium text-slate-800">
                    {elevesCount} élève(s) actuellement enregistré(s) pour le classement automatique.
                  </span>
                )}
              </p>

              <section className="rounded-xl border border-indigo-200 bg-indigo-50/50 p-4 space-y-3">
                <h3 className="text-sm font-bold text-slate-800">1 — Liste élèves (référentiel global)</h3>
                <p className="text-xs text-slate-600 leading-relaxed">
                  L&apos;import Excel des élèves se fait désormais dans les{" "}
                  <a href="/parametres?tab=referentiel" className="font-bold text-indigo-800 underline">
                    Paramètres généraux → Référentiel scolaire
                  </a>
                  . Le fichier <code className="bg-white px-1 rounded">eleves.json</code> alimente tous les modules
                  (stages, certificats, répartition des classes, reconnaissance IA…).
                </p>
                {elevesCount != null && (
                  <p className="text-sm font-medium text-slate-800">
                    {elevesCount} élève(s) actuellement enregistré(s).
                  </p>
                )}
                <a
                  href="/parametres?tab=referentiel"
                  className="inline-flex rounded-xl bg-indigo-600 px-4 py-2 text-sm font-bold text-white hover:bg-indigo-700"
                >
                  Mettre à jour la liste élèves →
                </a>
              </section>

              <section
                data-tour="mef-table"
                className="rounded-xl border border-slate-200 bg-slate-50/50 p-4 space-y-3"
              >
                <h3 className="text-sm font-bold text-slate-800">2 — Table des formations (MEF)</h3>
                <p className="text-xs text-slate-600 leading-relaxed">
                  Associe chaque <strong>code ou libellé MEF</strong> (colonne E de l&apos;Excel : 3EME, 2NDE,
                  Cycle 2…) au secteur <strong>Lycée</strong>, <strong>Collège</strong> ou{" "}
                  <strong>École</strong>. Cela permet à chaque secrétariat de ne traiter que ses élèves et de
                  créer les bons dossiers OneDrive.
                </p>
                <p className="text-xs text-slate-500">
                  Configuration habituelle :{" "}
                  <a href="/parametres" className="text-indigo-600 font-medium hover:underline">
                    Paramètres → Formations MEF
                  </a>{" "}
                  (une fois par an ou si les formations changent). Le bouton ci-dessous est un raccourci pour
                  importer un fichier JSON déjà préparé.
                </p>
                {mefCounts && mefCounts.total > 0 && (
                  <p className="text-xs font-medium text-emerald-800 bg-emerald-50 border border-emerald-100 rounded-lg px-3 py-2">
                    Table MEF active : {mefCounts.total} formation(s) — lycée {mefCounts.lycee}, collège{" "}
                    {mefCounts.college}, école {mefCounts.ecole}.
                  </p>
                )}
                <div>
                  <button
                    type="button"
                    disabled={mefUploading}
                    onClick={() => mefInputRef.current?.click()}
                    className="px-4 py-2 border border-slate-300 hover:bg-white disabled:opacity-50 text-slate-700 text-sm font-bold rounded-xl bg-white"
                  >
                    {mefUploading ? "Envoi…" : "Importer table MEF (JSON)"}
                  </button>
                </div>
                {mefMessage && (
                  <p className={`text-sm ${mefMessage.startsWith("Erreur") ? "text-red-600" : "text-slate-700"}`}>
                    {mefMessage}
                  </p>
                )}
              </section>

              <section
                data-tour="sync-onedrive"
                className="rounded-xl border border-slate-200 bg-slate-50/50 p-4 space-y-3"
              >
                <h3 className="text-sm font-bold text-slate-800">3 — Créer les dossiers sur OneDrive</h3>
                <p className="text-xs text-slate-600 leading-relaxed">
                  Crée un dossier par élève de <strong>votre secteur</strong> (format « NOM Prenom » — sans
                  tirets, sans classe)
                  dans l&apos;arborescence OneDrive connectée en haut de page.
                </p>
                <div className="rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-xs text-emerald-950 leading-relaxed space-y-1">
                  <p className="font-bold">Sans risque pour les dossiers existants</p>
                  <p>
                    Ce bouton <strong>ne supprime ni ne renomme rien</strong>. Il ajoute uniquement les dossiers
                    manquants pour les élèves de la liste actuelle. Les dossiers déjà là sont laissés tels quels.
                  </p>
                  <p>
                    Il est normal d&apos;avoir <strong>plus de dossiers sur OneDrive</strong> que d&apos;élèves dans
                    la liste : les anciens élèves partis restent archivés sur OneDrive et ne sont{" "}
                    <strong>jamais supprimés</strong> par cette action.
                  </p>
                </div>
                <div>
                  <button
                    type="button"
                    disabled={!dropsAvailable || syncingFolders || checkingOneDrive}
                    onClick={handleSyncOneDriveFolders}
                    className="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white text-sm font-bold rounded-xl"
                  >
                    {syncingFolders ? "Synchronisation…" : "Créer les dossiers sur OneDrive"}
                  </button>
                  {!dropsAvailable && (
                    <p className="mt-2 text-xs text-amber-700">
                      Connectez OneDrive en haut de page pour activer ce bouton.
                    </p>
                  )}
                </div>
                {syncReport && (
                  <div className="p-4 bg-white rounded-xl border border-slate-200 text-sm text-slate-700 space-y-3">
                    <div>
                      <p className="font-bold text-slate-900">
                        {syncReport.secteurLabel} — {syncReport.basePath}
                      </p>
                      <p className="text-slate-600 mt-1">{syncReport.message}</p>
                    </div>

                    <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 text-xs">
                      <div className="rounded-lg bg-emerald-50 border border-emerald-100 px-3 py-2">
                        <p className="font-bold text-emerald-900">{syncReport.created ?? 0}</p>
                        <p className="text-emerald-800">créé(s)</p>
                      </div>
                      <div className="rounded-lg bg-slate-50 border border-slate-200 px-3 py-2">
                        <p className="font-bold text-slate-800">{syncReport.alreadyThere ?? 0}</p>
                        <p className="text-slate-600">déjà présent(s)</p>
                      </div>
                      <div className="rounded-lg bg-amber-50 border border-amber-100 px-3 py-2">
                        <p className="font-bold text-amber-900">{syncReport.extraFoldersCount ?? 0}</p>
                        <p className="text-amber-800">archives OneDrive</p>
                      </div>
                      <div className="rounded-lg bg-slate-50 border border-slate-200 px-3 py-2">
                        <p className="font-bold text-slate-800">{syncReport.jsonForYourSecteur ?? "—"}</p>
                        <p className="text-slate-600">élèves secteur</p>
                      </div>
                    </div>

                    {(syncReport.createdFolders?.length ?? 0) > 0 && (
                      <div>
                        <p className="text-xs font-bold text-slate-800 mb-1">
                          Dossiers créés ({syncReport.createdFolders!.length})
                        </p>
                        <ul className="max-h-48 overflow-y-auto rounded-lg border border-emerald-100 bg-emerald-50/50 text-xs font-mono divide-y divide-emerald-100">
                          {syncReport.createdFolders!.map((name) => (
                            <li key={name} className="px-3 py-1.5 text-emerald-950">
                              {name}
                            </li>
                          ))}
                        </ul>
                      </div>
                    )}

                    {(syncReport.created ?? 0) === 0 && (syncReport.alreadyThere ?? 0) > 0 && (
                      <p className="text-xs text-slate-600 leading-relaxed">
                        Tous les élèves de la liste avaient déjà leur dossier — rien à ajouter, c&apos;est normal.
                      </p>
                    )}

                    {(syncReport.extraFoldersCount ?? 0) > 0 && (
                      <p className="text-xs text-amber-900 bg-amber-50 border border-amber-100 rounded-lg px-3 py-2 leading-relaxed">
                        <strong>{syncReport.extraFoldersCount} dossier(s)</strong> sur OneDrive ne correspondent plus
                        à un élève de la liste actuelle (anciens élèves, archives…). Ils ont été{" "}
                        <strong>laissés en place</strong> — cette action ne les supprime pas.
                      </p>
                    )}

                    {(syncReport.ambiguousCount ?? 0) > 0 && (
                      <div>
                        <p className="text-xs font-bold text-amber-800 mb-1">
                          Non traités — MEF manquant ou inconnu ({syncReport.ambiguousCount})
                        </p>
                        <ul className="max-h-32 overflow-y-auto text-xs text-amber-900 space-y-0.5">
                          {syncReport.ambiguous?.map((a) => (
                            <li key={a.folderName}>
                              {a.folderName}
                              {a.mef ? ` (${a.mef})` : ""} — {a.reason}
                            </li>
                          ))}
                        </ul>
                      </div>
                    )}

                    {(syncReport.errors?.length ?? 0) > 0 && (
                      <div>
                        <p className="text-xs font-bold text-red-700 mb-1">Erreurs</p>
                        <ul className="text-xs text-red-600 space-y-0.5">
                          {syncReport.errors!.map((e) => (
                            <li key={e.folderName}>
                              {e.folderName} — {e.error}
                            </li>
                          ))}
                        </ul>
                      </div>
                    )}
                  </div>
                )}
              </section>

              <input
                ref={mefInputRef}
                type="file"
                accept=".json,application/json"
                className="hidden"
                onChange={(e) => {
                  const f = e.target.files?.[0];
                  if (f) handleMefUpload(f);
                }}
              />
            </div>
          </details>
          <ReplayModuleTourButton moduleId="agent-ia-ocr" />
      </>
    </div>
  );
}

export default function OneDriveUpDocsOCRAI() {
  return (
    <Suspense fallback={<div className="p-8 text-slate-500 text-sm">Chargement…</div>}>
      <OneDriveUpDocsOCRAIContent />
    </Suspense>
  );
}
