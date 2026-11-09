"use client";

import MessagingPresenceBadge from "@/app/components/messaging/MessagingPresenceBadge";
import { avatarTone, initialsOf } from "@/app/components/messaging/conversation/Avatar";
import type { MessagingPresenceStatus } from "@/app/lib/messaging/types";

export default function ListAvatar({
  imageUrl,
  letter,
  name,
  status,
  size = "h-11 w-11",
  rounded = "rounded-full",
}: {
  imageUrl: string | null;
  /** Repli si le nom n'est pas fourni. */
  letter: string;
  name?: string;
  status?: MessagingPresenceStatus | null;
  size?: string;
  rounded?: string;
}) {
  const label = name?.trim() || letter;
  const inner = (
    <span
      className={`flex ${size} select-none items-center justify-center overflow-hidden ${rounded} text-sm font-semibold`}
      style={imageUrl ? { backgroundColor: "#d4d4d8" } : avatarTone(label)}
      aria-hidden
    >
      {imageUrl ? (
        // eslint-disable-next-line @next/next/no-img-element
        <img src={imageUrl} alt="" className="h-full w-full object-cover" />
      ) : name?.trim() ? (
        initialsOf(name)
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
