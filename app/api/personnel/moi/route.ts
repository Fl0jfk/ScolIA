import { safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { NextResponse } from "next/server";

import { requireAuth } from "@/app/lib/intranet-auth";
import {
  findPersonnelByExternalId,
  findPersonnelByEmail,
  getSharedPersonnelDocuments,
} from "@/app/lib/personnel-storage";
import {
  canAccessPersonnelModule,
  sanitizeRecordForViewer,
} from "@/app/lib/personnel-types";

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);

  if (!canAccessPersonnelModule(roles)) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
  }

  try {
    const email = user?.primaryEmailAddress?.emailAddress || "";
    let record =
      (user?.id ? await findPersonnelByExternalId(user.id) : null) ||
      (email ? await findPersonnelByEmail(email) : null);

    const sharedDocs = await getSharedPersonnelDocuments();

    if (!record) {
      return NextResponse.json({ record: null, sharedDocs });
    }

    return NextResponse.json({
      record: sanitizeRecordForViewer(record, roles, user?.id, email),
      sharedDocs,
    });
  } catch (e) {
    console.error("[personnel/moi]", e);
    return NextResponse.json({ error: "Impossible de charger votre dossier." }, { status: 500 });
  }
}
