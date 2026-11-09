"use client";

import MessagingPresenceBadge from "@/app/components/messaging/MessagingPresenceBadge";
import type { MessagingPresenceStatus } from "@/app/lib/messaging/types";

export default function ListAvatar({
  imageUrl,
  letter,
  status,
  size = "h-11 w-11",
  rounded = "rounded-2xl",
}: {
  imageUrl: string | null;
  letter: string;
  status?: MessagingPresenceStatus | null;
  size?: string;
  rounded?: string;
}) {
  const inner = (
    <span
      className={`flex ${size} items-center justify-center overflow-hidden ${rounded} bg-[var(--dash-ink)] text-sm font-semibold text-white`}
    >
      {imageUrl ? (
        // eslint-disable-next-line @next/next/no-img-element
        <img src={imageUrl} alt="" className="h-full w-full object-cover" />
      ) : (
        letter
      )}
    </span>
  );
  if (status === undefined) return inner;
  return (
    <MessagingPresenceBadge status={status} showRing={false} dotClassName="h-3 w-3">
      {inner}
    </MessagingPresenceBadge>
  );
}
