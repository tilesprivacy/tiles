import { n as AjvJsonSchemaValidator } from "./ajvProvider-INpkxUi1.mjs";

//#region src/shimsNode.d.ts

/**
 * Whether `fetch()` may throw `TypeError` due to CORS. CORS is a browser-only concept —
 * in Node.js, a `TypeError` from `fetch` is always a real network/configuration error
 * (DNS resolution, connection refused, invalid URL), never a CORS error.
 */
declare const CORS_IS_POSSIBLE = false;
//#endregion
export { CORS_IS_POSSIBLE, AjvJsonSchemaValidator as DefaultJsonSchemaValidator };
//# sourceMappingURL=shimsNode.d.mts.map