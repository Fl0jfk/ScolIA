"use client";

import { createAuthClient } from "better-auth/client";

export const authClient = createAuthClient({
  baseURL: typeof window !== "undefined" ? window.location.origin : undefined,
});

export type AuthClient = typeof authClient;
