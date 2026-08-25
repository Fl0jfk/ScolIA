import { NextResponse } from "next/server";
import { requireMobileStaffAccess } from "@/app/lib/mobile-auth";

/** Contexte staff-lite pour l’app. */
export async function GET() {
  const gate = await requireMobileStaffAccess();
  if (!gate.ok) return gate.response;

  return NextResponse.json({
    channel: "mobile",
    mode: "staff-lite",
    user: {
      id: gate.ctx.authUserId,
      email: gate.ctx.email,
      name: gate.ctx.name,
    },
    etablissementId: gate.ctx.etablissementId,
    roles: gate.ctx.roles,
    capabilities: {
      appel: true,
      edtPerso: true,
      observations: true,
      dossierAdmin: false,
      paie: false,
      siecle: false,
      parametres: false,
      facturationInterne: false,
    },
  });
}
