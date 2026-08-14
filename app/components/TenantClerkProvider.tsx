import { ClerkProvider } from "@clerk/nextjs";
import { frFR } from "@clerk/localizations";
import { headers } from "next/headers";
import LocalClerkSetupBanner from "@/app/components/LocalClerkSetupBanner";
import {
  clerkKeysFromEnv,
  needsLocalClerkDevKeys,
  resolveClerkKeysForHostname,
} from "@/app/lib/clerk-tenant-keys";
import { getTenant } from "@/app/lib/tenant-context";
import { clerkAfterSignInUrl, clerkSignInPageUrl } from "@/app/lib/tenant-auth-urls";
import { isMultiTenantEnabled } from "@/app/lib/tenant-registry";

type Props = {
  children: React.ReactNode;
  nonce?: string;
};

async function requestHostname(): Promise<string> {
  const hdrs = await headers();
  return hdrs.get("x-forwarded-host") || hdrs.get("host") || "";
}

/** ClerkProvider — clé dynamique par tenant uniquement si registry multi-tenant actif. */
export default async function TenantClerkProvider({ children, nonce }: Props) {
  const host = await requestHostname();

  if (!isMultiTenantEnabled()) {
    const env = clerkKeysFromEnv();
    const publishableKey =
      env?.publishableKey ?? process.env.NEXT_PUBLIC_CLERK_PUBLISHABLE_KEY?.trim() ?? "";

    if (needsLocalClerkDevKeys(host, publishableKey)) {
      return (
        <>
          <LocalClerkSetupBanner />
          {children}
        </>
      );
    }

    return (
      <ClerkProvider
        publishableKey={publishableKey || undefined}
        localization={frFR}
        nonce={nonce}
        dynamic
      >
        {children}
      </ClerkProvider>
    );
  }

  const tenant = await getTenant();
  const keys = resolveClerkKeysForHostname(host, {
    clerkPublishableKey: tenant.clerkPublishableKey,
    clerkSecretKey: tenant.clerkSecretKey,
    clerkDevPublishableKey: tenant.secrets?.clerkDevPublishableKey,
    clerkDevSecretKey: tenant.secrets?.clerkDevSecretKey,
  });
  const afterAuthUrl = clerkAfterSignInUrl(tenant, host);
  const signInUrl = clerkSignInPageUrl(tenant, host);

  if (needsLocalClerkDevKeys(host, keys.publishableKey)) {
    return (
      <>
        <LocalClerkSetupBanner />
        {children}
      </>
    );
  }

  return (
    <ClerkProvider
      publishableKey={keys.publishableKey}
      signInUrl={signInUrl}
      signUpUrl={signInUrl.replace(/\/sign-in$/, "/sign-up")}
      signInFallbackRedirectUrl={afterAuthUrl}
      signUpFallbackRedirectUrl={afterAuthUrl}
      localization={frFR}
      nonce={nonce}
      dynamic
    >
      {children}
    </ClerkProvider>
  );
}
