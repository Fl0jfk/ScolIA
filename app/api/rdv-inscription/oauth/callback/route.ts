import { NextRequest, NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canManageRdvInscription } from "@/app/lib/rdv-inscription-access";
import { saveRdvInscriptionGoogleLinkSecret } from "@/app/lib/rdv-inscription-google";
import {
  RDV_INSCRIPTION_OAUTH_STATE_COOKIE,
  exchangeRdvInscriptionOAuthCode,
  fetchGoogleUserEmail,
} from "@/app/lib/rdv-inscription-oauth";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";

async function redirectAdmin(query: string) {
  const url = await tenantAbsolutePath(`/etablissement/rdv-inscription?${query}`);
  return NextResponse.redirect(url);
}

/** Callback OAuth Google — stocke le refresh token et redirige vers le paramétrage. */
export async function GET(req: NextRequest) {
  const gate = await requireAuth();
  if (!gate.ok) {
    return NextResponse.redirect(await tenantAbsolutePath("/sign-in"));
  }

  const user = await safeCurrentUser();
  if (!user || !canManageRdvInscription(rolesFromUserLike(user))) {
    return redirectAdmin("google=forbidden");
  }

  const sp = req.nextUrl.searchParams;
  const err = sp.get("error");
  if (err) {
    return redirectAdmin(`google=error&detail=${encodeURIComponent(err.slice(0, 200))}`);
  }

  const code = sp.get("code")?.trim();
  const state = sp.get("state")?.trim();
  const expected = req.cookies.get(RDV_INSCRIPTION_OAUTH_STATE_COOKIE)?.value;
  if (!code || !state || !expected || state !== expected) {
    return redirectAdmin("google=error&detail=state_invalid");
  }

  try {
    const tokens = await exchangeRdvInscriptionOAuthCode(code);
    const me = await fetchGoogleUserEmail(tokens.accessToken);
    await saveRdvInscriptionGoogleLinkSecret({
      refreshToken: tokens.refreshToken,
      linkedEmail: me.email,
      linkedDisplayName: me.name,
    });

    const res = await redirectAdmin("google=linked");
    res.cookies.set(RDV_INSCRIPTION_OAUTH_STATE_COOKIE, "", { path: "/", maxAge: 0 });
    return res;
  } catch (e) {
    const raw = e instanceof Error ? e.message : "oauth_failed";
    return redirectAdmin(`google=error&detail=${encodeURIComponent(raw.slice(0, 400))}`);
  }
}
