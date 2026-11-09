import type { MessagingPeer } from "@/app/lib/messaging/types";
import Avatar from "./Avatar";

const MAX = 5;

/** Petits avatars « vu par » (max 5 + compteur) sous mon dernier message lu. */
export default function SeenBy({ readers }: { readers: MessagingPeer[] }) {
  if (readers.length === 0) return null;
  const shown = readers.slice(0, MAX);
  const extra = readers.length - shown.length;
  return (
    <span
      className="inline-flex items-center"
      title={`Vu par ${readers.map((r) => r.name).join(", ")}`}
      aria-hidden
    >
      <span className="flex -space-x-1.5">
        {shown.map((r) => (
          <Avatar key={r.id} name={r.name} imageUrl={r.imageUrl} size={16} className="ring-2 ring-white" />
        ))}
      </span>
      {extra > 0 ? (
        <span className="ml-1 text-[10px] font-semibold text-[var(--dash-mid)]">+{extra}</span>
      ) : null}
    </span>
  );
}
