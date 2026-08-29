import { redirect } from "next/navigation";

/** Absences absorbées dans le module RH. */
export default async function AbsencesPage({
  searchParams,
}: {
  searchParams: Promise<{ tab?: string }>;
}) {
  const sp = await searchParams;
  const view = sp.tab === "declarer" || sp.tab === "mes-demandes" ? "se-declarer" : sp.tab || "se-declarer";
  redirect(`/rh?tab=dashboard&section=absences&view=${encodeURIComponent(view)}`);
}
