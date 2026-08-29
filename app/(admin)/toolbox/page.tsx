import { redirect } from "next/navigation";

/** Ancienne URL hub — redirige vers le pilier Services. */
export default function LegacyToolboxRedirect() {
  redirect("/services");
}
