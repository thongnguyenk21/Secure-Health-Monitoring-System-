import { afterEach, describe, expect, it, vi } from "vitest";
import { clearSession, getSession, setSession } from "../auth/session";
import { makePair } from "../test/fixtures";
import { apiRequest } from "./client";
import { SessionExpiredError } from "./http";

function json(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), { status, headers: { "Content-Type": "application/json" } });
}

afterEach(() => {
  clearSession();
  vi.unstubAllGlobals();
});

describe("apiRequest", () => {
  it("refreshes once on 401 and retries with the new token", async () => {
    const first = makePair("clinician");
    const second = { ...makePair("clinician", 20 * 60_000), refresh_token: "refresh-2" };
    setSession(first);
    const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      if (url === "/api/v1/auth/refresh") return json(second);
      const auth = (init?.headers as Record<string, string>).Authorization;
      return auth === `Bearer ${second.access_token}` ? json([]) : json({ code: "unauthorized", message: "x", traceId: "t" }, 401);
    });
    vi.stubGlobal("fetch", fetchMock);

    // Two parallel calls must share one refresh: a reused refresh token revokes the session.
    await Promise.all([apiRequest("/patients"), apiRequest("/alerts")]);
    const refreshCalls = fetchMock.mock.calls.filter(([u]) => String(u) === "/api/v1/auth/refresh");
    expect(refreshCalls).toHaveLength(1);
    expect(getSession()?.pair.refresh_token).toBe("refresh-2");
  });

  it("ends the session when the refresh is refused", async () => {
    setSession(makePair("clinician"));
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => json({ code: "unauthorized", message: "x", traceId: "t" }, 401)),
    );
    await expect(apiRequest("/patients")).rejects.toBeInstanceOf(SessionExpiredError);
    expect(getSession()).toBeNull();
  });
});
