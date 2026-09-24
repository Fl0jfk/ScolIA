import type { Metadata } from "next";
import { headers } from "next/headers";
import { redirect } from "next/navigation";
import LandingPage from "./components/landing/LandingPage";
import { isDemoClickAllowed } from "@/app/lib/demo-click";
import { isPlatformHostname } from "@/app/lib/platform-hostname";
import { getScolaRuntimeEnv } from "@/app/lib/scola-env";
import {
  isPlatformMasterFromPublicMetadata,
  resolveSession,
  safeCurrentUser,
} from "@/app/lib/intranet-session";

export const metadata: Metadata = {
  title: "ScolIA — Intranet tout-en-un pour établissements scolaires",
  description:
    "Moins de papier grâce à l’IA, mieux communiquer, serveurs en France. Intranet scolaire tout inclus.",
};

export default async function HomePage() {
  const hdrs = await headers();
  const host = hdrs.get("x-forwarded-host") || hdrs.get("host") || "";
  const session = await resolveSession();
  if (!session) {
    // Labo / Cloud Agent : lander directement sur la démo one-click.
    if (isDemoClickAllowed() && getScolaRuntimeEnv() === "lab") {
      redirect("/demo");
    }
    return <LandingPage />;
  }
  if (isPlatformHostname(host)) {
    const user = await safeCurrentUser();
    if (isPlatformMasterFromPublicMetadata(user?.publicMetadata)) {
      redirect("/plateforme");
    }
    return <LandingPage />;
  }
  redirect("/dashboard");
}
