import { redirect } from "next/navigation";

/** Ancienne URL — redirige vers le formulaire unifié. */
export default async function StageEleveRedirectPage({
  searchParams,
}: {
  searchParams: Promise<{ token?: string }>;
}) {
  const { token } = await searchParams;
  if (token?.trim()) {
    redirect(`/stages/preconvention?token=${encodeURIComponent(token.trim())}`);
  }
  redirect("/stages/preconvention");
}
