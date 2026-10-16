/** Backend origin used when `EXPO_PUBLIC_API_URL` is not set (local dev stack). */
export const DEFAULT_API_URL = "http://localhost:8080";

/**
 * Resolve the backend origin from the build-time environment.
 *
 * @param raw - Value of `EXPO_PUBLIC_API_URL`, if any.
 * @returns The origin without a trailing slash.
 * @remarks
 * Expo inlines `EXPO_PUBLIC_*` variables at bundle time. The Android emulator
 * reaches the host machine at `http://10.0.2.2:8080`. The backend keeps
 * port 8080 because Metro, Expo's native dev server, uses 8081.
 */
export function resolveApiUrl(raw: string | undefined): string {
  const value = raw?.trim();
  return (value ? value : DEFAULT_API_URL).replace(/\/+$/, "");
}

/** Backend origin of this build. */
export const API_URL = resolveApiUrl(process.env.EXPO_PUBLIC_API_URL);
