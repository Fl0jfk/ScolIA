'use client';
import { Suspense, useState, useEffect, useRef, useCallback } from "react";
import { useSearchParams } from "next/navigation";
import { useUser } from "@clerk/nextjs";
import * as msal from "@azure/msal-browser";
import { consumeDashboardUpload } from "@/app/lib/dashboard-upload-bridge";
import type { OneDriveUserProfile } from "@/app/lib/onedrive-user-profiles";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import OcrBatchProgress from "@/app/components/ocr/OcrBatchProgress";
import OcrConfigPanel from "@/app/components/ocr/OcrConfigPanel";
import OcrDropZones from "@/app/components/ocr/OcrDropZones";
import OcrOneDriveConnectBar from "@/app/components/ocr/OcrOneDriveConnectBar";
import OcrResultsList from "@/app/components/ocr/OcrResultsList";
import OcrSessionStats from "@/app/components/ocr/OcrSessionStats";
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
import {
  BATCH_JOB_LAST_RESULTS_KEY,
  BATCH_JOB_STORAGE_KEY,
  INITIAL_OCR_PROCESSING_STATUS,
  buildOcrProgressCaption,
  isOcrBatchJobActive,
  isOcrBatchJobCancelled,
  mergeOcrResultsForUi,
  ocrSuggestedEleves,
  type BatchJobStatusPayload,
  type OcrMefCounts,
  type OcrProgressDetail,
  type OcrSyncReport,
  type ProcessResult,
} from "@/app/lib/ocr-page-model";

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

function OneDriveUpDocsOCRAIContent() {
  const searchParams = useSearchParams();
  const { user: clerkUser } = useUser();
  const [oneDriveProfile, setOneDriveProfile] = useState<OneDriveUserProfile | null>(null);
  useEffect(() => {
    if (!clerkUser) {
      setOneDriveProfile(null);
      return;
    }
    let cancelled = false;
    fetch("/api/onedrive/profile")
      .then((r) => r.json())
      .then((j) => {
        if (!cancelled) setOneDriveProfile(j.profile || null);
      })
      .catch(() => {
        if (!cancelled) setOneDriveProfile(null);
      });
    return () => {
      cancelled = true;
    };
  }, [clerkUser]);
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
  const [syncReport, setSyncReport] = useState<OcrSyncReport | null>(null);
  const [mefCounts, setMefCounts] = useState<OcrMefCounts | null>(null);
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

  const forgetPersistedBatchJob = useCallback(() => {
    localStorage.removeItem(BATCH_JOB_STORAGE_KEY);
    localStorage.removeItem(BATCH_JOB_LAST_RESULTS_KEY);
  }, []);

  const postCancelBatchJob = useCallback(async (jobId: string): Promise<ProcessResult[] | null> => {
    try {
      const res = await fetch("/api/agentIAOCR/batch-job/cancel", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ jobId }),
      });
      const data = (await res.json().catch(() => ({}))) as {
        error?: string;
        results?: ProcessResult[];
      };
      if (!res.ok) return null;
      return Array.isArray(data.results) ? data.results : [];
    } catch {
      return null;
    }
  }, []);

  const cancelOrphanActiveJobs = useCallback(
    async (keepJobId?: string | null) => {
      try {
        const listRes = await fetch("/api/agentIAOCR/batch-job/list");
        if (!listRes.ok) return;
        const listData = await listRes.json();
        const jobs =
          (listData.jobs as Array<{ jobId: string; status: string }> | undefined) ?? [];
        const ids = jobs
          .filter((j) => isOcrBatchJobActive(j.status) && j.jobId !== keepJobId)
          .map((j) => j.jobId);
        await Promise.all(ids.map((id) => postCancelBatchJob(id)));
      } catch {
        /* ignore */
      }
    },
    [postCancelBatchJob],
  );

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

    const knownJobId = activeBatchJobId;
    // Couper le poll tout de suite : un statut encore « processing » ne doit plus réécrire l'UI.
    activeBatchJobIdRef.current = null;
    setActiveBatchJobId(null);
    setOcrProcessing(false);
    setBatchJobNeedsToken(false);
    setProgressDetail(null);
    forgetPersistedBatchJob();
    setProcessingStatus({
      ...INITIAL_OCR_PROCESSING_STATUS,
      label: "Traitement annulé",
    });
    setError("");

    const ids = new Set<string>();
    if (knownJobId) ids.add(knownJobId);
    try {
      const listRes = await fetch("/api/agentIAOCR/batch-job/list");
      if (listRes.ok) {
        const listData = await listRes.json();
        const jobs =
          (listData.jobs as Array<{ jobId: string; status: string }> | undefined) ?? [];
        for (const j of jobs) {
          if (isOcrBatchJobActive(j.status)) ids.add(j.jobId);
        }
      }
    } catch {
      /* le job connu ci-dessous reste la cible principale */
    }

    let lastResults: ProcessResult[] | null = null;
    for (const jobId of ids) {
      const results = await postCancelBatchJob(jobId);
      if (results && results.length > 0) lastResults = results;
    }
    if (lastResults) {
      setOcrResults(lastResults);
      setOcrResultsSessionId((id) => id + 1);
    }
  }, [abortOcrInFlight, activeBatchJobId, forgetPersistedBatchJob, postCancelBatchJob]);

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
      if (isOcrBatchJobCancelled(st.status, st.error)) {
        setOcrProcessing(false);
        activeBatchJobIdRef.current = null;
        setActiveBatchJobId(null);
        forgetPersistedBatchJob();
        return;
      }
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
  }, [applyBatchJobStatusToUi, forgetPersistedBatchJob, persistFinishedBatchJob, triggerBatchWorker]);

  useEffect(() => {
    if (!clerkUser?.id) return;
    void (async () => {
      try {
        const listRes = await fetch("/api/agentIAOCR/batch-job/list");
        if (!listRes.ok) return;
        const listData = await listRes.json();
        const jobs =
          (listData.jobs as Array<{ jobId: string; status: string; error?: string | null }> | undefined) ??
          [];
        const active = jobs.find((j) => isOcrBatchJobActive(j.status));
        const storedActive = localStorage.getItem(BATCH_JOB_STORAGE_KEY);
        const storedLast = localStorage.getItem(BATCH_JOB_LAST_RESULTS_KEY);
        const jobId = active?.jobId || storedActive || storedLast;
        if (!jobId) return;

        const stRes = await fetch(
          `/api/agentIAOCR/batch-job/status?jobId=${encodeURIComponent(jobId)}`,
        );
        if (!stRes.ok) return;
        const st = (await stRes.json()) as BatchJobStatusPayload & { jobId?: string };

        if (isOcrBatchJobCancelled(st.status, st.error)) {
          forgetPersistedBatchJob();
          return;
        }

        if (isOcrBatchJobActive(st.status)) {
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
  }, [clerkUser?.id, applyBatchJobStatusToUi, forgetPersistedBatchJob, persistFinishedBatchJob]);

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

        if (isOcrBatchJobCancelled(st.status, st.error)) {
          setOcrProcessing(false);
          activeBatchJobIdRef.current = null;
          setActiveBatchJobId(null);
          forgetPersistedBatchJob();
          setError("");
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
          if (st.status === "failed" && st.error && !isOcrBatchJobCancelled(st.status, st.error)) {
            setError(String(st.error));
          }
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
    forgetPersistedBatchJob,
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

      if (!isActiveOcrSession(sessionId) || signal.aborted) {
        await postCancelBatchJob(jobId);
        forgetPersistedBatchJob();
        return;
      }

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
      if (err instanceof DOMException && err.name === "AbortError") {
        await cancelOrphanActiveJobs(activeBatchJobIdRef.current);
        return;
      }
      if (!isActiveOcrSession(sessionId)) {
        await cancelOrphanActiveJobs(activeBatchJobIdRef.current);
        return;
      }
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
    ocrSessionIdRef.current += 1;
    prepareOcrSessionForNewBatch();
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
      <ModulePageShell>
        <div className="flex items-center justify-center min-h-[60vh]">
          <div className="flex flex-col items-center gap-4">
            <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600"></div>
            <p className="text-gray-500 font-medium">Initialisation de MSAL...</p>
          </div>
        </div>
      </ModulePageShell>
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
  const sessionDocReview = ocrResults.filter((r) => !r.success && ocrSuggestedEleves(r).length > 0).length;
  const sessionDocProcessed = sessionDocSucceeded + sessionDocFailed;
  const rawDocTotal = Math.max(
    progressDetail?.documentsTotal ?? 0,
    progressDetail?.fileTotal ?? 0,
    progressDetail?.phase === "segments" ? (progressDetail.segmentTotal ?? 0) : 0,
  );
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
  const progressCaption = buildOcrProgressCaption({
    isUploadPhase,
    processingStatus,
    progressDetail,
    sessionDocTotal,
    sessionDocProcessed,
  });

  return (
    <ModulePageShell tourModuleId="agent-ia-ocr">
      <ModulePageHeader
        eyebrow="Élèves"
        title="Ajout de documents IA"
        description="Numérisez et rangez vos PDF dans les dossiers élèves sur OneDrive."
      />

      {error ? (
        <div className="mb-6 p-4 bg-red-50 border-l-4 border-red-500 text-red-700 rounded-r-xl">
          <p className="font-bold">Attention</p>
          <p className="text-sm">{error}</p>
        </div>
      ) : null}

      <OcrOneDriveConnectBar
        dropsAvailable={dropsAvailable}
        accountName={account?.name}
        clerkUnmapped={
          clerkUser && !oneDriveProfile
            ? {
                lastName: clerkUser.lastName,
                email: clerkUser.primaryEmailAddress?.emailAddress,
              }
            : null
        }
        oneDriveProfile={oneDriveProfile}
        checkingOneDrive={checkingOneDrive}
        showReconnect={Boolean(account)}
        onLogin={() => void login()}
        onReconnect={() => void reconnectOneDrive()}
      />

      <OcrBatchProgress
        batchPollIssue={batchPollIssue}
        activeBatchJobId={activeBatchJobId}
        batchJobNeedsToken={batchJobNeedsToken}
        batchServerSelfRelays={batchServerSelfRelays}
        isServerPhase={isServerPhase}
        isUploadPhase={isUploadPhase}
        ocrProcessing={ocrProcessing}
        progressPercent={progressPercent}
        progressCaption={progressCaption}
        progressDetail={progressDetail}
        processingStatus={processingStatus}
        sessionDocTotal={sessionDocTotal}
        sessionDocProcessed={sessionDocProcessed}
        onResumeBatchTracking={() => void resumeBatchTracking()}
        onResumeBatchWithOneDrive={() => void resumeBatchWithOneDrive()}
        onCancel={() => void cancelOcrProcessing()}
      />

      <OcrDropZones
        dropsAvailable={dropsAvailable}
        dropDisabled={dropDisabled}
        ocrProcessing={ocrProcessing}
        checkingOneDrive={checkingOneDrive}
        isDraggingClass={isDraggingClass}
        inputRef={classInputRef}
        onDraggingChange={setIsDraggingClass}
        onFiles={(files) => void enqueueAuto(files)}
      />

      <OcrSessionStats
        processingStatus={processingStatus}
        progressDetail={progressDetail}
        sessionDocTotal={sessionDocTotal}
        sessionDocProcessed={sessionDocProcessed}
        sessionDocSucceeded={sessionDocSucceeded}
        sessionDocFailed={sessionDocFailed}
        sessionDocReview={sessionDocReview}
        canStartFreshSession={canStartFreshSession}
        onStartFreshSession={handleStartFreshOcrSession}
      />

      <OcrResultsList
        ocrResults={ocrResults}
        ocrResultsSessionId={ocrResultsSessionId}
        openingOneDrivePath={openingOneDrivePath}
        onOpenOneDrivePath={(path) => void openOneDrivePath(path)}
        accessToken={accessToken}
        onManualFiled={(fileName, candidate, finalFileName) => {
          setOcrResults((prev) =>
            prev.map((r) =>
              r.fileName === fileName && !r.success
                ? {
                    ...r,
                    success: true,
                    error: undefined,
                    result: {
                      ...r.result,
                      fileName: finalFileName.replace(/\.pdf$/i, ""),
                      oneDriveItemPath: candidate.folderPath
                        ? `${candidate.folderPath}/${finalFileName}`
                        : r.result?.oneDriveItemPath,
                      matchedEleve: {
                        nom: candidate.nom,
                        prenom: candidate.prenom,
                        folderName: candidate.folderName,
                      },
                      matchDebug: { ...r.result?.matchDebug, matchedBy: "manual", decision: "auto" },
                    },
                  }
                : r,
            ),
          );
        }}
      />

      <OcrConfigPanel
        elevesCount={elevesCount}
        mefCounts={mefCounts}
        mefUploading={mefUploading}
        mefMessage={mefMessage}
        mefInputRef={mefInputRef}
        dropsAvailable={dropsAvailable}
        checkingOneDrive={checkingOneDrive}
        syncingFolders={syncingFolders}
        syncReport={syncReport}
        onMefFile={(file) => void handleMefUpload(file)}
        onSyncFolders={() => void handleSyncOneDriveFolders()}
      />
    </ModulePageShell>
  );
}

export default function OneDriveUpDocsOCRAI() {
  return (
    <Suspense
      fallback={
        <ModulePageShell>
          <p className="text-slate-500 text-sm">Chargement…</p>
        </ModulePageShell>
      }
    >
      <OneDriveUpDocsOCRAIContent />
    </Suspense>
  );
}
