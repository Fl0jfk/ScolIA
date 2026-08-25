import { redirect } from "next/navigation";

/** Ancienne URL — fusionnée dans Appels & absences. */
export default async function VsAbsencesLegacyRedirect({
  searchParams,
}: {
  searchParams: Promise<{ filtre?: string }>;
}) {
  const { filtre } = await searchParams;
  const q = new URLSearchParams({ tab: "absences" });
  if (filtre) q.set("filtre", filtre);
  redirect(`/vie-scolaire/presence?${q.toString()}`);
}
