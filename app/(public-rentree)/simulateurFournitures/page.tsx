import type { Metadata } from "next";
import { requireSimulateurFournituresPublicPage } from "@/app/lib/toolbox-public-gate";
import { getToolboxConfig } from "@/app/lib/toolbox-config";
import { resolveFournituresConfigPublicUrls } from "@/app/lib/fournitures-public-urls.server";
import SimulateurFournituresClient from "./SimulateurFournituresClient";

export async function generateMetadata(): Promise<Metadata> {
  const toolbox = await getToolboxConfig();
  const title =
    toolbox.tools["simulateur-fournitures"].title?.trim() || "Simulateur de fournitures";
  return {
    title,
    description: "Liste de fournitures scolaires par niveau et options.",
  };
}

export default async function SimulateurFournituresPage() {
  await requireSimulateurFournituresPublicPage();
  const toolbox = await getToolboxConfig();
  const config = await resolveFournituresConfigPublicUrls(toolbox.tools["simulateur-fournitures"]);
  return <SimulateurFournituresClient config={config} />;
}
