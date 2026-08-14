import { NextResponse } from "next/server";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canManagePersonnel } from "@/app/lib/personnel-types";
import {
  createOnboardingInvite,
  listOnboardingRecords,
} from "@/app/lib/rh/onboarding-storage";
import { RH_ONBOARDING_STATUS_LABELS } from "@/app/lib/rh/onboarding-types";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";


export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  if (!user || !canManagePersonnel(rolesFromUserLike(user))) {
    return NextResponse.json({ error: "Réservé à la RH." }, { status: 403 });
  }

  const records = await listOnboardingRecords();
  return NextResponse.json({
    records: records.map((r) => ({
      ...r,
      statusLabel: RH_ONBOARDING_STATUS_LABELS[r.status],
    })),
  });
}

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  if (!user || !canManagePersonnel(rolesFromUserLike(user))) {
    return NextResponse.json({ error: "Réservé à la RH." }, { status: 403 });
  }

  try {
    const body = await req.json();
    const candidateEmailHint = String(body?.candidateEmailHint || "").trim() || null;
    const publicPath = `/onboarding-rh/{token}`;
    const record = await createOnboardingInvite({
      createdBy: {
        userId: user.id,
        name: user.fullName || user.firstName || "RH",
        email: user.primaryEmailAddress?.emailAddress || null,
      },
      candidateEmailHint,
      publicPath,
    });
    const publicUrl = await tenantAbsolutePath(`/onboarding-rh/${record.token}`);
    return NextResponse.json({
      record: { ...record, statusLabel: RH_ONBOARDING_STATUS_LABELS[record.status] },
      publicUrl,
    });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Création impossible." },
      { status: 500 },
    );
  }
}
