import { n as preloadSchemas } from "./src-D_zzAWoS.mjs";
import { t as CfWorkerJsonSchemaValidator } from "./cfWorkerProvider-BylNJ1R5.mjs";

//#region src/shimsWorkerd.ts
/**
* Cloudflare Workers runtime shims for client package
*
* This file is selected via package.json export conditions when running in workerd.
*/
preloadSchemas();
/**
* Whether `fetch()` may throw `TypeError` due to CORS. CORS is a browser-only concept —
* in Cloudflare Workers, a `TypeError` from `fetch` is always a real network/configuration
* error, never a CORS error.
*/
const CORS_IS_POSSIBLE = false;

//#endregion
export { CORS_IS_POSSIBLE, CfWorkerJsonSchemaValidator as DefaultJsonSchemaValidator };
//# sourceMappingURL=shimsWorkerd.mjs.map