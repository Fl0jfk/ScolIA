"use client";

import ModuleTabNav from "@/app/components/module-chrome/ModuleTabNav";
import type { TravelsHubTab } from "@/app/lib/travels-types";
import { TRAVELS_HUB_TABS } from "@/app/lib/travels-types";

export function TripHubNav({
  active,
  onChange,
  badges,
  tabs = TRAVELS_HUB_TABS,
}: {
  active: TravelsHubTab;
  onChange: (tab: TravelsHubTab) => void;
  badges?: Partial<Record<TravelsHubTab, number>>;
  tabs?: typeof TRAVELS_HUB_TABS;
}) {
  return (
    <ModuleTabNav
      tabs={tabs}
      active={active}
      onChange={onChange}
      badges={badges}
      scroll
    />
  );
}
