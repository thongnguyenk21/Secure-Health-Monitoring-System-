import type { components } from "./schema";

/** Schemas generated from the OpenAPI contract (`npm run gen:api`). */
type Schemas = components["schemas"];

/** Login and refresh response. */
export type TokenPair = Schemas["TokenPair"];
/** Account role. */
export type Role = Schemas["Role"];
/** Common error body of every failed request. */
export type ApiErrorBody = Schemas["Error"];
/** Clinician view of one patient. */
export type PatientSummary = Schemas["PatientSummary"];
/** Priority level of a patient. */
export type PatientStatus = Schemas["PatientStatus"];
/** One raw summary-vitals row. */
export type Measurement = Schemas["Measurement"];
/** One rollup bucket of vitals. */
export type MeasurementBucket = Schemas["MeasurementBucket"];
/** One page of measurements (raw items or rollup buckets). */
export type MeasurementPage = Schemas["MeasurementPage"];
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
/** Clinical note about a patient. */
export type ClinicalNote = Schemas["ClinicalNote"];
/** Body of a new clinical note. */
export type ClinicalNoteCreate = Schemas["ClinicalNoteCreate"];
/** Wearable or gateway. */
export type Device = Schemas["Device"];
/** One page of devices. */
export type DevicePage = Schemas["DevicePage"];
/** Body of a device registration. */
export type DeviceCreate = Schemas["DeviceCreate"];
/** Body of a label change. */
export type DevicePatch = Schemas["DevicePatch"];
/** Body of a device assignment. */
export type DeviceAssign = Schemas["DeviceAssign"];
/** Account fields of a user. */
export type User = Schemas["User"];
/** One page of users. */
export type UserPage = Schemas["UserPage"];
/** Body of a user registration. */
export type RegisterUser = Schemas["RegisterUser"];
/** Signed model version. */
export type Model = Schemas["Model"];
/** One audit log entry. */
export type AuditEntry = Schemas["AuditEntry"];
/** One page of audit entries. */
export type AuditPage = Schemas["AuditPage"];
/** State of one system component. */
export type ComponentStatus = Schemas["ComponentStatus"];
/** One frame of the live alert WebSocket. */
export type LiveEvent = Schemas["LiveEvent"];
