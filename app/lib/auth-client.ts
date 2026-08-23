"use client";

import { createAuthClient } from "better-auth/client";
import { twoFactorClient } from "better-auth/client/plugins";

export const authClient = createAuthClient({
  baseURL: typeof window !== "undefined" ? window.location.origin : undefined,
  plugins: [
    twoFactorClient({
      onTwoFactorRedirect() {
        const params = new URLSearchParams(window.location.search);
        const redirect = params.get("redirect_url") || "/dashboard";
        window.location.href = `/auth/two-factor?redirect_url=${encodeURIComponent(redirect)}`;
      },
    }),
  ],
});

export type AuthClient = typeof authClient;
