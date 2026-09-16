import { NextRequest, NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { placeOfficeDocument } from "@/app/lib/office-create";
import type { OfficeKind, OfficeScope } from "@/app/lib/office-types";
import type { DocumentScope } from "@/app/lib/documents-cloud";

function parseKind(raw: unknown): OfficeKind | null {
  const k = String(raw || "");
  return k === "writer" || k === "calc" || k === "impress" ? k : null;
}

function parseOfficeScope(raw: unknown): OfficeScope | null {
  const s = String(raw || "");
  return s === "personal" || s === "shared" || s === "fileshare" ? s : null;
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
  const fromScope = parseOfficeScope(body.fromScope);
  if (!kind || !fromScope) {
    return NextResponse.json({ error: "Paramètres invalides." }, { status: 400 });
  }

  const toScopeRaw = String(body.toScope || "personal");
  const toScope: DocumentScope = toScopeRaw === "shared" ? "shared" : "personal";

  const placed = await placeOfficeDocument({
    userId: gate.ctx.userId,
    kind,
    fromScope,
    fromShareId: body.fromShareId ? String(body.fromShareId) : null,
    fromRelPath: String(body.fromRelPath || ""),
    newName: String(body.newName || "Sans titre"),
    toScope,
    toShareId: body.toShareId ? String(body.toShareId) : null,
    toParentRelPath: String(body.toParentRelPath || ""),
  });

  if (!placed.ok) {
    return NextResponse.json({ error: placed.error }, { status: 400 });
  }

  const params = new URLSearchParams({
    kind,
    scope: placed.scope,
    path: placed.relPath,
  });
  if (placed.shareId) params.set("shareId", placed.shareId);

  return NextResponse.json({
    success: true,
    ...placed,
    editUrl: `/documents/edit?${params.toString()}`,
  });
}
