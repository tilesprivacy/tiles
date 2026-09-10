const require_ajvProvider = require('./ajvProvider-1b6pVpkg.cjs');

//#region src/shimsNode.ts
/**
* Whether `fetch()` may throw `TypeError` due to CORS. CORS is a browser-only concept —
* in Node.js, a `TypeError` from `fetch` is always a real network/configuration error
* (DNS resolution, connection refused, invalid URL), never a CORS error.
*/
const CORS_IS_POSSIBLE = false;

//#endregion
exports.CORS_IS_POSSIBLE = CORS_IS_POSSIBLE;
exports.DefaultJsonSchemaValidator = require_ajvProvider.AjvJsonSchemaValidator;
//# sourceMappingURL=shimsNode.cjs.map