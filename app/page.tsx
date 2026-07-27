import type { Metadata } from "next";
import { auth, currentUser } from "@clerk/nextjs/server";
import { headers } from "next/headers";
import { redirect } from "next/navigation";
import LandingPage from "./components/landing/LandingPage";
import { isPlatformHostname } from "@/app/lib/platform-hostname";
import { isPlatformMasterFromPublicMetadata, resolveSession, safeCurrentUser } from "@/app/lib/intranet-session";
import { isMultiTenantEnabled } from "@/app/lib/tenant-registry";

export const metadata: Metadata = {
  title: "ScolIA — Intranet tout-en-un pour établissements scolaires",
  description:"Workflows documents élèves, sorties, absences, RH, internat… Un abonnement tout inclus. Hébergement France.",
};

export default async function HomePage() {
  const hdrs = await headers();
  const host = hdrs.get("x-forwarded-host") || hdrs.get("host") || "";
  if (isMultiTenantEnabled()) {
    const session = await resolveSession();
    if (!session) return <LandingPage />;
    if (isPlatformHostname(host)) {
      const user = await safeCurrentUser();
      if (isPlatformMasterFromPublicMetadata(user?.publicMetadata)) { redirect("/plateforme")}
      return <LandingPage />;
    }
    redirect("/dashboard");
  }
  const { userId } = await auth();
  if (!userId) return <LandingPage />;
  if (isPlatformHostname(host)) {
    const user = await currentUser();
    if (isPlatformMasterFromPublicMetadata(user?.publicMetadata)) { redirect("/plateforme")}
    return <LandingPage />;
  }
  redirect("/dashboard");
}
