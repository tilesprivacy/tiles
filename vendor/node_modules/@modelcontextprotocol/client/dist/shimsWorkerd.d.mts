import { t as CfWorkerJsonSchemaValidator } from "./cfWorkerProvider-B3ZJEoRM.mjs";

//#region src/shimsWorkerd.d.ts

/**
 * Whether `fetch()` may throw `TypeError` due to CORS. CORS is a browser-only concept —
 * in Cloudflare Workers, a `TypeError` from `fetch` is always a real network/configuration
 * error, never a CORS error.
 */
declare const CORS_IS_POSSIBLE = false;
//#endregion
export { CORS_IS_POSSIBLE, CfWorkerJsonSchemaValidator as DefaultJsonSchemaValidator };
//# sourceMappingURL=shimsWorkerd.d.mts.map