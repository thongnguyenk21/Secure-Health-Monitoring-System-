import {
  clearSession,
  freshAccessToken,
  getSession,
  refreshSession,
  setSession,
} from "../features/auth/session";
import { ApiError, buildUrl, postPublic, readJson, send, toApiError, type QueryValue } from "./http";
import type {
  Alert,
  AlertPage,
  AlertQuery,
  Consent,
  ConsentCreate,
  Device,
  DevicePage,
  LoginRequest,
  Measurement,
  MeasurementBucket,
  MeasurementPage,
  MeasurementQuery,
  RefreshRequest,
  TokenPair,
  User,
} from "./types";

/** Options of one authenticated request. */
export interface RequestOptions {
  /** HTTP method; GET by default. */
  method?: "GET" | "POST";
  /** Query parameters. */
  query?: Record<string, QueryValue>;
  /** JSON body. */
  body?: unknown;
}

function sendAuthed(path: string, opts: RequestOptions, token: string): Promise<Response> {
  const headers: Record<string, string> = { Accept: "application/json", Authorization: `Bearer ${token}` };
  if (opts.body !== undefined) headers["Content-Type"] = "application/json";
  return send(buildUrl(path, opts.query), {
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
 * @throws {@link NetworkError} When the backend cannot be reached.
 * @remarks
 * A 401 triggers one refresh and one retry; a second 401 ends the session so
 * the app returns to the login screen.
 */
export async function apiRequest<T>(path: string, opts: RequestOptions = {}): Promise<T> {
  let res = await sendAuthed(path, opts, await freshAccessToken());
  if (res.status === 401) {
    const session = await refreshSession();
    res = await sendAuthed(path, opts, session.pair.access_token);
    if (res.status === 401) clearSession("expired");
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

/** Typed functions for each backend route the patient app uses. */
export const api = {
  /**
   * Log in and store the session.
   *
   * @param body - Email and password.
   * @returns The token pair.
   */
  async login(body: LoginRequest): Promise<TokenPair> {
    const pair = await postPublic<TokenPair>("/auth/login", body);
    setSession(pair);
    return pair;
  },
  /** Revoke the refresh token family and forget the session, ignoring network errors. */
  async logout(): Promise<void> {
    const session = getSession();
    try {
      if (session) {
        const body: RefreshRequest = { refresh_token: session.pair.refresh_token };
        await apiRequest<undefined>("/auth/logout", { method: "POST", body });
      }
    } catch (err) {
      if (!(err instanceof Error)) throw err;
    } finally {
      clearSession("logout");
    }
  },
  /** Read the account of the signed-in user. */
  getMe: () => apiRequest<User>("/me"),
  /** List the devices assigned to the patient. */
  listDevices: () => fetchAllPages<DevicePage, Device>("/devices", { limit: 100 }, (page) => page.items),
  /**
   * Latest raw vitals of a device.
   *
   * @param deviceId - Pseudonymous device id.
   * @returns The row, or null when the device has not uploaded anything yet (404).
   */
  async getLatest(deviceId: string): Promise<Measurement | null> {
    try {
      return await apiRequest<Measurement>("/measurements/latest", { query: { device_id: deviceId } });
    } catch (err) {
      if (err instanceof ApiError && err.status === 404) return null;
      throw err;
    }
  },
  /**
   * Every measurement page of a range, following `next_cursor`.
   *
   * @param query - Device, range, resolution and page size.
   * @returns All pages merged: raw rows in `items` or rollups in `buckets`, oldest first.
   */
  async listMeasurements(query: MeasurementQuery): Promise<Required<Pick<MeasurementPage, "items" | "buckets">>> {
    const items: Measurement[] = [];
    const buckets: MeasurementBucket[] = [];
    let cursor: string | null | undefined;
    for (let i = 0; i < MAX_PAGES; i += 1) {
      const page = await apiRequest<MeasurementPage>("/measurements", { query: { ...query, cursor } });
      items.push(...(page.items ?? []));
      buckets.push(...(page.buckets ?? []));
      cursor = page.next_cursor;
      if (!cursor) break;
    }
    return { items, buckets };
  },
  /**
   * One page of the patient's alerts, newest first.
   *
   * @param query - Device filter and page size.
   * @returns The page.
   */
  listAlerts: (query: AlertQuery) => apiRequest<AlertPage>("/alerts", { query: { ...query } }),
  /**
   * Acknowledge an alert ("I am fine"); stops its escalation.
   *
   * @param alertId - Alert id.
   * @returns The acknowledged alert.
   */
  ackAlert: (alertId: string) => apiRequest<Alert>(`/alerts/${encodeURIComponent(alertId)}/ack`, { method: "POST" }),
  /** List the consent records of the patient. */
  listConsents: () => apiRequest<Consent[]>("/consents"),
  /**
   * Append a consent decision.
   *
   * @param body - Scope, decision and form version.
   * @returns The stored record.
   */
  recordConsent: (body: ConsentCreate) => apiRequest<Consent>("/consents", { method: "POST", body }),
};
