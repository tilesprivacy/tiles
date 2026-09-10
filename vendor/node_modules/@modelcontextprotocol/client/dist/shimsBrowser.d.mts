import { t as CfWorkerJsonSchemaValidator } from "./cfWorkerProvider-B3ZJEoRM.mjs";

//#region src/shimsBrowser.d.ts

/**
 * Whether `fetch()` may throw `TypeError` due to CORS. Only true in browser contexts
 * (including Web Workers / Service Workers). In Node.js and Cloudflare Workers, a
 * `TypeError` from `fetch` is always a real network/configuration error.
 */
declare const CORS_IS_POSSIBLE = true;
//#endregion
export { CORS_IS_POSSIBLE, CfWorkerJsonSchemaValidator as DefaultJsonSchemaValidator };
//# sourceMappingURL=shimsBrowser.d.mts.map