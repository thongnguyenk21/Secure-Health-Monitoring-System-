import { clearSession, freshAccessToken, getSession, refreshSession, setSession } from "../auth/session";
import { ApiError, buildUrl, postPublic, readJson, toApiError, type QueryValue } from "./http";
import type {
  Alert,
  AlertPage,
  AlertStatus,
  AuditPage,
  ClinicalNote,
  ComponentStatus,
  Device,
  DeviceCreate,
  DevicePage,
  MeasurementBucket,
  MeasurementPage,
  Model,
  PatientSummary,
  RegisterUser,
  Role,
  TokenPair,
  User,
  UserPage,
} from "./types";

/** Options of one authenticated request. */
export interface RequestOptions {
  /** HTTP method; GET by default. */
  method?: "GET" | "POST" | "PATCH" | "DELETE";
  /** Query parameters. */
  query?: Record<string, QueryValue>;
  /** JSON body. */
  body?: unknown;
}

async function send(path: string, opts: RequestOptions, token: string): Promise<Response> {
  const headers: Record<string, string> = { Accept: "application/json", Authorization: `Bearer ${token}` };
  if (opts.body !== undefined) headers["Content-Type"] = "application/json";
  return fetch(buildUrl(path, opts.query), {
    method: opts.method ?? "GET",
    headers,
    body: opts.body === undefined ? undefined : JSON.stringify(opts.body),
  });
}

/**
 * Call an authenticated backend route.
 *
 * @param path - Route below `/api/v1`, starting with `/`.
 * @param opts - Method, query and body.
 * @returns The parsed JSON body (undefined for 204).
 * @throws {@link ApiError} On a non-2xx status.
 * @throws {@link SessionExpiredError} When the session cannot be refreshed.
 * @remarks
 * A 401 triggers one refresh and one retry; a second 401 ends the session so
 * the router sends the user to the login page.
 */
export async function apiRequest<T>(path: string, opts: RequestOptions = {}): Promise<T> {
  let res = await send(path, opts, await freshAccessToken());
  if (res.status === 401) {
    const session = await refreshSession();
    res = await send(path, opts, session.pair.access_token);
    if (res.status === 401) {
      clearSession();
    }
  }
  if (!res.ok) throw await toApiError(res);
  return readJson<T>(res);
}

// Pages are capped so a runaway cursor cannot loop forever.
const MAX_PAGES = 20;

/**
 * Fetch every page of a cursor-paginated collection.
 *
 * @param path - Collection route.
 * @param query - Filters; `cursor` is added per page.
 * @param pick - Extracts the items of one page.
 * @returns All items in server order.
 */
export async function fetchAllPages<P extends { next_cursor?: string | null }, I>(
  path: string,
  query: Record<string, QueryValue>,
  pick: (page: P) => I[] | undefined,
): Promise<I[]> {
  const all: I[] = [];
  let cursor: string | null | undefined;
  for (let i = 0; i < MAX_PAGES; i += 1) {
    const page = await apiRequest<P>(path, { query: { ...query, cursor } });
    all.push(...(pick(page) ?? []));
    cursor = page.next_cursor;
    if (!cursor) break;
  }
  return all;
}

/** Typed functions for each backend route the web app uses. */
export const api = {
  /**
   * Log in and store the session.
   *
   * @param email - Account email.
   * @param password - Account password.
   * @returns The token pair.
   */
  async login(email: string, password: string): Promise<TokenPair> {
    const pair = await postPublic<TokenPair>("/auth/login", { email, password });
    setSession(pair);
    return pair;
  },
  /** Revoke the refresh token family and forget the session, ignoring network errors. */
  async logout(): Promise<void> {
    const session = getSession();
    try {
      if (session) {
        await apiRequest<undefined>("/auth/logout", {
          method: "POST",
          body: { refresh_token: session.pair.refresh_token },
        });
      }
    } catch (err) {
      if (!(err instanceof Error)) throw err;
    } finally {
      clearSession();
    }
  },
  /** Read the account of the signed-in user. */
  getMe: () => apiRequest<User>("/me"),
  /** List patients, highest priority first. */
  listPatients: () => apiRequest<PatientSummary[]>("/patients"),
  /** Get one patient with the latest vitals. */
  getPatient: (userId: string) => apiRequest<PatientSummary>(`/patients/${encodeURIComponent(userId)}`),
  /** List the clinical notes of a patient, newest first. */
  listNotes: (userId: string) => apiRequest<ClinicalNote[]>(`/patients/${encodeURIComponent(userId)}/notes`),
  /** Add a clinical note to a patient. */
  createNote: (userId: string, text: string) =>
    apiRequest<ClinicalNote>(`/patients/${encodeURIComponent(userId)}/notes`, { method: "POST", body: { text } }),
  /**
   * Rollup buckets of one device over a range, following the cursor.
   *
   * @param deviceId - Device id.
   * @param fromMs - Start, inclusive.
   * @param toMs - End, exclusive.
   * @returns Buckets, oldest first.
   */
  listBuckets: (deviceId: string, fromMs: number, toMs: number) =>
    fetchAllPages<MeasurementPage, MeasurementBucket>(
      "/measurements",
      { device_id: deviceId, from_unix_ms: fromMs, to_unix_ms: toMs, resolution: "1m", limit: 1000 },
      (page) => page.buckets,
    ),
  /**
   * One page of alerts, newest first.
   *
   * @param filter - Optional status, device and page size.
   * @returns The page.
   */
  listAlerts: (filter: { status?: AlertStatus; deviceId?: string; limit?: number; cursor?: string | null }) =>
    apiRequest<AlertPage>("/alerts", {
      query: { status: filter.status, device_id: filter.deviceId, limit: filter.limit, cursor: filter.cursor },
    }),
  /** Acknowledge an alert. */
  ackAlert: (alertId: string) => apiRequest<Alert>(`/alerts/${encodeURIComponent(alertId)}/ack`, { method: "POST" }),
  /** List every device of the tenant. */
  listDevices: () => fetchAllPages<DevicePage, Device>("/devices", { limit: 1000 }, (page) => page.items),
  /** Register a device; it starts as pending. */
  createDevice: (body: DeviceCreate) => apiRequest<Device>("/devices", { method: "POST", body }),
  /** Change a device's label. */
  updateDeviceLabel: (deviceId: string, label: string) =>
    apiRequest<Device>(`/devices/${deviceId}`, { method: "PATCH", body: { label } }),
  /** Assign a device to a patient, or unassign it with null. */
  assignDevice: (deviceId: string, userId: string | null) =>
    apiRequest<Device>(`/devices/${deviceId}/assign`, { method: "POST", body: { user_id: userId } }),
  /** Revoke a device. */
  revokeDevice: (deviceId: string) => apiRequest<Device>(`/devices/${deviceId}/revoke`, { method: "POST" }),
  /** List users, oldest first, optionally of one role. */
  listUsers: (role?: Role) => fetchAllPages<UserPage, User>("/users", { role, limit: 1000 }, (page) => page.items),
  /** Create a user account. */
  registerUser: (body: RegisterUser) => apiRequest<User>("/auth/register", { method: "POST", body }),
  /** List model versions, newest first. */
  listModels: () => apiRequest<Model[]>("/models"),
  /** Read one page of the audit log, newest first. */
  listAudit: (limit: number, cursor?: string | null) => apiRequest<AuditPage>("/audit", { query: { limit, cursor } }),
  /** Read the state of the system components. */
  systemStatus: () => apiRequest<ComponentStatus[]>("/system/status"),
};

export { ApiError };
