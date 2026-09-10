import { t as CfWorkerJsonSchemaValidator } from "./cfWorkerProvider-BylNJ1R5.mjs";

//#region src/shimsBrowser.ts
/**
* Whether `fetch()` may throw `TypeError` due to CORS. Only true in browser contexts
* (including Web Workers / Service Workers). In Node.js and Cloudflare Workers, a
* `TypeError` from `fetch` is always a real network/configuration error.
*/
const CORS_IS_POSSIBLE = true;

//#endregion
export { CORS_IS_POSSIBLE, CfWorkerJsonSchemaValidator as DefaultJsonSchemaValidator };
//# sourceMappingURL=shimsBrowser.mjs.map