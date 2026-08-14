import { NextRequest, NextResponse } from "next/server";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canManagePersonnel } from "@/app/lib/personnel-types";
import {
  fetchRhGraphMe,
  saveRhDriveLinkSecret,
} from "@/app/lib/rh/graph-rh-drive";
import {
  formatRhOAuthError,
  rhOAuthErrorQueryParam,
} from "@/app/lib/rh/oauth-errors";
import {
  RH_OAUTH_STATE_COOKIE,
  exchangeRhOAuthCode,
  getRhOAuthRedirectUri,
} from "@/app/lib/rh/oauth-rh-drive";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";


async function redirectRh(query: string) {
  const url = await tenantAbsolutePath(`/rh?tab=admin&${query}`);
  return NextResponse.redirect(url);
}

/** Callback OAuth — stocke le refresh token RH et redirige vers le hub. */
export async function GET(req: NextRequest) {
  const gate = await requireAuth();
  if (!gate.ok) {
    return NextResponse.redirect(await tenantAbsolutePath("/sign-in"));
  }

  const user = await safeCurrentUser();
  if (!user || !canManagePersonnel(rolesFromUserLike(user))) {
    return redirectRh("rhDrive=forbidden");
  }

  const sp = req.nextUrl.searchParams;
  const err = sp.get("error");
  const errDesc = sp.get("error_description") || err || "";
  if (err) {
    let redirectUri: string | null = null;
    try {
      redirectUri = await getRhOAuthRedirectUri();
    } catch {
      redirectUri = null;
    }
    const formatted = formatRhOAuthError(errDesc, redirectUri);
    const code = rhOAuthErrorQueryParam(errDesc);
    return redirectRh(`rhDrive=${code}&detail=${encodeURIComponent(formatted.slice(0, 400))}`);
  }

  const code = sp.get("code")?.trim();
  const state = sp.get("state")?.trim();
  const expected = req.cookies.get(RH_OAUTH_STATE_COOKIE)?.value;
  if (!code || !state || !expected || state !== expected) {
    return redirectRh("rhDrive=error&detail=state_invalid");
  }

  try {
    const tokens = await exchangeRhOAuthCode(code);
    const me = await fetchRhGraphMe(tokens.accessToken);
    await saveRhDriveLinkSecret({
      refreshToken: tokens.refreshToken,
      linkedUpn: me.userPrincipalName || me.mail,
      linkedDisplayName: me.displayName,
    });

    const res = await redirectRh("rhDrive=linked");
    res.cookies.set(RH_OAUTH_STATE_COOKIE, "", { path: "/", maxAge: 0 });
    return res;
  } catch (e) {
    let redirectUri: string | null = null;
    try {
      redirectUri = await getRhOAuthRedirectUri();
    } catch {
      redirectUri = null;
    }
    const raw = e instanceof Error ? e.message : "oauth_failed";
    const formatted = formatRhOAuthError(raw, redirectUri);
    const code = rhOAuthErrorQueryParam(raw);
    return redirectRh(`rhDrive=${code}&detail=${encodeURIComponent(formatted.slice(0, 400))}`);
  }
}
