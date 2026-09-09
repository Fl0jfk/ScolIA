import { requireAuth } from "@/app/lib/intranet-auth";
import { getJson, putJson } from "@/app/lib/s3-storage";
import { isPhotocopiesDemandOwnedBy } from "@/app/lib/photocopies-couleur-access";
import { resolvePhotocopiesOpsViewer } from "@/app/lib/photocopies-couleur-ops-server";
import {
  isPhotocopieReadyUnseen,
  type PhotoCopieRecord,
} from "@/app/lib/photocopies-couleur-types";
import { NextResponse } from "next/server";

const INDEX_KEY = "photocopies-couleur/index.json";

/**
 * Ouverture du module par le demandeur = signal « photocopies prêtes » acquitté.
 * Idempotent : ne touche que les PRETE encore sans readySeenAt appartenant à l’utilisateur.
 */
export async function POST() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  try {
    const viewer = await resolvePhotocopiesOpsViewer();
    const businessUserId = viewer.businessUserId || gate.ctx.userId;
    const authUserId = viewer.authUserId || gate.ctx.userId;

    const hit = await getJson<PhotoCopieRecord[]>(INDEX_KEY);
    const all = hit?.data ?? [];
    if (all.length === 0) {
      return NextResponse.json({ success: true, marked: 0, ids: [] as string[] });
    }

    const seenAt = new Date().toISOString();
    const ids: string[] = [];
    let changed = false;

    const next = all.map((rec) => {
      if (!isPhotocopieReadyUnseen(rec)) return rec;
      if (!isPhotocopiesDemandOwnedBy(rec, businessUserId, authUserId)) return rec;
      changed = true;
      ids.push(rec.id);
      return { ...rec, readySeenAt: seenAt, updatedAt: seenAt };
    });

    if (changed) {
      await putJson(INDEX_KEY, next);
    }

    return NextResponse.json({ success: true, marked: ids.length, ids, seenAt });
  } catch (e) {
    console.error("[photocopies-couleur/mark-ready-seen]", e);
    return NextResponse.json(
      { error: "Impossible d'acquitter le signal photocopies prêtes." },
      { status: 500 },
    );
  }
}
