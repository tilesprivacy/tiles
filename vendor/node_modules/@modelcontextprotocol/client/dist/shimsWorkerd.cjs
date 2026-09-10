const require_src = require('./src-NAgB4Mp8.cjs');
const require_cfWorkerProvider = require('./cfWorkerProvider-B3dvjU3F.cjs');

//#region src/shimsWorkerd.ts
/**
* Cloudflare Workers runtime shims for client package
*
* This file is selected via package.json export conditions when running in workerd.
*/
require_src.preloadSchemas();
/**
* Whether `fetch()` may throw `TypeError` due to CORS. CORS is a browser-only concept —
* in Cloudflare Workers, a `TypeError` from `fetch` is always a real network/configuration
* error, never a CORS error.
*/
const CORS_IS_POSSIBLE = false;

//#endregion
exports.CORS_IS_POSSIBLE = CORS_IS_POSSIBLE;
exports.DefaultJsonSchemaValidator = require_cfWorkerProvider.CfWorkerJsonSchemaValidator;
//# sourceMappingURL=shimsWorkerd.cjs.map