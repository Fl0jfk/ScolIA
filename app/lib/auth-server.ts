import "server-only";

import { betterAuth } from "better-auth";
import { drizzleAdapter } from "better-auth/adapters/drizzle";
import { APIError, createAuthMiddleware } from "better-auth/api";
import { nextCookies } from "better-auth/next-js";
import { twoFactor } from "better-auth/plugins";
import { eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { authSchema, user as userTable } from "@/db/schema";
import { loadAppConfig } from "@/app/lib/app-config";
import { betterAuthBaseUrl, isBetterAuthConfigured } from "@/app/lib/auth-config";
import { ensureEtablissementFromSlug } from "@/app/lib/etablissement-db";
import { ensureUserMembership } from "@/app/lib/user-membership";
import {
  PASSWORD_MIN_LENGTH,
  validatePasswordPolicy,
} from "@/app/lib/password-policy";
import { s3Key } from "@/app/lib/s3-path";
import { getObjectBytes } from "@/app/lib/s3-storage";
import { getTenant } from "@/app/lib/tenant-context";
import { createPlatformTransporter } from "@/app/lib/tenant-mail";
import { TENANT_SLUG_HEADER } from "@/app/lib/tenant-types";
import { parseTravelsS3KeyFromUrl } from "@/app/lib/travels-s3";
import {
  mfaTrustAfterSignIn,
  mfaTrustAfterVerify,
  mfaTrustBeforeSignIn,
} from "@/app/lib/mfa-trust-hooks";
import { MFA_TRUST_STAFF_SECONDS } from "@/app/lib/two-factor-policy";

type MailAttachment = {
  filename: string;
  content: Buffer;
  contentType: string;
  cid?: string;
};

async function sendPlatformMail(opts: {
  to: string;
  subject: string;
  text: string;
  html?: string;
  fromName?: string;
  attachments?: MailAttachment[];
}): Promise<boolean> {
  const transporter = createPlatformTransporter();
  if (!transporter) {
    console.warn("[auth] SMTP plateforme indisponible — e-mail non envoyé:", opts.subject);
    return false;
  }
  try {
    const fromAddr =
      process.env.MAILER_EMAIL?.trim() ||
      process.env.SMTP_USER?.trim() ||
      "mailer@scolia.fr";
    const fromLabel = (opts.fromName || "ScolIA").replace(/["\r\n]/g, "").trim() || "ScolIA";
    await transporter.sendMail({
      from: `"${fromLabel}" <${fromAddr}>`,
      to: opts.to,
      subject: opts.subject,
      text: opts.text,
      ...(opts.html ? { html: opts.html } : {}),
      ...(opts.attachments?.length
        ? {
            attachments: opts.attachments.map((a) => ({
              filename: a.filename,
              content: a.content,
              contentType: a.contentType,
              ...(a.cid ? { cid: a.cid, contentDisposition: "inline" as const } : {}),
            })),
          }
        : {}),
    });
    return true;
  } catch (error) {
    console.error("[auth] envoi mail", error);
    return false;
  }
}

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

async function resolveInvitationEstablishment(): Promise<{
  name: string;
  shortName: string;
  logo: MailAttachment | null;
}> {
  const [bundle, tenant] = await Promise.all([loadAppConfig(), getTenant()]);
  const name =
    bundle.identity.name?.trim() ||
    tenant.label?.trim() ||
    tenant.slug?.trim() ||
    "Votre établissement";
  const shortName = bundle.identity.shortName?.trim() || name;

  const rawLogo = bundle.identity.headerLogoUrl?.trim() || tenant.logoUrl?.trim() || "";
  let logo: MailAttachment | null = null;
  if (rawLogo) {
    try {
      let key: string | null = null;
      if (!rawLogo.startsWith("http://") && !rawLogo.startsWith("https://")) {
        key = s3Key(rawLogo.split("?")[0].split("#")[0]) || null;
      } else {
        key = await parseTravelsS3KeyFromUrl(rawLogo);
      }
      if (key) {
        const bytes = await getObjectBytes(key);
        if (bytes?.length) {
          const lower = key.toLowerCase();
          const contentType = lower.endsWith(".png")
            ? "image/png"
            : lower.endsWith(".webp")
              ? "image/webp"
              : lower.endsWith(".gif")
                ? "image/gif"
                : "image/jpeg";
          const ext =
            contentType === "image/png"
              ? "png"
              : contentType === "image/webp"
                ? "webp"
                : contentType === "image/gif"
                  ? "gif"
                  : "jpg";
          logo = {
            filename: `logo-${tenant.slug || "etablissement"}.${ext}`,
            content: bytes,
            contentType,
            cid: "etablissement-logo",
          };
        }
      }
    } catch (e) {
      console.warn("[auth] logo invitation indisponible", e);
    }
  }

  return { name, shortName, logo };
}

/** Corps du mail d’activation / reset MDP (texte + HTML), branding tenant inclus. */
export async function buildPasswordActivationEmail(opts: {
  firstName?: string | null;
  url: string;
}): Promise<{
  subject: string;
  text: string;
  html: string;
  fromName: string;
  attachments: MailAttachment[];
}> {
  const branding = await resolveInvitationEstablishment();
  const prenom = opts.firstName?.trim() || "";
  const hello = prenom ? `Bonjour ${prenom},` : "Bonjour,";
  const etab = branding.name;
  const short = branding.shortName;
  const subject = `[${short}] Invitation ScolIA — activez votre compte`;
  const fromName = `${short} · ScolIA`;

  const text = `${hello}

Cet e-mail vous est envoyé par ${etab}.
Il s’agit de l’invitation pour activer votre espace personnel sur la plateforme ScolIA (intranet / ENT de votre établissement).

━━━━━━━━━━━━━━━━━━━━
ÉTAPE 1 — Créer votre mot de passe (lien valable 12 heures)
━━━━━━━━━━━━━━━━━━━━
Ouvrez ce lien uniquement :
${opts.url}

Important : si un ancien mot de passe existait, il ne fonctionne plus. Utilisez uniquement ce lien.

━━━━━━━━━━━━━━━━━━━━
ÉTAPE 2 — Double authentification (MFA) — obligatoire
━━━━━━━━━━━━━━━━━━━━
La MFA, c’est un code à 6 chiffres généré par une application sur votre téléphone (ou tablette).
Sans cette étape, vous ne pourrez pas utiliser pleinement la plateforme (cloud, dossiers partagés, etc.).

À faire après avoir créé votre mot de passe :

A) Installez UNE application gratuite d’authentification, par exemple :
   • Microsoft Authenticator (Windows, Android, iPhone)
   • Google Authenticator (Android, iPhone)
   • « Mots de passe » d’Apple (iPhone / Mac) — codes à deux facteurs

B) Connectez-vous sur ScolIA avec votre e-mail + le nouveau mot de passe.

C) L’écran affiche un QR code :
   1. Ouvrez l’application d’authentification
   2. Ajoutez un compte → « Scanner un QR code »
   3. Scannez le QR code affiché à l’écran
   4. Saisissez le code à 6 chiffres proposé par l’appli pour valider

D) À chaque connexion suivante : e-mail + mot de passe + code de l’appli.

━━━━━━━━━━━━━━━━━━━━
Lien expiré ?
Demandez un nouveau lien d’invitation à l’administrateur de ${etab}, ou utilisez « Mot de passe oublié » sur la page de connexion.

Si vous n’êtes pas concerné par ce message, ignorez-le.

— ${etab} via ScolIA
`;

  const logoBlock = branding.logo
    ? `<div style="text-align:center;margin:0 0 20px;">
  <img src="cid:etablissement-logo" alt="${escapeHtml(etab)}" width="120" style="max-width:140px;height:auto;border:0;" />
</div>`
    : "";

  const html = `<!DOCTYPE html>
<html lang="fr">
<body style="font-family:Segoe UI,Helvetica,Arial,sans-serif;line-height:1.55;color:#0f172a;max-width:600px;margin:0 auto;padding:24px;background:#f8fafc;">
  <div style="background:#ffffff;border:1px solid #e2e8f0;border-radius:16px;padding:28px 24px;">
    ${logoBlock}
    <p style="margin:0 0 8px;font-size:13px;color:#64748b;text-transform:uppercase;letter-spacing:0.04em;">Invitation officielle</p>
    <h1 style="margin:0 0 12px;font-size:22px;line-height:1.3;color:#0f172a;">Activez votre compte ScolIA</h1>
    <p style="margin:0 0 16px;font-size:15px;">${escapeHtml(hello)}</p>
    <p style="margin:0 0 16px;font-size:15px;">
      Cet e-mail vous est envoyé par <strong>${escapeHtml(etab)}</strong>.
      Il s’agit de l’invitation pour activer votre espace personnel sur la plateforme
      <strong>ScolIA</strong> (intranet / ENT de votre établissement).
    </p>

    <div style="background:#ecfdf5;border:1px solid #6ee7b7;border-radius:12px;padding:14px 16px;margin:20px 0;">
      <p style="margin:0;font-size:14px;"><strong>Étape 1 — Créer votre mot de passe</strong></p>
      <p style="margin:8px 0 0;font-size:14px;">Le lien ci-dessous est valable <strong>12 heures</strong>. Si un ancien mot de passe existait, il ne fonctionne plus.</p>
    </div>

    <p style="text-align:center;margin:24px 0;">
      <a href="${opts.url}" style="display:inline-block;background:#2F6B4A;color:#fff;text-decoration:none;padding:14px 26px;border-radius:12px;font-weight:700;font-size:15px;">Créer mon mot de passe</a>
    </p>
    <p style="font-size:12px;color:#64748b;word-break:break-all;">Si le bouton ne fonctionne pas, copiez cette adresse :<br/><a href="${opts.url}">${opts.url}</a></p>

    <div style="background:#eff6ff;border:1px solid #93c5fd;border-radius:12px;padding:16px;margin:28px 0 12px;">
      <p style="margin:0 0 8px;font-size:15px;"><strong>Étape 2 — Double authentification (MFA) — obligatoire</strong></p>
      <p style="margin:0 0 12px;font-size:14px;">
        La MFA, c’est un <strong>code à 6 chiffres</strong> généré par une application sur votre téléphone.
        Sans cette étape, vous ne pourrez pas utiliser pleinement la plateforme
        (cloud, dossiers partagés, outils métiers, etc.).
      </p>
      <p style="margin:0 0 6px;font-size:14px;"><strong>A. Installez une appli gratuite</strong> (une seule suffit) :</p>
      <ul style="margin:0 0 12px;padding-left:18px;font-size:14px;">
        <li><strong>Microsoft Authenticator</strong> — Windows / Android / iPhone</li>
        <li><strong>Google Authenticator</strong> — Android / iPhone</li>
        <li><strong>Mots de passe d’Apple</strong> — iPhone / Mac (codes à deux facteurs)</li>
      </ul>
      <p style="margin:0 0 6px;font-size:14px;"><strong>B. Connectez-vous</strong> avec votre e-mail et le nouveau mot de passe.</p>
      <p style="margin:0 0 6px;font-size:14px;"><strong>C. Quand le QR code s’affiche :</strong></p>
      <ol style="margin:0 0 12px;padding-left:18px;font-size:14px;">
        <li>Ouvrez l’application d’authentification</li>
        <li>Ajoutez un compte → « Scanner un QR code »</li>
        <li>Scannez le QR code à l’écran</li>
        <li>Saisissez le code à 6 chiffres pour valider</li>
      </ol>
      <p style="margin:0;font-size:14px;"><strong>D. Ensuite</strong> : à chaque connexion = e-mail + mot de passe + code de l’appli.</p>
    </div>

    <p style="font-size:13px;color:#64748b;margin:20px 0 0;">
      Lien expiré ? Demandez un nouveau lien à l’administrateur de ${escapeHtml(etab)},
      ou utilisez « Mot de passe oublié » sur la page de connexion.
    </p>
    <p style="margin-top:28px;font-size:13px;color:#64748b;">— ${escapeHtml(etab)} via ScolIA</p>
  </div>
</body>
</html>`;

  return {
    subject,
    text,
    html,
    fromName,
    attachments: branding.logo ? [branding.logo] : [],
  };
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
      resetPasswordTokenExpiresIn: 12 * 60 * 60,
      revokeSessionsOnPasswordReset: true,
      sendResetPassword: async ({ user, url }) => {
        const u = user as {
          email: string;
          name?: string | null;
          firstName?: string | null;
        };
        const mail = await buildPasswordActivationEmail({
          firstName:
            typeof u.firstName === "string" && u.firstName.trim()
              ? u.firstName
              : typeof u.name === "string"
                ? u.name.split(/\s+/)[0] || null
                : null,
          url,
        });
        const ok = await sendPlatformMail({
          to: u.email,
          subject: mail.subject,
          text: mail.text,
          html: mail.html,
          fromName: mail.fromName,
          attachments: mail.attachments,
        });
        if (!ok) {
          throw new Error("Échec d'envoi de l'e-mail d'invitation (SMTP).");
        }
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
          // Abandon mid-QR : secret orphelin bloquerait le prochain setup MFA.
          const { clearIncompleteTwoFactorSetup } = await import("@/app/lib/two-factor-setup");
          await clearIncompleteTwoFactorSetup(user.id);
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
        /** Plafond staff (30 j). Direction / admin sont recalés dans les hooks MFA. */
        trustDeviceMaxAge: MFA_TRUST_STAFF_SECONDS,
        totpOptions: {
          digits: 6,
          period: 30,
        },
      }),
      nextCookies(),
    ],
    hooks: {
      before: createAuthMiddleware(async (ctx) => {
        await mfaTrustBeforeSignIn(ctx as never);

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
      after: createAuthMiddleware(async (ctx) => {
        await mfaTrustAfterVerify(ctx as never);
        await mfaTrustAfterSignIn(ctx as never);
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
