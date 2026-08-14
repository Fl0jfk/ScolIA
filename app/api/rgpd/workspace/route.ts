import { NextResponse } from "next/server";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canAccessRgpdModule } from "@/app/lib/rgpd-access";
import {
  buildActionItems,
  buildDocumentList,
  computeRgpdComplianceScore,
} from "@/app/lib/rgpd-scoring";
import { buildAllRgpdDocumentContentPreviews } from "@/app/lib/rgpd-document-content";
import { loadRgpdWorkspace, saveRgpdWorkspace } from "@/app/lib/rgpd-storage";
import type { RgpdQuestionnaireAnswers, RgpdWorkspace } from "@/app/lib/rgpd-types";


async function gateRgpd() {
  const gate = await requireAuth();
  if (!gate.ok) return { ok: false as const, response: gate.response };
  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  if (!canAccessRgpdModule(roles)) {
    return {
      ok: false as const,
      response: NextResponse.json({ error: "Accès refusé au module RGPD." }, { status: 403 }),
    };
  }
  return {
    ok: true as const,
    user,
    roles,
    userName:
      [user?.firstName, user?.lastName].filter(Boolean).join(" ") ||
      user?.emailAddresses?.[0]?.emailAddress ||
      "Utilisateur",
    userId: user?.id || "",
  };
}

async function buildWorkspacePayload(workspace: RgpdWorkspace) {
  const score = computeRgpdComplianceScore(workspace);
  const documents = buildDocumentList(workspace);
  const actions = buildActionItems(workspace);
  const documentPreviews = await buildAllRgpdDocumentContentPreviews(workspace);
  return { workspace, score, documents, actions, documentPreviews };
}

export async function GET() {
  const gate = await gateRgpd();
  if (!gate.ok) return gate.response;
  const workspace = await loadRgpdWorkspace();
  return NextResponse.json(await buildWorkspacePayload(workspace));
}

export async function PUT(req: Request) {
  const gate = await gateRgpd();
  if (!gate.ok) return gate.response;

  try {
    const body = await req.json();
    const workspace = await loadRgpdWorkspace();

    if (body.answers && typeof body.answers === "object") {
      workspace.answers = {
        ...workspace.answers,
        ...(body.answers as Partial<RgpdQuestionnaireAnswers>),
      };
    }

    if (body.markQuestionnaireComplete === true) {
      workspace.answers.questionnaireCompleted = true;
      workspace.answers.questionnaireStep = 7;
    }

    workspace.history = [
      ...(workspace.history ?? []).slice(-99),
      {
        at: new Date().toISOString(),
        by: gate.userName,
        action: "WORKSPACE_UPDATE",
        note: body.note ? String(body.note) : "Questionnaire mis à jour",
      },
    ];

    await saveRgpdWorkspace(workspace);
    return NextResponse.json(await buildWorkspacePayload(workspace));
  } catch (e) {
    console.error("[rgpd/workspace PUT]", e);
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Erreur enregistrement" },
      { status: 500 },
    );
  }
}
