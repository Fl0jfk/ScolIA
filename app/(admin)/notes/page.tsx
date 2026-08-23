import { redirect } from "next/navigation";

/** Ancien hub pilier Notes → Administratif (Notes est un module, pas un pilier). */
export default function NotesHubRedirectPage() {
  redirect("/administratif");
}
