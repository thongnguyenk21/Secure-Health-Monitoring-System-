import { ApiError } from "../src/api/http";
import {
  clearSession,
  freshAccessToken,
  getSessionState,
  refreshSession,
  setSession,
  setTokenStoreForTesting,
} from "../src/features/auth/session";
import { decodeAccessToken, needsRefresh, refreshDelayMs, REFRESH_LEAD_MS, TokenFormatError } from "../src/features/auth/token";
import { createWebTokenStore } from "../src/features/auth/tokenStore";
import { fakeJwt, fakePair, json } from "./helpers/fixtures";

describe("decodeAccessToken", () => {
  it("reads sub, role and exp", () => {
    const token = fakeJwt({ sub: "a8b7", role: "patient", exp: 1_800_000_000 });
    expect(decodeAccessToken(token)).toEqual({ sub: "a8b7", role: "patient", exp: 1_800_000_000 });
  });

  it("decodes UTF-8 claims", () => {
    const token = fakeJwt({ sub: "Nam, 54 – bệnh nhân", role: "clinician", exp: 1 });
    expect(decodeAccessToken(token).sub).toBe("Nam, 54 – bệnh nhân");
  });

  it.each([
    ["not a jwt", "abc"],
    ["bad payload", "a.b-not-json.c"],
    ["unknown role", fakeJwt({ sub: "x", role: "researcher", exp: 1 })],
    ["missing exp", fakeJwt({ sub: "x", role: "patient" })],
  ])("rejects %s", (_name, token) => {
    expect(() => decodeAccessToken(token)).toThrow(TokenFormatError);
  });
});

describe("refresh timing", () => {
  it("waits until one lead time before expiry", () => {
    expect(refreshDelayMs(1_000_000, 100_000)).toBe(1_000_000 - REFRESH_LEAD_MS - 100_000);
  });

  it("never returns a negative delay", () => {
    expect(refreshDelayMs(1_000, 5_000)).toBe(0);
  });

  it("asks for a refresh inside the lead window only", () => {
    const exp = 10 * 60_000;
    expect(needsRefresh(exp, exp - REFRESH_LEAD_MS - 1)).toBe(false);
    expect(needsRefresh(exp, exp - REFRESH_LEAD_MS)).toBe(true);
  });
});

describe("session refresh", () => {
  const fetchMock = jest.fn<Promise<Response>, [string, RequestInit]>();

  beforeEach(() => {
    jest.useFakeTimers({ now: 1_000_000_000_000 });
    setTokenStoreForTesting(createWebTokenStore(null));
    globalThis.fetch = fetchMock as unknown as typeof fetch;
    fetchMock.mockReset();
  });

  afterEach(() => {
    clearSession(null);
    jest.useRealTimers();
  });

  it("shares one refresh request between concurrent callers", async () => {
    setSession(fakePair(Date.now() + 30_000));
    fetchMock.mockResolvedValue(json(fakePair(Date.now() + 15 * 60_000, "patient", "u-1", "r-2")));
    const [a, b] = await Promise.all([freshAccessToken(), freshAccessToken()]);
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(a).toBe(b);
    expect(getSessionState().session?.pair.refresh_token).toBe("r-2");
  });

  it("uses the stored token while it is far from expiry", async () => {
    const pair = fakePair(Date.now() + 15 * 60_000);
    setSession(pair);
    await expect(freshAccessToken()).resolves.toBe(pair.access_token);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("ends the session when the refresh token was revoked", async () => {
    setSession(fakePair(Date.now() + 30_000));
    fetchMock.mockResolvedValue(json({ code: "token_reused", message: "revoked", traceId: "t" }, 401));
    await expect(refreshSession()).rejects.toThrow("refresh refused");
    expect(getSessionState()).toMatchObject({ session: null, endReason: "expired" });
  });

  it("keeps the session on a server error", async () => {
    setSession(fakePair(Date.now() + 30_000));
    fetchMock.mockResolvedValue(json({ code: "internal", message: "boom", traceId: "t" }, 503));
    await expect(refreshSession()).rejects.toBeInstanceOf(ApiError);
    expect(getSessionState().session).not.toBeNull();
  });
});
