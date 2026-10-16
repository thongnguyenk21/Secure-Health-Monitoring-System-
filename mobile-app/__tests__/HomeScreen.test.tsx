import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, render, screen } from "@testing-library/react-native";
import { SafeAreaProvider } from "react-native-safe-area-context";
import { clearSession, setSession, setTokenStoreForTesting } from "../src/features/auth/session";
import { createWebTokenStore } from "../src/features/auth/tokenStore";
import { HomeScreen } from "../src/features/home/screens/HomeScreen";
import { alert, DEVICE_ID, fakePair, json, reading } from "./helpers/fixtures";

const METRICS = { frame: { x: 0, y: 0, width: 390, height: 844 }, insets: { top: 0, left: 0, right: 0, bottom: 0 } };

type Route = (url: URL) => Response | undefined;

function mockBackend(routes: Record<string, Route>) {
  const fetchMock = jest.fn(async (input: string) => {
    const url = new URL(input);
    const route = routes[url.pathname];
    const res = route?.(url);
    return res ?? json({ code: "not_found", message: "not found", traceId: "t" }, 404);
  });
  globalThis.fetch = fetchMock as unknown as typeof fetch;
  return fetchMock;
}

let client: QueryClient;

async function renderHome() {
  client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } });
  return await render(
    <SafeAreaProvider initialMetrics={METRICS}>
      <QueryClientProvider client={client}>
        <HomeScreen />
      </QueryClientProvider>
    </SafeAreaProvider>,
  );
}

const me = {
  id: "u-1",
  email: "bn017@iomt.local",
  role: "patient",
  display_name: "Nam, 54",
  subject_code: "BN-017",
  created_at_unix_ms: 1,
};

describe("HomeScreen", () => {
  beforeEach(() => {
    setTokenStoreForTesting(createWebTokenStore(null));
    setSession(fakePair(Date.now() + 15 * 60_000));
  });

  afterEach(async () => {
    await cleanup();
    client.clear();
    clearSession(null);
  });

  it("shows the greeting, status, vitals and footer for a connected watch", async () => {
    const now = Date.now();
    const fetchMock = mockBackend({
      "/api/v1/me": () => json(me),
      "/api/v1/devices": () => json({ items: [{ device_id: DEVICE_ID, kind: "wearable", status: "active" }], next_cursor: null }),
      "/api/v1/measurements/latest": () => json(reading({ ts_unix_ms: now - 2_000, hr_bpm: 92, spo2_pct: 97, activity: "walk" })),
      "/api/v1/measurements": (url) =>
        url.searchParams.get("resolution") === "1h"
          ? json({ buckets: [{ bucket_start_unix_ms: now - 3_600_000, sample_count: 3600, step_count: 4000 }, { bucket_start_unix_ms: now, sample_count: 10, step_count: 812 }], next_cursor: null })
          : json({ items: [reading({ ts_unix_ms: now - 3_000, hr_bpm: 90 }), reading({ ts_unix_ms: now - 2_000, hr_bpm: 92 })], next_cursor: null }),
      "/api/v1/alerts": () => json({ items: [], next_cursor: null }),
    });

    await renderHome();

    expect(await screen.findByText("Xin chào, Nam, 54")).toBeTruthy();
    expect(await screen.findByText("Nhịp tim 92 hợp với lúc đi bộ")).toBeTruthy();
    expect(screen.getByText("Sức khoẻ: bình thường")).toBeTruthy();
    expect(screen.getByText("Đang kết nối")).toBeTruthy();
    expect(screen.getByTestId("hr-value")).toHaveTextContent("92");
    expect(screen.getByTestId("spo2-value")).toHaveTextContent("97 %");
    expect(screen.getByText("Đi bộ")).toBeTruthy();
    expect(await screen.findByText("4.812")).toBeTruthy();
    expect(screen.getByText("Bác sĩ đã nhận đủ số đo mới nhất")).toBeTruthy();

    const auth = fetchMock.mock.calls.map((c) => (c as unknown as [string, RequestInit])[1].headers as Record<string, string>);
    expect(auth.every((h) => h.Authorization?.startsWith("Bearer "))).toBe(true);
  });

  it("puts an open alert in the status card", async () => {
    const now = Date.now();
    mockBackend({
      "/api/v1/me": () => json(me),
      "/api/v1/devices": () => json({ items: [{ device_id: DEVICE_ID, kind: "wearable", status: "active" }], next_cursor: null }),
      "/api/v1/measurements/latest": () => json(reading({ ts_unix_ms: now - 1_000, hr_bpm: 150, activity: "rest" })),
      "/api/v1/measurements": () => json({ items: [], buckets: [], next_cursor: null }),
      "/api/v1/alerts": () => json({ items: [alert({ raised_at_unix_ms: now - 30_000, value: 150 })], next_cursor: null }),
    });

    await renderHome();

    expect(await screen.findByText("Nhịp nhanh khi nghỉ")).toBeTruthy();
    expect(screen.getByText(/^150 nhịp\/phút lúc/)).toBeTruthy();
  });

  it("explains that no watch is paired", async () => {
    mockBackend({
      "/api/v1/me": () => json({ ...me, display_name: "Hoa, 61" }),
      "/api/v1/devices": () => json({ items: [], next_cursor: null }),
      "/api/v1/alerts": () => json({ items: [], next_cursor: null }),
    });

    await renderHome();

    expect(await screen.findByText("Bạn chưa ghép đôi đồng hồ")).toBeTruthy();
    expect(screen.queryByText("Đang kết nối")).toBeNull();
  });
});
