import { graphDriveRootItemUrl } from "@/app/lib/graph-onedrive-path";
import { BATCH_JOB_STORAGE_KEY } from "@/app/lib/ocr-page-model";

export type SidebarOcrDepositProgress = {
  phase: "uploading" | "starting" | "processing" | "done" | "error";
  percent: number;
  label: string;
  fileIndex?: number;
  fileTotal?: number;
  jobId?: string | null;
  completed?: number;
  failed?: number;
  serverSelfRelays?: boolean;
};

function sleep(ms: number) {
  return new Promise((r) => setTimeout(r, ms));
}

function parseRetryAfterMs(res: Response, attempt: number): number {
  const raw = res.headers.get("Retry-After");
  if (raw) {
    const sec = Number(raw);
    if (!Number.isNaN(sec) && sec > 0) return sec * 1000;
  }
  return Math.min(60_000, 4000 * 2 ** attempt);
}

async function uploadOnePdf(opts: {
  file: File;
  accessToken: string;
  signal?: AbortSignal;
  refreshToken: () => Promise<string | null>;
}): Promise<{ s3Key: string; tempPath: string }> {
  const { file, signal } = opts;
  let accessToken = opts.accessToken;

  const r1 = await fetch("/api/agentIAOCR/upload-url", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      filename: file.name,
      contentType: file.type || "application/pdf",
    }),
    signal,
  });
  if (!r1.ok) throw new Error(await r1.text());
  const { url, key } = (await r1.json()) as { url: string; key: string };

  const upload = await fetch(url, {
    method: "PUT",
    headers: { "Content-Type": file.type || "application/pdf" },
    body: file,
    signal,
  });
  if (!upload.ok) throw new Error("Échec upload S3 : " + (await upload.text()));

  const tempPath = `Temp/${file.name}`;
  const putOneDrive = (token: string) =>
    fetch(graphDriveRootItemUrl(tempPath, "/content"), {
      method: "PUT",
      headers: {
        Authorization: `Bearer ${token}`,
        "Content-Type": file.type || "application/pdf",
      },
      body: file,
      signal,
    });

  let odRes: Response | null = null;
  for (let attempt = 0; attempt < 4; attempt++) {
    odRes = await putOneDrive(accessToken);
    if (odRes.status === 401) {
      const refreshed = await opts.refreshToken();
      if (!refreshed) {
        throw new Error("Session OneDrive expirée — reconnectez-vous.");
      }
      accessToken = refreshed;
      continue;
    }
    if (odRes.status === 429) {
      await sleep(parseRetryAfterMs(odRes, attempt));
      continue;
    }
    break;
  }
  if (!odRes || !odRes.ok) {
    const detail = odRes ? await odRes.text() : "réponse vide";
    const status = odRes?.status ?? 0;
    throw new Error(`Échec upload OneDrive Temp (${status}) : ${detail.slice(0, 280)}`);
  }
  return { s3Key: key, tempPath };
}

/**
 * Dépose des PDF depuis la sidebar : upload S3 + OneDrive Temp, crée le lot OCR,
 * déclenche le worker. Ne navigue pas — le traitement continue côté serveur.
 */
export async function startSidebarOcrDeposit(opts: {
  files: File[];
  getAccessToken: () => Promise<string | null>;
  onProgress: (p: SidebarOcrDepositProgress) => void;
  signal?: AbortSignal;
}): Promise<{ jobId: string; serverSelfRelays: boolean }> {
  const pdfs = opts.files.filter(
    (f) => f.type === "application/pdf" || f.name.toLowerCase().endsWith(".pdf"),
  );
  if (pdfs.length === 0) {
    throw new Error("PDF uniquement.");
  }

  let token = await opts.getAccessToken();
  if (!token) throw new Error("Connectez OneDrive avant de déposer.");

  opts.onProgress({
    phase: "uploading",
    percent: 2,
    label: "Envoi des fichiers…",
    fileIndex: 0,
    fileTotal: pdfs.length,
  });

  const items: Array<{
    fileName: string;
    mode: "standard" | "class";
    s3Key: string;
    tempPath: string;
  }> = [];

  for (let i = 0; i < pdfs.length; i++) {
    if (opts.signal?.aborted) throw new DOMException("Aborted", "AbortError");
    const file = pdfs[i]!;
    opts.onProgress({
      phase: "uploading",
      percent: Math.max(3, Math.round((32 * (i + 1)) / pdfs.length)),
      label: `Envoi ${i + 1} / ${pdfs.length} — ${file.name}`,
      fileIndex: i + 1,
      fileTotal: pdfs.length,
    });
    if (i > 0 && i % 25 === 0) {
      const refreshed = await opts.getAccessToken();
      if (refreshed) token = refreshed;
    }
    const uploaded = await uploadOnePdf({
      file,
      accessToken: token,
      signal: opts.signal,
      refreshToken: opts.getAccessToken,
    });
    items.push({
      fileName: file.name,
      mode: "class",
      s3Key: uploaded.s3Key,
      tempPath: uploaded.tempPath,
    });
  }

  opts.onProgress({
    phase: "starting",
    percent: 38,
    label: "Lancement du traitement OCR…",
    fileTotal: pdfs.length,
  });

  const freshToken = (await opts.getAccessToken()) ?? token;
  const createRes = await fetch("/api/agentIAOCR/batch-job", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ accessToken: freshToken, items }),
    signal: opts.signal,
  });
  if (!createRes.ok) throw new Error(await createRes.text());
  const created = (await createRes.json()) as {
    jobId?: string;
    serverSelfRelays?: boolean;
  };
  const jobId = created.jobId?.trim();
  if (!jobId) throw new Error("Impossible de créer le traitement serveur.");

  try {
    localStorage.setItem(BATCH_JOB_STORAGE_KEY, jobId);
  } catch {
    /* ignore */
  }

  const serverSelfRelays = Boolean(created.serverSelfRelays);
  try {
    await fetch("/api/agentIAOCR/batch-job/process", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ jobId }),
      signal: opts.signal,
    });
  } catch {
    /* le statut reste la source de vérité */
  }

  opts.onProgress({
    phase: "processing",
    percent: 42,
    label: serverSelfRelays
      ? `OCR en cours (${pdfs.length} PDF) — vous pouvez rester ici`
      : `OCR lancé (${pdfs.length} PDF)`,
    jobId,
    fileTotal: pdfs.length,
    serverSelfRelays,
    completed: 0,
    failed: 0,
  });

  return { jobId, serverSelfRelays };
}

export async function pollSidebarOcrJobStatus(
  jobId: string,
  signal?: AbortSignal,
): Promise<{
  status: string;
  percent: number;
  label: string;
  completed: number;
  failed: number;
  total: number;
  done: boolean;
}> {
  const res = await fetch(
    `/api/agentIAOCR/batch-job/status?jobId=${encodeURIComponent(jobId)}`,
    { cache: "no-store", signal },
  );
  if (!res.ok) throw new Error(await res.text());
  const st = (await res.json()) as {
    status?: string;
    percent?: number;
    label?: string;
    completed?: number;
    failed?: number;
    totalItems?: number;
    currentItemIndex?: number;
    progress?: {
      percent?: number;
      label?: string;
      documentsSucceeded?: number;
      documentsFailed?: number;
    };
  };
  const status = String(st.status || "");
  const percent = st.progress?.percent ?? st.percent ?? 0;
  const label = st.progress?.label || st.label || "Traitement…";
  const completed = st.progress?.documentsSucceeded ?? st.completed ?? 0;
  const failed = st.progress?.documentsFailed ?? st.failed ?? 0;
  const total = st.totalItems ?? 0;
  const done =
    status === "completed" ||
    status === "failed" ||
    status === "cancelled" ||
    status === "partial";
  return {
    status,
    percent: typeof percent === "number" ? percent : 0,
    label,
    completed,
    failed,
    total,
    done,
  };
}
