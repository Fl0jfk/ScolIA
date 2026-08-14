type DashboardUploadMode = "standard" | "class";

type PendingDashboardUpload = {
  mode: DashboardUploadMode;
  files: File[];
};

let pending: PendingDashboardUpload | null = null;

export function stageDashboardUpload(mode: DashboardUploadMode, fileList: FileList | File[]) {
  const files = Array.from(fileList).filter(
    (f) => f.type === "application/pdf" || f.name.toLowerCase().endsWith(".pdf"),
  );
  if (files.length === 0) return false;
  pending = { mode, files };
  return true;
}

export function consumeDashboardUpload(): PendingDashboardUpload | null {
  const hit = pending;
  pending = null;
  return hit;
}
