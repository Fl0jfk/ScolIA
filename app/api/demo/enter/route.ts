import { createOTP } from "@better-auth/utils/otp";
import { NextResponse } from "next/server";
import { getBetterAuth } from "@/app/lib/auth-server";
import {
  DEMO_PARENT,
  DEMO_STAFF,
  isDemoClickAllowed,
  type DemoRole,
} from "@/app/lib/demo-click";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

function parseRole(raw: string | null): DemoRole | null {
  if (raw === "parent" || raw === "staff") return raw;
  return null;
}

/** Copie Set-Cookie d’une Response Better-Auth vers une autre. */
function forwardSetCookies(from: Response, to: NextResponse) {
  const getSetCookie = (
    from.headers as Headers & { getSetCookie?: () => string[] }
  ).getSetCookie?.();
  if (getSetCookie && getSetCookie.length > 0) {
    for (const c of getSetCookie) to.headers.append("Set-Cookie", c);
    return;
  }
  const single = from.headers.get("set-cookie");
  if (single) to.headers.append("Set-Cookie", single);
}

function cookieHeaderFromResponse(res: Response): string {
  const getSetCookie = (
    res.headers as Headers & { getSetCookie?: () => string[] }
  ).getSetCookie?.();
  if (getSetCookie && getSetCookie.length > 0) {
    return getSetCookie
      .map((c) => c.split(";")[0]?.trim())
      .filter(Boolean)
      .join("; ");
  }
  const single = res.headers.get("set-cookie");
  if (!single) return "";
  return single.split(",").map((p) => p.split(";")[0]?.trim()).filter(Boolean).join("; ");
}

/**
 * GET /api/demo/enter?as=parent|staff
 * Connexion seed + redirect. Refusé si runtime = prod.
 */
export async function GET(req: Request) {
  if (!isDemoClickAllowed()) {
    return NextResponse.json(
      { error: "Démo one-click désactivée en production." },
      { status: 403 },
    );
  }

  const url = new URL(req.url);
  const role = parseRole(url.searchParams.get("as"));
  if (!role) {
    return NextResponse.json(
      { error: "Paramètre as=parent|staff requis." },
      { status: 400 },
    );
  }

  const auth = getBetterAuth();
  const creds = role === "staff" ? DEMO_STAFF : DEMO_PARENT;
  const dest = new URL(creds.redirect, url.origin);

  try {
    const signInRes = await auth.api.signInEmail({
      body: {
        email: creds.email,
        password: creds.password,
      },
      asResponse: true,
    });

    const signInBody = (await signInRes.clone().json().catch(() => null)) as {
      twoFactorRedirect?: boolean;
      user?: unknown;
      session?: unknown;
      error?: { message?: string };
      message?: string;
    } | null;

    if (!signInRes.ok && !signInBody?.twoFactorRedirect) {
      const msg =
        signInBody?.error?.message ||
        signInBody?.message ||
        `Connexion échouée (${signInRes.status})`;
      return NextResponse.redirect(
        new URL(`/demo?error=${encodeURIComponent(msg)}`, url.origin),
        { status: 303 },
      );
    }

    if (signInBody?.twoFactorRedirect) {
      if (role !== "staff") {
        return NextResponse.redirect(
          new URL(
            `/demo?error=${encodeURIComponent("2FA inattendu pour le parent")}`,
            url.origin,
          ),
          { status: 303 },
        );
      }

      const otp = createOTP(DEMO_STAFF.totpSecret, { period: 30, digits: 6 });
      const code = await otp.totp();
      const cookieHeader = cookieHeaderFromResponse(signInRes);

      const verifyRes = await auth.api.verifyTOTP({
        body: { code, trustDevice: true },
        headers: cookieHeader
          ? new Headers({ cookie: cookieHeader })
          : undefined,
        asResponse: true,
      });

      if (!verifyRes.ok) {
        const vBody = (await verifyRes.clone().json().catch(() => null)) as {
          message?: string;
          error?: { message?: string };
        } | null;
        const msg =
          vBody?.error?.message ||
          vBody?.message ||
          `TOTP échoué (${verifyRes.status})`;
        return NextResponse.redirect(
          new URL(`/demo?error=${encodeURIComponent(msg)}`, url.origin),
          { status: 303 },
        );
      }

      const out = NextResponse.redirect(dest, { status: 303 });
      forwardSetCookies(signInRes, out);
      forwardSetCookies(verifyRes, out);
      return out;
    }

    const out = NextResponse.redirect(dest, { status: 303 });
    forwardSetCookies(signInRes, out);
    return out;
  } catch (e) {
    const msg = e instanceof Error ? e.message : "Erreur démo";
    return NextResponse.redirect(
      new URL(`/demo?error=${encodeURIComponent(msg)}`, url.origin),
      { status: 303 },
    );
  }
}
