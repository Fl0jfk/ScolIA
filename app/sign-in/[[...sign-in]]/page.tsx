import { SignIn } from "@clerk/nextjs";
import { headers } from "next/headers";
import { loadPublicSiteIdentity } from "@/app/lib/site-public";
import { getTenant } from "@/app/lib/tenant-context";
import { clerkAfterSignInUrl } from "@/app/lib/tenant-auth-urls";

export default async function SignInPage() {
  const hdrs = await headers();
  const host = hdrs.get("x-forwarded-host") || hdrs.get("host") || "";
  const [tenant, siteIdentity] = await Promise.all([getTenant(), loadPublicSiteIdentity()]);
  const afterSignIn = clerkAfterSignInUrl(tenant, host);
  const logoImageUrl = siteIdentity.headerLogoUrl ?? undefined;

  return (
    <div className="sign-in-clerk-card flex min-h-[calc(100vh-3.5rem)] items-center justify-center px-4 py-10">
      <SignIn
        fallbackRedirectUrl={afterSignIn}
        forceRedirectUrl={afterSignIn}
        appearance={{
          variables: {
            colorPrimary: "#2F6B4A",
            colorBackground: "white",
            borderRadius: "1rem",
          },
          options: {
            logoPlacement: "inside",
            ...(logoImageUrl ? { logoImageUrl } : {}),
            logoLinkUrl: "/",
          },
          elements: {
            card: "shadow-xl shadow-emerald-900/10 border border-emerald-100",
            formButtonPrimary: "bg-gradient-to-r from-[#2F6B4A] to-[#1E4A32] hover:brightness-110",
            headerTitle: "hidden",
            headerSubtitle: "hidden",
            logoBox: "flex justify-center items-center my-5",
            logoImage: {
              width: "auto",
              height: "4.5rem",
              maxWidth: "11rem",
              objectFit: "contain",
            },
          },
        }}
      />
    </div>
  );
}
