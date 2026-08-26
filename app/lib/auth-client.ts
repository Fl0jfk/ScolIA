"use client";

import { createAuthClient } from "better-auth/client";
import { twoFactorClient } from "better-auth/client/plugins";

const MFA_EMAIL_HINT_KEY = "scolia_mfa_email_hint";

export function rememberMfaEmailHint(email: string): void {
  try {
    const v = email.trim().toLowerCase();
    if (v) sessionStorage.setItem(MFA_EMAIL_HINT_KEY, v);
  } catch {
    /* private mode */
  }
}

export function consumeMfaEmailHint(): string {
  try {
    const v = sessionStorage.getItem(MFA_EMAIL_HINT_KEY) || "";
    sessionStorage.removeItem(MFA_EMAIL_HINT_KEY);
    return v;
  } catch {
    return "";
  }
}

export const authClient = createAuthClient({
  baseURL: typeof window !== "undefined" ? window.location.origin : undefined,
  plugins: [
    twoFactorClient({
      onTwoFactorRedirect() {
        const params = new URLSearchParams(window.location.search);
        const redirect = params.get("redirect_url") || "/dashboard";
        let email = "";
        try {
          email = sessionStorage.getItem(MFA_EMAIL_HINT_KEY) || "";
        } catch {
          email = "";
        }
        const emailQs = email ? `&email=${encodeURIComponent(email)}` : "";
        window.location.href = `/auth/two-factor?redirect_url=${encodeURIComponent(redirect)}${emailQs}`;
      },
    }),
  ],
});

export type AuthClient = typeof authClient;
