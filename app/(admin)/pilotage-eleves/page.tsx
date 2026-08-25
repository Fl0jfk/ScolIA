import { redirect } from "next/navigation";

/** Module masqué — redirigé vers le pilier Administratif. */
export default function PilotageElevesHiddenPage() {
  redirect("/administratif");
}
