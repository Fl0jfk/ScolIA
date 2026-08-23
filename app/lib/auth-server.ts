import "server-only";

import { betterAuth } from "better-auth";
import { drizzleAdapter } from "better-auth/adapters/drizzle";
import { APIError, createAuthMiddleware } from "better-auth/api";
import { nextCookies } from "better-auth/next-js";
import { twoFactor } from "better-auth/plugins";
import { getDb } from "@/db/index";
import { authSchema } from "@/db/schema";
import { betterAuthBaseUrl, isBetterAuthConfigured } from "@/app/lib/auth-config";
import { ensureEtablissementFromSlug } from "@/app/lib/etablissement-db";
import {
  PASSWORD_MIN_LENGTH,
  validatePasswordPolicy,
} from "@/app/lib/password-policy";
import { createPlatformTransporter } from "@/app/lib/tenant-mail";
import { TENANT_SLUG_HEADER } from "@/app/lib/tenant-types";

async function sendPlatformMail(opts: {
  to: string;
  subject: string;
  text: string;
}): Promise<boolean> {
  const transporter = createPlatformTransporter();
  if (!transporter) {
    console.warn("[auth] SMTP plateforme indisponible — e-mail non envoyé:", opts.subject);
    return false;
  }
  try {
    const from =
      process.env.MAILER_EMAIL?.trim() ||
      process.env.SMTP_USER?.trim() ||
      "mailer@scolia.fr";
    await transporter.sendMail({
      from: `"ScolIA" <${from}>`,
      to: opts.to,
      subject: opts.subject,
      text: opts.text,
    });
    return true;
  } catch (error) {
    console.error("[auth] envoi mail", error);
    return false;
  }
}

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

  const requireEmailVerification =
    process.env.REQUIRE_EMAIL_VERIFICATION === "true" ||
    (process.env.NODE_ENV === "production" &&
      process.env.REQUIRE_EMAIL_VERIFICATION !== "false");

  return betterAuth({
    appName: "ScolIA",
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
      requireEmailVerification,
      minPasswordLength: PASSWORD_MIN_LENGTH,
    },
    emailVerification: {
      sendOnSignUp: true,
      autoSignInAfterVerification: true,
      sendVerificationEmail: async ({ user, url }) => {
        void sendPlatformMail({
          to: user.email,
          subject: "Vérifiez votre e-mail ScolIA",
          text: `Bonjour,\n\nPour activer votre compte ScolIA, ouvrez ce lien :\n\n${url}\n\nSi vous n'êtes pas à l'origine de cette inscription, ignorez ce message.\n`,
        });
      },
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
    rateLimit: {
      enabled: true,
      storage: "database",
      window: 60,
      max: 100,
      customRules: {
        "/sign-in/email": { window: 60, max: 8 },
        "/sign-up/email": { window: 60, max: 5 },
        "/forget-password": { window: 60, max: 5 },
        "/two-factor/verify-totp": { window: 60, max: 8 },
        "/two-factor/verify-backup-code": { window: 60, max: 8 },
      },
    },
    advanced: {
      // Cookie partagé *.scolia.fr pour le portail → intranet (prod uniquement).
      // En local, Domain=scolia.fr empêcherait la session sur localhost.
      ...(process.env.NODE_ENV === "production"
        ? {
            crossSubDomainCookies: {
              enabled: true,
              domain: process.env.BETTER_AUTH_COOKIE_DOMAIN?.trim() || "scolia.fr",
            },
          }
        : {}),
      useSecureCookies: process.env.NODE_ENV === "production",
      ipAddress: {
        // Scaleway Containers / reverse proxy : IP client réelle.
        ipAddressHeaders: ["x-real-ip", "x-forwarded-for"],
      },
    },
    plugins: [
      twoFactor({
        issuer: "ScolIA",
        totpOptions: {
          digits: 6,
          period: 30,
        },
      }),
      nextCookies(),
    ],
    hooks: {
      before: createAuthMiddleware(async (ctx) => {
        if (
          ctx.path !== "/sign-up/email" &&
          ctx.path !== "/change-password" &&
          ctx.path !== "/reset-password"
        ) {
          return;
        }
        const body = ctx.body as Record<string, unknown> | undefined;
        const candidate =
          (typeof body?.newPassword === "string" && body.newPassword) ||
          (typeof body?.password === "string" && body.password) ||
          "";
        if (!candidate) return;
        const check = validatePasswordPolicy(candidate);
        if (!check.ok) {
          throw new APIError("BAD_REQUEST", { message: check.error });
        }
      }),
    },
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
