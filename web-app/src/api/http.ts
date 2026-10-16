import type { ApiErrorBody } from "./types";

/** Base path of every backend route. */
export const API_BASE = "/api/v1";

/** Query string values accepted by {@link buildUrl}; null and undefined are skipped. */
export type QueryValue = string | number | boolean | null | undefined;

/** Raised when the backend answers with a non-2xx status. */
export class ApiError extends Error {
  /** HTTP status code. */
  readonly status: number;
  /** Stable machine-readable code from the error body, or `http_<status>`. */
  readonly code: string;
  /** Request id in the backend logs, when the body carried one. */
  readonly traceId: string | null;

  /**
   * @param status - HTTP status code.
   * @param body - Parsed error body, or null when the body was not an `Error`.
   */
  constructor(status: number, body: ApiErrorBody | null) {
    super(body?.message ?? `HTTP ${status}`);
    this.name = "ApiError";
    this.status = status;
    this.code = body?.code ?? `http_${status}`;
    this.traceId = body?.traceId ?? null;
  }
}

/** Raised when the session cannot be refreshed and the user must log in again. */
export class SessionExpiredError extends Error {
  /** @param message - Why the session ended. */
  constructor(message = "session expired") {
    super(message);
    this.name = "SessionExpiredError";
  }
}

/**
 * Build a backend URL with a query string.
 *
 * @param path - Route below {@link API_BASE}, starting with `/`.
 * @param query - Query parameters; null and undefined values are left out.
 * @returns A same-origin URL string.
 */
export function buildUrl(path: string, query?: Record<string, QueryValue>): string {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(query ?? {})) {
    if (value !== null && value !== undefined) params.set(key, String(value));
  }
  const qs = params.toString();
  return `${API_BASE}${path}${qs ? `?${qs}` : ""}`;
}

function isErrorBody(value: unknown): value is ApiErrorBody {
  return typeof value === "object" && value !== null && "code" in value && "message" in value;
}

/**
 * Turn a failed response into an {@link ApiError}.
 *
 * @param res - Response with a non-2xx status.
 * @returns The error to throw.
 */
export async function toApiError(res: Response): Promise<ApiError> {
  let body: unknown = null;
  try {
    body = await res.json();
  } catch {
    body = null;
  }
  return new ApiError(res.status, isErrorBody(body) ? body : null);
}

/**
 * Read a successful response body as JSON, or undefined for 204.
 *
 * @param res - Response with a 2xx status.
 * @returns The parsed body.
 */
export async function readJson<T>(res: Response): Promise<T> {
  if (res.status === 204) return undefined as T;
  return (await res.json()) as T;
}

/**
 * POST a JSON body without authentication (login, refresh).
 *
 * @param path - Route below {@link API_BASE}.
 * @param body - Request body.
 * @returns The parsed response body.
 * @throws {@link ApiError} On a non-2xx status.
 */
export async function postPublic<T>(path: string, body: unknown): Promise<T> {
  const res = await fetch(buildUrl(path), {
    method: "POST",
    headers: { "Content-Type": "application/json", Accept: "application/json" },
    body: JSON.stringify(body),
  });
  if (!res.ok) throw await toApiError(res);
  return readJson<T>(res);
}
