import "server-only";

import { betterAuth } from "better-auth";
import { drizzleAdapter } from "better-auth/adapters/drizzle";
import { APIError, createAuthMiddleware } from "better-auth/api";
import { nextCookies } from "better-auth/next-js";
import { twoFactor } from "better-auth/plugins";
import { eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { authSchema, user as userTable } from "@/db/schema";
import { betterAuthBaseUrl, isBetterAuthConfigured } from "@/app/lib/auth-config";
import { ensureEtablissementFromSlug } from "@/app/lib/etablissement-db";
import { ensureUserMembership } from "@/app/lib/user-membership";
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
  html?: string;
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
      ...(opts.html ? { html: opts.html } : {}),
    });
    return true;
  } catch (error) {
    console.error("[auth] envoi mail", error);
    return false;
  }
}

/** Corps du mail d’activation / reset MDP (texte + HTML). */
export function buildPasswordActivationEmail(opts: {
  firstName?: string | null;
  url: string;
}): { subject: string; text: string; html: string } {
  const prenom = opts.firstName?.trim() || "";
  const hello = prenom ? `Bonjour ${prenom},` : "Bonjour,";
  const subject = "ScolIA — Activez votre compte et créez votre mot de passe";
  const text = `${hello}

Pour simplifier la connexion à l’intranet ScolIA, nous vous invitons à activer votre compte et à créer un nouveau mot de passe personnel.

Important : l’ancien mot de passe (s’il existait) ne fonctionne plus. Utilisez uniquement le lien ci-dessous.

1) Ouvrez ce lien (valable une heure) pour choisir votre nouveau mot de passe :
${opts.url}

2) Après connexion, vous devrez activer la double authentification (MFA) avec une application d’authentification. C’est obligatoire pour protéger les données de l’établissement.

Applications possibles (gratuites) :
- Microsoft Authenticator (Windows / Android / iPhone)
- Google Authenticator (Android / iPhone)
- Mots de passe d’Apple (iPhone / Mac) — section « Codes » / authentification à deux facteurs

Comment ça marche en bref :
- Lors de la première connexion après ce mail, l’écran affiche un QR code.
- Ouvrez votre appli d’authentification → ajoutez un compte → scannez le QR code.
- Saisissez le code à 6 chiffres généré par l’appli pour valider.
- Ensuite, à chaque connexion : e-mail + mot de passe + code de l’appli.

Si le lien a expiré, demandez un nouveau lien via « Mot de passe oublié » sur la page de connexion, ou contactez l’établissement.

Si vous n’êtes pas concerné par ce message, ignorez-le.

— L’équipe ScolIA
`;

  const html = `<!DOCTYPE html>
<html lang="fr">
<body style="font-family:Segoe UI,Helvetica,Arial,sans-serif;line-height:1.5;color:#0f172a;max-width:560px;margin:0 auto;padding:24px;">
  <p>${hello}</p>
  <p>Pour simplifier la connexion à l’intranet <strong>ScolIA</strong>, activez votre compte et créez un <strong>nouveau mot de passe personnel</strong>.</p>
  <p style="background:#fef3c7;border:1px solid #f59e0b;border-radius:8px;padding:12px;"><strong>Important :</strong> l’ancien mot de passe (s’il existait) ne fonctionne plus. Utilisez uniquement le bouton ci-dessous.</p>
  <p style="text-align:center;margin:28px 0;">
    <a href="${opts.url}" style="display:inline-block;background:#2F6B4A;color:#fff;text-decoration:none;padding:12px 22px;border-radius:10px;font-weight:600;">Créer mon mot de passe</a>
  </p>
  <p style="font-size:13px;color:#64748b;">Lien valable <strong>1 heure</strong>. Si le bouton ne fonctionne pas, copiez cette adresse :<br/><a href="${opts.url}">${opts.url}</a></p>
  <h2 style="font-size:16px;margin-top:28px;">Ensuite : double authentification (MFA)</h2>
  <p>Après connexion, vous activerez une application d’authentification (obligatoire pour la sécurité de l’établissement) :</p>
  <ul>
    <li><strong>Microsoft Authenticator</strong> (Windows / Android / iPhone)</li>
    <li><strong>Google Authenticator</strong> (Android / iPhone)</li>
    <li><strong>Mots de passe d’Apple</strong> (iPhone / Mac) — codes à deux facteurs</li>
  </ul>
  <ol>
    <li>L’écran affiche un QR code.</li>
    <li>Dans l’appli : ajouter un compte → scanner le QR code.</li>
    <li>Saisir le code à 6 chiffres pour valider.</li>
    <li>À chaque connexion suivante : e-mail + mot de passe + code de l’appli.</li>
  </ol>
  <p style="font-size:13px;color:#64748b;">Lien expiré ? Utilisez « Mot de passe oublié » sur la page de connexion, ou contactez l’établissement.</p>
  <p style="margin-top:32px;font-size:13px;color:#64748b;">— L’équipe ScolIA</p>
</body>
</html>`;

  return { subject, text, html };
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
      resetPasswordTokenExpiresIn: 60 * 60,
      revokeSessionsOnPasswordReset: true,
      sendResetPassword: async ({ user, url }) => {
        const mail = buildPasswordActivationEmail({
          firstName:
            typeof user.firstName === "string"
              ? user.firstName
              : typeof (user as { name?: string }).name === "string"
                ? (user as { name?: string }).name?.split(/\s+/)[0]
                : null,
          url,
        });
        void sendPlatformMail({
          to: user.email,
          subject: mail.subject,
          text: mail.text,
          html: mail.html,
        });
      },
      onPasswordReset: async ({ user }) => {
        try {
          await getDb()
            .update(userTable)
            .set({
              mustChangePassword: false,
              emailVerified: true,
              updatedAt: new Date(),
            })
            .where(eq(userTable.id, user.id));
        } catch (e) {
          console.error("[auth] onPasswordReset update user", e);
        }
      },
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
          after: async (created) => {
            const etabId =
              typeof created.etablissementId === "string" ? created.etablissementId : null;
            if (created.id && etabId) {
              try {
                await ensureUserMembership({
                  userId: created.id,
                  etablissementId: etabId,
                  context: "staff",
                });
              } catch (e) {
                console.error("[auth] ensureUserMembership after create", e);
              }
            }
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
