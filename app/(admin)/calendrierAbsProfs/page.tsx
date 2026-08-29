import { redirect } from "next/navigation";

export default function CalendrierAbsProfsPage() {
  redirect("/rh?tab=dashboard&section=absences&view=calendrier");
}
