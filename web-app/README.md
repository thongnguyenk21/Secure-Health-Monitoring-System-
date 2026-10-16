# web-app

Clinician and admin web app of the Secure Health Monitoring IoMT system.
Vite, React 18, TypeScript (strict), React Router 6 and TanStack Query 5. Charts
are plain SVG and the styles are one hand-written stylesheet; there is no UI kit.

Patients do not use this app. They use the mobile app (`mobile-app/`). A patient
who logs in here sees a short notice and a logout button.

## Run

```bash
npm install
npm run gen:api      # regenerate src/api/schema.d.ts from the OpenAPI contract
npm run dev          # http://localhost:5173, backend expected on :8080
```

The dev server proxies `/api` (including the `/api/v1/live` WebSocket) to
`http://localhost:8080`. To use another backend, set `BACKEND_URL`:

```bash
BACKEND_URL=http://localhost:8099 npm run dev
```

## Scripts

| Script | What it does |
| --- | --- |
| `dev` | Vite dev server on port 5173 with the `/api` proxy (`ws: true`). |
| `build` | `tsc --noEmit && vite build`; output in `dist/`. |
| `lint` | ESLint flat config: typescript-eslint strict, `eslint-plugin-tsdoc`, `eslint-plugin-jsdoc` (TSDoc on every export), react-hooks, and a rule that rejects literal text in JSX. |
| `test` | Vitest with jsdom and Testing Library. |
| `gen:api` | `openapi-typescript ../shared/protocols/openapi/cloud-api.openapi.yaml -o src/api/schema.d.ts`. |

## Layout

```
src/
  main.tsx              entry: restores the session, mounts <App>
  App.tsx               query client and route table
  styles.css            design tokens and every style
  api/
    schema.d.ts         generated from the contract (do not edit)
    types.ts            aliases of the generated schemas
    http.ts             URL builder, ApiError, SessionExpiredError, public POST
    client.ts           authenticated fetch (refresh and one retry on 401) and one function per route
    queries.ts          TanStack Query hooks and query keys
  auth/
    token.ts            JWT payload decoding and refresh timing
    session.ts          token store (memory + sessionStorage), refresh timer, single-flight refresh
    useSession.ts       React binding of the store
    RequireRole.tsx     route guard and role home paths
  live/useLiveAlerts.ts WebSocket to /api/v1/live with backoff and a fresh token per reconnect
  i18n/vi.ts            every user-visible (Vietnamese) string
  lib/                  formatting, status labels and colours, useNow
  components/           shell and navigation, pills and dots, toasts, logout
  pages/
    LoginPage.tsx, PatientRolePage.tsx
    clinician/          layout, patient list and detail, 24-hour chart, open alerts
    admin/              layout, overview tiles, forms, full pages
  test/                 Vitest setup and fixtures
```

## Routes

| Path | Role | Screen |
| --- | --- | --- |
| `/dang-nhap` | everyone | Login |
| `/benh-nhan`, `/benh-nhan/:userId` | clinician | Patient list and the selected patient |
| `/canh-bao` | clinician | Every open alert |
| `/quan-tri` | admin | Overview dashboard |
| `/quan-tri/thiet-bi`, `/nguoi-dung`, `/phat-hanh`, `/federated-learning`, `/audit-log` | admin | Full pages |
| `/chi-danh-cho-nhan-vien` | patient | Notice: use the mobile app |

## Sessions

- Tokens are kept in memory and in `sessionStorage` (never `localStorage`), so
  they are dropped when the tab closes.
- The access token is refreshed 60 s before it expires. A 401 triggers one
  refresh and one retry. If the refresh is refused, the session ends and the
  user returns to the login page.
- Refreshes are single-flight, because reusing an old refresh token makes the
  backend revoke the whole session.
- The cached query data is cleared when the session ends.

## Design source

The screens follow the wireframes on slides 26 and 27 of
`docs/reports/system-design-slides.html` ("Web app cho bác sĩ" and "Web app cho
quản trị viên"). `src/styles.css` reuses their colour tokens, fonts (Be Vietnam
Pro, Source Sans 3, IBM Plex Mono) and class names (`.bnav`, `.bcol`, `.ptab`,
`.tile`, `.pill`, `.dot`, `.alog`, `.cap`, `.num`, `.rel`), scaled up from the
1280x720 slide to a laptop screen of at least 1280 px.
