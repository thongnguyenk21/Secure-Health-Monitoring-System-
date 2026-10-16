import { ApiError, NetworkError } from "../../api/http";
import { vi } from "../../i18n/vi";

/**
 * Vietnamese message for a failed login.
 *
 * @param err - Error thrown by the login call.
 * @returns The message to show under the form.
 */
export function loginErrorMessage(err: unknown): string {
  if (err instanceof NetworkError) return vi.login.offline;
  if (err instanceof ApiError) {
    if (err.status === 429) return vi.login.tooMany;
    if (err.status === 401 || err.status === 400) return vi.login.wrongCredentials;
  }
  return vi.login.failed;
}
