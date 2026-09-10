const require_cfWorkerProvider = require('./cfWorkerProvider-B3dvjU3F.cjs');

//#region src/shimsBrowser.ts
/**
* Whether `fetch()` may throw `TypeError` due to CORS. Only true in browser contexts
* (including Web Workers / Service Workers). In Node.js and Cloudflare Workers, a
* `TypeError` from `fetch` is always a real network/configuration error.
*/
const CORS_IS_POSSIBLE = true;

//#endregion
exports.CORS_IS_POSSIBLE = CORS_IS_POSSIBLE;
exports.DefaultJsonSchemaValidator = require_cfWorkerProvider.CfWorkerJsonSchemaValidator;
//# sourceMappingURL=shimsBrowser.cjs.map