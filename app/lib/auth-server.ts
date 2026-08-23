import "server-only";

import { betterAuth } from "better-auth";
import { drizzleAdapter } from "better-auth/adapters/drizzle";
import { nextCookies } from "better-auth/next-js";
import { getDb } from "@/db/index";
import { authSchema } from "@/db/schema";
import { betterAuthBaseUrl, isBetterAuthConfigured } from "@/app/lib/auth-config";
import { ensureEtablissementFromSlug } from "@/app/lib/etablissement-db";
import { TENANT_SLUG_HEADER } from "@/app/lib/tenant-types";

function createAuth() {
  if (!isBetterAuthConfigured()) {
    throw new Error("Better-Auth non configuré (DATABASE_URL + BETTER_AUTH_SECRET).");
  }

  const db = getDb();

  const base = betterAuthBaseUrl();
  const localDevOrigins =
    process.env.NODE_ENV !== "production"
      ? [
          "http://localhost:3000",
          "http://127.0.0.1:3000",
          "http://localhost:3001",
          "http://127.0.0.1:3001",
        ]
      : [];

  return betterAuth({
    secret: process.env.BETTER_AUTH_SECRET!,
    baseURL: base,
    trustedOrigins: [
      base,
      ...localDevOrigins,
      ...(process.env.BETTER_AUTH_TRUSTED_ORIGINS?.split(",").map((s) => s.trim()) ?? []),
      "https://www.scolia.fr",
      "https://scolia.fr",
      "https://lpnb.scolia.fr",
      "https://lp.docslapro.com",
    ].filter(Boolean),
    database: drizzleAdapter(db, {
      provider: "pg",
      schema: authSchema,
    }),
    emailAndPassword: {
      enabled: true,
      requireEmailVerification: false,
      minPasswordLength: 8,
    },
    session: {
      expiresIn: 60 * 60 * 24 * 7,
      updateAge: 60 * 60 * 12,
      cookieCache: {
        enabled: true,
        maxAge: 60 * 5,
      },
    },
    user: {
      additionalFields: {
        etablissementId: { type: "string", required: true, input: false },
        externalUserId: { type: "string", required: false, input: false },
        firstName: { type: "string", required: false },
        lastName: { type: "string", required: false },
        platformAdmin: { type: "boolean", required: false, defaultValue: false, input: false },
        orgAdmin: { type: "boolean", required: false, defaultValue: false, input: false },
        mustChangePassword: {
          type: "boolean",
          required: false,
          defaultValue: true,
          input: false,
        },
      },
      // Changement d’e-mail : flux custom via /api/account/security (token + mail).
      changeEmail: {
        enabled: false,
      },
    },
    advanced: {
      // Cookie partagé *.scolia.fr pour le portail → intranet.
      // L’isolation est garantie par assertUserBelongsToTenant dans le proxy.
      crossSubDomainCookies: {
        enabled: true,
        domain: process.env.BETTER_AUTH_COOKIE_DOMAIN?.trim() || "scolia.fr",
      },
      useSecureCookies: process.env.NODE_ENV === "production",
    },
    plugins: [nextCookies()],
    databaseHooks: {
      user: {
        create: {
          before: async (payload, ctx) => {
            if (payload.etablissementId) {
              return {
                data: {
                  ...payload,
                  mustChangePassword: payload.mustChangePassword ?? true,
                },
              };
            }
            const slug = ctx?.request?.headers.get(TENANT_SLUG_HEADER)?.trim();
            if (!slug) {
              throw new Error("Établissement introuvable pour la création du compte.");
            }
            const etablissementId = await ensureEtablissementFromSlug(slug);
            return {
              data: {
                ...payload,
                etablissementId,
                mustChangePassword: true,
                name:
                  payload.name ||
                  `${payload.firstName ?? ""} ${payload.lastName ?? ""}`.trim() ||
                  payload.email,
              },
            };
          },
        },
      },
    },
  });
}

type AuthInstance = ReturnType<typeof createAuth>;

let authInstance: AuthInstance | null = null;

export function getBetterAuth(): AuthInstance {
  if (!authInstance) {
    authInstance = createAuth();
  }
  return authInstance;
}

export type BetterAuthSession = AuthInstance["$Infer"]["Session"];
