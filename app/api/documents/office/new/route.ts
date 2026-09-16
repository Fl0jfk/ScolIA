import { NextRequest, NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { createOfficeDocument } from "@/app/lib/office-create";
import type { OfficeKind } from "@/app/lib/office-types";
import { OFFICE_KIND_META } from "@/app/lib/office-types";
import type { DocumentScope } from "@/app/lib/documents-cloud";

function parseKind(raw: unknown): OfficeKind | null {
  const k = String(raw || "");
  return k === "writer" || k === "calc" || k === "impress" ? k : null;
}

export async function POST(req: NextRequest) {
  const gate = await requireModule("documents");
  if (!gate.ok) return gate.response;

  let body: Record<string, unknown>;
  try {
    body = (await req.json()) as Record<string, unknown>;
  } catch {
    return NextResponse.json({ error: "JSON invalide." }, { status: 400 });
  }

  const kind = parseKind(body.kind);
  if (!kind) return NextResponse.json({ error: "Type de document invalide." }, { status: 400 });

  const scopeRaw = String(body.scope || "personal");
  const scope: DocumentScope = scopeRaw === "shared" ? "shared" : "personal";
  const shareId = body.shareId ? String(body.shareId) : null;
  const parentRelPath = String(body.parentRelPath || body.path || "");

  const created = await createOfficeDocument({
    userId: gate.ctx.userId,
    kind,
    scope,
    shareId,
    parentRelPath,
  });
  if (!created.ok) {
    return NextResponse.json({ error: created.error }, { status: 400 });
  }

  const params = new URLSearchParams({
    kind,
    scope: created.scope,
    path: created.relPath,
    draft: created.fromModuleRoot ? "1" : "0",
  });
  if (created.shareId) params.set("shareId", created.shareId);

  return NextResponse.json({
    success: true,
    kind,
    label: OFFICE_KIND_META[kind].label,
    scope: created.scope,
    shareId: created.shareId,
    relPath: created.relPath,
    fileName: created.fileName,
    fromModuleRoot: created.fromModuleRoot,
    editUrl: `/documents/edit?${params.toString()}`,
  });
}
