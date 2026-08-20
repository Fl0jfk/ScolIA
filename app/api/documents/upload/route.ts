import { NextRequest, NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import {
  DOCUMENTS_MAX_FILE_BYTES,
  DOCUMENTS_MAX_FILE_LABEL,
  prepareDocumentUpload,
  uploadDocumentFile,
  type DocumentScope,
} from "@/app/lib/documents-cloud";
import { documentsMaxFileError } from "@/app/lib/documents-page-model";

export const maxDuration = 60;

function parseScope(raw: unknown): DocumentScope | null {
  const scope = String(raw || "personal") as DocumentScope;
  return scope === "personal" || scope === "shared" ? scope : null;
}

function resolveTargetPath(path: string, relPath: string, fallbackName: string) {
  const parts = relPath.replace(/\\/g, "/").split("/").filter(Boolean);
  const fileName = parts.pop() ?? fallbackName;
  const subDir = parts.join("/");
  const base = path.endsWith("/") ? path : path ? `${path}/` : "";
  const targetPath = subDir ? `${base}${subDir}/` : base;
  return { fileName, targetPath };
}

/** Prépare un PUT direct S3 (recommandé — gros fichiers hors limite body conteneur). */
async function prepareFromJson(req: NextRequest, userId: string) {
  const body = await req.json();
  const scope = parseScope(body.scope);
  if (!scope) {
    return NextResponse.json({ error: "Scope invalide." }, { status: 400 });
  }

  const shareId = body.shareId ? String(body.shareId) : null;
  const path = String(body.path ?? "");
  const fileName = String(body.fileName || "").trim();
  const contentType = String(body.contentType || "application/octet-stream");
  const size = Number(body.size);
  const relPath = String(body.relPath || fileName);
  const { fileName: resolvedName, targetPath } = resolveTargetPath(path, relPath, fileName);

  if (!resolvedName) {
    return NextResponse.json({ error: "Nom de fichier manquant." }, { status: 400 });
  }

  const result = await prepareDocumentUpload(
    userId,
    scope,
    shareId,
    targetPath,
    resolvedName,
    contentType,
    size,
  );
  if (!result.ok) {
    const status = result.used !== undefined || result.error.includes("trop volumineux") ? 413 : 400;
    return NextResponse.json(
      { error: result.error, used: result.used, quota: result.quota },
      { status },
    );
  }

  return NextResponse.json({
    uploadUrl: result.uploadUrl,
    key: result.key,
    maxFileLabel: DOCUMENTS_MAX_FILE_LABEL,
  });
}

async function uploadFromFormData(req: NextRequest, userId: string) {
  const formData = await req.formData();
  const scope = parseScope(formData.get("scope"));
  if (!scope) {
    return NextResponse.json({ error: "Scope invalide." }, { status: 400 });
  }

  const shareIdRaw = formData.get("shareId");
  const shareId = shareIdRaw ? String(shareIdRaw) : null;
  const path = String(formData.get("path") ?? "");

  const files = formData.getAll("file").filter((f): f is File => f instanceof File);
  if (files.length === 0) {
    return NextResponse.json({ error: "Aucun fichier." }, { status: 400 });
  }

  const errors: string[] = [];
  for (const file of files) {
    if (file.size > DOCUMENTS_MAX_FILE_BYTES) {
      return NextResponse.json({ error: documentsMaxFileError(file.name) }, { status: 413 });
    }
    const relPath = String(formData.get(`relPath:${file.name}`) ?? file.name);
    const { fileName, targetPath } = resolveTargetPath(path, relPath, file.name);

    const buffer = Buffer.from(await file.arrayBuffer());
    const result = await uploadDocumentFile(
      userId,
      scope,
      shareId,
      targetPath,
      fileName,
      buffer,
      file.type,
    );
    if (!result.ok) {
      errors.push(`${file.name}: ${result.error}`);
      if (result.used !== undefined || result.error.includes("trop volumineux")) {
        return NextResponse.json(
          { error: result.error, used: result.used, quota: result.quota },
          { status: 413 },
        );
      }
    }
  }

  if (errors.length > 0) {
    return NextResponse.json({ error: errors.join(" ") }, { status: 400 });
  }

  return NextResponse.json({ success: true, count: files.length });
}

export async function POST(req: NextRequest) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  try {
    const contentType = req.headers.get("content-type") || "";
    if (contentType.includes("application/json")) {
      return await prepareFromJson(req, gate.ctx.userId);
    }
    return await uploadFromFormData(req, gate.ctx.userId);
  } catch (e) {
    console.error("[documents/upload]", e);
    return NextResponse.json({ error: "Échec de l'envoi." }, { status: 500 });
  }
}
