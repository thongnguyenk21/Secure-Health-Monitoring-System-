import type { components, operations } from "./schema";

/** Schemas generated from the OpenAPI contract (`npm run gen:api`). */
type Schemas = components["schemas"];

/** Login and refresh response. */
export type TokenPair = Schemas["TokenPair"];
/** Login request body. */
export type LoginRequest = Schemas["LoginRequest"];
/** Refresh and logout request body. */
export type RefreshRequest = Schemas["RefreshRequest"];
/** Account role. */
export type Role = Schemas["Role"];
/** Common error body of every failed request. */
export type ApiErrorBody = Schemas["Error"];
/** Account fields of the signed-in user. */
export type User = Schemas["User"];
/** Wearable or gateway. */
export type Device = Schemas["Device"];
/** One page of devices. */
export type DevicePage = Schemas["DevicePage"];
/** One raw summary-vitals row. */
export type Measurement = Schemas["Measurement"];
/** One rollup bucket of vitals. */
export type MeasurementBucket = Schemas["MeasurementBucket"];
/** One page of measurements (raw items or rollup buckets). */
export type MeasurementPage = Schemas["MeasurementPage"];
/** Raw rows or a rollup bucket size. */
export type Resolution = Schemas["Resolution"];
/** Activity class of a measurement. */
export type Activity = Schemas["Activity"];
/** Clinical alert. */
export type Alert = Schemas["Alert"];
/** One page of alerts. */
export type AlertPage = Schemas["AlertPage"];
/** Alert type. */
export type AlertType = Schemas["AlertType"];
/** Alert severity. */
export type Severity = Schemas["Severity"];
/** Alert lifecycle state. */
export type AlertStatus = Schemas["AlertStatus"];
/** One consent record. */
export type Consent = Schemas["Consent"];
/** Body of a new consent record. */
export type ConsentCreate = Schemas["ConsentCreate"];
/** Purpose a consent record covers. */
export type ConsentScope = Schemas["ConsentScope"];

/** Query parameters of `GET /api/v1/measurements`. */
export type MeasurementQuery = operations["listMeasurements"]["parameters"]["query"];
/** Query parameters of `GET /api/v1/alerts`. */
export type AlertQuery = NonNullable<operations["listAlerts"]["parameters"]["query"]>;
