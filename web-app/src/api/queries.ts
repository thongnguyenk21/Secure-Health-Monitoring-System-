import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "./client";
import type { AlertStatus, DeviceCreate, RegisterUser, Role } from "./types";

/** Query keys shared by the hooks and the live-stream invalidation. */
export const queryKeys = {
  me: ["me"] as const,
  patients: ["patients"] as const,
  patient: (userId: string) => ["patients", userId] as const,
  notes: (userId: string) => ["notes", userId] as const,
  buckets: (deviceId: string) => ["buckets", deviceId] as const,
  alerts: ["alerts"] as const,
  alertList: (status: AlertStatus | undefined, deviceId: string | undefined) => ["alerts", status ?? "all", deviceId ?? "all"] as const,
  devices: ["devices"] as const,
  users: (role?: Role) => ["users", role ?? "all"] as const,
  models: ["models"] as const,
  audit: (limit: number, cursor: string | null) => ["audit", limit, cursor] as const,
  system: ["system"] as const,
};

/** Vitals are polled every 30 s; alerts arrive over the WebSocket. */
export const VITALS_POLL_MS = 30_000;
const DAY_MS = 24 * 60 * 60_000;
// The open-alert list is one page; more than this shows as "N+".
const OPEN_ALERT_PAGE = 200;

/** Account of the signed-in user (display name for the navigation). */
export function useMe() {
  return useQuery({ queryKey: queryKeys.me, queryFn: api.getMe, staleTime: 5 * 60_000 });
}

/** Patients ordered by priority, refreshed every 30 s. */
export function usePatients() {
  return useQuery({ queryKey: queryKeys.patients, queryFn: api.listPatients, refetchInterval: VITALS_POLL_MS });
}

/**
 * One patient with the latest vitals, refreshed every 30 s.
 *
 * @param userId - Patient id.
 */
export function usePatient(userId: string) {
  return useQuery({
    queryKey: queryKeys.patient(userId),
    queryFn: () => api.getPatient(userId),
    refetchInterval: VITALS_POLL_MS,
  });
}

/**
 * Clinical notes of a patient.
 *
 * @param userId - Patient id.
 */
export function useNotes(userId: string) {
  return useQuery({ queryKey: queryKeys.notes(userId), queryFn: () => api.listNotes(userId) });
}

/**
 * Add a clinical note, then refetch the notes.
 *
 * @param userId - Patient id.
 */
export function useCreateNote(userId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (text: string) => api.createNote(userId, text),
    onSuccess: () => qc.invalidateQueries({ queryKey: queryKeys.notes(userId) }),
  });
}

/**
 * One-minute heart-rate buckets of the last 24 hours.
 *
 * @param deviceId - Device id, or null when the patient has none.
 */
export function useDayBuckets(deviceId: string | null | undefined) {
  return useQuery({
    queryKey: queryKeys.buckets(deviceId ?? ""),
    enabled: Boolean(deviceId),
    refetchInterval: 60_000,
    queryFn: async () => {
      const to = Date.now();
      const from = to - DAY_MS;
      const buckets = await api.listBuckets(deviceId ?? "", from, to);
      return { from, to, buckets };
    },
  });
}

/**
 * Alerts of one device, all statuses, newest first.
 *
 * @param deviceId - Device id, or null when the patient has none.
 */
export function useDeviceAlerts(deviceId: string | null | undefined) {
  return useQuery({
    queryKey: queryKeys.alertList(undefined, deviceId ?? ""),
    enabled: Boolean(deviceId),
    staleTime: 0,
    queryFn: () => api.listAlerts({ deviceId: deviceId ?? "", limit: 500 }),
  });
}

/** Open alerts of the tenant, newest first. */
export function useOpenAlerts() {
  return useQuery({
    queryKey: queryKeys.alertList("open", undefined),
    staleTime: 0,
    queryFn: () => api.listAlerts({ status: "open", limit: OPEN_ALERT_PAGE }),
  });
}

/** Acknowledge an alert, then refetch alerts and patients. */
export function useAckAlert() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (alertId: string) => api.ackAlert(alertId),
    onSuccess: async () => {
      await Promise.all([
        qc.invalidateQueries({ queryKey: queryKeys.alerts }),
        qc.invalidateQueries({ queryKey: queryKeys.patients }),
      ]);
    },
  });
}

/** Every device of the tenant. */
export function useDevices() {
  return useQuery({ queryKey: queryKeys.devices, queryFn: api.listDevices, refetchInterval: 60_000 });
}

/**
 * Users of the tenant.
 *
 * @param role - Optional role filter.
 */
export function useUsers(role?: Role) {
  return useQuery({ queryKey: queryKeys.users(role), queryFn: () => api.listUsers(role) });
}

/** Model versions. */
export function useModels() {
  return useQuery({ queryKey: queryKeys.models, queryFn: api.listModels });
}

/**
 * One page of the audit log.
 *
 * @param limit - Page size.
 * @param cursor - Page cursor, null for the newest page.
 */
export function useAudit(limit: number, cursor: string | null) {
  return useQuery({ queryKey: queryKeys.audit(limit, cursor), queryFn: () => api.listAudit(limit, cursor) });
}

/** State of the system components, refreshed every 30 s. */
export function useSystemStatus() {
  return useQuery({ queryKey: queryKeys.system, queryFn: api.systemStatus, refetchInterval: 30_000 });
}

/** Register a device, then refetch the device list. */
export function useCreateDevice() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: DeviceCreate) => api.createDevice(body),
    onSuccess: () => qc.invalidateQueries({ queryKey: queryKeys.devices }),
  });
}

/** Change a device label, assign it or revoke it, then refetch the device list. */
export function useDeviceActions() {
  const qc = useQueryClient();
  const onSuccess = () => qc.invalidateQueries({ queryKey: queryKeys.devices });
  const label = useMutation({
    mutationFn: (v: { deviceId: string; label: string }) => api.updateDeviceLabel(v.deviceId, v.label),
    onSuccess,
  });
  const assign = useMutation({
    mutationFn: (v: { deviceId: string; userId: string | null }) => api.assignDevice(v.deviceId, v.userId),
    onSuccess,
  });
  const revoke = useMutation({ mutationFn: (deviceId: string) => api.revokeDevice(deviceId), onSuccess });
  return { label, assign, revoke };
}

/** Create a user account, then refetch the user lists. */
export function useRegisterUser() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: RegisterUser) => api.registerUser(body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["users"] }),
  });
}
