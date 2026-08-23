import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import {
  canEditAwardLines,
  canManageAwardSignatories,
  canSignAwardAsDirectionForUserId,
  canViewAward,
  eligibleSignatoryIds,
} from "@/app/lib/certificates-auth";
import {
  CERTIFICATE_DIRECTION_LABELS,
  certificateUid,
} from "@/app/lib/certificates-types";
import { loadAward, loadProgram, saveAward } from "@/app/lib/certificates-storage";
import {
  buildCertificateLineFromInput,
  parseCertificateLineInput,
} from "@/app/lib/certificates-line";
import {
  buildDesignatedSignatories,
  mergeAwardSignatories,
  removePendingSignatory,
  recomputeAwardSigningStatus,
  resolveDirectoryDisplayName,
} from "@/app/lib/certificates-workflow";

type Params = { params: Promise<{ id: string }> };

export async function GET(_req: Request, { params }: Params) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const user = await safeCurrentUser();
  const { id } = await params;
  const award = await loadAward(id);
  if (!award) return NextResponse.json({ error: "Fiche introuvable." }, { status: 404 });
  const program = await loadProgram(award.programId);
  if (!program) return NextResponse.json({ error: "Parcours introuvable." }, { status: 404 });
  if (!canViewAward(award, program, gate.ctx.userId, user)) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }
  return NextResponse.json({
    award,
    program,
    eligibleSignatoryIds: eligibleSignatoryIds(program),
    directionLabel: CERTIFICATE_DIRECTION_LABELS[award.student.secteur],
    permissions: {
      canManageSignatories: canManageAwardSignatories(award, program, gate.ctx.userId),
      canSignDirection: await canSignAwardAsDirectionForUserId(gate.ctx.userId, award),
    },
  });
}

export async function PATCH(req: Request, { params }: Params) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const user = await safeCurrentUser();
  const { id } = await params;
  const award = await loadAward(id);
  if (!award) return NextResponse.json({ error: "Fiche introuvable." }, { status: 404 });
  const program = await loadProgram(award.programId);
  if (!program) return NextResponse.json({ error: "Parcours introuvable." }, { status: 404 });
  if (!canViewAward(award, program, gate.ctx.userId, user)) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }
  const body = await req.json().catch(() => ({}));
  let next = { ...award };
  const now = new Date().toISOString();

  if (body.addLine !== undefined) {
    if (!canEditAwardLines(award, program, gate.ctx.userId)) {
      return NextResponse.json({ error: "Cette fiche n'est plus modifiable." }, { status: 403 });
    }
    const parsed = parseCertificateLineInput(body.addLine);
    if ("error" in parsed) {
      return NextResponse.json({ error: parsed.error }, { status: 400 });
    }
    const line = buildCertificateLineFromInput(parsed, {
      id: certificateUid("line"),
      addedBy: gate.ctx.userId,
      addedByName: user?.fullName || "Enseignant",
      addedAt: now,
    });
    next.lines = [...next.lines, line];
  }

  if (body.removeLineId) {
    if (!canEditAwardLines(award, program, gate.ctx.userId)) {
      return NextResponse.json({ error: "Cette fiche n'est plus modifiable." }, { status: 403 });
    }
    const lineId = String(body.removeLineId);
    const line = next.lines.find((l) => l.id === lineId);
    if (!line) return NextResponse.json({ error: "Ligne introuvable." }, { status: 404 });
    if (line.addedBy !== gate.ctx.userId && program.ownerId !== gate.ctx.userId) {
      return NextResponse.json({ error: "Vous ne pouvez supprimer que vos propres lignes." }, { status: 403 });
    }
    next.lines = next.lines.filter((l) => l.id !== lineId);
  }

  if (body.removeSignatoryId) {
    if (!canManageAwardSignatories(award, program, gate.ctx.userId)) {
      return NextResponse.json({ error: "Signataires non modifiables." }, { status: 403 });
    }
    const removed = removePendingSignatory(next, String(body.removeSignatoryId).trim());
    if ("error" in removed) {
      return NextResponse.json({ error: removed.error }, { status: 400 });
    }
    next = removed;
  }

  if (body.designatedSignatoryIds !== undefined) {
    if (!canManageAwardSignatories(award, program, gate.ctx.userId)) {
      return NextResponse.json({ error: "Signataires non modifiables." }, { status: 403 });
    }
    const ids = Array.isArray(body.designatedSignatoryIds)
      ? body.designatedSignatoryIds.map((x: unknown) => String(x).trim()).filter(Boolean)
      : [];
    if (!ids.length && !next.designatedSignatories.some((s) => s.status === "signed")) {
      return NextResponse.json({ error: "Au moins un signataire requis." }, { status: 400 });
    }
    const nameById = new Map<string, string>();
    for (const cid of eligibleSignatoryIds(program)) {
      nameById.set(cid, await resolveDirectoryDisplayName(cid));
    }
    if (award.status === "draft") {
      if (!ids.length) {
        return NextResponse.json({ error: "Au moins un signataire requis." }, { status: 400 });
      }
      next.designatedSignatories = buildDesignatedSignatories(
        program,
        ids,
        gate.ctx.userId,
        nameById,
      );
    } else {
      const merged = mergeAwardSignatories(next, program, ids, gate.ctx.userId, nameById);
      if (!merged.length) {
        return NextResponse.json({ error: "Au moins un signataire requis." }, { status: 400 });
      }
      next.designatedSignatories = merged;
      recomputeAwardSigningStatus(next);
    }
  }

  next.updatedAt = now;
  await saveAward(next);
  return NextResponse.json({ award: next });
}
