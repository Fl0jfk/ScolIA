import type { ReactNode, SVGProps } from "react";

type P = SVGProps<SVGSVGElement> & { className?: string };

function I({ className = "h-4 w-4", children, ...rest }: P & { children: ReactNode }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden
      {...rest}
    >
      {children}
    </svg>
  );
}

export const IconPin = (p: P) => (
  <I {...p}>
    <path d="M12 17v5" />
    <path d="M9 3h6l-1 6 3 3v2H7v-2l3-3-1-6z" />
  </I>
);
export const IconBellOff = (p: P) => (
  <I {...p}>
    <path d="M8.7 3A6 6 0 0 1 18 8c0 3 .6 5 1.4 6.4M6.3 6.3A6 6 0 0 0 6 8c0 7-3 9-3 9h14" />
    <path d="M10.3 21a1.9 1.9 0 0 0 3.4 0" />
    <path d="m2 2 20 20" />
  </I>
);
export const IconBell = (p: P) => (
  <I {...p}>
    <path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9" />
    <path d="M10.3 21a1.9 1.9 0 0 0 3.4 0" />
  </I>
);
export const IconMore = (p: P) => (
  <I {...p}>
    <circle cx="5" cy="12" r="1" />
    <circle cx="12" cy="12" r="1" />
    <circle cx="19" cy="12" r="1" />
  </I>
);
export const IconAt = (p: P) => (
  <I {...p}>
    <circle cx="12" cy="12" r="4" />
    <path d="M16 8v5a3 3 0 0 0 6 0v-1a10 10 0 1 0-4 8" />
  </I>
);
export const IconMailOpen = (p: P) => (
  <I {...p}>
    <path d="M3 9v10a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V9l-9-6-9 6z" />
    <path d="m3 9 9 6 9-6" />
  </I>
);
export const IconMailUnread = (p: P) => (
  <I {...p}>
    <rect x="3" y="5" width="18" height="14" rx="2" />
    <path d="m3 7 9 6 9-6" />
  </I>
);
export const IconTrashList = (p: P) => (
  <I {...p}>
    <path d="M3 6h18M8 6V4h8v2M6 6l1 14h10l1-14" />
  </I>
);
export const IconInfo = (p: P) => (
  <I {...p}>
    <circle cx="12" cy="12" r="10" />
    <path d="M12 16v-4M12 8h.01" />
  </I>
);
export const IconLogOut = (p: P) => (
  <I {...p}>
    <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
    <path d="m16 17 5-5-5-5M21 12H9" />
  </I>
);
