import { redirect } from "next/navigation";

type PageProps = {
  searchParams?: Promise<Record<string, string | string[] | undefined>>;
};

/** Ancienne route de connexion — bascule vers Better-Auth. */
export default async function LegacySignInRedirect({ searchParams }: PageProps) {
  const params = (await searchParams) ?? {};
  const redirectUrl = params.redirect_url;
  const dest = new URLSearchParams();
  if (typeof redirectUrl === "string" && redirectUrl.trim()) {
    dest.set("redirect_url", redirectUrl);
  }
  const qs = dest.toString();
  redirect(qs ? `/auth/sign-in?${qs}` : "/auth/sign-in");
}
