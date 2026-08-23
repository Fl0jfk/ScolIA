import { getJson, putJson } from "@/app/lib/s3-storage";
import { PERSONNEL_LEAVES_KEY, type PersonnelLeaveRequest } from "@/app/lib/personnel-types";

export async function getPersonnelLeaveRequests(): Promise<PersonnelLeaveRequest[]> {
  const hit = await getJson<PersonnelLeaveRequest[]>(PERSONNEL_LEAVES_KEY);
  return Array.isArray(hit?.data)
    ? [...hit.data].sort((a, b) => b.createdAt.localeCompare(a.createdAt))
    : [];
}

async function savePersonnelLeaveRequests(requests: PersonnelLeaveRequest[]) {
  const sorted = [...requests].sort((a, b) => b.createdAt.localeCompare(a.createdAt));
  await putJson(PERSONNEL_LEAVES_KEY, sorted);
}

export async function upsertPersonnelLeaveRequest(request: PersonnelLeaveRequest) {
  const list = await getPersonnelLeaveRequests();
  const idx = list.findIndex((r) => r.id === request.id);
  if (idx >= 0) list[idx] = request;
  else list.push(request);
  await savePersonnelLeaveRequests(list);
  return request;
}
