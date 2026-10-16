import { QueryClient } from "@tanstack/react-query";
import { ApiError, SessionExpiredError } from "../api/http";

/** Polling interval of the live data (latest reading, alerts) in ms; no push in M1. */
export const POLL_MS = 10_000;

/**
 * Decide whether a failed query is worth retrying.
 *
 * @param failureCount - Retries done so far.
 * @param error - The last error.
 * @returns False for client errors and an ended session, true for up to two transient failures.
 */
export function shouldRetry(failureCount: number, error: unknown): boolean {
  if (error instanceof SessionExpiredError) return false;
  if (error instanceof ApiError && error.status < 500 && error.status !== 429) return false;
  return failureCount < 2;
}

/**
 * Create the query client of the app.
 *
 * @returns A client with retry rules suited to a phone on a flaky network.
 */
export function createQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: shouldRetry, staleTime: 5_000, refetchOnWindowFocus: true },
      mutations: { retry: false },
    },
  });
}
