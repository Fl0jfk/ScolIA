import { redirect } from "next/navigation";

/** Landing historique — le portail quotidien est la face web familles. */
export default function AppMobileLandingPage() {
  redirect("/quotidien");
}
