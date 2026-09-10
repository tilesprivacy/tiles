import { $ as legacyProtocolVersions, A as isInitializeRequest, At as SdkErrorCode, B as DiscoverResultSchema, C as HEADER_MISMATCH_ERROR_CODE, Ct as OAuthClientInformationFullSchema, D as assertCompleteRequestPrompt, Dt as OAuthTokensSchema, E as scanXMcpHeaderDeclarations, Et as OAuthProtectedResourceMetadataSchema, F as isJSONRPCRequest, Ft as stampErrorBrands, G as MissingRequiredClientCapabilityError, H as ListChangedOptionsBaseSchema, I as isJSONRPCResponse, J as UnsupportedProtocolVersionError, K as ProtocolError, L as isJSONRPCResultResponse, M as isInputRequiredResult, Mt as OAuthError, N as isJSONRPCErrorResponse, Nt as OAuthErrorCode, O as assertCompleteRequestResourceTemplate, Ot as OpenIdProviderDiscoveryMetadataSchema, P as isJSONRPCNotification, Pt as brandedHasInstance, Q as isModernProtocolVersion, R as isTaskAugmentedRequestParams, S as parseSchema, St as IdJagTokenExchangeResponseSchema, T as encodeMcpParamValue, Tt as OAuthMetadataSchema, U as MODERN_WIRE_REVISION, V as JSONRPCMessageSchema, W as codecForVersion, X as ProtocolErrorCode, Y as UrlElicitationRequiredError, Z as SUPPORTED_MODERN_PROTOCOL_VERSIONS, _ as mergeCapabilities, _t as SUPPORTED_PROTOCOL_VERSIONS, a as createFetchWithInit, at as INTERNAL_ERROR, b as resolveInputRequiredDriverConfig, bt as checkResourceAllowed, c as STDIO_DEFAULT_MAX_BUFFER_SIZE, ct as JSONRPC_VERSION, d as getDisplayName, dt as METHOD_NOT_FOUND, et as modernProtocolVersions, f as isJsonContentType, ft as PARSE_ERROR, g as Protocol, gt as SUBSCRIPTION_ID_META_KEY, h as DEFAULT_REQUEST_TIMEOUT_MSEC, ht as SERVER_INFO_META_KEY, i as UriTemplate, it as DEFAULT_NEGOTIATED_PROTOCOL_VERSION, j as isInitializedNotification, jt as SdkHttpError, k as isCallToolResult, kt as SdkError, l as deserializeMessage, lt as LATEST_PROTOCOL_VERSION, m as runInputRequiredFlow, mt as RELATED_TASK_META_KEY, n as preloadSchemas, nt as CLIENT_CAPABILITIES_META_KEY, o as normalizeHeaders, ot as INVALID_PARAMS, p as mediaTypeEssence, pt as PROTOCOL_VERSION_META_KEY, q as ResourceNotFoundError, r as InMemoryTransport, rt as CLIENT_INFO_META_KEY, s as ReadBuffer, st as INVALID_REQUEST, t as fromJsonSchema$1, tt as BAGGAGE_META_KEY, u as serializeMessage, ut as LOG_LEVEL_META_KEY, v as isSpecType, vt as TRACEPARENT_META_KEY, w as buildMcpParamHeaders, wt as OAuthErrorResponseSchema, x as withInputRequired, xt as resourceUrlFromServerUrl, y as specTypeSchemas, yt as TRACESTATE_META_KEY, z as parseJSONRPCMessage } from "./src-D_zzAWoS.mjs";
import { CORS_IS_POSSIBLE, DefaultJsonSchemaValidator } from "@modelcontextprotocol/client/_shims";
import pkceChallenge from "pkce-challenge";
import { EventSource } from "eventsource";
import { EventSourceParserStream } from "eventsource-parser/stream";

//#region src/client/authErrors.ts
/**
* Base class for the OAuth-client-flow error family. Concrete subclasses are
* added to this module alongside the SEP-2468/837/2207/2350/2352 behavior
* changes that throw them, so callers can catch the whole family with a single
* `instanceof OAuthClientFlowError` guard once those land.
*
* @remarks Nothing in the SDK throws this base class directly. In the release
* that introduces it no subclass exists yet — the guard is a forward-compat
* hook and will not match anything until the first behavior change ships.
*/
var OAuthClientFlowError = class extends Error {
	static {
		Object.defineProperty(this, "mcpBrand", { value: "mcp.OAuthClientFlowError" });
	}
	static [Symbol.hasInstance](value) {
		return brandedHasInstance(this, value);
	}
	/**
	* Brand-based type guard: equivalent to `value instanceof this`, as an
	* explicit static predicate (the axios/AWS-SDK `isInstance` style). Reads
	* the caller's own brand via `this`, so every branded subclass gets a
	* correctly-scoped guard by inheritance. Must be invoked on the class —
	* in callback position write `v => SdkError.isInstance(v)`, not
	* `.filter(SdkError.isInstance)` (detached calls throw rather than
	* silently matching nothing).
	*/
	static isInstance(value) {
		if (typeof this !== "function") throw new TypeError("isInstance must be called on the class (e.g. `SdkError.isInstance(value)`); for callbacks use `v => SdkError.isInstance(v)`");
		return brandedHasInstance(this, value);
	}
	constructor(message) {
		super(message);
		this.name = new.target.name;
		stampErrorBrands(this, new.target);
	}
};
/**
* Thrown when an authorization-server issuer identifier fails validation.
*
* Two checks raise this error, distinguished by {@linkcode IssuerMismatchError.kind | kind}:
* - `'metadata'` — the `issuer` in fetched authorization-server metadata does
*   not match the issuer identifier the well-known URL was constructed from
*   (RFC 8414 §3.3 / OpenID Connect Discovery §4.3).
* - `'authorization_response'` — the `iss` parameter on the authorization
*   callback failed RFC 9207 §2.4 validation against the recorded issuer.
*
* Intentionally does **not** extend `OAuthError`: the `auth()`
* orchestrator's `OAuthError` retry block must not swallow this — a mix-up
* indication is fatal for the flow, not a retryable credential problem.
*
* On the `'authorization_response'` path the {@linkcode IssuerMismatchError.received | received}
* value is attacker-controllable in a mix-up attack; callers **MUST NOT** display
* it (or any `error`/`error_description`/`error_uri` from the same callback) to
* end users. The values are JSON-encoded in the message to neutralize log-injection.
*/
var IssuerMismatchError = class extends OAuthClientFlowError {
	static {
		Object.defineProperty(this, "mcpBrand", { value: "mcp.IssuerMismatchError" });
	}
	/** Which check failed — metadata echo (RFC 8414 §3.3) or authorization-response `iss` (RFC 9207). */
	kind;
	/** The issuer the client expected (from validated metadata / discovery input). */
	expected;
	/** The issuer value that was received. Attacker-controllable on the `'authorization_response'` path. */
	received;
	constructor(kind, expected, received) {
		super(`Issuer mismatch in ${kind === "metadata" ? "authorization server metadata (RFC 8414 §3.3)" : "authorization response (RFC 9207)"}: expected ${JSON.stringify(expected)}, received ${JSON.stringify(received)}`);
		this.kind = kind;
		this.expected = expected;
		this.received = received;
	}
};
/**
* Thrown by `registerClient()` when the authorization server rejects a
* Dynamic Client Registration request. Carries the HTTP status, the raw
* response body, and the metadata that was submitted, so callers can inspect
* the AS's `error` / `error_description` and retry with adjusted metadata
* (for example a different `application_type`) per SEP-837.
*
* The `body` is the raw RFC 7591 error JSON; compare `JSON.parse(body).error`
* against `OAuthErrorCode` (e.g. `OAuthErrorCode.InvalidRedirectUri`,
* `OAuthErrorCode.InvalidClientMetadata`).
*
* Intentionally does **not** extend `OAuthError`: registration rejection is
* not a recoverable-by-credential-invalidation condition, and staying outside
* that hierarchy keeps it from being caught by `auth()`'s `OAuthError` retry
* path.
*/
var RegistrationRejectedError = class extends OAuthClientFlowError {
	static {
		Object.defineProperty(this, "mcpBrand", { value: "mcp.RegistrationRejectedError" });
	}
	/** HTTP status code returned by the registration endpoint. */
	status;
	/** Raw response body text (typically an RFC 7591 error JSON document). */
	body;
	/** The exact client metadata that was POSTed (after SDK defaults were applied). */
	submittedMetadata;
	constructor(args) {
		super(`Dynamic Client Registration rejected (HTTP ${args.status}): ${args.body}`);
		this.status = args.status;
		this.body = args.body;
		this.submittedMetadata = args.submittedMetadata;
	}
};
/**
* Thrown by the token-exchange and refresh paths when the resolved token
* endpoint is not `https:` and is not a loopback host (SEP-2207). This is a
* configuration error — re-authorizing cannot fix it — so it intentionally does
* **not** extend `OAuthError` and `auth()`'s refresh branch rethrows it instead
* of falling through to a fresh `/authorize` redirect.
*/
var InsecureTokenEndpointError = class extends OAuthClientFlowError {
	static {
		Object.defineProperty(this, "mcpBrand", { value: "mcp.InsecureTokenEndpointError" });
	}
	/** The token endpoint URL that was rejected. */
	tokenEndpoint;
	constructor(tokenEndpoint) {
		super(`Refusing to send credentials to non-https token endpoint '${tokenEndpoint}'. OAuth token requests MUST use TLS (localhost / 127.0.0.1 / ::1 are exempt).`);
		this.tokenEndpoint = tokenEndpoint;
	}
};
/**
* Thrown by the HTTP client transport when the server responds with
* `403 Forbidden` and `WWW-Authenticate: Bearer error="insufficient_scope"`,
* and either (a) the transport's `onInsufficientScope` option is `'throw'`, or
* (b) `onInsufficientScope` is the default `'reauthorize'` but the transport
* has no {@linkcode index.OAuthClientProvider | OAuthClientProvider} to drive
* step-up (e.g. a minimal `AuthProvider`, `requestInit`-only headers, or no
* `authProvider`).
*
* Carries the challenge parameters so the host can decide whether to initiate
* step-up authorization itself (e.g., behind a UX gate) or surface the error.
*
* Does **not** extend `OAuthError`: that class represents OAuth protocol errors
* from the authorization server; this is a resource-server challenge surfaced
* at the transport layer.
*
* All fields originate from the resource server's `WWW-Authenticate` header;
* treat them as untrusted input when displaying or logging (this includes
* `requiredScope`, which appears in the error message).
*/
/**
* Thrown by `auth()` on the authorization-code callback leg when the
* authorization server resolved by discovery differs from the one recorded in
* `discoveryState()` at redirect time. The `authorization_code` and PKCE
* `code_verifier` are bound to the AS that minted the code (RFC 7636); sending
* them to a different AS's token endpoint is a credential-exfiltration vector.
*
* This is the only runtime check left in the SEP-2352 model — stored tokens and
* client credentials are protected structurally by the `issuer` stamp instead.
*/
var AuthorizationServerMismatchError = class extends OAuthClientFlowError {
	static {
		Object.defineProperty(this, "mcpBrand", { value: "mcp.AuthorizationServerMismatchError" });
	}
	constructor(recordedIssuer, currentIssuer) {
		super(`Authorization server changed between redirect and callback (redirected to ${JSON.stringify(recordedIssuer)}, callback resolved ${JSON.stringify(currentIssuer)}); refusing to send authorization_code/code_verifier to a different token endpoint`);
		this.recordedIssuer = recordedIssuer;
		this.currentIssuer = currentIssuer;
	}
};
var InsufficientScopeError = class extends OAuthClientFlowError {
	static {
		Object.defineProperty(this, "mcpBrand", { value: "mcp.InsufficientScopeError" });
	}
	/** The `scope` value from the `WWW-Authenticate` challenge — the scopes the resource server says are required. */
	requiredScope;
	/** The `resource_metadata` URL from the `WWW-Authenticate` challenge, if present. */
	resourceMetadataUrl;
	/** The `error_description` from the `WWW-Authenticate` challenge, if present. */
	errorDescription;
	constructor(init) {
		super(`Insufficient scope${init.requiredScope ? `: required "${init.requiredScope}"` : ""}`);
		this.requiredScope = init.requiredScope;
		this.resourceMetadataUrl = init.resourceMetadataUrl;
		this.errorDescription = init.errorDescription;
	}
};

//#endregion
//#region src/client/auth.ts
/**
* SEP-2352 stamp check: returns `stored` only when its `issuer` stamp matches the
* resolved authorization server. A stamp that names a *different* issuer reads back
* as `undefined`, so a credential issued by one authorization server is never reused
* at another — the flow falls through to re-registration / re-authorization exactly
* as if nothing were stored. An unstamped value (legacy provider or pre-SEP-2352
* storage) is returned as-is with a `console.warn`; {@linkcode auth} writes the
* stamp back on first use so the window closes after one call.
*
* {@linkcode auth} stamps every value it writes via `saveTokens` / `saveClientInformation`,
* so a provider that round-trips the stored object verbatim is protected with no extra
* code. Providers that hold credentials for multiple authorization servers key their
* storage on `ctx.issuer` instead.
*
* @param opts.canPersistStamp - When `false`, suppresses the unstamped-credential
*   warning: the caller cannot back-stamp (no `saveClientInformation`), so the
*   "binding on first use" claim would be false and would fire on every call.
*/
function discardIfIssuerMismatch(stored, issuer, opts) {
	if (stored === void 0) return void 0;
	if (stored.issuer === void 0) {
		if (opts?.canPersistStamp !== false) console.warn("[mcp-sdk] SEP-2352: stored OAuth credential has no 'issuer' stamp (pre-upgrade storage or provider not round-tripping the value). SEP-2352 isolation is inactive for this read; ensure your provider round-trips the issuer field.");
		return stored;
	}
	return issuersMatch(stored.issuer, issuer) ? stored : void 0;
}
/**
* SEP-2352 issuer-identity comparison. Tolerates a single trailing `/` difference,
* mirroring the RFC 8414 §3.3 "one narrow tolerance" applied at metadata-echo
* validation in {@linkcode discoverAuthorizationServerMetadata}: when the SDK
* derives an issuer from `String(new URL(...))` (always slash-suffixed) and the AS
* publishes a slash-free `metadata.issuer`, the two name the same authorization
* server.
*/
function issuersMatch(a, b) {
	return a === b || a.endsWith("/") && a.slice(0, -1) === b || b.endsWith("/") && b.slice(0, -1) === a;
}
/**
* Type guard distinguishing `OAuthClientProvider` from a minimal `AuthProvider`.
* Transports use this at construction time to classify the `authProvider` option.
*
* Checks for `tokens()` + `clientInformation()` — two required `OAuthClientProvider`
* methods that a minimal `AuthProvider` `{ token: ... }` would never have.
*/
function isOAuthClientProvider(provider) {
	if (provider == null) return false;
	const p = provider;
	return typeof p.tokens === "function" && typeof p.clientInformation === "function";
}
/**
* Standard `onUnauthorized` behavior for OAuth providers: extracts
* `WWW-Authenticate` parameters from the 401 response and runs {@linkcode auth}.
* Used by {@linkcode adaptOAuthProvider} to bridge `OAuthClientProvider` to `AuthProvider`.
*/
async function handleOAuthUnauthorized(provider, ctx, extraAuthOptions) {
	const { resourceMetadataUrl, scope } = extractWWWAuthenticateParams(ctx.response);
	if (await auth(provider, {
		serverUrl: ctx.serverUrl,
		resourceMetadataUrl,
		scope,
		fetchFn: ctx.fetchFn,
		...extraAuthOptions
	}) !== "AUTHORIZED") throw new UnauthorizedError();
}
/**
* Adapts an `OAuthClientProvider` to the minimal `AuthProvider` interface that
* transports consume. Called once at transport construction — the transport stores
* the adapted provider for `_commonHeaders()` and 401 handling, while keeping the
* original `OAuthClientProvider` for OAuth-specific paths (`finishAuth()`, 403 `insufficient_scope` step-up).
*
* SEP-2352 note: `token()` here is the per-request `Authorization: Bearer …` read for
* the *resource server* (the MCP transport URL), not an authorization server. No OAuth
* discovery has run at this layer, so there is no `issuer` to pass as `ctx` and no
* {@linkcode discardIfIssuerMismatch} check to apply — the access token is sent only to
* the resource server, never to an AS, so the SEP-2352 cross-AS isolation invariant is
* not in scope. Providers that key storage on `ctx.issuer` MUST treat `ctx === undefined`
* as "return the most-recently-saved token set" (the only consumer is the resource server
* the token was minted for); providers that round-trip a single blob need no change.
*/
function adaptOAuthProvider(provider, extraAuthOptions) {
	return {
		token: async () => {
			return (await provider.tokens())?.access_token;
		},
		onUnauthorized: async (ctx) => handleOAuthUnauthorized(provider, ctx, extraAuthOptions)
	};
}
var UnauthorizedError = class extends Error {
	static {
		Object.defineProperty(this, "mcpBrand", { value: "mcp.UnauthorizedError" });
	}
	static [Symbol.hasInstance](value) {
		return brandedHasInstance(this, value);
	}
	/**
	* Brand-based type guard: equivalent to `value instanceof this`, as an
	* explicit static predicate (the axios/AWS-SDK `isInstance` style). Reads
	* the caller's own brand via `this`, so every branded subclass gets a
	* correctly-scoped guard by inheritance. Must be invoked on the class —
	* in callback position write `v => SdkError.isInstance(v)`, not
	* `.filter(SdkError.isInstance)` (detached calls throw rather than
	* silently matching nothing).
	*/
	static isInstance(value) {
		if (typeof this !== "function") throw new TypeError("isInstance must be called on the class (e.g. `SdkError.isInstance(value)`); for callbacks use `v => SdkError.isInstance(v)`");
		return brandedHasInstance(this, value);
	}
	constructor(message) {
		super(message ?? "Unauthorized");
		this.name = "UnauthorizedError";
		stampErrorBrands(this, new.target);
	}
};
/**
* Validates the `iss` parameter from an authorization response against the
* issuer recorded from the authorization server's validated metadata, per
* RFC 9207 §2.4 and the MCP specification's four-row decision table.
*
* | `issParameterSupported` | `iss`   | Action                                           |
* | ----------------------- | ------- | ------------------------------------------------ |
* | `true`                  | present | compare; throw {@linkcode IssuerMismatchError} on mismatch |
* | `true`                  | absent  | throw {@linkcode IssuerMismatchError}            |
* | `false`                 | present | compare; throw {@linkcode IssuerMismatchError} on mismatch |
* | `false`                 | absent  | proceed (no-op)                                  |
*
* Comparison is **simple string equality** (RFC 3986 §6.2.1). Scheme/host case
* folding, default-port elision, trailing-slash, and percent-encoding
* normalization are explicitly **not** applied — any difference is a mismatch.
*
* When `expectedIssuer` is `undefined` (no validated metadata document exists),
* the check has no authentic baseline and degenerates to a no-op.
*
* @throws {IssuerMismatchError} with `kind: 'authorization_response'`
*/
/**
* Reads RFC 9207's `authorization_response_iss_parameter_supported` from
* authorization-server metadata. Only a literal `true` counts as advertised;
* absent, `false`, or a non-boolean wire value (coerced to `undefined` by the
* schema) means not advertised.
*/
function isIssParameterSupported(metadata) {
	return metadata?.authorization_response_iss_parameter_supported === true;
}
function validateAuthorizationResponseIssuer({ iss, expectedIssuer, issParameterSupported }) {
	if (expectedIssuer === void 0) return;
	if (iss === void 0) {
		if (issParameterSupported) throw new IssuerMismatchError("authorization_response", expectedIssuer, void 0);
		return;
	}
	if (iss !== expectedIssuer) throw new IssuerMismatchError("authorization_response", expectedIssuer, iss);
}
/**
* Computes the union of one or more OAuth `scope` strings.
*
* Each argument is a space-delimited scope string per RFC 6749 §3.3, or
* `undefined`. The result is a single space-delimited string containing each
* distinct scope token exactly once, in first-seen order, or `undefined` if
* every input is empty/undefined.
*
* No hierarchical deduplication is performed: a union may contain semantically
* redundant entries (e.g., a broad scope alongside a narrower one it implies).
* Authorization servers normalize such redundancy during token issuance; the
* spec's step-up flow does not require clients to.
*
* Used by the transport's `403 insufficient_scope` step-up path to accumulate
* previously-requested scopes with newly-challenged scopes so re-authorization
* does not lose previously-granted permissions.
*/
function computeScopeUnion(...scopes) {
	const seen = /* @__PURE__ */ new Set();
	for (const scope of scopes) {
		if (!scope) continue;
		for (const token of scope.split(/\s+/)) if (token) seen.add(token);
	}
	return seen.size > 0 ? [...seen].join(" ") : void 0;
}
/**
* Whether `union` contains at least one scope token not present in `current`.
* Both arguments are space-delimited scope strings per RFC 6749 §3.3.
*
* Used to gate the step-up refresh bypass: when the union of previously-requested
* and newly-challenged scopes is a strict superset of the current token's
* granted scope, refreshing cannot widen the grant (RFC 6749 §6), so the
* transport must force a fresh authorization request instead. When the current
* token already covers the union, refresh remains valid.
*
* An undefined or empty `current` is treated as the empty set, so any non-empty
* `union` is a strict superset. Note that per RFC 6749 §3.3 an authorization
* server MAY omit the token's `scope` field when it equals the requested scope;
* this helper is conservative and treats an absent token `scope` as empty, so
* step-up always forces a fresh authorization request in that case rather than
* risking a refresh that silently drops the widened scope.
*/
function isStrictScopeSuperset(union, current) {
	if (!union) return false;
	const currentSet = new Set((current ?? "").split(/\s+/).filter(Boolean));
	for (const token of union.split(/\s+/)) if (token && !currentSet.has(token)) return true;
	return false;
}
/**
* Shared `finishAuth` resolver for the `(code, iss?)` and `(URLSearchParams)` overloads.
*
* For the `URLSearchParams` form, only `iss` and `code` are read up front. When a `code` is
* present the returned values flow into {@linkcode auth}, which runs
* {@linkcode validateAuthorizationResponseIssuer} against freshly-discovered metadata before
* the code is redeemed — so on mismatch the thrown {@linkcode IssuerMismatchError} carries no
* `error`/`error_description`/`error_uri` text from the callback (those are attacker-controlled
* in a mix-up). When no `code` is present (an error-shaped callback), `iss` is validated here
* against the provider's recorded discovery state — or, when the provider does not implement
* `discoveryState`, against freshly-discovered metadata mirroring what {@linkcode auth} does on
* the code-present path — **before** the callback's error parameters are read; only after that
* passes are they surfaced as an {@linkcode OAuthError}. When no issuer baseline can be
* obtained either way, a generic {@linkcode UnauthorizedError} is thrown without surfacing the
* callback's `error`/`error_description`/`error_uri`.
*
* @internal Exported for the transport `finishAuth` overloads; not part of the public barrel.
*/
async function resolveAuthorizationCallbackParams(codeOrParams, iss, provider, serverUrl, opts) {
	if (typeof codeOrParams === "string") return {
		authorizationCode: codeOrParams,
		iss
	};
	const issParam = codeOrParams.get("iss") ?? void 0;
	const code = codeOrParams.get("code");
	if (code) return {
		authorizationCode: code,
		iss: issParam
	};
	let metadata = (await provider.discoveryState?.())?.authorizationServerMetadata;
	if (!metadata) try {
		metadata = (await discoverOAuthServerInfo(serverUrl, opts)).authorizationServerMetadata;
	} catch {
		metadata = void 0;
	}
	if (!metadata) throw new UnauthorizedError("Authorization callback failed and the issuer could not be verified");
	validateAuthorizationResponseIssuer({
		iss: issParam,
		expectedIssuer: metadata.issuer,
		issParameterSupported: isIssParameterSupported(metadata)
	});
	const error = codeOrParams.get("error");
	if (error) throw new OAuthError(error, codeOrParams.get("error_description") ?? error, codeOrParams.get("error_uri") ?? void 0);
	throw new UnauthorizedError("Authorization callback contained neither `code` nor `error`");
}
function isClientAuthMethod(method) {
	return [
		"client_secret_basic",
		"client_secret_post",
		"none"
	].includes(method);
}
const AUTHORIZATION_CODE_RESPONSE_TYPE = "code";
const AUTHORIZATION_CODE_CHALLENGE_METHOD = "S256";
/**
* Determines the best client authentication method to use based on server support and client configuration.
*
* Priority order (highest to lowest):
* 1. `client_secret_basic` (if client secret is available)
* 2. `client_secret_post` (if client secret is available)
* 3. `none` (for public clients)
*
* @param clientInformation - OAuth client information containing credentials
* @param supportedMethods - Authentication methods supported by the authorization server
* @returns The selected authentication method
*/
function selectClientAuthMethod(clientInformation, supportedMethods) {
	const hasClientSecret = clientInformation.client_secret !== void 0;
	if ("token_endpoint_auth_method" in clientInformation && clientInformation.token_endpoint_auth_method && isClientAuthMethod(clientInformation.token_endpoint_auth_method) && (supportedMethods.length === 0 || supportedMethods.includes(clientInformation.token_endpoint_auth_method))) return clientInformation.token_endpoint_auth_method;
	if (supportedMethods.length === 0) return hasClientSecret ? "client_secret_basic" : "none";
	if (hasClientSecret && supportedMethods.includes("client_secret_basic")) return "client_secret_basic";
	if (hasClientSecret && supportedMethods.includes("client_secret_post")) return "client_secret_post";
	if (supportedMethods.includes("none")) return "none";
	return hasClientSecret ? "client_secret_post" : "none";
}
/**
* Applies client authentication to the request based on the specified method.
*
* Implements OAuth 2.1 client authentication methods:
* - `client_secret_basic`: HTTP Basic authentication (RFC 6749 Section 2.3.1)
* - `client_secret_post`: Credentials in request body (RFC 6749 Section 2.3.1)
* - `none`: Public client authentication (RFC 6749 Section 2.1)
*
* @param method - The authentication method to use
* @param clientInformation - OAuth client information containing credentials
* @param headers - HTTP headers object to modify
* @param params - URL search parameters to modify
* @throws {Error} When required credentials are missing
*/
function applyClientAuthentication(method, clientInformation, headers, params) {
	const { client_id, client_secret } = clientInformation;
	switch (method) {
		case "client_secret_basic":
			applyBasicAuth(client_id, client_secret, headers);
			return;
		case "client_secret_post":
			applyPostAuth(client_id, client_secret, params);
			return;
		case "none":
			applyPublicAuth(client_id, params);
			return;
		default: throw new Error(`Unsupported client authentication method: ${method}`);
	}
}
/**
* Applies HTTP Basic authentication (RFC 6749 Section 2.3.1)
*/
function applyBasicAuth(clientId, clientSecret, headers) {
	if (!clientSecret) throw new Error("client_secret_basic authentication requires a client_secret");
	const credentials = btoa(`${clientId}:${clientSecret}`);
	headers.set("Authorization", `Basic ${credentials}`);
}
/**
* Applies POST body authentication (RFC 6749 Section 2.3.1)
*/
function applyPostAuth(clientId, clientSecret, params) {
	params.set("client_id", clientId);
	if (clientSecret) params.set("client_secret", clientSecret);
}
/**
* Applies public client authentication (RFC 6749 Section 2.1)
*/
function applyPublicAuth(clientId, params) {
	params.set("client_id", clientId);
}
/** Loopback hosts exempt from the in-transit `https:` requirement (RFC 8252 §7.3). */
function isLoopbackHost(hostname) {
	return hostname === "localhost" || hostname === "127.0.0.1" || hostname === "[::1]" || hostname === "::1";
}
/**
* SEP-2207: refuse to send credentials to a non-TLS, non-loopback token endpoint.
* Throws {@linkcode InsecureTokenEndpointError}. Loopback hosts are exempt.
*/
function assertSecureTokenEndpoint(tokenEndpoint) {
	const url = new URL(String(tokenEndpoint));
	if (url.protocol !== "https:" && !isLoopbackHost(url.hostname)) throw new InsecureTokenEndpointError(url.href);
	return url;
}
/**
* Derives an OIDC `application_type` from a client's registered redirect URIs
* when the consumer has not set one explicitly (SEP-837). Loopback hosts and
* non-`http(s)` custom URI schemes indicate a native application (RFC 8252);
* everything else is treated as a web application. The result is a heuristic
* default — callers that know better should set `clientMetadata.application_type`
* themselves, which {@linkcode resolveClientMetadata} never overwrites.
*
* A mixed redirect set (for example a public `https:` URI alongside a loopback
* URI) is inherently ambiguous under OIDC DCR §2 — neither value satisfies the
* AS for both URIs — so consumers with mixed sets should set `application_type`
* explicitly rather than relying on this heuristic.
*/
function deriveApplicationType(redirectUris) {
	for (const raw of redirectUris ?? []) {
		let url;
		try {
			url = new URL(raw);
		} catch {
			continue;
		}
		if (url.protocol !== "http:" && url.protocol !== "https:") return "native";
		if (isLoopbackHost(url.hostname)) return "native";
	}
	return "web";
}
/**
* Reads {@linkcode OAuthClientProvider.clientMetadata | clientMetadata} from the
* provider and fills the SEP-837 / SEP-2207 defaults the SDK relies on, so
* {@linkcode registerClient} sees a consistent, fully-populated document.
*
* - `grant_types` defaults to `['authorization_code', 'refresh_token']` for
*   interactive providers (those with a {@linkcode OAuthClientProvider.redirectUrl | redirectUrl})
*   so authorization servers that gate refresh-token issuance on the registered
*   grant types issue one (SEP-2207). Non-interactive providers (no
*   `redirectUrl`) get no `grant_types` default. This default applies to the
*   Dynamic Client Registration body only — it does **not** drive
*   {@linkcode determineScope}'s `offline_access` augmentation.
* - `application_type` defaults from `redirect_uris`: loopback redirect hosts
*   and custom URI schemes → `'native'`, otherwise `'web'` (SEP-837 / RFC 8252).
*
* A field the consumer set explicitly is **never** overwritten. {@linkcode auth}
* calls this once at the top of the flow; direct callers of
* {@linkcode registerClient} that want the same defaults should pass the result
* of this function as `clientMetadata`.
*/
function resolveClientMetadata(provider) {
	const clientMetadata = provider.clientMetadata;
	return {
		...clientMetadata,
		grant_types: clientMetadata.grant_types ?? (provider.redirectUrl === void 0 ? void 0 : ["authorization_code", "refresh_token"]),
		application_type: clientMetadata.application_type ?? deriveApplicationType(clientMetadata.redirect_uris)
	};
}
/**
* Parses an OAuth error response from a string or Response object.
*
* If the input is a standard OAuth2.0 error response, it will be parsed according to the spec
* and an {@linkcode OAuthError} will be returned with the appropriate error code.
* If parsing fails, it falls back to a generic {@linkcode OAuthErrorCode.ServerError | ServerError} that includes
* the response status (if available) and original content.
*
* @param input - A Response object or string containing the error response
* @returns A Promise that resolves to an {@linkcode OAuthError} instance
*/
async function parseErrorResponse(input) {
	const statusCode = input instanceof Response ? input.status : void 0;
	const body = input instanceof Response ? await input.text() : input;
	try {
		const result = OAuthErrorResponseSchema.parse(JSON.parse(body));
		return OAuthError.fromResponse(result);
	} catch (error) {
		const errorMessage = `${statusCode ? `HTTP ${statusCode}: ` : ""}Invalid OAuth error response: ${error}. Raw body: ${body}`;
		return new OAuthError(OAuthErrorCode.ServerError, errorMessage);
	}
}
/**
* Orchestrates the full auth flow with a server.
*
* This can be used as a single entry point for all authorization functionality,
* instead of linking together the other lower-level functions in this module.
*/
async function auth(provider, options) {
	try {
		return await authInternal(provider, options);
	} catch (error) {
		if (error instanceof OAuthError) {
			if (error.code === OAuthErrorCode.InvalidClient || error.code === OAuthErrorCode.UnauthorizedClient) {
				await provider.invalidateCredentials?.("client");
				await provider.invalidateCredentials?.("tokens");
				return await authInternal(provider, options);
			} else if (error.code === OAuthErrorCode.InvalidGrant) {
				await provider.invalidateCredentials?.("tokens");
				return await authInternal(provider, options);
			}
		}
		throw error;
	}
}
/**
* Selects scopes per the MCP spec and augment for refresh token support.
*/
function determineScope(options) {
	const { requestedScope, resourceMetadata, authServerMetadata, clientMetadata } = options;
	let effectiveScope = requestedScope || resourceMetadata?.scopes_supported?.join(" ") || clientMetadata.scope;
	if (effectiveScope && authServerMetadata?.scopes_supported?.includes("offline_access") && !effectiveScope.split(" ").includes("offline_access") && clientMetadata.grant_types?.includes("refresh_token")) effectiveScope = `${effectiveScope} offline_access`;
	return effectiveScope;
}
async function authInternal(provider, { serverUrl, authorizationCode, iss, scope, resourceMetadataUrl, fetchFn, skipIssuerMetadataValidation, forceReauthorization }) {
	const clientMetadata = resolveClientMetadata(provider);
	const cachedState = await provider.discoveryState?.();
	let resourceMetadata;
	let authorizationServerUrl;
	let metadata;
	let freshDiscoveryState;
	let effectiveResourceMetadataUrl = resourceMetadataUrl;
	if (!effectiveResourceMetadataUrl && cachedState?.resourceMetadataUrl) effectiveResourceMetadataUrl = new URL(cachedState.resourceMetadataUrl);
	if (cachedState?.authorizationServerUrl) {
		authorizationServerUrl = cachedState.authorizationServerUrl;
		resourceMetadata = cachedState.resourceMetadata;
		metadata = cachedState.authorizationServerMetadata ?? await discoverAuthorizationServerMetadata(authorizationServerUrl, {
			fetchFn,
			skipIssuerValidation: skipIssuerMetadataValidation
		});
		if (!resourceMetadata) try {
			resourceMetadata = await discoverOAuthProtectedResourceMetadata(serverUrl, { resourceMetadataUrl: effectiveResourceMetadataUrl }, fetchFn);
		} catch (error) {
			if (error instanceof TypeError) throw error;
		}
		if (metadata !== cachedState.authorizationServerMetadata || resourceMetadata !== cachedState.resourceMetadata) await provider.saveDiscoveryState?.({
			authorizationServerUrl: String(authorizationServerUrl),
			resourceMetadataUrl: effectiveResourceMetadataUrl?.toString(),
			resourceMetadata,
			authorizationServerMetadata: metadata
		});
	} else {
		const serverInfo = await discoverOAuthServerInfo(serverUrl, {
			resourceMetadataUrl: effectiveResourceMetadataUrl,
			fetchFn,
			skipIssuerMetadataValidation
		});
		authorizationServerUrl = serverInfo.authorizationServerUrl;
		metadata = serverInfo.authorizationServerMetadata;
		resourceMetadata = serverInfo.resourceMetadata;
		freshDiscoveryState = {
			authorizationServerUrl: String(authorizationServerUrl),
			resourceMetadataUrl: effectiveResourceMetadataUrl?.toString(),
			resourceMetadata,
			authorizationServerMetadata: metadata
		};
	}
	const issuer = metadata?.issuer ?? String(authorizationServerUrl);
	const infoCtx = { issuer };
	await provider.saveAuthorizationServerUrl?.(issuer);
	if (authorizationCode !== void 0) {
		const recordedIssuer = cachedState?.authorizationServerMetadata?.issuer ?? cachedState?.authorizationServerUrl;
		if (recordedIssuer === void 0) {
			if (provider.saveDiscoveryState !== void 0) throw new AuthorizationServerMismatchError("discoveryState was not available on the callback leg; ensure your provider persists discoveryState alongside codeVerifier", issuer);
			console.warn("[mcp-sdk] OAuthClientProvider does not implement saveDiscoveryState()/discoveryState(); the SEP-2352 callback-leg authorization-server binding cannot be checked. Implement discoveryState (persist alongside codeVerifier) — see docs/migration/upgrade-to-v2.md §SEP-2352.");
		} else if (!issuersMatch(recordedIssuer, issuer)) throw new AuthorizationServerMismatchError(recordedIssuer, issuer);
	}
	if (freshDiscoveryState) await provider.saveDiscoveryState?.(freshDiscoveryState);
	const resource = await selectResourceURL(serverUrl, provider, resourceMetadata);
	if (resource) await provider.saveResourceUrl?.(String(resource));
	const resolvedScope = determineScope({
		requestedScope: scope,
		resourceMetadata,
		authServerMetadata: metadata,
		clientMetadata: provider.clientMetadata
	});
	const rawClientInfo = await Promise.resolve(provider.clientInformation(infoCtx));
	let clientInformation = discardIfIssuerMismatch(rawClientInfo, issuer, { canPersistStamp: provider.saveClientInformation !== void 0 });
	if (clientInformation === void 0 && rawClientInfo?.issuer && provider.saveClientInformation === void 0) throw new AuthorizationServerMismatchError(rawClientInfo.issuer, issuer);
	if (clientInformation && clientInformation.issuer === void 0) {
		clientInformation = {
			...clientInformation,
			issuer
		};
		await provider.saveClientInformation?.(clientInformation, infoCtx);
	}
	if (!clientInformation) {
		if (authorizationCode !== void 0) throw new Error("Existing OAuth client information is required when exchanging an authorization code");
		const supportsUrlBasedClientId = metadata?.client_id_metadata_document_supported === true;
		const clientMetadataUrl = provider.clientMetadataUrl;
		if (clientMetadataUrl && !isHttpsUrl(clientMetadataUrl)) throw new OAuthError(OAuthErrorCode.InvalidClientMetadata, `clientMetadataUrl must be a valid HTTPS URL with a non-root pathname, got: ${clientMetadataUrl}`);
		if (supportsUrlBasedClientId && clientMetadataUrl) {
			clientInformation = {
				client_id: clientMetadataUrl,
				issuer
			};
			await provider.saveClientInformation?.(clientInformation, infoCtx);
		} else {
			if (!provider.saveClientInformation) throw new Error("OAuth client information must be saveable for dynamic registration");
			clientInformation = {
				...await registerClient(authorizationServerUrl, {
					metadata,
					clientMetadata,
					scope: resolvedScope,
					fetchFn
				}),
				issuer
			};
			await provider.saveClientInformation(clientInformation, infoCtx);
		}
	}
	const nonInteractiveFlow = !provider.redirectUrl;
	if (authorizationCode !== void 0 || nonInteractiveFlow) {
		if (authorizationCode !== void 0) validateAuthorizationResponseIssuer({
			iss,
			expectedIssuer: metadata?.issuer,
			issParameterSupported: isIssParameterSupported(metadata)
		});
		const tokens$1 = await fetchToken(provider, authorizationServerUrl, {
			metadata,
			resource,
			authorizationCode,
			iss,
			scope: resolvedScope,
			fetchFn
		});
		await provider.saveTokens({
			...tokens$1,
			issuer
		}, infoCtx);
		return "AUTHORIZED";
	}
	let tokens = discardIfIssuerMismatch(await provider.tokens(infoCtx), issuer);
	if (tokens && tokens.issuer === void 0) {
		tokens = {
			...tokens,
			issuer
		};
		await provider.saveTokens(tokens, infoCtx);
	}
	if (tokens?.refresh_token && !forceReauthorization) try {
		const newTokens = await refreshAuthorization(authorizationServerUrl, {
			metadata,
			clientInformation,
			refreshToken: tokens.refresh_token,
			resource,
			addClientAuthentication: provider.addClientAuthentication,
			fetchFn
		});
		await provider.saveTokens({
			...newTokens,
			issuer
		}, infoCtx);
		return "AUTHORIZED";
	} catch (error) {
		if (error instanceof InsecureTokenEndpointError) throw error;
		if (!(error instanceof OAuthError) || error.code === OAuthErrorCode.ServerError) {} else throw error;
	}
	const state = provider.state ? await provider.state() : void 0;
	const { authorizationUrl, codeVerifier } = await startAuthorization(authorizationServerUrl, {
		metadata,
		clientInformation,
		state,
		redirectUrl: provider.redirectUrl,
		scope: resolvedScope,
		resource
	});
	await provider.saveCodeVerifier(codeVerifier);
	await provider.redirectToAuthorization(authorizationUrl);
	return "REDIRECT";
}
/**
* Validates that the given `clientMetadataUrl` is a valid HTTPS URL with a non-root pathname.
*
* No-op when `url` is `undefined` or empty (providers that do not use URL-based client IDs
* are unaffected). When the value is defined but invalid, throws an {@linkcode OAuthError}
* with code {@linkcode OAuthErrorCode.InvalidClientMetadata}.
*
* {@linkcode OAuthClientProvider} implementations that accept a `clientMetadataUrl` should
* call this in their constructors for early validation.
*
* @param url - The `clientMetadataUrl` value to validate (from `OAuthClientProvider.clientMetadataUrl`)
* @throws {OAuthError} When `url` is defined but is not a valid HTTPS URL with a non-root pathname
*/
function validateClientMetadataUrl(url) {
	if (url && !isHttpsUrl(url)) throw new OAuthError(OAuthErrorCode.InvalidClientMetadata, `clientMetadataUrl must be a valid HTTPS URL with a non-root pathname, got: ${url}`);
}
/**
* SEP-991: URL-based Client IDs
* Validate that the `client_id` is a valid URL with `https` scheme
*/
function isHttpsUrl(value) {
	if (!value) return false;
	try {
		const url = new URL(value);
		return url.protocol === "https:" && url.pathname !== "/";
	} catch {
		return false;
	}
}
async function selectResourceURL(serverUrl, provider, resourceMetadata) {
	const defaultResource = resourceUrlFromServerUrl(serverUrl);
	if (provider.validateResourceURL) return await provider.validateResourceURL(defaultResource, resourceMetadata?.resource);
	if (!resourceMetadata) return;
	if (!checkResourceAllowed({
		requestedResource: defaultResource,
		configuredResource: resourceMetadata.resource
	})) throw new Error(`Protected resource ${resourceMetadata.resource} does not match expected ${defaultResource} (or origin)`);
	return new URL(resourceMetadata.resource);
}
/**
* Extract `resource_metadata`, `scope`, `error`, and `error_description` from a
* `WWW-Authenticate` header.
*/
function extractWWWAuthenticateParams(res) {
	const authenticateHeader = res.headers.get("WWW-Authenticate");
	if (!authenticateHeader) return {};
	const [type, scheme] = authenticateHeader.split(" ");
	if (type?.toLowerCase() !== "bearer" || !scheme) return {};
	const resourceMetadataMatch = extractFieldFromWwwAuth(res, "resource_metadata") || void 0;
	let resourceMetadataUrl;
	if (resourceMetadataMatch) try {
		resourceMetadataUrl = new URL(resourceMetadataMatch);
	} catch {}
	const scope = extractFieldFromWwwAuth(res, "scope") || void 0;
	const error = extractFieldFromWwwAuth(res, "error") || void 0;
	const errorDescription = extractFieldFromWwwAuth(res, "error_description") || void 0;
	return {
		resourceMetadataUrl,
		scope,
		error,
		errorDescription
	};
}
/**
* Extracts a specific field's value from the `WWW-Authenticate` header string.
*
* @param response The HTTP response object containing the headers.
* @param fieldName The name of the field to extract (e.g., `"realm"`, `"nonce"`).
* @returns The field value
*/
function extractFieldFromWwwAuth(response, fieldName) {
	const wwwAuthHeader = response.headers.get("WWW-Authenticate");
	if (!wwwAuthHeader) return null;
	const pattern = new RegExp(String.raw`${fieldName}=(?:"([^"]+)"|([^\s,]+))`);
	const match = wwwAuthHeader.match(pattern);
	if (match) {
		const result = match[1] || match[2];
		if (result) return result;
	}
	return null;
}
/**
* Extract `resource_metadata` from response header.
* @deprecated Use {@linkcode extractWWWAuthenticateParams} instead.
*/
function extractResourceMetadataUrl(res) {
	const authenticateHeader = res.headers.get("WWW-Authenticate");
	if (!authenticateHeader) return;
	const [type, scheme] = authenticateHeader.split(" ");
	if (type?.toLowerCase() !== "bearer" || !scheme) return;
	const match = /resource_metadata="([^"]*)"/.exec(authenticateHeader);
	if (!match || !match[1]) return;
	try {
		return new URL(match[1]);
	} catch {
		return;
	}
}
/**
* Looks up {@link https://datatracker.ietf.org/doc/html/rfc9728 | RFC 9728}
* OAuth 2.0 Protected Resource Metadata.
*
* If the server returns a 404 for the well-known endpoint, this function will
* return `undefined`. Any other errors will be thrown as exceptions.
*/
async function discoverOAuthProtectedResourceMetadata(serverUrl, opts, fetchFn = fetch) {
	const response = await discoverMetadataWithFallback(serverUrl, "oauth-protected-resource", fetchFn, {
		protocolVersion: opts?.protocolVersion,
		metadataUrl: opts?.resourceMetadataUrl
	});
	if (!response || response.status === 404) {
		await response?.text?.().catch(() => {});
		throw new Error(`Resource server does not implement OAuth 2.0 Protected Resource Metadata.`);
	}
	if (!response.ok) {
		await response.text?.().catch(() => {});
		throw new Error(`HTTP ${response.status} trying to load well-known OAuth protected resource metadata.`);
	}
	return OAuthProtectedResourceMetadataSchema.parse(await response.json());
}
/**
* Fetch with a retry heuristic for CORS errors caused by custom headers.
*
* In browsers, adding a custom header (e.g. `MCP-Protocol-Version`) triggers a CORS preflight.
* If the server doesn't allow that header, the browser throws a `TypeError` before any response
* is received. Retrying without custom headers often succeeds because the request becomes
* "simple" (no preflight). If the server sends no CORS headers at all, the retry also fails
* with `TypeError` and we return `undefined` so callers can fall through to an alternate URL.
*
* However, `fetch()` also throws `TypeError` for non-CORS failures (DNS resolution, connection
* refused, invalid URL). Swallowing those and returning `undefined` masks real errors and can
* cause callers to silently fall through to a different discovery URL. CORS is a browser-only
* concept, so in non-browser runtimes (Node.js, Workers) a `TypeError` from `fetch` is never a
* CORS error — there we propagate the error instead of swallowing it.
*
* In browsers, we cannot reliably distinguish CORS `TypeError` from network `TypeError` from the
* error object alone, so the swallow-and-fallthrough heuristic is preserved there.
*/
async function fetchWithCorsRetry(url, headers, fetchFn = fetch) {
	try {
		return await fetchFn(url, { headers });
	} catch (error) {
		if (!(error instanceof TypeError) || !CORS_IS_POSSIBLE) throw error;
		if (headers) try {
			return await fetchFn(url, {});
		} catch (retryError) {
			if (!(retryError instanceof TypeError)) throw retryError;
			return;
		}
		return;
	}
}
/**
* Constructs the well-known path for auth-related metadata discovery
*/
function buildWellKnownPath(wellKnownPrefix, pathname = "", options = {}) {
	if (pathname.endsWith("/")) pathname = pathname.slice(0, -1);
	return options.prependPathname ? `${pathname}/.well-known/${wellKnownPrefix}` : `/.well-known/${wellKnownPrefix}${pathname}`;
}
/**
* Tries to discover OAuth metadata at a specific URL
*/
async function tryMetadataDiscovery(url, protocolVersion, fetchFn = fetch) {
	return await fetchWithCorsRetry(url, { "MCP-Protocol-Version": protocolVersion }, fetchFn);
}
/**
* Determines if fallback to root discovery should be attempted
*/
function shouldAttemptFallback(response, pathname) {
	if (!response) return true;
	if (pathname === "/") return false;
	return response.status >= 400 && response.status < 500 || response.status === 502;
}
/**
* Generic function for discovering OAuth metadata with fallback support
*/
async function discoverMetadataWithFallback(serverUrl, wellKnownType, fetchFn, opts) {
	const issuer = new URL(serverUrl);
	const protocolVersion = opts?.protocolVersion ?? LATEST_PROTOCOL_VERSION;
	let url;
	if (opts?.metadataUrl) url = new URL(opts.metadataUrl);
	else {
		const wellKnownPath = buildWellKnownPath(wellKnownType, issuer.pathname);
		url = new URL(wellKnownPath, opts?.metadataServerUrl ?? issuer);
		url.search = issuer.search;
	}
	let response = await tryMetadataDiscovery(url, protocolVersion, fetchFn);
	if (!opts?.metadataUrl && shouldAttemptFallback(response, issuer.pathname)) response = await tryMetadataDiscovery(new URL(`/.well-known/${wellKnownType}`, issuer), protocolVersion, fetchFn);
	return response;
}
/**
* Looks up RFC 8414 OAuth 2.0 Authorization Server Metadata.
*
* If the server returns a 404 for the well-known endpoint, this function will
* return `undefined`. Any other errors will be thrown as exceptions.
*
* @deprecated This function is deprecated in favor of {@linkcode discoverAuthorizationServerMetadata}.
*/
async function discoverOAuthMetadata(issuer, { authorizationServerUrl, protocolVersion } = {}, fetchFn = fetch) {
	if (typeof issuer === "string") issuer = new URL(issuer);
	if (!authorizationServerUrl) authorizationServerUrl = issuer;
	if (typeof authorizationServerUrl === "string") authorizationServerUrl = new URL(authorizationServerUrl);
	protocolVersion ??= LATEST_PROTOCOL_VERSION;
	const response = await discoverMetadataWithFallback(authorizationServerUrl, "oauth-authorization-server", fetchFn, {
		protocolVersion,
		metadataServerUrl: authorizationServerUrl
	});
	if (!response || response.status === 404) {
		await response?.text?.().catch(() => {});
		return;
	}
	if (!response.ok) {
		await response.text?.().catch(() => {});
		throw new Error(`HTTP ${response.status} trying to load well-known OAuth metadata`);
	}
	return OAuthMetadataSchema.parse(await response.json());
}
/**
* Builds a list of discovery URLs to try for authorization server metadata.
* URLs are returned in priority order:
* 1. OAuth metadata at the given URL
* 2. OIDC metadata endpoints at the given URL
*/
function buildDiscoveryUrls(authorizationServerUrl) {
	const url = typeof authorizationServerUrl === "string" ? new URL(authorizationServerUrl) : authorizationServerUrl;
	const hasPath = url.pathname !== "/";
	const urlsToTry = [];
	if (!hasPath) {
		urlsToTry.push({
			url: new URL("/.well-known/oauth-authorization-server", url.origin),
			type: "oauth"
		}, {
			url: new URL(`/.well-known/openid-configuration`, url.origin),
			type: "oidc"
		});
		return urlsToTry;
	}
	let pathname = url.pathname;
	if (pathname.endsWith("/")) pathname = pathname.slice(0, -1);
	urlsToTry.push({
		url: new URL(`/.well-known/oauth-authorization-server${pathname}`, url.origin),
		type: "oauth"
	}, {
		url: new URL(`/.well-known/openid-configuration${pathname}`, url.origin),
		type: "oidc"
	}, {
		url: new URL(`${pathname}/.well-known/openid-configuration`, url.origin),
		type: "oidc"
	});
	return urlsToTry;
}
/**
* Discovers authorization server metadata with support for
* {@link https://datatracker.ietf.org/doc/html/rfc8414 | RFC 8414} OAuth 2.0
* Authorization Server Metadata and
* {@link https://openid.net/specs/openid-connect-discovery-1_0.html | OpenID Connect Discovery 1.0}
* specifications.
*
* This function implements a fallback strategy for authorization server discovery:
* 1. Attempts RFC 8414 OAuth metadata discovery first
* 2. If OAuth discovery fails, falls back to OpenID Connect Discovery
*
* @param authorizationServerUrl - The authorization server URL obtained from the MCP Server's
*                                 protected resource metadata, or the MCP server's URL if the
*                                 metadata was not found.
* The returned metadata's `issuer` is validated against `authorizationServerUrl`
* per RFC 8414 §3.3 (and OIDC Discovery §4.3): if they differ the metadata is
* **rejected** with {@linkcode IssuerMismatchError} and not returned. Set
* `skipIssuerValidation: true` to suppress this check — **security-weakening**,
* intended only for known-misconfigured authorization servers.
*
* @param options - Configuration options
* @param options.fetchFn - Optional fetch function for making HTTP requests, defaults to global fetch
* @param options.protocolVersion - MCP protocol version to use, defaults to {@linkcode LATEST_PROTOCOL_VERSION}
* @param options.skipIssuerValidation - Skip the RFC 8414 §3.3 `issuer` echo check. **Security-weakening.**
* @returns Promise resolving to authorization server metadata, or undefined if discovery fails
* @throws {IssuerMismatchError} when the metadata's `issuer` does not match `authorizationServerUrl`
*/
async function discoverAuthorizationServerMetadata(authorizationServerUrl, { fetchFn = fetch, protocolVersion = LATEST_PROTOCOL_VERSION, skipIssuerValidation = false } = {}) {
	const headers = {
		"MCP-Protocol-Version": protocolVersion,
		Accept: "application/json"
	};
	const urlsToTry = buildDiscoveryUrls(authorizationServerUrl);
	for (const { url: endpointUrl, type } of urlsToTry) {
		const response = await fetchWithCorsRetry(endpointUrl, headers, fetchFn);
		if (!response)
 /**
		* CORS error occurred - don't throw as the endpoint may not allow CORS,
		* continue trying other possible endpoints
		*/
		continue;
		if (!response.ok) {
			await response.text?.().catch(() => {});
			if (response.status >= 400 && response.status < 500 || response.status === 502) continue;
			throw new Error(`HTTP ${response.status} trying to load ${type === "oauth" ? "OAuth" : "OpenID provider"} metadata from ${endpointUrl}`);
		}
		const parsed = type === "oauth" ? OAuthMetadataSchema.parse(await response.json()) : OpenIdProviderDiscoveryMetadataSchema.parse(await response.json());
		if (!skipIssuerValidation) {
			const expectedIssuer = typeof authorizationServerUrl === "string" ? authorizationServerUrl : authorizationServerUrl.href;
			if (!(parsed.issuer === expectedIssuer || expectedIssuer.endsWith("/") && parsed.issuer === expectedIssuer.slice(0, -1))) throw new IssuerMismatchError("metadata", expectedIssuer, parsed.issuer);
		}
		return parsed;
	}
}
/**
* Discovers the authorization server for an MCP server following
* {@link https://datatracker.ietf.org/doc/html/rfc9728 | RFC 9728} (OAuth 2.0 Protected
* Resource Metadata), with fallback to treating the server URL as the
* authorization server.
*
* This function combines two discovery steps into one call:
* 1. Probes `/.well-known/oauth-protected-resource` on the MCP server to find the
*    authorization server URL (RFC 9728).
* 2. Fetches authorization server metadata from that URL (RFC 8414 / OpenID Connect Discovery).
*
* Use this when you need the authorization server metadata for operations outside the
* {@linkcode auth} orchestrator, such as token refresh or token revocation.
*
* @param serverUrl - The MCP resource server URL
* @param opts - Optional configuration
* @param opts.resourceMetadataUrl - Override URL for the protected resource metadata endpoint
* @param opts.fetchFn - Custom fetch function for HTTP requests
* @returns Authorization server URL, metadata, and resource metadata (if available)
*/
async function discoverOAuthServerInfo(serverUrl, opts) {
	let resourceMetadata;
	let authorizationServerUrl;
	try {
		resourceMetadata = await discoverOAuthProtectedResourceMetadata(serverUrl, { resourceMetadataUrl: opts?.resourceMetadataUrl }, opts?.fetchFn);
		if (resourceMetadata.authorization_servers && resourceMetadata.authorization_servers.length > 0) authorizationServerUrl = resourceMetadata.authorization_servers[0];
	} catch (error) {
		if (error instanceof TypeError) throw error;
	}
	if (!authorizationServerUrl) authorizationServerUrl = String(new URL("/", serverUrl));
	const authorizationServerMetadata = await discoverAuthorizationServerMetadata(authorizationServerUrl, {
		fetchFn: opts?.fetchFn,
		skipIssuerValidation: opts?.skipIssuerMetadataValidation
	});
	return {
		authorizationServerUrl,
		authorizationServerMetadata,
		resourceMetadata
	};
}
/**
* Begins the authorization flow with the given server, by generating a PKCE challenge and constructing the authorization URL.
*/
async function startAuthorization(authorizationServerUrl, { metadata, clientInformation, redirectUrl, scope, state, resource }) {
	let authorizationUrl;
	if (metadata) {
		authorizationUrl = new URL(metadata.authorization_endpoint);
		if (!metadata.response_types_supported.includes(AUTHORIZATION_CODE_RESPONSE_TYPE)) throw new Error(`Incompatible auth server: does not support response type ${AUTHORIZATION_CODE_RESPONSE_TYPE}`);
		if (metadata.code_challenge_methods_supported && !metadata.code_challenge_methods_supported.includes(AUTHORIZATION_CODE_CHALLENGE_METHOD)) throw new Error(`Incompatible auth server: does not support code challenge method ${AUTHORIZATION_CODE_CHALLENGE_METHOD}`);
	} else authorizationUrl = new URL("/authorize", authorizationServerUrl);
	const challenge = await pkceChallenge();
	const codeVerifier = challenge.code_verifier;
	const codeChallenge = challenge.code_challenge;
	authorizationUrl.searchParams.set("response_type", AUTHORIZATION_CODE_RESPONSE_TYPE);
	authorizationUrl.searchParams.set("client_id", clientInformation.client_id);
	authorizationUrl.searchParams.set("code_challenge", codeChallenge);
	authorizationUrl.searchParams.set("code_challenge_method", AUTHORIZATION_CODE_CHALLENGE_METHOD);
	authorizationUrl.searchParams.set("redirect_uri", String(redirectUrl));
	if (state) authorizationUrl.searchParams.set("state", state);
	if (scope) authorizationUrl.searchParams.set("scope", scope);
	if (scope?.split(" ").includes("offline_access")) authorizationUrl.searchParams.append("prompt", "consent");
	if (resource) authorizationUrl.searchParams.set("resource", resource.href);
	return {
		authorizationUrl,
		codeVerifier
	};
}
/**
* Prepares token request parameters for an authorization code exchange.
*
* This is the default implementation used by {@linkcode fetchToken} when the provider
* doesn't implement {@linkcode OAuthClientProvider.prepareTokenRequest | prepareTokenRequest}.
*
* @param authorizationCode - The authorization code received from the authorization endpoint
* @param codeVerifier - The PKCE code verifier
* @param redirectUri - The redirect URI used in the authorization request
* @returns URLSearchParams for the `authorization_code` grant
*/
function prepareAuthorizationCodeRequest(authorizationCode, codeVerifier, redirectUri) {
	return new URLSearchParams({
		grant_type: "authorization_code",
		code: authorizationCode,
		code_verifier: codeVerifier,
		redirect_uri: String(redirectUri)
	});
}
/**
* Internal helper to execute a token request with the given parameters.
* Used by {@linkcode exchangeAuthorization}, {@linkcode refreshAuthorization}, and {@linkcode fetchToken}.
*/
async function executeTokenRequest(authorizationServerUrl, { metadata, tokenRequestParams, clientInformation, addClientAuthentication, resource, fetchFn }) {
	const tokenUrl = assertSecureTokenEndpoint(metadata?.token_endpoint ?? new URL("/token", authorizationServerUrl));
	const headers = new Headers({
		"Content-Type": "application/x-www-form-urlencoded",
		Accept: "application/json"
	});
	if (resource) tokenRequestParams.set("resource", resource.href);
	if (addClientAuthentication) await addClientAuthentication(headers, tokenRequestParams, tokenUrl, metadata);
	else if (clientInformation) applyClientAuthentication(selectClientAuthMethod(clientInformation, metadata?.token_endpoint_auth_methods_supported ?? []), clientInformation, headers, tokenRequestParams);
	const response = await (fetchFn ?? fetch)(tokenUrl, {
		method: "POST",
		headers,
		body: tokenRequestParams
	});
	if (!response.ok) throw await parseErrorResponse(response);
	const json = await response.json();
	try {
		return OAuthTokensSchema.parse(json);
	} catch (parseError) {
		if (typeof json === "object" && json !== null && "error" in json) throw await parseErrorResponse(JSON.stringify(json));
		throw parseError;
	}
}
/**
* Exchanges an authorization code for an access token with the given server.
*
* Supports multiple client authentication methods as specified in OAuth 2.1:
* - Automatically selects the best authentication method based on server support
* - Falls back to appropriate defaults when server metadata is unavailable
*
* @param authorizationServerUrl - The authorization server's base URL
* @param options - Configuration object containing client info, auth code, etc.
* @returns Promise resolving to OAuth tokens
* @throws {Error} When token exchange fails or authentication is invalid
*/
async function exchangeAuthorization(authorizationServerUrl, { metadata, clientInformation, authorizationCode, iss, codeVerifier, redirectUri, resource, addClientAuthentication, fetchFn }) {
	validateAuthorizationResponseIssuer({
		iss,
		expectedIssuer: metadata?.issuer,
		issParameterSupported: isIssParameterSupported(metadata)
	});
	return executeTokenRequest(authorizationServerUrl, {
		metadata,
		tokenRequestParams: prepareAuthorizationCodeRequest(authorizationCode, codeVerifier, redirectUri),
		clientInformation,
		addClientAuthentication,
		resource,
		fetchFn
	});
}
/**
* Exchange a refresh token for an updated access token.
*
* Supports multiple client authentication methods as specified in OAuth 2.1:
* - Automatically selects the best authentication method based on server support
* - Preserves the original refresh token if a new one is not returned
*
* @param authorizationServerUrl - The authorization server's base URL
* @param options - Configuration object containing client info, refresh token, etc.
* @returns Promise resolving to OAuth tokens (preserves original `refresh_token` if not replaced)
* @throws {Error} When token refresh fails or authentication is invalid
*/
async function refreshAuthorization(authorizationServerUrl, { metadata, clientInformation, refreshToken, resource, addClientAuthentication, fetchFn }) {
	return {
		refresh_token: refreshToken,
		...await executeTokenRequest(authorizationServerUrl, {
			metadata,
			tokenRequestParams: new URLSearchParams({
				grant_type: "refresh_token",
				refresh_token: refreshToken
			}),
			clientInformation,
			addClientAuthentication,
			resource,
			fetchFn
		})
	};
}
/**
* Unified token fetching that works with any grant type via {@linkcode OAuthClientProvider.prepareTokenRequest | prepareTokenRequest()}.
*
* This function provides a single entry point for obtaining tokens regardless of the
* OAuth grant type. The provider's `prepareTokenRequest()` method determines which grant
* to use and supplies the grant-specific parameters.
*
* @param provider - OAuth client provider that implements `prepareTokenRequest()`
* @param authorizationServerUrl - The authorization server's base URL
* @param options - Configuration for the token request
* @returns Promise resolving to OAuth tokens
* @throws {Error} When provider doesn't implement `prepareTokenRequest` or token fetch fails
*
* @example
* ```ts source="./auth.examples.ts#fetchToken_clientCredentials"
* // Provider for client_credentials:
* class MyProvider extends MyProviderBase implements OAuthClientProvider {
*     prepareTokenRequest(scope?: string) {
*         const params = new URLSearchParams({ grant_type: 'client_credentials' });
*         if (scope) params.set('scope', scope);
*         return params;
*     }
* }
*
* const tokens = await fetchToken(new MyProvider(), authServerUrl, { metadata });
* ```
*/
async function fetchToken(provider, authorizationServerUrl, { metadata, resource, authorizationCode, iss, scope, fetchFn } = {}) {
	if (authorizationCode !== void 0) validateAuthorizationResponseIssuer({
		iss,
		expectedIssuer: metadata?.issuer,
		issParameterSupported: isIssParameterSupported(metadata)
	});
	const effectiveScope = scope ?? provider.clientMetadata.scope;
	let tokenRequestParams;
	if (provider.prepareTokenRequest) tokenRequestParams = await provider.prepareTokenRequest(effectiveScope);
	if (!tokenRequestParams) {
		if (!authorizationCode) throw new Error("Either provider.prepareTokenRequest() or authorizationCode is required");
		if (!provider.redirectUrl) throw new Error("redirectUrl is required for authorization_code flow");
		tokenRequestParams = prepareAuthorizationCodeRequest(authorizationCode, await provider.codeVerifier(), provider.redirectUrl);
	}
	const clientInformation = await provider.clientInformation({ issuer: metadata?.issuer ?? String(authorizationServerUrl) });
	return executeTokenRequest(authorizationServerUrl, {
		metadata,
		tokenRequestParams,
		clientInformation: clientInformation ?? void 0,
		addClientAuthentication: provider.addClientAuthentication,
		resource,
		fetchFn
	});
}
/**
* Performs OAuth 2.0 Dynamic Client Registration according to
* {@link https://datatracker.ietf.org/doc/html/rfc7591 | RFC 7591}.
*
* If `scope` is provided, it overrides `clientMetadata.scope` in the registration
* request body. This allows callers to apply the Scope Selection Strategy (SEP-835)
* consistently across both DCR and the subsequent authorization request.
*
* @deprecated Dynamic Client Registration is deprecated as of protocol version
* 2026-07-28 (SEP-2577) in favor of Client ID Metadata Documents (SEP-991).
* Remains functional during the deprecation window (at least twelve months).
* Prefer a CIMD URL `client_id` when the authorization server advertises
* `client_id_metadata_document_supported`; the SDK already gates on this for you.
*/
async function registerClient(authorizationServerUrl, { metadata, clientMetadata, scope, fetchFn }) {
	let registrationUrl;
	if (metadata) {
		if (!metadata.registration_endpoint) throw new Error("Incompatible auth server: does not support dynamic client registration");
		registrationUrl = new URL(metadata.registration_endpoint);
	} else registrationUrl = new URL("/register", authorizationServerUrl);
	const submittedMetadata = {
		...clientMetadata,
		...scope === void 0 ? {} : { scope }
	};
	const response = await (fetchFn ?? fetch)(registrationUrl, {
		method: "POST",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify(submittedMetadata)
	});
	if (!response.ok) throw new RegistrationRejectedError({
		status: response.status,
		body: await response.text(),
		submittedMetadata
	});
	return OAuthClientInformationFullSchema.parse(await response.json());
}

//#endregion
//#region src/client/authExtensions.ts
/**
* Helper to produce a `private_key_jwt` client authentication function.
*
* @example
* ```ts source="./authExtensions.examples.ts#createPrivateKeyJwtAuth_basicUsage"
* const addClientAuth = createPrivateKeyJwtAuth({
*     issuer: 'my-client',
*     subject: 'my-client',
*     privateKey: pemEncodedPrivateKey,
*     alg: 'RS256'
* });
* // pass addClientAuth as provider.addClientAuthentication implementation
* ```
*/
function createPrivateKeyJwtAuth(options) {
	return async (_headers, params, url, metadata) => {
		if (globalThis.crypto === void 0) throw new TypeError("crypto is not available, please ensure you have Web Crypto API support for older Node.js versions (see https://github.com/modelcontextprotocol/typescript-sdk#nodejs-web-crypto-globalthiscrypto-compatibility)");
		const jose = await import("jose");
		const audience = String(options.audience ?? metadata?.issuer ?? url);
		const lifetimeSeconds = options.lifetimeSeconds ?? 300;
		const now = Math.floor(Date.now() / 1e3);
		const jti = `${Date.now()}-${Math.random().toString(36).slice(2)}`;
		const baseClaims = {
			iss: options.issuer,
			sub: options.subject,
			aud: audience,
			exp: now + lifetimeSeconds,
			iat: now,
			jti
		};
		const claims = options.claims ? {
			...baseClaims,
			...options.claims
		} : baseClaims;
		const alg = options.alg;
		let key;
		if (typeof options.privateKey === "string") if (alg.startsWith("RS") || alg.startsWith("ES") || alg.startsWith("PS")) key = await jose.importPKCS8(options.privateKey, alg);
		else if (alg.startsWith("HS")) key = new TextEncoder().encode(options.privateKey);
		else throw new Error(`Unsupported algorithm ${alg}`);
		else if (options.privateKey instanceof Uint8Array) key = alg.startsWith("HS") ? options.privateKey : await jose.importPKCS8(new TextDecoder().decode(options.privateKey), alg);
		else key = await jose.importJWK(options.privateKey, alg);
		const assertion = await new jose.SignJWT(claims).setProtectedHeader({
			alg,
			typ: "JWT"
		}).setIssuer(options.issuer).setSubject(options.subject).setAudience(audience).setIssuedAt(now).setExpirationTime(now + lifetimeSeconds).setJti(jti).sign(key);
		params.set("client_assertion", assertion);
		params.set("client_assertion_type", "urn:ietf:params:oauth:client-assertion-type:jwt-bearer");
	};
}
/**
* OAuth provider for `client_credentials` grant with `client_secret_basic` authentication.
*
* This provider is designed for machine-to-machine authentication where
* the client authenticates using a `client_id` and `client_secret`.
*
* @example
* ```ts source="./authExtensions.examples.ts#ClientCredentialsProvider_basicUsage"
* const provider = new ClientCredentialsProvider({
*     clientId: 'my-client',
*     clientSecret: 'my-secret'
* });
*
* const transport = new StreamableHTTPClientTransport(serverUrl, {
*     authProvider: provider
* });
* ```
*/
var ClientCredentialsProvider = class {
	_tokens;
	_clientInfo;
	_clientMetadata;
	constructor(options) {
		this._clientInfo = {
			client_id: options.clientId,
			client_secret: options.clientSecret,
			issuer: options.expectedIssuer
		};
		this._clientMetadata = {
			client_name: options.clientName ?? "client-credentials-client",
			redirect_uris: [],
			grant_types: ["client_credentials"],
			token_endpoint_auth_method: "client_secret_basic",
			scope: options.scope
		};
	}
	get redirectUrl() {}
	get clientMetadata() {
		return this._clientMetadata;
	}
	clientInformation() {
		return this._clientInfo;
	}
	tokens() {
		return this._tokens;
	}
	saveTokens(tokens) {
		this._tokens = tokens;
	}
	redirectToAuthorization() {
		throw new Error("redirectToAuthorization is not used for client_credentials flow");
	}
	saveCodeVerifier() {}
	codeVerifier() {
		throw new Error("codeVerifier is not used for client_credentials flow");
	}
	prepareTokenRequest(scope) {
		const params = new URLSearchParams({ grant_type: "client_credentials" });
		if (scope) params.set("scope", scope);
		return params;
	}
};
/**
* OAuth provider for `client_credentials` grant with `private_key_jwt` authentication.
*
* This provider is designed for machine-to-machine authentication where
* the client authenticates using a signed JWT assertion
* ({@link https://datatracker.ietf.org/doc/html/rfc7523#section-2.2 | RFC 7523 Section 2.2}).
*
* @example
* ```ts source="./authExtensions.examples.ts#PrivateKeyJwtProvider_basicUsage"
* const provider = new PrivateKeyJwtProvider({
*     clientId: 'my-client',
*     privateKey: pemEncodedPrivateKey,
*     algorithm: 'RS256'
* });
*
* const transport = new StreamableHTTPClientTransport(serverUrl, {
*     authProvider: provider
* });
* ```
*/
var PrivateKeyJwtProvider = class {
	_tokens;
	_clientInfo;
	_clientMetadata;
	addClientAuthentication;
	constructor(options) {
		this._clientInfo = {
			client_id: options.clientId,
			issuer: options.expectedIssuer
		};
		this._clientMetadata = {
			client_name: options.clientName ?? "private-key-jwt-client",
			redirect_uris: [],
			grant_types: ["client_credentials"],
			token_endpoint_auth_method: "private_key_jwt",
			scope: options.scope
		};
		this.addClientAuthentication = createPrivateKeyJwtAuth({
			issuer: options.clientId,
			subject: options.clientId,
			privateKey: options.privateKey,
			alg: options.algorithm,
			lifetimeSeconds: options.jwtLifetimeSeconds,
			claims: options.claims
		});
	}
	get redirectUrl() {}
	get clientMetadata() {
		return this._clientMetadata;
	}
	clientInformation() {
		return this._clientInfo;
	}
	tokens() {
		return this._tokens;
	}
	saveTokens(tokens) {
		this._tokens = tokens;
	}
	redirectToAuthorization() {
		throw new Error("redirectToAuthorization is not used for client_credentials flow");
	}
	saveCodeVerifier() {}
	codeVerifier() {
		throw new Error("codeVerifier is not used for client_credentials flow");
	}
	prepareTokenRequest(scope) {
		const params = new URLSearchParams({ grant_type: "client_credentials" });
		if (scope) params.set("scope", scope);
		return params;
	}
};
/**
* OAuth provider for `client_credentials` grant with a static `private_key_jwt` assertion.
*
* This provider mirrors {@linkcode PrivateKeyJwtProvider} but instead of constructing and
* signing a JWT on each request, it accepts a pre-built JWT assertion string and
* uses it directly for authentication.
*/
var StaticPrivateKeyJwtProvider = class {
	_tokens;
	_clientInfo;
	_clientMetadata;
	addClientAuthentication;
	constructor(options) {
		this._clientInfo = {
			client_id: options.clientId,
			issuer: options.expectedIssuer
		};
		this._clientMetadata = {
			client_name: options.clientName ?? "static-private-key-jwt-client",
			redirect_uris: [],
			grant_types: ["client_credentials"],
			token_endpoint_auth_method: "private_key_jwt",
			scope: options.scope
		};
		const assertion = options.jwtBearerAssertion;
		this.addClientAuthentication = async (_headers, params) => {
			params.set("client_assertion", assertion);
			params.set("client_assertion_type", "urn:ietf:params:oauth:client-assertion-type:jwt-bearer");
		};
	}
	get redirectUrl() {}
	get clientMetadata() {
		return this._clientMetadata;
	}
	clientInformation() {
		return this._clientInfo;
	}
	tokens() {
		return this._tokens;
	}
	saveTokens(tokens) {
		this._tokens = tokens;
	}
	redirectToAuthorization() {
		throw new Error("redirectToAuthorization is not used for client_credentials flow");
	}
	saveCodeVerifier() {}
	codeVerifier() {
		throw new Error("codeVerifier is not used for client_credentials flow");
	}
	prepareTokenRequest(scope) {
		const params = new URLSearchParams({ grant_type: "client_credentials" });
		if (scope) params.set("scope", scope);
		return params;
	}
};
/**
* OAuth provider for Cross-App Access (Enterprise Managed Authorization) using JWT Authorization Grant.
*
* This provider implements the Enterprise Managed Authorization flow (SEP-990) where:
* 1. User authenticates with an enterprise IdP and the client obtains an ID Token
* 2. Client exchanges the ID Token for a JWT Authorization Grant (ID-JAG) via RFC 8693 token exchange
* 3. Client uses the JAG to obtain an access token from the MCP server via RFC 7523 JWT bearer grant
*
* The provider handles steps 2-3 automatically, with the JAG acquisition delegated to
* a callback function that you provide. This allows flexibility in how you obtain and
* cache ID Tokens from the IdP.
*
* @see https://github.com/modelcontextprotocol/ext-auth/blob/main/specification/stable/enterprise-managed-authorization.mdx
*
* @example
* ```ts
* const provider = new CrossAppAccessProvider({
*     assertion: async (ctx) => {
*         const result = await discoverAndRequestJwtAuthGrant({
*             idpUrl: 'https://idp.example.com',
*             audience: ctx.authorizationServerUrl,
*             resource: ctx.resourceUrl,
*             idToken: await getIdToken(), // Your function to get ID token
*             clientId: 'my-idp-client',
*             clientSecret: 'my-idp-secret',
*             scope: ctx.scope,
*             fetchFn: ctx.fetchFn
*         });
*         return result.jwtAuthGrant;
*     },
*     clientId: 'my-mcp-client',
*     clientSecret: 'my-mcp-secret'
* });
*
* const transport = new StreamableHTTPClientTransport(serverUrl, {
*     authProvider: provider
* });
* ```
*/
var CrossAppAccessProvider = class {
	_tokens;
	_clientInfo;
	_clientMetadata;
	_assertionCallback;
	_fetchFn;
	_authorizationServerUrl;
	_resourceUrl;
	_scope;
	constructor(options) {
		this._clientInfo = {
			client_id: options.clientId,
			client_secret: options.clientSecret,
			issuer: options.expectedIssuer
		};
		this._clientMetadata = {
			client_name: options.clientName ?? "cross-app-access-client",
			redirect_uris: [],
			grant_types: ["urn:ietf:params:oauth:grant-type:jwt-bearer"],
			token_endpoint_auth_method: "client_secret_basic"
		};
		this._assertionCallback = options.assertion;
		this._fetchFn = options.fetchFn ?? fetch;
	}
	get redirectUrl() {}
	get clientMetadata() {
		return this._clientMetadata;
	}
	clientInformation() {
		return this._clientInfo;
	}
	tokens() {
		return this._tokens;
	}
	saveTokens(tokens) {
		this._tokens = tokens;
	}
	redirectToAuthorization() {
		throw new Error("redirectToAuthorization is not used for jwt-bearer flow");
	}
	saveCodeVerifier() {}
	codeVerifier() {
		throw new Error("codeVerifier is not used for jwt-bearer flow");
	}
	/**
	* Saves the authorization server URL discovered during OAuth flow.
	* This is called by the auth() function after RFC 9728 discovery.
	*/
	saveAuthorizationServerUrl(authorizationServerUrl) {
		this._authorizationServerUrl = authorizationServerUrl;
	}
	/**
	* Returns the cached authorization server URL if available.
	*/
	authorizationServerUrl() {
		return this._authorizationServerUrl;
	}
	/**
	* Saves the resource URL discovered during OAuth flow.
	* This is called by the auth() function after RFC 9728 discovery.
	*/
	saveResourceUrl(resourceUrl) {
		this._resourceUrl = resourceUrl;
	}
	/**
	* Returns the cached resource URL if available.
	*/
	resourceUrl() {
		return this._resourceUrl;
	}
	async prepareTokenRequest(scope) {
		const authServerUrl = this._authorizationServerUrl;
		const resourceUrl = this._resourceUrl;
		if (!authServerUrl) throw new Error("Authorization server URL not available. Ensure auth() has been called first.");
		if (!resourceUrl) throw new Error("Resource URL not available — server may not implement RFC 9728 Protected Resource Metadata (required for Cross-App Access), or auth() has not been called");
		this._scope = scope;
		const jwtAuthGrant = await this._assertionCallback({
			authorizationServerUrl: authServerUrl,
			resourceUrl,
			scope: this._scope,
			fetchFn: this._fetchFn
		});
		const params = new URLSearchParams({
			grant_type: "urn:ietf:params:oauth:grant-type:jwt-bearer",
			assertion: jwtAuthGrant
		});
		if (scope) params.set("scope", scope);
		return params;
	}
};

//#endregion
//#region src/client/responseCache.ts
/**
* Methods whose entries are exempt from the
* {@linkcode InMemoryResponseCacheStoreOptions.maxEntries} cap. Each holds at
* most one entry per partition (a small bounded set) and the
* `tools/list`-derived index depends on its entry surviving regardless of the
* `resources/read` working-set size. Only `resources/read` keys count toward
* the cap and are eligible for FIFO eviction.
*/
const CAP_EXEMPT_METHODS = new Set([
	"tools/list",
	"prompts/list",
	"resources/list",
	"resources/templates/list",
	"server/discover"
]);
/**
* In-memory default. Bounded by an insertion-ordered size cap (default `512`;
* see {@linkcode InMemoryResponseCacheStoreOptions.maxEntries}) on the
* `resources/read` keyspace so an unbounded stream of distinct URIs cannot
* grow it without limit; the list-singleton methods are exempt and never
* evicted by the cap. `Map` preserves insertion order, so the oldest live
* capped key is the first matching iteration entry.
*/
var InMemoryResponseCacheStore = class {
	_entries = /* @__PURE__ */ new Map();
	_maxEntries;
	_stamp = 0;
	/** Count of held entries that are subject to the cap (i.e. not in {@linkcode CAP_EXEMPT_METHODS}). */
	_cappedSize = 0;
	constructor(options) {
		this._maxEntries = options?.maxEntries ?? 512;
	}
	/** Number of held entries (for diagnostics / bounding tests). */
	get size() {
		return this._entries.size;
	}
	get(key) {
		return this._entries.get(keyOf(key));
	}
	set(key, entry) {
		const k = keyOf(key);
		const exempt = CAP_EXEMPT_METHODS.has(key.method);
		const isNew = !this._entries.has(k);
		if (!exempt && isNew && this._maxEntries > 0 && this._cappedSize >= this._maxEntries) {
			for (const oldKey of this._entries.keys()) if (!CAP_EXEMPT_METHODS.has(oldKey.slice(0, oldKey.indexOf("\0")))) {
				this._entries.delete(oldKey);
				this._cappedSize--;
				break;
			}
		}
		const stamp = ++this._stamp;
		this._entries.set(k, {
			...entry,
			stamp
		});
		if (isNew && !exempt) this._cappedSize++;
		return stamp;
	}
	delete(key) {
		if (this._entries.delete(keyOf(key)) && !CAP_EXEMPT_METHODS.has(key.method)) this._cappedSize--;
	}
	evict(method) {
		const prefix = `${method}\0`;
		const exempt = CAP_EXEMPT_METHODS.has(method);
		for (const k of this._entries.keys()) if (k.startsWith(prefix)) {
			this._entries.delete(k);
			if (!exempt) this._cappedSize--;
		}
	}
	clear() {
		this._entries.clear();
		this._cappedSize = 0;
	}
};
/**
* Serialize a {@linkcode CacheKey} for the in-memory map. `method` is always
* an SDK-set MCP method string (never contains a NUL), so the `\0` prefix
* delimiter is safe and lets {@linkcode InMemoryResponseCacheStore.evict} do a
* cheap prefix scan. `partition` (already a JSON-encoded
* `[serverIdentity, principal]` pair) and `params` (a resource URI on the
* `resources/read` path — caller-controlled) are JSON-array-encoded together,
* which is collision-free regardless of any NUL or delimiter characters they
* carry.
*/
function keyOf(key) {
	return `${key.method}\0${JSON.stringify([key.partition ?? "", key.params ?? ""])}`;
}
/**
* Serialize a `{method, params}` pair for the eviction-generation map. The
* list singletons key on `method` alone (their {@linkcode ClientResponseCache.evict}
* is whole-method); `resources/read` keys on `` `${method}\0${uri}` `` so
* {@linkcode ClientResponseCache.evictKey} bumps a per-URI counter.
*/
function genKey(method, params) {
	return params === void 0 ? method : `${method}\0${params}`;
}
/**
* Upper bound on the server-supplied `ttlMs` honoured by
* {@linkcode ClientResponseCache} (24h). A server cannot pin an entry
* indefinitely.
*/
const MAX_CACHE_TTL_MS = 864e5;
/**
* The `Client`'s cache-coordination collaborator.
*
* Owns the per-connection cache state that used to live as five private
* fields on `Client` — the backing {@linkcode ResponseCacheStore}, the
* per-method eviction-generation counter, the user-supplied/default flag, and
* the stamp-memoized derived indices over the `tools/list` entry. `Client`
* holds exactly one instance and never reaches past it to the store.
*
* Not exported from the package index — internal to the client package.
*
* @internal
*/
var ClientResponseCache = class {
	/**
	* Per-logical-key eviction-generation counter. {@linkcode evict} (whole
	* method) and {@linkcode evictKey} (single `{method, params}`) bump it
	* before touching the store; {@linkcode captureGeneration} reads it before
	* the request; {@linkcode write} skips when it moved — so a `list_changed`
	* arriving mid-walk, or a `resources/updated` arriving while a
	* `readResource()` for the same URI is in flight, is not overwritten by
	* the in-flight request's stale write. The map key is `method` for the
	* list singletons and `` `${method}\0${params}` `` for per-URI keys.
	*
	* Growth is bounded by keys the CLIENT has issued a `captureGeneration`
	* for: {@linkcode captureGeneration} records the key (so an interleaved
	* {@linkcode evictKey} sees there is an in-flight write to suppress);
	* {@linkcode evictKey} only bumps a key that is already recorded — a
	* server streaming `notifications/resources/updated` for URIs the client
	* has never read therefore cannot grow this map.
	*/
	_evictionGeneration = /* @__PURE__ */ new Map();
	/**
	* `name → Tool` index derived from the cached `tools/list` entry, memoized
	* against the entry's `stamp` so it re-derives only when the backing entry
	* changes (mcp.d's `cachedTool` pattern).
	*/
	_toolIndex;
	/**
	* `name → compiled output-schema validator` derived from the cached
	* `tools/list` entry; same stamp-keyed memoization as `_toolIndex`. Typed
	* `unknown` so this class stays free of any validator-provider dependency
	* — the compile callback supplied to {@linkcode outputValidator} owns the
	* concrete type.
	*/
	_toolOutputValidatorIndex;
	/**
	* The connected server's identity (`serverInfo.name@version`, the
	* transport's `sessionId`, or a client-generated per-connection
	* surrogate). Set by the `Client` immediately after a successful connect;
	* `''` is the pre-connect sentinel. Every storage partition is derived
	* from this (see `_partitionFor`), so two clients sharing one store but
	* connected to different servers never collide on `tools/list` and a
	* server cannot read another server's `'public'` entries.
	*/
	_serverIdentity = "";
	constructor(_store, _isUserSupplied, _reportError = () => {}, _cachePartition = "", _now = Date.now) {
		this._store = _store;
		this._isUserSupplied = _isUserSupplied;
		this._reportError = _reportError;
		this._cachePartition = _cachePartition;
		this._now = _now;
	}
	/** The clock used for every freshness computation and check. */
	now() {
		return this._now();
	}
	/**
	* Record the connected server's identity. Called by `Client` immediately
	* after a successful connect: `serverInfo.name@version` when the server
	* identified itself, else the transport's `sessionId`, else a
	* client-generated per-connection surrogate (`serverInfo` is a spec
	* SHOULD on 2026-07-28, so anonymous servers exist). Surrogate-keyed
	* partitions are NOT stable across reconnects — no identity means no
	* cross-connection cache reuse, and a shared long-lived store should
	* bound its own size accordingly. Every partition derived after this
	* call is scoped to this identity; entries written under the pre-connect
	* `''` sentinel are no longer reachable.
	*/
	setServerIdentity(identity) {
		this._serverIdentity = identity;
	}
	/**
	* Derive the storage partition for `scope`. The encoding is
	* `JSON.stringify([serverIdentity, principal])` — JSON escaping makes it
	* collision-free by construction: a malicious server cannot craft a
	* `serverInfo.name`/`version` whose concatenated form bleeds into another
	* server's namespace or another principal's slot, regardless of `@` / `|`
	* / `"` / NUL in the server-controlled strings. `'public'` →
	* `[serverIdentity, '']` (shared within this server); `'private'` →
	* `[serverIdentity, cachePartition]`. When `cachePartition` is `''` the
	* two coincide.
	*/
	_partitionFor(scope) {
		return JSON.stringify([this._serverIdentity, scope === "public" ? "" : this._cachePartition]);
	}
	/**
	* Two-probe lookup: this client's own (private) partition first, then the
	* connected server's shared (public) partition. The shared probe is gated
	* on `entry.scope === 'public'` — a co-tenant client that omits
	* `cachePartition` writes its `'private'`-scoped entries at the public
	* partition, and serving those to a correctly-partitioned client would
	* leak private bodies (mcp.d's `cachedEntry` two-probe order; the scope
	* gate is defence-in-depth on top of the partition split). When
	* `cachePartition` is `''` the two partitions are identical and only one
	* probe is issued.
	*/
	async _probe(method, params) {
		const key = {
			method,
			params: params ?? ""
		};
		const ownPartition = this._partitionFor("private");
		const own = await this._store.get({
			...key,
			partition: ownPartition
		});
		if (own !== void 0) return own;
		const sharedPartition = this._partitionFor("public");
		if (sharedPartition === ownPartition) return void 0;
		const shared = await this._store.get({
			...key,
			partition: sharedPartition
		});
		return shared?.scope === "public" ? shared : void 0;
	}
	/**
	* Bump the per-method generation (so an in-flight {@linkcode write} for the
	* same method becomes a no-op) and drop the connected server's two list
	* singletons (own + shared partition; `params: ''`). The generation bump
	* is unconditional and FIRST — the {@linkcode write} race guard relies on
	* the bump, not on the store's deletes completing.
	*
	* Eviction is scoped to this client's `[serverIdentity, principal]`
	* partitions (mirroring {@linkcode evictKey}) — the method-wide
	* `store.evict()` is NOT called, so on a shared store one server's
	* `list_changed` cannot wipe a co-tenant's entry. A custom store's
	* `delete()` may throw or reject; each partition is guarded
	* independently so a failure on one does not skip the other, the failure
	* is reported via the constructor's sink, and the call resolves so
	* dispatch proceeds.
	*/
	async evict(method) {
		this._evictionGeneration.set(method, (this._evictionGeneration.get(method) ?? 0) + 1);
		await this._deleteBoth(method, "");
	}
	/**
	* Guarded two-partition delete of `{method, params}`: each partition's
	* `delete` is independently wrapped so a custom store's failure on one is
	* reported and does not skip the other, and the call always resolves.
	*/
	async _deleteBoth(method, params) {
		const ownPartition = this._partitionFor("private");
		const sharedPartition = this._partitionFor("public");
		try {
			await this._store.delete({
				method,
				params,
				partition: ownPartition
			});
		} catch (error) {
			this._reportError(error);
		}
		if (sharedPartition !== ownPartition) try {
			await this._store.delete({
				method,
				params,
				partition: sharedPartition
			});
		} catch (error) {
			this._reportError(error);
		}
	}
	/**
	* Drop the single logical entry `{method, params}` from BOTH the private
	* and public partitions for this client's connected server (mcp.d's
	* `invalidateLogical`). Used for `notifications/resources/updated`'s
	* per-URI eviction. The per-key generation is bumped FIRST (so an
	* in-flight {@linkcode write} for the same `{method, params}` becomes a
	* no-op and cannot re-cache the now-stale body) but only when the key was
	* already recorded by {@linkcode captureGeneration} — bounding the map to
	* keys the client has actually read. A custom store's `delete()` may
	* throw or reject; each partition's delete is guarded independently so a
	* failure on one does not skip the other, and the call resolves so
	* dispatch proceeds.
	*/
	async evictKey(method, params) {
		const gk = genKey(method, params);
		const current = this._evictionGeneration.get(gk);
		if (current !== void 0) this._evictionGeneration.set(gk, current + 1);
		await this._deleteBoth(method, params);
	}
	/**
	* Snapshot the eviction generation for `{method, params}` before issuing
	* the request (a list walk's page 1, or a `resources/read` for `params`).
	* Records the key so an interleaved {@linkcode evictKey} for the same
	* `{method, params}` knows there is an in-flight write to suppress and
	* bumps; without the record, `evictKey`'s recorded-only bump would skip
	* and the stale body would be cached.
	*/
	captureGeneration(method, params) {
		const gk = genKey(method, params);
		const current = this._evictionGeneration.get(gk) ?? 0;
		this._evictionGeneration.set(gk, current);
		return current;
	}
	/**
	* Write `value` under `{method}` unless the per-method generation moved
	* since `capturedGen` was taken — a `list_changed` that landed mid-walk has
	* already invalidated the result the caller is about to write, and
	* overwriting the eviction with the stale aggregate would lose the
	* invalidation.
	*
	* The value is stored as its JSON-serialized document; serialization
	* doubles as the mutation barrier, so a caller mutating the returned
	* aggregate cannot reach the cache or its derived indices. A value that
	* is not JSON-serializable (reachable only via in-process transports)
	* fails the write loudly into the `reportError` sink. A custom store
	* whose `set()` throws or rejects is routed to the same sink and the
	* write resolves — cache bookkeeping never costs the caller a result it
	* already fetched.
	*
	* `freshness` carries the client-computed `expiresAt` (absolute ms epoch,
	* `now + ttlMs`) and the server-reported `cacheScope`. The storage
	* `partition` is derived from the scope via `_partitionFor`:
	* `'public'` → `[serverIdentity, '']` (shared within this server);
	* `'private'` → `[serverIdentity, cachePartition]` (so a shared store
	* never serves a private entry to another identity). Absent `freshness`
	* preserves the substrate write (no `expiresAt`, private partition) — the
	* `tools/list` retain-for-schema posture: never served by
	* {@linkcode read}'s freshness gate, always readable by
	* {@linkcode toolDefinition}.
	*
	* After storing under the derived partition, the same `{method, params}`
	* is deleted from the OPPOSITE partition (mirroring {@linkcode evictKey}'s
	* two-partition posture). A server that flips a result's `cacheScope` for
	* the same key would otherwise leave the previous entry in the other slot
	* — and since `_probe` checks own-partition first, a stale private entry
	* would shadow the fresh public one (or a stale public entry would keep
	* serving co-tenants). Both store calls are independently guarded so a
	* custom store's failure on one does not skip the other.
	*/
	async write(method, value, capturedGen, freshness) {
		if ((this._evictionGeneration.get(genKey(method, freshness?.params)) ?? 0) !== capturedGen) return;
		const params = freshness?.params ?? "";
		const ownPartition = this._partitionFor("private");
		const sharedPartition = this._partitionFor("public");
		const partition = (freshness?.scope ?? "private") === "public" ? sharedPartition : ownPartition;
		try {
			await this._store.set({
				method,
				params,
				partition
			}, {
				value: encodeCacheValue(value),
				expiresAt: freshness?.expiresAt,
				scope: freshness?.scope
			});
		} catch (error) {
			this._reportError(error);
		}
		if (sharedPartition !== ownPartition) try {
			await this._store.delete({
				method,
				params,
				partition: partition === ownPartition ? sharedPartition : ownPartition
			});
		} catch (error) {
			this._reportError(error);
		}
	}
	/**
	* Serve the fresh cached result for `{method, params}`, or `undefined`.
	* Lookup is the two-probe order (own-partition then this server's shared
	* partition, gated on `scope === 'public'`); freshness is
	* `entry.expiresAt > now()` (a missing `expiresAt` is never fresh),
	* checked BEFORE decoding so stale entries cost no parse. Every hit is
	* freshly parsed, so the caller owns the value outright. An entry whose
	* document does not parse or is not an object (corrupted external
	* store) is reported,
	* deleted, and treated as a miss — deleted because a fresh-but-corrupt
	* entry would otherwise re-parse and re-report on every read until its
	* `expiresAt` passes.
	*/
	async read(method, params) {
		const entry = await this._probe(method, params);
		if (entry?.expiresAt === void 0 || !(entry.expiresAt > this.now())) return void 0;
		try {
			const parsed = JSON.parse(entry.value);
			if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) throw new TypeError("cached document is not an object");
			return { value: parsed };
		} catch (error) {
			this._reportError(error);
			await this._deleteBoth(method, params ?? "");
			return;
		}
	}
	/**
	* Connection reset. The per-instance default store IS cleared
	* (connection-scoped); a user-supplied store is NOT — that would defeat
	* the only reason to supply one. The generation map and every derived
	* index are dropped regardless: they are connection-scoped even when the
	* backing store survives, so the next read re-derives from whatever the
	* store still holds. The server identity returns to the pre-connect
	* sentinel. The default impl is synchronous, so the `MaybePromise<void>`
	* return is a plain void here and the caller need not await.
	*/
	resetForReconnect() {
		if (!this._isUserSupplied) this._store.clear();
		this._evictionGeneration.clear();
		this._toolIndex = void 0;
		this._toolOutputValidatorIndex = void 0;
		this._serverIdentity = "";
	}
	/**
	* The descriptor for tool `name` taken from the cached `tools/list` entry.
	* The `name → Tool` index is memoized against the entry's `stamp` and
	* re-derived only when the backing entry changes (mcp.d's `cachedTool`).
	* Returns `undefined` only when no `tools/list` response is held at all,
	* or the held list does not contain `name`.
	*
	* Consumed by `callTool()`'s SEP-2243 `_resolveXMcpHeaderScan` (mirroring)
	* and, via {@linkcode outputValidator}, its output-schema validation.
	*/
	async toolDefinition(name) {
		const entry = await this._probe("tools/list");
		if (entry === void 0) {
			this._toolIndex = void 0;
			return;
		}
		if (this._toolIndex?.stamp !== entry.stamp) {
			const list = this._decodeListTools(entry);
			const byName = /* @__PURE__ */ new Map();
			if (list !== void 0) for (const tool of list.tools) byName.set(tool.name, tool);
			this._toolIndex = {
				stamp: entry.stamp,
				byName
			};
		}
		return this._toolIndex.byName.get(name);
	}
	/**
	* The compiled output-schema validator for tool `name`, derived from the
	* cached `tools/list` entry — same source and same stamp-keyed
	* memoization as {@linkcode toolDefinition}. The `name → validator` index
	* re-derives only when the backing entry's stamp changes (a refetched
	* `tools/list` recompiles; a `list_changed` eviction drops it). Returns
	* `undefined` when no `tools/list` is held, the tool is absent, or it has
	* no `outputSchema`.
	*
	* `compile` is the caller-supplied validator-compile callback (the
	* `Client` passes its `_jsonSchemaValidator` wrapper) so this
	* class carries no validator-provider dependency. One tool's uncompilable
	* `outputSchema` (e.g. an invalid `pattern` regex or unresolvable `$ref`)
	* must not poison every other tool's `callTool` — the callback isolates
	* that compile error per tool by returning a per-tool error variant which
	* the index stores alongside the good ones, and `callTool` surfaces it as
	* a typed `InvalidParams` only for that name. Because the error is held on
	* this stamp-keyed substrate (not a parallel map), it inherits the
	* substrate's invalidation lifecycle: a `list_changed` eviction drops it,
	* a refetched `tools/list` re-derives it, and `resetForReconnect` clears
	* the lot.
	*/
	async outputValidator(name, compile) {
		const entry = await this._probe("tools/list");
		if (entry === void 0) {
			this._toolOutputValidatorIndex = void 0;
			return;
		}
		if (this._toolOutputValidatorIndex?.stamp !== entry.stamp) {
			const list = this._decodeListTools(entry) ?? { tools: [] };
			const byName = /* @__PURE__ */ new Map();
			for (const tool of list.tools) {
				const compiled = compile(tool);
				if (compiled !== void 0) byName.set(tool.name, compiled);
			}
			this._toolOutputValidatorIndex = {
				stamp: entry.stamp,
				byName
			};
		}
		return this._toolOutputValidatorIndex.byName.get(name);
	}
	/** Parse a held `tools/list` document for the index builders; a document
	* that does not parse OR whose `tools` is not an array of objects
	* (both mean a corrupted external store) is reported and treated as if
	* nothing were held. Callers memoize the outcome against the entry's
	* stamp, so a corrupt document costs one parse + report per stamp, not
	* per lookup. */
	_decodeListTools(entry) {
		try {
			const parsed = JSON.parse(entry.value);
			if (!Array.isArray(parsed?.tools) || !parsed.tools.every((t) => t !== null && typeof t === "object")) throw new TypeError("cached tools/list document has a malformed tools array");
			return parsed;
		} catch (error) {
			this._reportError(error);
			return;
		}
	}
};
/**
* Serialize a result for storage; throws TypeError for anything without a
* JSON representation (legal wire results always have one). Two failure
* modes need folding into that one throw: `JSON.stringify` throws on cycles
* and BigInt, but silently returns `undefined` (not a string) for a
* top-level function, symbol, or `undefined`.
*/
function encodeCacheValue(value) {
	let json;
	try {
		json = JSON.stringify(value);
	} catch (error) {
		throw new TypeError(`cache value is not JSON-serializable: ${error instanceof Error ? error.message : String(error)}`);
	}
	if (typeof json !== "string") throw new TypeError("cache value is not JSON-serializable: it has no JSON representation");
	return json;
}

//#endregion
//#region src/client/authSeam.ts
/**
* Auth-seam provenance stamp (internal — not part of the public API).
*
* Errors escaping a transport auth seam — the `authProvider.token()` read,
* an `onUnauthorized` invocation (SDK adapter or custom user callback), the
* 403 step-up flow, the transport's own auth-failure constructions — are
* stamped at the throw boundary. The version-negotiation probe routes stamped
* errors as auth outcomes; provenance is recorded where it is known, never
* reconstructed from error types downstream.
*
* `Symbol.for` uses the global symbol registry, so the stamp survives a
* duplicated SDK copy in one process (bundler double-install, version skew)
* by design: both copies resolve the same symbol.
*/
const AUTH_SEAM = Symbol.for("mcp.authSeamEscape");
/**
* Stamp `error` as an auth-seam escape and return it — identity-preserving
* (the same object flows on, `instanceof` and `.cause` chains intact). A
* frozen/sealed object is returned unstamped rather than replaced: identity
* outranks provenance. Primitive throws cannot carry the stamp.
*/
function markAuthSeamEscape(error) {
	if (typeof error === "object" && error !== null || typeof error === "function") try {
		Object.defineProperty(error, AUTH_SEAM, {
			value: true,
			configurable: true
		});
	} catch {}
	return error;
}
/** Whether `error` escaped through a transport auth seam. */
function isAuthSeamEscape(error) {
	return (typeof error === "object" && error !== null || typeof error === "function") && error[AUTH_SEAM] === true;
}

//#endregion
//#region src/client/probeClassifier.ts
/** The `-32022` UnsupportedProtocolVersion protocol error code (negotiation-phase recognition). */
const UNSUPPORTED_PROTOCOL_VERSION = -32022;
/**
* Deliberately not probe-recognized in either direction: deployed servers
* overload `-32001` (the SDK-conventional `Session not found` body on a 2025
* stateful server), and the spec-assigned `-32020` (`HeaderMismatch`) /
* `-32021` (`MissingRequiredClientCapability`) are not era evidence — all
* fall into the conservative legacy default.
*/
const NOT_PROBE_RECOGNIZED = new Set([
	-32001,
	-32020,
	-32021
]);
/**
* Classify a single probe outcome. Pure: no I/O, no state — loop-guard and
* retry state live in the caller.
*/
function classifyProbeOutcome(outcome, context) {
	switch (outcome.kind) {
		case "result": return classifyResult(outcome.result, context);
		case "rpc-error": return classifyRpcError(outcome, context);
		case "http-error": return classifyHttpError(outcome, context);
		case "network-error": return classifyNetworkError(outcome.error, context);
		case "auth-required": return {
			kind: "error",
			error: outcome.error
		};
		case "closed":
			if (context.transportKind === "stdio") return { kind: "legacy" };
			return classifyNetworkError(/* @__PURE__ */ new Error("Connection closed during the version negotiation probe"), context);
		case "timeout":
			if (context.transportKind === "stdio") return { kind: "legacy" };
			return {
				kind: "error",
				error: new SdkError(SdkErrorCode.RequestTimeout, `Version negotiation probe timed out after ${outcome.timeoutMs}ms`, { timeout: outcome.timeoutMs })
			};
	}
}
function classifyResult(result, context) {
	const parsed = codecForVersion(MODERN_WIRE_REVISION).validateResult("server/discover", result);
	if (!parsed.ok) return { kind: "legacy" };
	const supportedVersions = parsed.value.supportedVersions;
	const overlap = context.clientModernVersions.find((version) => supportedVersions.includes(version));
	if (overlap !== void 0) return {
		kind: "modern",
		version: overlap,
		discover: parsed.value
	};
	if (context.fallbackAvailable) return { kind: "legacy" };
	return {
		kind: "error",
		error: new UnsupportedProtocolVersionError({
			supported: [...supportedVersions],
			requested: context.requestedVersion
		})
	};
}
function classifyRpcError(outcome, context) {
	const { code, message, data } = outcome;
	if (code === UNSUPPORTED_PROTOCOL_VERSION) {
		const supported = parseSupportedList(data);
		if (supported === void 0) return { kind: "legacy" };
		const error = new UnsupportedProtocolVersionError({
			supported,
			requested: parseRequested(data) ?? context.requestedVersion
		}, message);
		const supportedModern = modernProtocolVersions(supported);
		const mutual = context.clientModernVersions.find((version) => supportedModern.includes(version));
		if (mutual !== void 0) return {
			kind: "corrective",
			version: mutual,
			error
		};
		if (supportedModern.length > 0) return {
			kind: "error",
			error
		};
		return context.fallbackAvailable ? { kind: "legacy" } : {
			kind: "error",
			error
		};
	}
	if (NOT_PROBE_RECOGNIZED.has(code)) return { kind: "legacy" };
	return { kind: "legacy" };
}
function classifyHttpError(outcome, context) {
	if (outcome.status === 401 || outcome.status === 403) {
		const isDenial = outcome.status === 403;
		return {
			kind: "error",
			error: new SdkHttpError(isDenial ? SdkErrorCode.ClientHttpForbidden : SdkErrorCode.ClientHttpAuthentication, `Version negotiation failed: ${isDenial ? "the server denied access (HTTP 403)" : "the server requires authorization (HTTP 401)"}`, {
				status: outcome.status,
				statusText: outcome.statusText,
				text: outcome.body
			})
		};
	}
	if (outcome.status >= 500) return {
		kind: "error",
		error: new SdkHttpError(SdkErrorCode.EraNegotiationFailed, `Version negotiation failed: the server answered the probe with HTTP ${outcome.status}`, {
			status: outcome.status,
			statusText: outcome.statusText,
			text: outcome.body
		})
	};
	const rpcError = parseJsonRpcErrorBody(outcome.body);
	if (rpcError !== void 0) return classifyRpcError(rpcError, context);
	return { kind: "legacy" };
}
function classifyNetworkError(error, context) {
	if (context.environment === "browser" && isOpaqueFetchTypeError(error)) return { kind: "legacy" };
	return {
		kind: "error",
		error: new SdkError(SdkErrorCode.EraNegotiationFailed, `Version negotiation probe failed: ${describeError(error)}`, { cause: error })
	};
}
function isOpaqueFetchTypeError(error) {
	return error instanceof TypeError || error instanceof Error && error.name === "TypeError";
}
function describeError(error) {
	return error instanceof Error ? error.message : String(error);
}
function parseSupportedList(data) {
	if (typeof data !== "object" || data === null) return void 0;
	const supported = data.supported;
	if (!Array.isArray(supported) || supported.length === 0 || !supported.every((v) => typeof v === "string")) return;
	return supported;
}
function parseRequested(data) {
	if (typeof data !== "object" || data === null) return void 0;
	const requested = data.requested;
	return typeof requested === "string" ? requested : void 0;
}
function parseJsonRpcErrorBody(body) {
	if (body === void 0 || body === "") return void 0;
	let parsed;
	try {
		parsed = JSON.parse(body);
	} catch {
		return;
	}
	if (typeof parsed !== "object" || parsed === null) return void 0;
	const error = parsed.error;
	if (typeof error !== "object" || error === null) return void 0;
	const { code, message, data } = error;
	if (typeof code !== "number") return void 0;
	return {
		code,
		message: typeof message === "string" ? message : "",
		data
	};
}

//#endregion
//#region src/client/versionNegotiation.ts
/**
* The default mode when `versionNegotiation` (or its `mode`) is absent;
* changing the default later is a flip of this single line.
*/
const DEFAULT_VERSION_NEGOTIATION_MODE = "legacy";
/**
* Resolve the negotiation options into a per-connect plan. The raw (not
* defaulted) `supportedProtocolVersions` option supplies the modern offer list;
* a list without any legacy version makes this a modern-only client (no fallback).
*/
function resolveVersionNegotiation(options, supportedProtocolVersionsOption) {
	const mode = options?.mode ?? DEFAULT_VERSION_NEGOTIATION_MODE;
	if (mode === "legacy") return { kind: "legacy" };
	const probe = options?.probe ?? {};
	if (typeof mode === "object") {
		if (!isModernProtocolVersion(mode.pin)) throw new TypeError(`versionNegotiation: { pin: '${mode.pin}' } is not a modern protocol revision — pinning is for 2026-07-28 and later; omit versionNegotiation (or use mode: 'legacy') for 2025-era servers.`);
		return {
			kind: "pin",
			version: mode.pin,
			probe
		};
	}
	const explicitModern = supportedProtocolVersionsOption ? modernProtocolVersions(supportedProtocolVersionsOption) : [];
	return {
		kind: "auto",
		modernVersions: explicitModern.length > 0 ? explicitModern : [...SUPPORTED_MODERN_PROTOCOL_VERSIONS],
		fallbackAvailable: supportedProtocolVersionsOption ? legacyProtocolVersions(supportedProtocolVersionsOption).length > 0 : true,
		probe
	};
}
/** Detect the probe environment for the network-failure row — see {@linkcode ProbeEnvironment}. */
function detectProbeEnvironment() {
	const g = globalThis;
	return g.window !== void 0 && g.document !== void 0 ? "browser" : "node";
}
/**
* Detect the transport class for the transport-aware timeout and closed verdicts (see
* {@linkcode ProbeTransportKind}). The stdio child-process transport is
* recognized structurally (`stderr`/`pid` accessors, no `instanceof` — safe
* across bundles); everything else is treated like HTTP.
*/
function detectProbeTransportKind(transport) {
	return "stderr" in transport && "pid" in transport ? "stdio" : "http";
}
/**
* Temporary ownership of a raw transport for the negotiation exchange, before
* the Protocol machinery attaches. `open()` installs the window's handlers and
* starts the transport; `release()` detaches them and arms a one-shot `start()`
* pass-through so the subsequent Protocol connect (which always starts its
* transport) takes over the already-started channel without a double-start error.
*
* Handlers a caller pre-set on the transport before `connect()` are saved on
* open and restored on detach, so `Protocol.connect()` finds and chains them
* exactly as it would on a plain (non-negotiating) connect. Error and close
* events are forwarded to the saved handlers during the window, so negotiation
* does not change what a pre-attached observer sees of the transport
* lifecycle. (Inbound messages are deliberately NOT forwarded — the window's
* drop-guard is a module invariant, and the probe reply has no plain-connect
* counterpart; a pre-set `onmessage` is restored at detach and chained by
* `Protocol.connect()` like the others.)
*/
var ProbeWindow = class ProbeWindow {
	_pending;
	_probeCounter = 0;
	_savedOnMessage;
	_savedOnError;
	_savedOnClose;
	_closeDelivered = false;
	constructor(_transport) {
		this._transport = _transport;
		this._savedOnMessage = _transport.onmessage;
		this._savedOnError = _transport.onerror;
		this._savedOnClose = _transport.onclose;
	}
	static async open(transport) {
		const window = new ProbeWindow(transport);
		transport.onmessage = (message) => {
			const pending = window._pending;
			if (pending !== void 0 && (isJSONRPCResultResponse(message) || isJSONRPCErrorResponse(message)) && message.id === pending.id) {
				window._pending = void 0;
				if (isJSONRPCResultResponse(message)) pending.resolve({
					kind: "response",
					result: message.result
				});
				else pending.resolve({
					kind: "response",
					error: message.error
				});
				return;
			}
		};
		transport.onerror = (error) => {
			window._savedOnError?.(error);
		};
		transport.onclose = () => {
			const pending = window._pending;
			if (pending !== void 0) {
				window._pending = void 0;
				pending.resolve({ kind: "closed" });
			}
			window._closeDelivered = true;
			window._savedOnClose?.();
		};
		try {
			await transport.start();
		} catch (error) {
			window.detach();
			throw error;
		}
		return window;
	}
	/**
	* Send one probe request and await its reply. Probe ids are strings, so they
	* never collide with Protocol's numeric ids (e.g. on a shared stdio pipe).
	*/
	async exchange(buildRequest, timeoutMs) {
		const id = `server-discover-probe-${++this._probeCounter}`;
		return new Promise((resolve) => {
			let settled = false;
			const settle = (reply) => {
				if (settled) return;
				settled = true;
				clearTimeout(timer);
				if (this._pending?.id === id) this._pending = void 0;
				resolve(reply);
			};
			const timer = setTimeout(() => settle({ kind: "timeout" }), timeoutMs);
			this._pending = {
				id,
				resolve: settle
			};
			this._transport.send(buildRequest(id)).catch((error) => settle({
				kind: "send-error",
				error
			}));
		});
	}
	/** Detach the window's handlers, restoring any the caller pre-set, leaving the transport's own `start` untouched. */
	detach() {
		this._pending = void 0;
		this._transport.onmessage = this._savedOnMessage;
		this._transport.onerror = this._savedOnError;
		if (this._closeDelivered && this._savedOnClose !== void 0) {
			const saved = this._savedOnClose;
			const transport = this._transport;
			let spent = false;
			const wrapper = () => {
				if (!spent) {
					spent = true;
					return;
				}
				saved();
			};
			transport.onclose = wrapper;
			pendingSpentCloseGuards.set(transport, () => {
				if (transport.onclose === wrapper) transport.onclose = saved;
			});
		} else this._transport.onclose = this._savedOnClose;
	}
	/** Detach the handlers and arm the one-shot `start()` pass-through for the `Protocol.connect()` handover. */
	release() {
		this.detach();
		const transport = this._transport;
		const originalStart = transport.start;
		let armed = true;
		transport.start = async function() {
			if (armed) {
				armed = false;
				transport.start = originalStart;
				return;
			}
			return originalStart.call(transport);
		};
	}
};
/**
* Spent-close guards from failed negotiations, pending disarm by the cleanup
* site (keyed weakly — an abandoned transport carries its guard to GC).
*/
const pendingSpentCloseGuards = /* @__PURE__ */ new WeakMap();
/**
* Disarm a failed negotiation's spent-close guard once the cleanup `close()`
* has settled. Any re-delivery of the spent mid-window close happens DURING
* that close (HTTP transports re-fire `onclose` there; stdio transports never
* do — their close event already fired), so after it every close is genuine
* and must reach the caller's pre-set observer.
*/
function disarmSpentCloseGuard(transport) {
	const disarm = pendingSpentCloseGuards.get(transport);
	pendingSpentCloseGuards.delete(transport);
	disarm?.();
}
/** Build the probe request: `server/discover` carrying the full per-request `_meta` envelope. */
function buildProbeRequest(id, protocolVersion, clientInfo, capabilities) {
	return {
		jsonrpc: "2.0",
		id,
		method: "server/discover",
		params: { _meta: codecForVersion(protocolVersion).outboundEnvelope({
			protocolVersion,
			clientInfo,
			clientCapabilities: capabilities
		}) }
	};
}
function normalizeReply(reply, timeoutMs) {
	switch (reply.kind) {
		case "response": return reply.error === void 0 ? {
			kind: "result",
			result: reply.result
		} : {
			kind: "rpc-error",
			...reply.error
		};
		case "send-error": {
			const error = reply.error;
			if (isAuthSeamEscape(error) || error instanceof UnauthorizedError || error instanceof Error && error.name === "UnauthorizedError") return {
				kind: "auth-required",
				error
			};
			if (error instanceof SdkHttpError) {
				const text = error.data?.text;
				return {
					kind: "http-error",
					status: error.data.status,
					body: typeof text === "string" ? text : void 0,
					statusText: error.data.statusText
				};
			}
			return {
				kind: "network-error",
				error
			};
		}
		case "closed": return { kind: "closed" };
		case "timeout": return {
			kind: "timeout",
			timeoutMs
		};
	}
}
/**
* Run the negotiation probe state machine on a raw (not yet Protocol-connected)
* transport. Resolves with the negotiated era; throws typed connect errors. On
* return the probe window has been released: the transport is started,
* handler-free, and ready for `Protocol.connect()` handover. On throw the
* window is detached and the transport's `start` is left untouched.
*/
async function negotiateEra(negotiation, deps) {
	const timeoutMs = negotiation.probe.timeoutMs ?? deps.defaultTimeoutMs;
	const maxRetries = Math.max(0, negotiation.probe.maxRetries ?? 0);
	const clientModernVersions = negotiation.kind === "pin" ? [negotiation.version] : negotiation.modernVersions;
	const fallbackAvailable = negotiation.kind === "auto" && negotiation.fallbackAvailable;
	const window = await ProbeWindow.open(deps.transport);
	const probe = async () => {
		let requestedVersion = clientModernVersions[0];
		let correctiveUsed = false;
		let timeoutRetriesRemaining = maxRetries;
		for (;;) {
			const reply = await window.exchange((id) => buildProbeRequest(id, requestedVersion, deps.clientInfo, deps.capabilities), timeoutMs);
			if (reply.kind === "timeout" && timeoutRetriesRemaining > 0) {
				timeoutRetriesRemaining--;
				continue;
			}
			const outcome = normalizeReply(reply, timeoutMs);
			const verdict = classifyProbeOutcome(outcome, {
				clientModernVersions,
				requestedVersion,
				fallbackAvailable,
				environment: deps.environment,
				transportKind: deps.transportKind
			});
			switch (verdict.kind) {
				case "modern": return {
					era: "modern",
					version: verdict.version,
					discover: verdict.discover
				};
				case "corrective":
					if (correctiveUsed) throw verdict.error;
					correctiveUsed = true;
					requestedVersion = verdict.version;
					continue;
				case "legacy": {
					const closedCause = outcome.kind === "closed" ? "the connection closed during the server/discover probe" : void 0;
					if (negotiation.kind === "pin") throw new SdkError(SdkErrorCode.EraNegotiationFailed, closedCause === void 0 ? `Version negotiation failed: the server did not offer pinned protocol version ${negotiation.version} via server/discover (no fallback in pin mode)` : `Version negotiation failed: ${closedCause} before the server offered pinned protocol version ${negotiation.version} (no fallback in pin mode)`);
					if (!negotiation.fallbackAvailable) throw new SdkError(SdkErrorCode.EraNegotiationFailed, closedCause === void 0 ? "Version negotiation failed: the server gave no modern evidence and this client supports no pre-2026-07-28 protocol version to fall back to" : `Version negotiation failed: ${closedCause} and this client supports no pre-2026-07-28 protocol version to fall back to`);
					if (closedCause !== void 0 && deps.disposableProbe !== true) throw new SdkError(SdkErrorCode.EraNegotiationFailed, `Version negotiation failed: ${closedCause} (this transport probed in place — the disposable sibling probe requires the SDK's base StdioClientTransport)`);
					return { era: "legacy" };
				}
				case "error": throw verdict.error;
			}
		}
	};
	let result;
	try {
		result = await probe();
	} catch (error) {
		window.detach();
		throw error;
	}
	window.release();
	return result;
}
/**
* Structural read of the SDK stdio transport's retained spawn parameters —
* `undefined` for stdio-shaped transports that do not expose them, and for
* SUBCLASSES of the SDK transport (those probe in place, where a mid-probe
* close stays a typed connect error). The sibling path requires the transport
* to be exactly the base class: a subclass cannot be faithfully cloned by
* re-invoking its constructor with the retained params alone (extra ctor
* arguments, transformed state, side effects). Exactness is checked without
* importing the class (the negotiation graph must stay runtime-neutral): only
* the base class's own prototype carries the internal `_dispose` reaper the
* sibling flow depends on — a subclass's prototype does not own it.
*/
function readStdioServerParams(transport) {
	const proto = Object.getPrototypeOf(transport);
	if (proto === null || !Object.prototype.hasOwnProperty.call(proto, "_dispose")) return;
	const params = transport._serverParams;
	return typeof params === "object" && params !== null && typeof params.command === "string" ? params : void 0;
}
/**
* stdio era negotiation on a DISPOSABLE SIBLING: stdio servers built on SDKs
* that terminate on any pre-`initialize` request exit when the probe arrives,
* so the probe must not spend the caller's one process life. `server/discover`
* runs on a short-lived sibling spawned from the same parameters; the caller's
* transport starts exactly once, after the era is known. The sibling is
* invisible infrastructure: its stderr is discarded, and it is reaped before
* this resolves (signal escalation awaiting process exit — a helper holding
* its pipes never blocks disposal). A caller `close()` on the session
* transport during the probe aborts the connect with the typed negotiation
* error, and the session transport is never started.
*/
async function negotiateStdioViaSibling(negotiation, sessionTransport, params, deps) {
	const SiblingTransport = sessionTransport.constructor;
	const sibling = new SiblingTransport({
		...params,
		stderr: "ignore"
	});
	const originalClose = sessionTransport.close;
	let callerClosed = false;
	let signalClosed;
	const closedSignal = new Promise((_, reject) => {
		signalClosed = () => reject(callerCloseAbortError());
	});
	sessionTransport.close = async function() {
		callerClosed = true;
		signalClosed?.();
		return originalClose.call(sessionTransport);
	};
	let result;
	try {
		const negotiated = negotiateEra(negotiation, {
			...deps,
			transport: sibling,
			transportKind: "stdio",
			disposableProbe: true
		});
		negotiated.catch(() => {});
		result = await Promise.race([negotiated, closedSignal]);
	} finally {
		await disposeSibling(sibling);
		sessionTransport.close = originalClose;
	}
	if (callerClosed) throw callerCloseAbortError();
	return result;
}
/** The typed abort for a caller `close()` landing while the sibling probes. */
function callerCloseAbortError() {
	return new SdkError(SdkErrorCode.EraNegotiationFailed, "Version negotiation failed: the transport was closed during the server/discover probe");
}
/** Reap a probe sibling — best-effort; a sibling that cannot be reaped must not turn a settled verdict into an error. */
async function disposeSibling(sibling) {
	try {
		const dispose = sibling._dispose;
		await (typeof dispose === "function" ? dispose.call(sibling) : sibling.close());
	} catch {}
}

//#endregion
//#region src/client/client.ts
/**
* The server identity a `DiscoverResult` carries in
* `_meta['io.modelcontextprotocol/serverInfo']` (a spec SHOULD — absent means
* the server offered no identity). Runtime-validated with the spec-type
* guard because `connect({ prior })` accepts caller-supplied values that
* never saw a wire parse.
*/
function serverInfoFromDiscover(discover) {
	const fromMeta = discover._meta?.[SERVER_INFO_META_KEY];
	return isSpecType.Implementation(fromMeta) ? fromMeta : void 0;
}
/**
* Elicitation default application helper. Applies defaults to the `data` based on the `schema`.
*
* @param schema - The schema to apply defaults to.
* @param data - The data to apply defaults to.
*/
function applyElicitationDefaults(schema, data) {
	if (!schema || data === null || typeof data !== "object") return;
	if (schema.type === "object" && schema.properties && typeof schema.properties === "object") {
		const obj = data;
		const props = schema.properties;
		for (const key of Object.keys(props)) {
			const propSchema = props[key];
			if (obj[key] === void 0 && Object.prototype.hasOwnProperty.call(propSchema, "default")) obj[key] = propSchema.default;
			if (obj[key] !== void 0) applyElicitationDefaults(propSchema, obj[key]);
		}
	}
	if (Array.isArray(schema.anyOf)) {
		for (const sub of schema.anyOf) if (typeof sub !== "boolean") applyElicitationDefaults(sub, data);
	}
	if (Array.isArray(schema.oneOf)) {
		for (const sub of schema.oneOf) if (typeof sub !== "boolean") applyElicitationDefaults(sub, data);
	}
}
/**
* Determines which elicitation modes are supported based on declared client capabilities.
*
* According to the spec:
* - An empty elicitation capability object defaults to form mode support (backwards compatibility)
* - URL mode is only supported if explicitly declared
*
* @param capabilities - The client's elicitation capabilities
* @returns An object indicating which modes are supported
*/
function getSupportedElicitationModes(capabilities) {
	if (!capabilities) return {
		supportsFormMode: false,
		supportsUrlMode: false
	};
	const hasFormCapability = capabilities.form !== void 0;
	const hasUrlCapability = capabilities.url !== void 0;
	return {
		supportsFormMode: hasFormCapability || !hasFormCapability && !hasUrlCapability,
		supportsUrlMode: hasUrlCapability
	};
}
/**
* Rejects malformed persisted blobs with a typed `SdkError` before any state
* change: unrecognized `kind`, a legacy verdict also carrying
* DiscoverResult-shaped members (a corrupt blob must never era-choose), or a
* modern `discover` payload failing the wire path's schema.
*/
function validatePrior(prior) {
	if (typeof prior === "object" && prior !== null) {
		if (prior.kind === "legacy" && !("supportedVersions" in prior) && !("discover" in prior)) return prior;
		if (prior.kind === "modern" && DiscoverResultSchema.safeParse(prior.discover).success) return prior;
	}
	throw new SdkError(SdkErrorCode.EraNegotiationFailed, "connect({ prior }): unrecognized prior — expected { kind: 'modern', discover } or { kind: 'legacy' }");
}
/**
* `list_changed` notification → response-cache method(s) to evict. `resources`
* covers both list verbs (the spec's "relevant notification ⇒ immediately
* stale").
*/
const LIST_CHANGED_EVICTIONS = {
	"notifications/tools/list_changed": ["tools/list"],
	"notifications/prompts/list_changed": ["prompts/list"],
	"notifications/resources/list_changed": ["resources/list", "resources/templates/list"]
};
const DEFAULT_LIST_MAX_PAGES = 64;
/**
* An MCP client on top of a pluggable transport.
*
* The client will automatically begin the initialization flow with the server when {@linkcode connect} is called.
*
* To handle server-initiated requests (sampling, elicitation, roots), call {@linkcode setRequestHandler}.
* The client must declare the corresponding capability for the handler to be accepted. For
* `sampling/createMessage` and `elicitation/create`, the handler is automatically wrapped with
* schema validation for both the incoming request and the returned result.
*
* Note: the `roots/list` and `sampling/createMessage` handler surfaces (and the corresponding
* `roots` and `sampling` capabilities) are deprecated as of protocol version 2026-07-28
* (SEP-2577). They remain functional during the deprecation window (at least twelve months).
* Migrate sampling to calling LLM provider APIs directly, and roots to passing paths via tool
* parameters, resource URIs, or configuration.
*
* @example Handling a sampling request
* ```ts source="./client.examples.ts#Client_setRequestHandler_sampling"
* client.setRequestHandler('sampling/createMessage', async request => {
*     const lastMessage = request.params.messages.at(-1);
*     console.log('Sampling request:', lastMessage);
*
*     // In production, send messages to your LLM here
*     return {
*         model: 'my-model',
*         role: 'assistant' as const,
*         content: {
*             type: 'text' as const,
*             text: 'Response from the model'
*         }
*     };
* });
* ```
*/
var Client = class extends Protocol {
	_serverCapabilities;
	_serverVersion;
	_capabilities;
	_instructions;
	_jsonSchemaValidator;
	/**
	* The response-cache substrate. Owns the backing store, the per-method
	* eviction-generation counter, the user-supplied/default flag, and the
	* stamp-memoized derived `name → Tool` / `name → output-validator`
	* indices — the cache-coordination state that used to live as separate
	* private fields here. The internal aggregating walk writes one entry per
	* list verb; `list_changed` evicts the matching method;
	* `_resetConnectionState` resets the lot. {@linkcode callTool}'s
	* output-schema validation reads the derived `outputValidator` index (the
	* substrate's first production caller); the stacked SEP-2243 PR wires
	* `Mcp-Param-*` mirroring through `toolDefinition` on top.
	*/
	_cache;
	_defaultCacheTtlMs;
	_listMaxPages;
	_listChangedDebounceTimers = /* @__PURE__ */ new Map();
	/**
	* The constructor `listChanged` configuration. Durable across reconnects:
	* read fresh on every connect (legacy or modern), never consumed.
	*/
	_listChangedConfig;
	_enforceStrictCapabilities;
	_versionNegotiation;
	_supportedProtocolVersionsOption;
	_inputRequiredDriverConfig;
	/**
	* Active subscriptions/listen state, keyed by subscription id (= the
	* listen request's JSON-RPC id verbatim). The id is a STRING from a
	* Client-owned counter (`'listen:' + N`) — JSON-RPC permits string ids,
	* and Protocol's numeric `_requestMessageId` counter only ever issues
	* numbers, so listen ids cannot collide with ordinary request ids.
	*/
	_listenState = /* @__PURE__ */ new Map();
	_nextListenId = 0;
	/** The auto-opened subscription backing ClientOptions.listChanged on a modern connection. */
	_autoOpenedSubscription;
	/** Backing store for {@linkcode getDiscoverResult}. Per-connection. */
	_discoverResult;
	/**
	* Clears every per-connection field in one place. Called at the start of
	* each fresh (non-resuming) connect and from `close()`, so a stale
	* negotiated era / server identity / auto-opened subscription cannot
	* survive a reconnect.
	*/
	_resetConnectionState() {
		this._negotiatedProtocolVersion = void 0;
		this._serverCapabilities = void 0;
		this._serverVersion = void 0;
		this._instructions = void 0;
		this._discoverResult = void 0;
		this._autoOpenedSubscription = void 0;
		if (this._listenState.size > 0) {
			const reason = new SdkError(SdkErrorCode.ConnectionClosed, "subscriptions/listen: client reconnected or closed; subscription state from the previous connection was reset");
			for (const entry of this._listenState.values()) entry.settle({
				cause: "remote",
				error: reason
			});
		}
		this._listenState.clear();
		for (const timer of this._listChangedDebounceTimers.values()) clearTimeout(timer);
		this._listChangedDebounceTimers.clear();
		this._cache.resetForReconnect();
	}
	async close() {
		try {
			await super.close();
		} finally {
			this._resetConnectionState();
		}
	}
	/**
	* Initializes this client with the given name and version information.
	*/
	constructor(_clientInfo, options) {
		super(options);
		this._clientInfo = _clientInfo;
		this._capabilities = options?.capabilities ? { ...options.capabilities } : {};
		this._jsonSchemaValidator = options?.jsonSchemaValidator ?? new DefaultJsonSchemaValidator();
		this._enforceStrictCapabilities = options?.enforceStrictCapabilities ?? false;
		this._versionNegotiation = options?.versionNegotiation;
		this._supportedProtocolVersionsOption = options?.supportedProtocolVersions;
		this._inputRequiredDriverConfig = resolveInputRequiredDriverConfig(options?.inputRequired);
		this._cache = new ClientResponseCache(options?.responseCacheStore ?? new InMemoryResponseCacheStore(), options?.responseCacheStore !== void 0, (error) => this._reportStoreError(error), options?.cachePartition ?? "");
		this._defaultCacheTtlMs = options?.defaultCacheTtlMs ?? 0;
		this._listMaxPages = options?.listMaxPages ?? DEFAULT_LIST_MAX_PAGES;
		if (options?.listChanged) this._listChangedConfig = options.listChanged;
	}
	buildContext(ctx, _transportInfo) {
		return ctx;
	}
	/**
	* Era-keyed direction enforcement for inbound traffic on channels whose
	* transport does not classify (e.g. stdio): the 2026-07-28 era has no
	* server→client JSON-RPC request channel — server-to-client interactions
	* are carried in-band in `input_required` results — and on stdio the
	* client must never write JSON-RPC responses. An inbound request arriving
	* on a connection that negotiated a modern era is therefore dropped
	* (surfaced via `onerror`) rather than answered. Connections on a legacy
	* era — and all responses and notifications — keep today's dispatch path.
	*/
	_shouldDropInbound(message) {
		if (this._negotiatedProtocolVersion !== void 0 && isModernProtocolVersion(this._negotiatedProtocolVersion) && isJSONRPCRequest(message)) return "drop";
	}
	/**
	* Per-request `_meta` envelope auto-emission (protocol revision 2026-07-28):
	* on a connection that negotiated a modern era — auto-negotiated or pinned —
	* every outgoing request and notification automatically carries the reserved
	* protocol-version / client-info / client-capabilities `_meta` keys (the
	* same envelope the connect-time `server/discover` probe sends).
	* User-supplied `_meta` keys take precedence over the auto-attached ones.
	*
	* Legacy-era connections return `undefined`: the envelope seam is a no-op
	* and outbound traffic is byte-identical to a 2025 client (the legacy
	* `'auto'` fallback included).
	*/
	_outboundMetaEnvelope() {
		const version = this._negotiatedProtocolVersion;
		if (version === void 0) return void 0;
		return this._wireCodec().outboundEnvelope({
			protocolVersion: version,
			clientInfo: this._clientInfo,
			clientCapabilities: this._capabilities
		});
	}
	/**
	* Wires the multi-round-trip auto-fulfilment engine (protocol revision
	* 2026-07-28) into the response funnel: an `input_required` answer is
	* fulfilled through the registered elicitation/sampling/roots handlers
	* and the original request retried via `flow.retry`, up to
	* `inputRequired.maxRounds` rounds. With auto-fulfilment disabled the
	* response surfaces as a typed error steering to manual mode.
	*/
	_resolveNonCompleteResult(decoded, flow) {
		if (!this._inputRequiredDriverConfig.autoFulfill) return Promise.reject(new SdkError(SdkErrorCode.UnsupportedResultType, `Unsupported result type 'input_required' for ${flow.request.method}: multi-round-trip auto-fulfilment is not enabled on this instance — pass allowInputRequired: true to handle it manually, or enable inputRequired.autoFulfill`, {
			resultType: "input_required",
			method: flow.request.method
		}));
		return runInputRequiredFlow({
			getRequestHandler: (method) => this._getRequestHandler(method),
			buildContext: (baseCtx) => this.buildContext(baseCtx, void 0),
			sessionId: this.transport?.sessionId
		}, this._inputRequiredDriverConfig, decoded, flow);
	}
	/**
	* Set up handlers for list changed notifications based on config and server capabilities.
	* This should only be called after initialization when server capabilities are known.
	* Handlers are silently skipped if the server doesn't advertise the corresponding listChanged capability.
	* @internal
	*/
	_setupListChangedHandlers(config) {
		if (config.tools && this._serverCapabilities?.tools?.listChanged) this._setupListChangedHandler("tools", "notifications/tools/list_changed", config.tools, async () => {
			return (await this.listTools(void 0, { cacheMode: "refresh" })).tools;
		});
		if (config.prompts && this._serverCapabilities?.prompts?.listChanged) this._setupListChangedHandler("prompts", "notifications/prompts/list_changed", config.prompts, async () => {
			return (await this.listPrompts(void 0, { cacheMode: "refresh" })).prompts;
		});
		if (config.resources && this._serverCapabilities?.resources?.listChanged) this._setupListChangedHandler("resources", "notifications/resources/list_changed", config.resources, async () => {
			return (await this.listResources(void 0, { cacheMode: "refresh" })).resources;
		});
	}
	/**
	* Registers new capabilities. This can only be called before connecting to a transport.
	*
	* The new capabilities will be merged with any existing capabilities previously given (e.g., at initialization).
	*/
	registerCapabilities(capabilities) {
		if (this.transport) throw new Error("Cannot register capabilities after connecting to transport");
		this._capabilities = mergeCapabilities(this._capabilities, capabilities);
	}
	/**
	* Configure protocol version negotiation before connecting (equivalent to
	* passing `versionNegotiation` at construction time). Can only be called
	* before connecting to a transport. Passing `undefined` clears a previously
	* configured negotiation, restoring the default `'legacy'` posture.
	*
	* See {@linkcode ClientOptions | ClientOptions.versionNegotiation} for the mode semantics.
	*/
	setVersionNegotiation(options) {
		if (this.transport) throw new Error("Cannot configure version negotiation after connecting to transport");
		this._versionNegotiation = options;
	}
	/**
	* Enforces client-side validation for `elicitation/create` and `sampling/createMessage`
	* regardless of how the handler was registered.
	*/
	_wrapHandler(method, handler) {
		if (method === "elicitation/create") return async (request, ctx) => {
			const codec = codecForVersion(this._negotiatedProtocolVersion);
			let validatedRequest = codec.validateRequest("elicitation/create", request);
			if (!validatedRequest.ok && validatedRequest.reason === "not-in-era") validatedRequest = codec.validateInputRequest("elicitation/create", request);
			if (!validatedRequest.ok) throw new ProtocolError(validatedRequest.reason === "not-in-era" ? ProtocolErrorCode.InternalError : ProtocolErrorCode.InvalidParams, validatedRequest.reason === "not-in-era" ? "No wire schema for elicitation/create in the resolved era" : `Invalid elicitation request: ${validatedRequest.message}`);
			const { params } = validatedRequest.value;
			params.mode = params.mode ?? "form";
			const { supportsFormMode, supportsUrlMode } = getSupportedElicitationModes(this._capabilities.elicitation);
			if (params.mode === "form" && !supportsFormMode) throw new ProtocolError(ProtocolErrorCode.InvalidParams, "Client does not support form-mode elicitation requests");
			if (params.mode === "url" && !supportsUrlMode) throw new ProtocolError(ProtocolErrorCode.InvalidParams, "Client does not support URL-mode elicitation requests");
			const result = await handler(request, ctx);
			let validationResult = codec.validateResult("elicitation/create", result);
			if (!validationResult.ok && validationResult.reason === "not-in-era") validationResult = codec.validateInputResponse("elicitation/create", result);
			if (!validationResult.ok) throw new ProtocolError(validationResult.reason === "not-in-era" ? ProtocolErrorCode.InternalError : ProtocolErrorCode.InvalidParams, validationResult.reason === "not-in-era" ? "No wire schema for elicitation/create in the resolved era" : `Invalid elicitation result: ${validationResult.message}`);
			const validatedResult = validationResult.value;
			const requestedSchema = params.mode === "form" ? params.requestedSchema : void 0;
			if (params.mode === "form" && validatedResult.action === "accept" && validatedResult.content && requestedSchema && this._capabilities.elicitation?.form?.applyDefaults) try {
				applyElicitationDefaults(requestedSchema, validatedResult.content);
			} catch {}
			return validatedResult;
		};
		if (method === "sampling/createMessage") return async (request, ctx) => {
			const codec = codecForVersion(this._negotiatedProtocolVersion);
			let validatedRequest = codec.validateRequest("sampling/createMessage", request);
			if (!validatedRequest.ok && validatedRequest.reason === "not-in-era") validatedRequest = codec.validateInputRequest("sampling/createMessage", request);
			if (!validatedRequest.ok) throw new ProtocolError(validatedRequest.reason === "not-in-era" ? ProtocolErrorCode.InternalError : ProtocolErrorCode.InvalidParams, validatedRequest.reason === "not-in-era" ? "No wire schema for sampling/createMessage in the resolved era" : `Invalid sampling request: ${validatedRequest.message}`);
			const { params } = validatedRequest.value;
			const result = await handler(request, ctx);
			const hasTools = Boolean(params.tools || params.toolChoice);
			let validatedResult = codec.samplingResultVariant(hasTools, result);
			if (!validatedResult.ok && validatedResult.reason === "not-in-era") validatedResult = codec.validateInputResponse("sampling/createMessage", result);
			if (!validatedResult.ok) throw new ProtocolError(validatedResult.reason === "not-in-era" ? ProtocolErrorCode.InternalError : ProtocolErrorCode.InvalidParams, validatedResult.reason === "not-in-era" ? "No result schema for sampling/createMessage in the resolved era" : `Invalid sampling result: ${validatedResult.message}`);
			return validatedResult.value;
		};
		return handler;
	}
	assertCapability(capability, method) {
		if (!this._serverCapabilities?.[capability]) throw new SdkError(SdkErrorCode.CapabilityNotSupported, `Server does not support ${capability} (required for ${method})`);
	}
	/**
	* Connects to a server via the given transport and performs the MCP initialization handshake.
	*
	* @example Basic usage (stdio)
	* ```ts source="./client.examples.ts#Client_connect_stdio"
	* const client = new Client({ name: 'my-client', version: '1.0.0' });
	* const transport = new StdioClientTransport({ command: 'my-mcp-server' });
	* await client.connect(transport);
	* ```
	*
	* @example Streamable HTTP with SSE fallback
	* ```ts source="./client.examples.ts#Client_connect_sseFallback"
	* const baseUrl = new URL(url);
	*
	* try {
	*     // Try modern Streamable HTTP transport first
	*     const client = new Client({ name: 'my-client', version: '1.0.0' });
	*     const transport = new StreamableHTTPClientTransport(baseUrl);
	*     await client.connect(transport);
	*     return { client, transport };
	* } catch {
	*     // Fall back to legacy SSE transport
	*     const client = new Client({ name: 'my-client', version: '1.0.0' });
	*     const transport = new SSEClientTransport(baseUrl);
	*     await client.connect(transport);
	*     return { client, transport };
	* }
	* ```
	*/
	async connect(transport, options) {
		if (options?.prior != null) return this._connectFromPrior(transport, validatePrior(options.prior), options);
		const negotiation = resolveVersionNegotiation(this._versionNegotiation, this._supportedProtocolVersionsOption);
		if (negotiation.kind !== "legacy") return this._connectNegotiated(transport, negotiation, options);
		return this._connectPlainLegacy(transport, options);
	}
	/**
	* Plain legacy connect — the pinned 2025 sequence, byte-untouched. The
	* `mode: 'legacy'` connect body, shared with the `prior` legacy verdict.
	*/
	async _connectPlainLegacy(transport, options) {
		await super.connect(transport);
		if (transport.sessionId !== void 0) {
			const negotiatedProtocolVersion = this._negotiatedProtocolVersion;
			if (negotiatedProtocolVersion !== void 0) transport.setProtocolVersion?.(negotiatedProtocolVersion);
			return;
		}
		this._resetConnectionState();
		await this._legacyHandshake(transport, options);
	}
	/**
	* The 2025 `initialize` handshake — the body of the plain legacy connect and
	* the `'auto'`-mode fallback path (same `initialize` body, zero 2026 headers;
	* on the stdio sibling path it opens the session child's fresh pipe, in the
	* in-place modes it rides the probed connection). Callers clear the negotiated protocol version before
	* the handshake; its completion sets the negotiated (legacy) version.
	*/
	async _legacyHandshake(transport, options) {
		const legacyVersions = legacyProtocolVersions(this._supportedProtocolVersions);
		try {
			const offeredVersion = legacyVersions[0];
			if (offeredVersion === void 0) throw new SdkError(SdkErrorCode.EraNegotiationFailed, "Cannot run the initialize handshake: supportedProtocolVersions contains no pre-2026-07-28 protocol version");
			const result = await this.request({
				method: "initialize",
				params: {
					protocolVersion: offeredVersion,
					capabilities: this._capabilities,
					clientInfo: this._clientInfo
				}
			}, options);
			if (result === void 0) throw new Error(`Server sent invalid initialize result: ${result}`);
			if (!legacyVersions.includes(result.protocolVersion)) throw new Error(`Server's protocol version is not supported: ${result.protocolVersion}`);
			this._serverCapabilities = result.capabilities;
			this._serverVersion = result.serverInfo;
			this._cache.setServerIdentity(this._deriveServerIdentity(transport));
			if (transport.setProtocolVersion) transport.setProtocolVersion(result.protocolVersion);
			this._instructions = result.instructions;
			await this.notification({ method: "notifications/initialized" });
			this._negotiatedProtocolVersion = result.protocolVersion;
			if (this._listChangedConfig) this._setupListChangedHandlers(this._listChangedConfig);
		} catch (error) {
			this.close();
			throw error;
		}
	}
	/**
	* Negotiated connect (mode `'auto'` or `{ pin }`): probe with `server/discover`
	* before the Protocol machinery attaches — on a disposable sibling process for
	* the SDK's stdio transport, in place otherwise — then either establish the
	* modern era or perform the plain legacy handshake.
	*/
	async _connectNegotiated(transport, negotiation, options) {
		if (transport.sessionId !== void 0) {
			await super.connect(transport);
			const negotiatedProtocolVersion = this._negotiatedProtocolVersion;
			if (negotiatedProtocolVersion !== void 0 && transport.setProtocolVersion) transport.setProtocolVersion(negotiatedProtocolVersion);
			return;
		}
		this._resetConnectionState();
		let result;
		try {
			const transportKind = detectProbeTransportKind(transport);
			const baseDeps = {
				clientInfo: this._clientInfo,
				capabilities: this._capabilities,
				environment: detectProbeEnvironment(),
				defaultTimeoutMs: options?.timeout ?? DEFAULT_REQUEST_TIMEOUT_MSEC
			};
			const stdioParams = transportKind === "stdio" ? readStdioServerParams(transport) : void 0;
			result = stdioParams === void 0 ? await negotiateEra(negotiation, {
				...baseDeps,
				transport,
				transportKind
			}) : await negotiateStdioViaSibling(negotiation, transport, stdioParams, baseDeps);
		} catch (error) {
			await transport.close().catch(() => {});
			disarmSpentCloseGuard(transport);
			throw error;
		}
		disarmSpentCloseGuard(transport);
		await super.connect(transport);
		if (result.era === "legacy") {
			await this._legacyHandshake(transport, options);
			return;
		}
		this._serverCapabilities = result.discover.capabilities;
		this._serverVersion = serverInfoFromDiscover(result.discover);
		this._cache.setServerIdentity(this._deriveServerIdentity(transport));
		this._instructions = result.discover.instructions;
		this._discoverResult = result.discover;
		this._negotiatedProtocolVersion = result.version;
		if (transport.setProtocolVersion) transport.setProtocolVersion(result.version);
		if (this._listChangedConfig) {
			const config = this._listChangedConfig;
			const advertised = this._serverCapabilities;
			const effective = {
				...config.tools && advertised?.tools?.listChanged && { tools: config.tools },
				...config.prompts && advertised?.prompts?.listChanged && { prompts: config.prompts },
				...config.resources && advertised?.resources?.listChanged && { resources: config.resources }
			};
			let handlersRegistered = true;
			try {
				this._setupListChangedHandlers(effective);
			} catch (error) {
				handlersRegistered = false;
				this.onerror?.(error instanceof Error ? error : new Error(String(error)));
			}
			const filter = handlersRegistered ? {
				...effective.tools && { toolsListChanged: true },
				...effective.prompts && { promptsListChanged: true },
				...effective.resources && { resourcesListChanged: true }
			} : {};
			if (Object.keys(filter).length > 0) {
				const ackAbort = new AbortController();
				const onConnectAbort = () => ackAbort.abort(options?.signal?.reason);
				if (options?.signal?.aborted) onConnectAbort();
				options?.signal?.addEventListener("abort", onConnectAbort);
				try {
					this._autoOpenedSubscription = await this.listen(filter, {
						timeout: options?.timeout,
						signal: ackAbort.signal
					});
				} catch (error) {
					if (options?.signal?.aborted) {
						await this.close().catch(() => {});
						throw error;
					}
					this.onerror?.(error instanceof Error ? error : new Error(String(error)));
				} finally {
					options?.signal?.removeEventListener("abort", onConnectAbort);
				}
			}
		}
	}
	/**
	* Connect from a validated {@linkcode PriorDiscovery}: the modern arm
	* adopts the `DiscoverResult` (zero round trips; `EraNegotiationFailed`
	* on no 2026-07-28+ overlap), the legacy arm runs the plain legacy connect.
	*/
	async _connectFromPrior(transport, prior, options) {
		if (prior.kind === "legacy") return this._connectPlainLegacy(transport, options);
		const discover = prior.discover;
		this._resetConnectionState();
		const explicit = this._supportedProtocolVersionsOption;
		const version = (explicit && modernProtocolVersions(explicit).length > 0 ? modernProtocolVersions(explicit) : SUPPORTED_MODERN_PROTOCOL_VERSIONS).find((v) => discover.supportedVersions.includes(v));
		if (version === void 0) throw new SdkError(SdkErrorCode.EraNegotiationFailed, "connect({ prior }) with a modern verdict requires a 2026-07-28+ mutual protocol version; the supplied DiscoverResult and this client's supportedProtocolVersions have no modern overlap. For a server known to be legacy, pass prior: { kind: 'legacy' } to skip the probe and initialize directly, or use versionNegotiation: { mode: 'auto' } to re-probe with legacy fallback.");
		await super.connect(transport);
		this._discoverResult = discover;
		this._serverCapabilities = discover.capabilities;
		this._serverVersion = serverInfoFromDiscover(discover);
		this._cache.setServerIdentity(this._deriveServerIdentity(transport));
		this._instructions = discover.instructions;
		this._negotiatedProtocolVersion = version;
		transport.setProtocolVersion?.(version);
		if (this._listChangedConfig) try {
			this._setupListChangedHandlers(this._listChangedConfig);
		} catch (error) {
			this.onerror?.(error instanceof Error ? error : new Error(String(error)));
		}
	}
	/**
	* After initialization has completed, this will be populated with the server's reported capabilities.
	*/
	getServerCapabilities() {
		return this._serverCapabilities;
	}
	/**
	* The connected server's self-reported name and version, when it
	* identified itself: required on the legacy `initialize` result; a spec
	* SHOULD in the discover result's `_meta` on 2026-07-28, so a successful
	* modern connect against an anonymous server leaves this `undefined`.
	*/
	getServerVersion() {
		return this._serverVersion;
	}
	/**
	* The connected server's identity for response-cache partitioning. The
	* `serverInfo` `name@version` pair when available (required on
	* `initialize`; a SHOULD in the discover result's `_meta` since spec PR
	* #3002); falls back to the transport's `sessionId`, then to a
	* per-connection surrogate. The surrogate matters since #3002 made
	* identity optional: without it, two identity-less servers reached over
	* sessionId-less transports would share the cache's pre-connect `''`
	* partition and read each other's entries — no stable identity means no
	* cross-connection cache reuse. The value itself is server-controlled —
	* the collision-safety of the storage partition comes from
	* {@linkcode ClientResponseCache}'s JSON-array encoding around it, not
	* from any character it does or does not contain.
	*/
	_deriveServerIdentity(transport) {
		const v = this._serverVersion;
		if (v !== void 0) return `${v.name}@${v.version}`;
		return transport.sessionId ?? `anonymous:${Date.now()}-${Math.random().toString(36).slice(2)}`;
	}
	/**
	* After initialization has completed, this will be populated with the protocol version negotiated
	* during the initialize handshake. When manually reconstructing a transport for reconnection, pass this
	* value to the new transport so it continues sending the required `mcp-protocol-version` header.
	*/
	getNegotiatedProtocolVersion() {
		return this._negotiatedProtocolVersion;
	}
	/**
	* After initialization has completed, this returns the protocol era of the
	* connection: `'modern'` when the connection negotiated a 2026-07-28+
	* revision (via `server/discover`), `'legacy'` for the 2025-era
	* `initialize` handshake, or `undefined` before the connection is
	* established.
	*/
	getProtocolEra() {
		const version = this._negotiatedProtocolVersion;
		if (version === void 0) return void 0;
		return isModernProtocolVersion(version) ? "modern" : "legacy";
	}
	/**
	* After initialization has completed, this may be populated with information about the server's instructions.
	*/
	getInstructions() {
		return this._instructions;
	}
	/**
	* The {@linkcode DiscoverResult} from the last `'auto'`/pinned probe,
	* {@linkcode discover} call, or `connect({ prior })` that adopted a
	* modern verdict (a legacy verdict leaves this `undefined` — there is no
	* `DiscoverResult` on that path). Persistable via `JSON.stringify`; wrap
	* as `{ kind: 'modern', discover }` and feed to {@linkcode ConnectOptions}
	* `prior`.
	*/
	getDiscoverResult() {
		return this._discoverResult;
	}
	assertCapabilityForMethod(method) {
		switch (method) {
			case "logging/setLevel":
				if (!this._serverCapabilities?.logging) throw new SdkError(SdkErrorCode.CapabilityNotSupported, `Server does not support logging (required for ${method})`);
				break;
			case "prompts/get":
			case "prompts/list":
				if (!this._serverCapabilities?.prompts) throw new SdkError(SdkErrorCode.CapabilityNotSupported, `Server does not support prompts (required for ${method})`);
				break;
			case "resources/list":
			case "resources/templates/list":
			case "resources/read":
			case "resources/subscribe":
			case "resources/unsubscribe":
				if (!this._serverCapabilities?.resources) throw new SdkError(SdkErrorCode.CapabilityNotSupported, `Server does not support resources (required for ${method})`);
				if (method === "resources/subscribe" && !this._serverCapabilities.resources.subscribe) throw new SdkError(SdkErrorCode.CapabilityNotSupported, `Server does not support resource subscriptions (required for ${method})`);
				break;
			case "tools/call":
			case "tools/list":
				if (!this._serverCapabilities?.tools) throw new SdkError(SdkErrorCode.CapabilityNotSupported, `Server does not support tools (required for ${method})`);
				break;
			case "completion/complete":
				if (!this._serverCapabilities?.completions) throw new SdkError(SdkErrorCode.CapabilityNotSupported, `Server does not support completions (required for ${method})`);
				break;
			case "initialize": break;
			case "server/discover": break;
			case "ping": break;
		}
	}
	assertNotificationCapability(method) {
		switch (method) {
			case "notifications/roots/list_changed":
				if (!this._capabilities.roots?.listChanged) throw new SdkError(SdkErrorCode.CapabilityNotSupported, `Client does not support roots list changed notifications (required for ${method})`);
				break;
			case "notifications/initialized": break;
			case "notifications/cancelled": break;
			case "notifications/progress": break;
		}
	}
	assertRequestHandlerCapability(method) {
		switch (method) {
			case "sampling/createMessage":
				if (!this._capabilities.sampling) throw new SdkError(SdkErrorCode.CapabilityNotSupported, `Client does not support sampling capability (required for ${method})`);
				break;
			case "elicitation/create":
				if (!this._capabilities.elicitation) throw new SdkError(SdkErrorCode.CapabilityNotSupported, `Client does not support elicitation capability (required for ${method})`);
				break;
			case "roots/list":
				if (!this._capabilities.roots) throw new SdkError(SdkErrorCode.CapabilityNotSupported, `Client does not support roots capability (required for ${method})`);
				break;
			case "ping": break;
		}
	}
	async ping(options) {
		return this.request({ method: "ping" }, options);
	}
	/**
	* Send `server/discover` (2026-07-28+) and record the result for
	* {@linkcode getDiscoverResult}.
	*/
	async discover(options) {
		const result = await this._requestWithSchema({ method: "server/discover" }, DiscoverResultSchema, options);
		this._discoverResult = result;
		return result;
	}
	/** Requests argument autocompletion suggestions from the server for a prompt or resource. */
	async complete(params, options) {
		return this.request({
			method: "completion/complete",
			params
		}, options);
	}
	/**
	* Sets the minimum severity level for log messages sent by the server.
	*
	* @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577).
	* Remains functional during the deprecation window (at least twelve months).
	* Migrate to stderr logging (STDIO servers) or OpenTelemetry.
	*/
	async setLoggingLevel(level, options) {
		return this.request({
			method: "logging/setLevel",
			params: { level }
		}, options);
	}
	/** Retrieves a prompt by name from the server, passing the given arguments for template substitution. */
	async getPrompt(params, options) {
		return this.request({
			method: "prompts/get",
			params
		}, options);
	}
	/**
	* Lists available prompts.
	*
	* Called without a `cursor` (the common case), this walks every page and
	* returns the complete aggregated list with no `nextCursor`; the
	* aggregate is also written to the {@linkcode ResponseCacheStore}. Pass an
	* explicit `{ cursor }` to fetch a single page and walk pagination
	* yourself — the per-page path returns the server's raw page (with
	* `nextCursor` for the next call) and does not write the response cache.
	* The auto-aggregate path is capped by
	* {@linkcode ClientOptions | ClientOptions.listMaxPages} (default 64); the per-page path
	* is not.
	*
	* Returns an empty list if the server does not advertise prompts capability
	* (or throws if {@linkcode ClientOptions.enforceStrictCapabilities} is enabled).
	*
	* @example
	* ```ts source="./client.examples.ts#Client_listPrompts_pagination"
	* // No cursor → all pages aggregated for you.
	* const { prompts } = await client.listPrompts();
	* console.log(
	*     'Available prompts:',
	*     prompts.map(p => p.name)
	* );
	* ```
	*/
	async listPrompts(params, options) {
		if (!this._serverCapabilities?.prompts && !this._enforceStrictCapabilities) {
			console.debug("Client.listPrompts() called but server does not advertise prompts capability - returning empty list");
			return { prompts: [] };
		}
		if (params?.cursor !== void 0) return this.request({
			method: "prompts/list",
			params
		}, options);
		const hit = await this._serveFromCache("prompts/list", void 0, options);
		if (hit !== void 0) return hit;
		return this._listAllPages("prompts/list", params, options, (acc, page) => acc.prompts.push(...page.prompts));
	}
	/**
	* Lists available resources.
	*
	* Called without a `cursor` (the common case), this walks every page and
	* returns the complete aggregated list with no `nextCursor`; the
	* aggregate is also written to the {@linkcode ResponseCacheStore}. Pass an
	* explicit `{ cursor }` to fetch a single page and walk pagination
	* yourself — the per-page path returns the server's raw page (with
	* `nextCursor` for the next call) and does not write the response cache.
	* The auto-aggregate path is capped by
	* {@linkcode ClientOptions | ClientOptions.listMaxPages} (default 64); the per-page path
	* is not.
	*
	* Returns an empty list if the server does not advertise resources capability
	* (or throws if {@linkcode ClientOptions.enforceStrictCapabilities} is enabled).
	*
	* @example
	* ```ts source="./client.examples.ts#Client_listResources_pagination"
	* // No cursor → all pages aggregated for you.
	* const { resources } = await client.listResources();
	* console.log(
	*     'Available resources:',
	*     resources.map(r => r.name)
	* );
	* ```
	*/
	async listResources(params, options) {
		if (!this._serverCapabilities?.resources && !this._enforceStrictCapabilities) {
			console.debug("Client.listResources() called but server does not advertise resources capability - returning empty list");
			return { resources: [] };
		}
		if (params?.cursor !== void 0) return this.request({
			method: "resources/list",
			params
		}, options);
		const hit = await this._serveFromCache("resources/list", void 0, options);
		if (hit !== void 0) return hit;
		return this._listAllPages("resources/list", params, options, (acc, page) => acc.resources.push(...page.resources));
	}
	/**
	* Lists available resource URI templates for dynamic resources.
	*
	* Called without a `cursor`, this walks every page and returns the
	* complete aggregated list with no `nextCursor`; the aggregate is
	* also written to the {@linkcode ResponseCacheStore}. Pass an explicit
	* `{ cursor }` to fetch a single page — see
	* {@linkcode listResources | listResources()} for the per-page contract.
	*
	* Returns an empty list if the server does not advertise resources capability
	* (or throws if {@linkcode ClientOptions.enforceStrictCapabilities} is enabled).
	*/
	async listResourceTemplates(params, options) {
		if (!this._serverCapabilities?.resources && !this._enforceStrictCapabilities) {
			console.debug("Client.listResourceTemplates() called but server does not advertise resources capability - returning empty list");
			return { resourceTemplates: [] };
		}
		if (params?.cursor !== void 0) return this.request({
			method: "resources/templates/list",
			params
		}, options);
		const hit = await this._serveFromCache("resources/templates/list", void 0, options);
		if (hit !== void 0) return hit;
		return this._listAllPages("resources/templates/list", params, options, (acc, page) => acc.resourceTemplates.push(...page.resourceTemplates));
	}
	/**
	* Walk every page of a paginated list verb, aggregate, and write ONE
	* entry to the response cache. Internal — backs the public `list*`
	* methods' no-`cursor` auto-aggregate path. Page 1's result object is
	* mutated in place (its items array is extended; `nextCursor` is
	* cleared); page-1 metadata (`ttlMs`, `cacheScope`, `_meta`) is preserved.
	* A `nextCursor` that repeats stops the walk (defence against a
	* non-converging server, mcp.d's `drainList` guard);
	* {@linkcode ClientOptions.listMaxPages} is a hard cap — hitting it
	* throws, so a partial aggregate is never cached. The
	* captured-generation guard skips the write when a `list_changed` landed
	* mid-walk, so the eviction is never overwritten by a stale aggregate.
	* `finalize` runs on the complete aggregate before the cache write — the
	* SEP-2243 invalid-`x-mcp-header` exclusion hooks here so the cached
	* `tools/list` entry is already filtered.
	*
	* The caller's `baseParams` (everything except `cursor`) is threaded into
	* every page request — page 1 sends `{...baseParams}`, later pages
	* `{...baseParams, cursor}` — so a typed, documented `_meta` (e.g. W3C
	* trace context) supplied to the public `list*()` reaches every wire
	* request the walk issues.
	*/
	async _listAllPages(method, baseParams, options, append, finalize) {
		const bypass = options?.cacheMode === "bypass";
		const generation = this._cache.captureGeneration(method);
		const acc = await this.request({
			method,
			...baseParams && { params: { ...baseParams } }
		}, options);
		let cursor = acc.nextCursor;
		const seen = /* @__PURE__ */ new Set();
		let pages = 1;
		while (cursor !== void 0 && !seen.has(cursor)) {
			if (this._listMaxPages !== 0 && pages >= this._listMaxPages) throw new SdkError(SdkErrorCode.ListPaginationExceeded, `${method}: exceeded listMaxPages (${this._listMaxPages}); server pagination did not terminate`, {
				method,
				listMaxPages: this._listMaxPages
			});
			seen.add(cursor);
			const page = await this.request({
				method,
				params: {
					...baseParams,
					cursor
				}
			}, options);
			append(acc, page);
			cursor = page.nextCursor;
			pages++;
		}
		delete acc.nextCursor;
		finalize?.(acc);
		if (bypass) return acc;
		await this._cache.write(method, acc, generation, this._freshness(acc));
		return acc;
	}
	/**
	* Compute the {@linkcode ClientResponseCache.write} freshness payload from
	* a cacheable result body. The single seam through which the client reads
	* `ttlMs`/`cacheScope` (mcp.d's `cachedFetch` engine). The fields pass
	* through the loose result schema, so they are read off the runtime body;
	* a missing `ttlMs` falls back to
	* {@linkcode ClientOptions | ClientOptions.defaultCacheTtlMs}; an explicit server-sent
	* `ttlMs` (including `0` — the spec's "immediately stale") is honoured
	* as-is. The default of `0` means `expiresAt === now()` ⇒ never served,
	* only stored. A missing `cacheScope` is treated as `'private'` — the
	* spec's `'public'` grant ("any client … MAY serve to any user") is too
	* strong to infer by default, and matches this SDK's server-side stamp
	* default.
	*/
	_freshness(result, params) {
		const body = result;
		const ttlMs = typeof body.ttlMs === "number" ? body.ttlMs : this._defaultCacheTtlMs;
		const scope = body.cacheScope === "public" ? "public" : "private";
		return {
			expiresAt: this._cache.now() + Math.min(Math.max(0, ttlMs), MAX_CACHE_TTL_MS),
			scope,
			params
		};
	}
	/**
	* The cache-serving front of every cacheable verb (mcp.d's `cachedFetch`
	* read half): under `cacheMode: 'use'` (the default), a fresh held entry
	* is served and the round trip is skipped. `'refresh'` and `'bypass'`
	* always fetch (the caller decides whether to write). Freshness and
	* decoding live in {@linkcode ClientResponseCache.read}; every hit is
	* freshly parsed, so the caller owns it outright. A custom store
	* whose `get()` rejects is routed to `onerror` and treated as a miss —
	* cache bookkeeping never blocks a request from reaching the wire.
	*/
	async _serveFromCache(method, params, options) {
		if (options?.cacheMode === "bypass" || options?.cacheMode === "refresh") return void 0;
		const hit = await this._cache.read(method, params).catch((error) => void this._reportStoreError(error));
		if (hit !== void 0) {
			if (options?.signal?.aborted) {
				const reason = options.signal.reason;
				throw reason instanceof SdkError ? reason : new SdkError(SdkErrorCode.RequestTimeout, String(reason));
			}
			return hit.value;
		}
	}
	/** Route a custom-store failure to `onerror` without aborting the surrounding dispatch. */
	_reportStoreError(e) {
		this.onerror?.(e instanceof Error ? e : new Error(String(e)));
	}
	/**
	* Compile a single tool's `outputSchema`. Passed as the compile callback to
	* {@linkcode ClientResponseCache.outputValidator} so the cache class stays
	* free of any validator-provider dependency, and called directly for the
	* `options.toolDefinition` path of {@linkcode callTool} (a one-off
	* caller-supplied definition is compiled in isolation and never enters the
	* cache, so it cannot poison the listed tool of the same name).
	*
	* Returns `undefined` when the tool has no `outputSchema`, or a
	* discriminated `{ok}` result otherwise. SEP-2106: ANY throw from the
	* validator engine — unsupported `$schema` dialect, invalid `pattern`
	* regex, unresolvable `$ref`, or any other engine error — is captured as
	* `{ok: false, compileError}` so one bad schema does not poison the rest
	* of the listing; `callTool()` surfaces it as an `InvalidParams` error
	* before the request. The `{ok}` discriminator (not
	* `compileError !== undefined`) means a custom provider that does
	* `throw undefined` is still treated as a captured failure.
	*/
	_compileOutputValidator(tool) {
		if (!tool.outputSchema) return void 0;
		try {
			return {
				ok: true,
				validator: this._jsonSchemaValidator.getValidator(tool.outputSchema)
			};
		} catch (error) {
			return {
				ok: false,
				compileError: error
			};
		}
	}
	/**
	* Resolve the SEP-2243 `x-mcp-header` declaration scan for a tool name.
	*
	* The caller-supplied `toolDefinition` escape hatch wins; otherwise the
	* cached `tools/list` entry (via the cache's `toolDefinition`) is the
	* source. Freshness is the response cache's lifecycle: `list_changed`
	* evicts, otherwise the held schema is the best information available
	* regardless of age, and a stale schema is recovered through the
	* `HEADER_MISMATCH` → evict-refetch-retry path in {@linkcode callTool}.
	* On a miss the call proceeds without `Mcp-Param-*` headers (the spec's
	* "client SHOULD send without custom headers" guidance) and relies on the
	* same recovery.
	*/
	async _resolveXMcpHeaderScan(name, override) {
		const tool = override ?? await this._cache.toolDefinition(name);
		return tool === void 0 ? void 0 : scanXMcpHeaderDeclarations(tool.inputSchema);
	}
	/**
	* Reads the contents of a resource by URI.
	*
	* Honours the result's `ttlMs`/`cacheScope` (SEP-2549): a still-fresh
	* cached body for the same `uri` is returned without a round trip
	* (`cacheMode: 'use'`, the default). The cache key is `{method, uri}`
	* partitioned by the resolved scope — `'private'` (the default when the
	* server omits the field) is stored under this client's
	* {@linkcode ClientOptions | ClientOptions.cachePartition}, so a shared
	* store cannot serve one principal's resource body to another. Unlike the
	* list verbs, a result whose resolved TTL is ≤0 is **not** stored
	* (`resources/read` has no derived index and the URI keyspace is
	* unbounded).
	*/
	async readResource(params, options) {
		const hit = await this._serveFromCache("resources/read", params.uri, options);
		if (hit !== void 0) return hit;
		const generation = this._cache.captureGeneration("resources/read", params.uri);
		const result = await this.request({
			method: "resources/read",
			params
		}, options);
		if (options?.cacheMode !== "bypass") {
			const freshness = this._freshness(result, params.uri);
			if (freshness.expiresAt > this._cache.now()) await this._cache.write("resources/read", result, generation, freshness);
			else if (options?.cacheMode === "refresh") await this._cache.evictKey("resources/read", params.uri);
		}
		return result;
	}
	/** Subscribes to change notifications for a resource. The server must support resource subscriptions. */
	async subscribeResource(params, options) {
		return this.request({
			method: "resources/subscribe",
			params
		}, options);
	}
	/** Unsubscribes from change notifications for a resource. */
	async unsubscribeResource(params, options) {
		return this.request({
			method: "resources/unsubscribe",
			params
		}, options);
	}
	/**
	* Opens a `subscriptions/listen` stream (protocol revision 2026-07-28).
	*
	* Resolves once the server's `notifications/subscriptions/acknowledged`
	* arrives (the standard request timeout applies to this ack phase). Change
	* notifications delivered on the stream are dispatched to the existing
	* {@linkcode setNotificationHandler} registrations — the same handlers the
	* 2025-era unsolicited notifications fire on a legacy connection — so
	* `listen()` is era-transparent for consumers that already register those.
	*
	* `close()` tears the subscription down by aborting the listen request's
	* `requestSignal` (closes the SSE stream where the transport honors it)
	* AND sending `notifications/cancelled` referencing the listen request id
	* — both, unconditionally, so any spec-compliant server on any transport
	* sees the cancel. No automatic re-listen — call `listen()` again to
	* re-establish.
	*
	* On a 2025-era connection this throws a typed
	* {@linkcode SdkErrorCode.MethodNotSupportedByProtocolVersion} steering to
	* `resources/subscribe` and `ClientOptions.listChanged` (the legacy
	* unsolicited delivery model still applies there); no transparent shim.
	*/
	async listen(filter, options) {
		if (this.transport === void 0) throw new SdkError(SdkErrorCode.NotConnected, "Not connected");
		const negotiated = this._negotiatedProtocolVersion;
		if (negotiated === void 0 || !isModernProtocolVersion(negotiated)) throw new SdkError(SdkErrorCode.MethodNotSupportedByProtocolVersion, `subscriptions/listen requires a 2026-07-28-era connection (negotiated: ${negotiated ?? "none"}). On a 2025-era connection, change notifications are delivered unsolicited: use ClientOptions.listChanged and resources/subscribe instead.`, {
			method: "subscriptions/listen",
			protocolVersion: negotiated
		});
		if (options?.signal?.aborted) {
			const reason = options.signal.reason;
			throw reason instanceof SdkError ? reason : new SdkError(SdkErrorCode.RequestTimeout, String(reason));
		}
		const requestAbort = new AbortController();
		const listenId = `listen:${this._nextListenId++}`;
		let state = "opening";
		let ackTimer;
		let onCallerAbort;
		let resolveOpening;
		let rejectOpening;
		const opening = new Promise((resolve, reject) => {
			resolveOpening = resolve;
			rejectOpening = reject;
		});
		let resolveClosed;
		const closed = new Promise((resolve) => {
			resolveClosed = resolve;
		});
		const settle = (outcome) => {
			if (state === "closed") return;
			const wasOpening = state === "opening";
			if (ackTimer !== void 0) {
				clearTimeout(ackTimer);
				ackTimer = void 0;
			}
			if ("ack" in outcome) {
				state = "open";
				resolveOpening(outcome.ack);
				return;
			}
			state = "closed";
			if (onCallerAbort !== void 0) options?.signal?.removeEventListener("abort", onCallerAbort);
			this._listenState.delete(listenId);
			requestAbort.abort();
			resolveClosed(outcome.cause);
			if (wasOpening) rejectOpening(outcome.error ?? new SdkError(SdkErrorCode.ConnectionClosed, "subscriptions/listen closed before the server acknowledged"));
		};
		const wireTeardown = async () => {
			requestAbort.abort();
			await this.notification({
				method: "notifications/cancelled",
				params: { requestId: listenId }
			}).catch(() => {});
		};
		const close = async () => {
			if (state === "closed") return;
			settle({ cause: "local" });
			await wireTeardown();
		};
		this._listenState.set(listenId, { settle });
		const ackTimeout = options?.timeout ?? DEFAULT_REQUEST_TIMEOUT_MSEC;
		ackTimer = setTimeout(() => {
			settle({
				cause: "remote",
				error: new SdkError(SdkErrorCode.RequestTimeout, "subscriptions/listen ack timed out", { timeout: ackTimeout })
			});
			wireTeardown().catch(() => {});
		}, ackTimeout);
		if (options?.signal) {
			const callerSignal = options.signal;
			onCallerAbort = () => {
				if (state === "closed") return;
				const reason = callerSignal.reason;
				settle({
					cause: "local",
					error: reason instanceof Error ? reason : new Error(String(reason ?? "Aborted"))
				});
				wireTeardown().catch(() => {});
			};
			callerSignal.addEventListener("abort", onCallerAbort, { once: true });
		}
		const jsonrpcRequest = {
			jsonrpc: "2.0",
			id: listenId,
			method: "subscriptions/listen",
			params: {
				_meta: { ...this._outboundMetaEnvelope() },
				notifications: filter
			}
		};
		try {
			await this.transport.send(jsonrpcRequest, {
				requestSignal: requestAbort.signal,
				onRequestStreamEnd: () => settle({
					cause: "remote",
					error: /* @__PURE__ */ new Error("subscriptions/listen: stream ended")
				})
			});
		} catch (error) {
			settle({
				cause: "remote",
				error: error instanceof Error ? error : new Error(String(error))
			});
		}
		return {
			honoredFilter: await opening,
			close,
			closed
		};
	}
	/**
	* The subscription auto-opened by `ClientOptions.listChanged` on a modern
	* connection — the listen filter is the intersection of the configured
	* sub-options and the server-advertised `listChanged` capabilities.
	* `undefined` on a legacy connection, before connect, or when that
	* intersection is empty (auto-open skipped). Exposed so the consumer can
	* `close()` it.
	*/
	get autoOpenedSubscription() {
		return this._autoOpenedSubscription;
	}
	/**
	* Transport-level demux for `subscriptions/listen` notifications, before
	* any decoding/era-gating/handler dispatch. Consumes the leading
	* `notifications/subscriptions/acknowledged` referencing a live
	* subscription id (resolves the ack waiter) and an inbound
	* `notifications/cancelled` referencing a live string-typed subscription
	* id (server-side teardown on stdio). Change notifications carrying a
	* subscription id pass through to the existing registered handlers via
	* `super`. An unmatched ack/cancelled is NOT consumed: it reaches
	* `setNotificationHandler` / `fallbackNotificationHandler` instead of
	* being silently swallowed.
	*/
	_onnotification(raw, extra) {
		const evicted = Object.hasOwn(LIST_CHANGED_EVICTIONS, raw.method) ? LIST_CHANGED_EVICTIONS[raw.method] : void 0;
		if (raw.method === "notifications/resources/updated") {
			const uri = raw.params?.uri;
			if (typeof uri === "string") this._cache.evictKey("resources/read", uri);
		} else if (evicted !== void 0) for (const method of evicted) this._cache.evict(method);
		if (raw.method === "notifications/subscriptions/acknowledged") {
			const subscriptionId = raw.params?._meta?.[SUBSCRIPTION_ID_META_KEY];
			const entry = typeof subscriptionId === "string" ? this._listenState.get(subscriptionId) : void 0;
			if (entry !== void 0) {
				const honored = this._wireCodec().validateNotification("notifications/subscriptions/acknowledged", raw);
				entry.settle({ ack: honored.ok ? honored.value.params.notifications : {} });
				return;
			}
		}
		if (raw.method === "notifications/cancelled") {
			const cancelledId = raw.params?.requestId;
			const entry = typeof cancelledId === "string" ? this._listenState.get(cancelledId) : void 0;
			if (entry !== void 0) {
				entry.settle({
					cause: "remote",
					error: /* @__PURE__ */ new Error("subscriptions/listen: server cancelled the subscription")
				});
				return;
			}
		}
		super._onnotification(raw, extra);
	}
	/**
	* Transport-level demux for `subscriptions/listen` responses. A JSON-RPC
	* ERROR for the listen id is the server's pre-ack capacity/params
	* rejection; a JSON-RPC RESULT for the listen id is the spec's
	* `SubscriptionsListenResult` — the server's GRACEFUL-close signal (sent
	* on shutdown). A string-id response that matches a live `_listenState`
	* entry is consumed here (Protocol's `_responseHandlers` map is keyed by
	* NUMBER and never holds a listen id, so passing a string-id response
	* through would surface as "unknown message ID" via `onerror`).
	*/
	_onresponse(response) {
		const id = response.id;
		const entry = typeof id === "string" ? this._listenState.get(id) : void 0;
		if (entry !== void 0) {
			if (isJSONRPCErrorResponse(response)) entry.settle({
				cause: "remote",
				error: ProtocolError.fromError(response.error.code, response.error.message, response.error.data)
			});
			else entry.settle({
				cause: "graceful",
				error: new SdkError(SdkErrorCode.ConnectionClosed, "subscriptions/listen: server closed the subscription gracefully before acknowledging")
			});
			return;
		}
		super._onresponse(response);
	}
	/**
	* Settle every live per-listen state machine on a transport-initiated
	* close (the server dropping the connection on stdio/InMemory) before
	* Protocol's `_onclose` tears the transport down. The base
	* `_responseHandlers` settlement does not reach `_listenState` (listen
	* ids are never registered there), so without this override a remote
	* close would leave an in-flight `listen()` / open `McpSubscription`
	* hanging.
	*/
	_onclose() {
		if (this._listenState.size > 0) {
			const reason = new SdkError(SdkErrorCode.ConnectionClosed, "Connection closed");
			for (const entry of this._listenState.values()) entry.settle({
				cause: "remote",
				error: reason
			});
			this._listenState.clear();
		}
		super._onclose();
	}
	/**
	* Calls a tool on the connected server and returns the result. Automatically validates structured output
	* if the tool has an `outputSchema`.
	*
	* Tool results have two error surfaces: `result.isError` for tool-level failures (the tool ran but reported
	* a problem), and thrown {@linkcode ProtocolError} for protocol-level failures or {@linkcode SdkError} for
	* SDK-level issues (timeouts, missing capabilities).
	*
	* @example Basic usage
	* ```ts source="./client.examples.ts#Client_callTool_basic"
	* const result = await client.callTool({
	*     name: 'calculate-bmi',
	*     arguments: { weightKg: 70, heightM: 1.75 }
	* });
	*
	* // Tool-level errors are returned in the result, not thrown
	* if (result.isError) {
	*     console.error('Tool error:', result.content);
	*     return;
	* }
	*
	* console.log(result.content);
	* ```
	*
	* @example Structured output
	* ```ts source="./client.examples.ts#Client_callTool_structuredOutput"
	* const result = await client.callTool({
	*     name: 'calculate-bmi',
	*     arguments: { weightKg: 70, heightM: 1.75 }
	* });
	*
	* // Machine-readable output for the client application. SEP-2106: structuredContent is
	* // `unknown` (any JSON value). Check for presence with `!== undefined` and narrow before use.
	* if (result.structuredContent !== undefined) {
	*     const sc: unknown = result.structuredContent; // e.g. { bmi: 22.86 }
	*     if (typeof sc === 'object' && sc !== null && 'bmi' in sc) {
	*         console.log(sc.bmi);
	*     }
	* }
	* ```
	*/
	async callTool(params, options) {
		const mirroringActive = this.getProtocolEra() === "modern" && detectProbeEnvironment() !== "browser";
		const buildSendOptions = async () => {
			if (!mirroringActive) return options;
			let scan;
			try {
				scan = await this._resolveXMcpHeaderScan(params.name, options?.toolDefinition);
			} catch (error) {
				this._reportStoreError(error);
			}
			if (!scan?.valid || scan.declarations.length === 0) return options;
			const paramHeaders = buildMcpParamHeaders(scan.declarations, params.arguments);
			return Object.keys(paramHeaders).length === 0 ? options : {
				...options,
				headers: {
					...options?.headers,
					...paramHeaders
				}
			};
		};
		let compiled = options?.toolDefinition === void 0 ? await this._cache.outputValidator(params.name, (tool) => this._compileOutputValidator(tool)).catch((error) => void this._reportStoreError(error)) : this._compileOutputValidator(options.toolDefinition);
		const assertCompiled = () => {
			if (compiled === void 0 || compiled.ok) return;
			const err = compiled.compileError;
			const message = (err instanceof Error ? err.message : String(err)).slice(0, 200);
			throw new ProtocolError(ProtocolErrorCode.InvalidParams, `Tool '${params.name}' has an invalid outputSchema: ${message}`);
		};
		assertCompiled();
		let result;
		try {
			result = await this.request({
				method: "tools/call",
				params
			}, await buildSendOptions());
		} catch (error) {
			const isHeaderMismatch = error instanceof ProtocolError && error.code === HEADER_MISMATCH_ERROR_CODE;
			if (!mirroringActive || !isHeaderMismatch || options?.toolDefinition !== void 0) throw error;
			const refreshOptions = {
				signal: options?.signal,
				timeout: options?.timeout,
				cacheMode: "refresh"
			};
			await this._cache.evict("tools/list");
			await this.listTools(void 0, refreshOptions).catch((error_) => this._reportStoreError(error_));
			compiled = await this._cache.outputValidator(params.name, (tool) => this._compileOutputValidator(tool)).catch((error_) => void this._reportStoreError(error_));
			assertCompiled();
			result = await this.request({
				method: "tools/call",
				params
			}, await buildSendOptions());
		}
		const validator = compiled !== void 0 && compiled.ok ? compiled.validator : void 0;
		if (validator) {
			if (result.structuredContent === void 0 && !result.isError) throw new ProtocolError(ProtocolErrorCode.InvalidRequest, `Tool ${params.name} has an output schema but did not return structured content`);
			if (result.structuredContent !== void 0 && !result.isError) try {
				const validationResult = validator(result.structuredContent);
				if (!validationResult.valid) throw new ProtocolError(ProtocolErrorCode.InvalidParams, `Structured content does not match the tool's output schema: ${validationResult.errorMessage}`);
			} catch (error) {
				if (error instanceof ProtocolError) throw error;
				throw new ProtocolError(ProtocolErrorCode.InvalidParams, `Failed to validate structured content: ${error instanceof Error ? error.message : String(error)}`);
			}
		}
		return result;
	}
	/**
	* Lists available tools.
	*
	* Called without a `cursor` (the common case), this walks every page and
	* returns the complete aggregated list with no `nextCursor`; the
	* aggregate is also written to the {@linkcode ResponseCacheStore} (the
	* source for {@linkcode callTool | callTool()}'s output-schema validation
	* and SEP-2243 `Mcp-Param-*` header mirroring). Pass an explicit
	* `{ cursor }` to fetch a single page and walk pagination yourself — the
	* per-page path returns the server's raw page (with `nextCursor` for the
	* next call) and does not write the response cache. The auto-aggregate
	* path is capped by {@linkcode ClientOptions | ClientOptions.listMaxPages} (default 64);
	* the per-page path is not.
	*
	* Returns an empty list if the server does not advertise tools capability
	* (or throws if {@linkcode ClientOptions.enforceStrictCapabilities} is enabled).
	*
	* @example
	* ```ts source="./client.examples.ts#Client_listTools_pagination"
	* // No cursor → all pages aggregated for you.
	* const { tools } = await client.listTools();
	* console.log(
	*     'Available tools:',
	*     tools.map(t => t.name)
	* );
	* ```
	*/
	async listTools(params, options) {
		if (!this._serverCapabilities?.tools && !this._enforceStrictCapabilities) {
			console.debug("Client.listTools() called but server does not advertise tools capability - returning empty list");
			return { tools: [] };
		}
		if (params?.cursor !== void 0) {
			const page = await this.request({
				method: "tools/list",
				params
			}, options);
			this._excludeInvalidXMcpHeaderTools(page);
			return page;
		}
		const hit = await this._serveFromCache("tools/list", void 0, options);
		if (hit !== void 0) return hit;
		return this._listAllPages("tools/list", params, options, (acc, page) => acc.tools.push(...page.tools), (acc) => this._excludeInvalidXMcpHeaderTools(acc));
	}
	/**
	* SEP-2243 (protocol revision 2026-07-28): a Streamable HTTP client MUST
	* exclude tool definitions whose `x-mcp-header` declarations violate the
	* constraints, and SHOULD log a warning naming the tool and the reason.
	* Applied to the CACHED aggregated `tools/list` result (so the entry
	* mirroring reads never holds an unmirrorable tool) AND to every public
	* per-page {@linkcode listTools | listTools()} return (the spec's MUST
	* has no carve-out for paginated reads). The gate is era-only on
	* non-stdio transports — `detectProbeTransportKind` cannot distinguish a
	* real HTTP transport from in-memory/custom transports (it only
	* positively recognizes stdio), and over-excluding on a non-HTTP modern
	* connection is harmless: those transports never carry per-request
	* headers, so an excluded tool would have been uncallable on a Streamable
	* HTTP arm of the same server. Mutates `result.tools` in place.
	*/
	_excludeInvalidXMcpHeaderTools(result) {
		if (this.getProtocolEra() !== "modern" || !this.transport || detectProbeTransportKind(this.transport) === "stdio") return;
		const filtered = result.tools.filter((tool) => {
			const scan = scanXMcpHeaderDeclarations(tool.inputSchema);
			if (!scan.valid) {
				console.warn(`[mcp-sdk] excluding tool '${tool.name}' from tools/list: invalid x-mcp-header declaration — ${scan.reason}`);
				return false;
			}
			return true;
		});
		if (filtered.length !== result.tools.length) result.tools = filtered;
	}
	/**
	* Set up a single list changed handler.
	* @internal
	*/
	_setupListChangedHandler(listType, notificationMethod, options, fetcher) {
		const parseResult = parseSchema(ListChangedOptionsBaseSchema, options);
		if (!parseResult.success) throw new Error(`Invalid ${listType} listChanged options: ${parseResult.error.message}`);
		if (typeof options.onChanged !== "function") throw new TypeError(`Invalid ${listType} listChanged options: onChanged must be a function`);
		const { autoRefresh, debounceMs } = parseResult.data;
		const { onChanged } = options;
		const refresh = async () => {
			if (!autoRefresh) {
				onChanged(null, null);
				return;
			}
			try {
				onChanged(null, await fetcher());
			} catch (error) {
				onChanged(error instanceof Error ? error : new Error(String(error)), null);
			}
		};
		const handler = () => {
			if (debounceMs) {
				const existingTimer = this._listChangedDebounceTimers.get(listType);
				if (existingTimer) clearTimeout(existingTimer);
				const timer = setTimeout(refresh, debounceMs);
				this._listChangedDebounceTimers.set(listType, timer);
			} else refresh();
		};
		this.setNotificationHandler(notificationMethod, handler);
	}
	/**
	* Notifies the server that the client's root list has changed. Requires the `roots.listChanged` capability.
	*
	* @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577).
	* Remains functional during the deprecation window (at least twelve months).
	* Migrate to passing paths via tool parameters, resource URIs, or configuration.
	*/
	async sendRootsListChanged() {
		return this.notification({ method: "notifications/roots/list_changed" });
	}
};

//#endregion
//#region src/client/crossAppAccess.ts
/**
* Requests a JWT Authorization Grant (ID-JAG) from an enterprise IdP using RFC 8693 Token Exchange.
*
* This function performs step 2 of the Enterprise Managed Authorization flow:
* exchanges an ID Token for a JWT Authorization Grant that can be used with the target MCP server.
*
* @param options - Configuration for the token exchange request
* @returns The JWT Authorization Grant and related metadata
* @throws {Error} If the token exchange fails or returns an error response
*
* @example
* ```ts
* const result = await requestJwtAuthorizationGrant({
*     tokenEndpoint: 'https://idp.example.com/token',
*     audience: 'https://auth.chat.example/',
*     resource: 'https://mcp.chat.example/',
*     idToken: 'eyJhbGciOiJS...',
*     clientId: 'my-idp-client',
*     clientSecret: 'my-idp-secret',
*     scope: 'chat.read chat.history'
* });
*
* // Use result.jwtAuthGrant with the MCP server's authorization server
* ```
*/
async function requestJwtAuthorizationGrant(options) {
	const { tokenEndpoint, audience, resource, idToken, clientId, clientSecret, scope, fetchFn = fetch } = options;
	const tokenUrl = assertSecureTokenEndpoint(tokenEndpoint);
	const params = new URLSearchParams({
		grant_type: "urn:ietf:params:oauth:grant-type:token-exchange",
		requested_token_type: "urn:ietf:params:oauth:token-type:id-jag",
		audience: String(audience),
		resource: String(resource),
		subject_token: idToken,
		subject_token_type: "urn:ietf:params:oauth:token-type:id_token",
		client_id: clientId
	});
	if (clientSecret) params.set("client_secret", clientSecret);
	if (scope) params.set("scope", scope);
	const response = await fetchFn(tokenUrl, {
		method: "POST",
		headers: { "Content-Type": "application/x-www-form-urlencoded" },
		body: params.toString()
	});
	if (!response.ok) {
		const errorBody = await response.json().catch(() => ({}));
		const parseResult$1 = OAuthErrorResponseSchema.safeParse(errorBody);
		if (parseResult$1.success) {
			const { error, error_description } = parseResult$1.data;
			throw new Error(`Token exchange failed: ${error}${error_description ? ` - ${error_description}` : ""}`);
		}
		throw new Error(`Token exchange failed with status ${response.status}: ${JSON.stringify(errorBody)}`);
	}
	const parseResult = IdJagTokenExchangeResponseSchema.safeParse(await response.json());
	if (!parseResult.success) throw new Error(`Invalid token exchange response: ${parseResult.error.message}`);
	return {
		jwtAuthGrant: parseResult.data.access_token,
		expiresIn: parseResult.data.expires_in,
		scope: parseResult.data.scope
	};
}
/**
* Discovers the IdP's token endpoint and requests a JWT Authorization Grant.
*
* This is a convenience wrapper around {@linkcode requestJwtAuthorizationGrant} that
* first performs OAuth metadata discovery to find the token endpoint.
*
* @param options - Configuration including IdP URL for discovery
* @returns The JWT Authorization Grant and related metadata
* @throws {Error} If discovery fails or the token exchange fails
*
* @example
* ```ts
* const result = await discoverAndRequestJwtAuthGrant({
*     idpUrl: 'https://idp.example.com',
*     audience: 'https://auth.chat.example/',
*     resource: 'https://mcp.chat.example/',
*     idToken: await getIdToken(),
*     clientId: 'my-idp-client',
*     clientSecret: 'my-idp-secret'
* });
* ```
*/
async function discoverAndRequestJwtAuthGrant(options) {
	const { idpUrl, fetchFn = fetch, ...restOptions } = options;
	const metadata = await discoverAuthorizationServerMetadata(String(idpUrl), { fetchFn });
	if (!metadata?.token_endpoint) throw new Error(`Failed to discover token endpoint for IdP: ${idpUrl}`);
	return requestJwtAuthorizationGrant({
		...restOptions,
		tokenEndpoint: metadata.token_endpoint,
		fetchFn
	});
}
/**
* Exchanges a JWT Authorization Grant for an access token at the MCP server's authorization server.
*
* This function performs step 3 of the Enterprise Managed Authorization flow:
* uses the JWT Authorization Grant to obtain an access token from the MCP server.
*
* @param options - Configuration for the JWT grant exchange
* @returns OAuth tokens (access token, token type, etc.)
* @throws {Error} If the exchange fails or returns an error response
*
* Defaults to `client_secret_basic` (HTTP Basic Authorization header), matching
* `CrossAppAccessProvider`'s declared `token_endpoint_auth_method` and the
* SEP-990 conformance test requirements. Use `authMethod: 'client_secret_post'` only
* when the authorization server explicitly requires it.
*
* @example
* ```ts
* const tokens = await exchangeJwtAuthGrant({
*     tokenEndpoint: 'https://auth.chat.example/token',
*     jwtAuthGrant: 'eyJhbGci...',
*     clientId: 'my-mcp-client',
*     clientSecret: 'my-mcp-secret'
* });
*
* // Use tokens.access_token to access the MCP server
* ```
*/
async function exchangeJwtAuthGrant(options) {
	const { tokenEndpoint, jwtAuthGrant, clientId, clientSecret, authMethod = "client_secret_basic", fetchFn = fetch } = options;
	const tokenUrl = assertSecureTokenEndpoint(tokenEndpoint);
	const params = new URLSearchParams({
		grant_type: "urn:ietf:params:oauth:grant-type:jwt-bearer",
		assertion: jwtAuthGrant
	});
	const headers = new Headers({ "Content-Type": "application/x-www-form-urlencoded" });
	applyClientAuthentication(authMethod, {
		client_id: clientId,
		client_secret: clientSecret
	}, headers, params);
	const response = await fetchFn(tokenUrl, {
		method: "POST",
		headers,
		body: params.toString()
	});
	if (!response.ok) {
		const errorBody = await response.json().catch(() => ({}));
		const parseResult$1 = OAuthErrorResponseSchema.safeParse(errorBody);
		if (parseResult$1.success) {
			const { error, error_description } = parseResult$1.data;
			throw new Error(`JWT grant exchange failed: ${error}${error_description ? ` - ${error_description}` : ""}`);
		}
		throw new Error(`JWT grant exchange failed with status ${response.status}: ${JSON.stringify(errorBody)}`);
	}
	const responseBody = await response.json();
	const parseResult = OAuthTokensSchema.safeParse(responseBody);
	if (!parseResult.success) throw new Error(`Invalid token response: ${parseResult.error.message}`);
	return parseResult.data;
}

//#endregion
//#region src/client/middleware.ts
/**
* Creates a fetch wrapper that handles OAuth authentication automatically.
*
* This wrapper will:
* - Add `Authorization` headers with access tokens
* - Handle 401 responses by attempting re-authentication
* - Retry the original request after successful auth
* - Handle OAuth errors appropriately ({@linkcode index.OAuthErrorCode.InvalidClient | OAuthErrorCode.InvalidClient}, etc.)
*
* The `baseUrl` parameter is optional and defaults to using the domain from the request URL.
* However, you should explicitly provide `baseUrl` when:
* - Making requests to multiple subdomains (e.g., api.example.com, cdn.example.com)
* - Using API paths that differ from OAuth discovery paths (e.g., requesting /api/v1/data but OAuth is at /)
* - The OAuth server is on a different domain than your API requests
* - You want to ensure consistent OAuth behavior regardless of request URLs
*
* For MCP transports, set `baseUrl` to the same URL you pass to the transport constructor.
*
* Note: This wrapper is designed for general-purpose fetch operations.
* MCP transports (SSE and StreamableHTTP) already have built-in OAuth handling
* and should not need this wrapper.
*
* @param provider - OAuth client provider for authentication
* @param baseUrl - Base URL for OAuth server discovery (defaults to request URL domain)
* @returns A fetch middleware function
*/
const withOAuth = (provider, baseUrl) => (next) => {
	return async (input, init) => {
		const makeRequest = async () => {
			const headers = new Headers(init?.headers);
			const tokens = await provider.tokens();
			if (tokens) headers.set("Authorization", `Bearer ${tokens.access_token}`);
			return await next(input, {
				...init,
				headers
			});
		};
		let response = await makeRequest();
		if (response.status === 401) try {
			const { resourceMetadataUrl, scope } = extractWWWAuthenticateParams(response);
			const result = await auth(provider, {
				serverUrl: baseUrl || (typeof input === "string" ? new URL(input).origin : input.origin),
				resourceMetadataUrl,
				scope,
				fetchFn: next
			});
			if (result === "REDIRECT") throw new UnauthorizedError("Authentication requires user authorization - redirect initiated");
			if (result !== "AUTHORIZED") throw new UnauthorizedError(`Authentication failed with result: ${result}`);
			response = await makeRequest();
		} catch (error) {
			if (error instanceof UnauthorizedError) throw error;
			throw new UnauthorizedError(`Failed to re-authenticate: ${error instanceof Error ? error.message : String(error)}`);
		}
		if (response.status === 401) throw new UnauthorizedError(`Authentication failed for ${typeof input === "string" ? input : input.toString()}`);
		return response;
	};
};
/**
* Creates a fetch middleware that logs HTTP requests and responses.
*
* When called without arguments `withLogging()`, it uses the default logger that:
* - Logs successful requests (2xx) to `console.log`
* - Logs error responses (4xx/5xx) and network errors to `console.error`
* - Logs all requests regardless of status (`statusLevel: 0`)
* - Does not include request or response headers in logs
* - Measures and displays request duration in milliseconds
*
* Important: the default logger uses both `console.log` and `console.error` so it should not be used with
* `stdio` transports and applications.
*
* @param options - Logging configuration options
* @returns A fetch middleware function
*/
const withLogging = (options = {}) => {
	const { logger, includeRequestHeaders = false, includeResponseHeaders = false, statusLevel = 0 } = options;
	const defaultLogger = (input) => {
		const { method, url, status, statusText, duration, requestHeaders, responseHeaders, error } = input;
		let message = error ? `HTTP ${method} ${url} failed: ${error.message} (${duration}ms)` : `HTTP ${method} ${url} ${status} ${statusText} (${duration}ms)`;
		if (includeRequestHeaders && requestHeaders) {
			const reqHeaders = [...requestHeaders.entries()].map(([key, value]) => `${key}: ${value}`).join(", ");
			message += `\n  Request Headers: {${reqHeaders}}`;
		}
		if (includeResponseHeaders && responseHeaders) {
			const resHeaders = [...responseHeaders.entries()].map(([key, value]) => `${key}: ${value}`).join(", ");
			message += `\n  Response Headers: {${resHeaders}}`;
		}
		if (error || status >= 400) console.error(message);
		else console.log(message);
	};
	const logFn = logger || defaultLogger;
	return (next) => async (input, init) => {
		const startTime = performance.now();
		const method = init?.method || "GET";
		const url = typeof input === "string" ? input : input.toString();
		const requestHeaders = includeRequestHeaders ? new Headers(init?.headers) : void 0;
		try {
			const response = await next(input, init);
			const duration = performance.now() - startTime;
			if (response.status >= statusLevel) logFn({
				method,
				url,
				status: response.status,
				statusText: response.statusText,
				duration,
				requestHeaders,
				responseHeaders: includeResponseHeaders ? response.headers : void 0
			});
			return response;
		} catch (error) {
			logFn({
				method,
				url,
				status: 0,
				statusText: "Network Error",
				duration: performance.now() - startTime,
				requestHeaders,
				error
			});
			throw error;
		}
	};
};
/**
* Composes multiple fetch middleware functions into a single middleware pipeline.
* Middleware are applied in the order they appear, creating a chain of handlers.
*
* @example
* ```ts source="./middleware.examples.ts#applyMiddlewares_basicUsage"
* // Create a middleware pipeline that handles both OAuth and logging
* const enhancedFetch = applyMiddlewares(withOAuth(oauthProvider, 'https://api.example.com'), withLogging({ statusLevel: 400 }))(fetch);
*
* // Use the enhanced fetch - it will handle auth and log errors
* const response = await enhancedFetch('https://api.example.com/data');
* ```
*
* @param middleware - Array of fetch middleware to compose into a pipeline
* @returns A single composed middleware function
*/
const applyMiddlewares = (...middleware) => {
	return (next) => {
		let handler = next;
		for (const mw of middleware) handler = mw(handler);
		return handler;
	};
};
/**
* Helper function to create custom fetch middleware with cleaner syntax.
* Provides the next handler and request details as separate parameters for easier access.
*
* @example
* ```ts source="./middleware.examples.ts#createMiddleware_examples"
* // Create custom authentication middleware
* const customAuthMiddleware = createMiddleware(async (next, input, init) => {
*     const headers = new Headers(init?.headers);
*     headers.set('X-Custom-Auth', 'my-token');
*
*     const response = await next(input, { ...init, headers });
*
*     if (response.status === 401) {
*         console.log('Authentication failed');
*     }
*
*     return response;
* });
*
* // Create conditional middleware
* const conditionalMiddleware = createMiddleware(async (next, input, init) => {
*     const url = typeof input === 'string' ? input : input.toString();
*
*     // Only add headers for API routes
*     if (url.includes('/api/')) {
*         const headers = new Headers(init?.headers);
*         headers.set('X-API-Version', 'v2');
*         return next(input, { ...init, headers });
*     }
*
*     // Pass through for non-API routes
*     return next(input, init);
* });
*
* // Create caching middleware
* const cacheMiddleware = createMiddleware(async (next, input, init) => {
*     const cacheKey = typeof input === 'string' ? input : input.toString();
*
*     // Check cache first
*     const cached = await getFromCache(cacheKey);
*     if (cached) {
*         return new Response(cached, { status: 200 });
*     }
*
*     // Make request and cache result
*     const response = await next(input, init);
*     if (response.ok) {
*         await saveToCache(cacheKey, await response.clone().text());
*     }
*
*     return response;
* });
* ```
*
* @param handler - Function that receives the next handler and request parameters
* @returns A fetch middleware function
*/
const createMiddleware = (handler) => {
	return (next) => (input, init) => handler(next, input, init);
};

//#endregion
//#region src/client/sse.ts
var SseError = class extends Error {
	static {
		Object.defineProperty(this, "mcpBrand", { value: "mcp.SseError" });
	}
	static [Symbol.hasInstance](value) {
		return brandedHasInstance(this, value);
	}
	/**
	* Brand-based type guard: equivalent to `value instanceof this`, as an
	* explicit static predicate (the axios/AWS-SDK `isInstance` style). Reads
	* the caller's own brand via `this`, so every branded subclass gets a
	* correctly-scoped guard by inheritance. Must be invoked on the class —
	* in callback position write `v => SdkError.isInstance(v)`, not
	* `.filter(SdkError.isInstance)` (detached calls throw rather than
	* silently matching nothing).
	*/
	static isInstance(value) {
		if (typeof this !== "function") throw new TypeError("isInstance must be called on the class (e.g. `SdkError.isInstance(value)`); for callbacks use `v => SdkError.isInstance(v)`");
		return brandedHasInstance(this, value);
	}
	constructor(code, message, event) {
		super(`SSE error: ${message}`);
		this.code = code;
		this.event = event;
		stampErrorBrands(this, new.target);
	}
};
/**
* Client transport for SSE: this will connect to a server using Server-Sent Events for receiving
* messages and make separate `POST` requests for sending messages.
* @deprecated SSEClientTransport is deprecated. Prefer to use {@linkcode index.StreamableHTTPClientTransport | StreamableHTTPClientTransport} where possible instead. Note that because some servers are still using SSE, clients may need to support both transports during the migration period.
*/
var SSEClientTransport = class {
	_eventSource;
	_endpoint;
	_abortController;
	_url;
	_resourceMetadataUrl;
	_scope;
	_eventSourceInit;
	_requestInit;
	_authProvider;
	_oauthProvider;
	_skipIssuerMetadataValidation;
	_fetch;
	_fetchWithInit;
	_protocolVersion;
	onclose;
	onerror;
	onmessage;
	constructor(url, opts) {
		this._url = url;
		this._resourceMetadataUrl = void 0;
		this._scope = void 0;
		this._eventSourceInit = opts?.eventSourceInit;
		this._requestInit = opts?.requestInit;
		this._skipIssuerMetadataValidation = opts?.skipIssuerMetadataValidation;
		if (isOAuthClientProvider(opts?.authProvider)) {
			this._oauthProvider = opts.authProvider;
			this._authProvider = adaptOAuthProvider(opts.authProvider, { skipIssuerMetadataValidation: opts.skipIssuerMetadataValidation });
		} else this._authProvider = opts?.authProvider;
		this._fetch = opts?.fetch;
		this._fetchWithInit = createFetchWithInit(opts?.fetch, opts?.requestInit);
	}
	_last401Response;
	async _commonHeaders() {
		const headers = {};
		let token;
		try {
			token = await this._authProvider?.token();
		} catch (error) {
			throw markAuthSeamEscape(error);
		}
		if (token) headers["Authorization"] = `Bearer ${token}`;
		if (this._protocolVersion) headers["mcp-protocol-version"] = this._protocolVersion;
		const extraHeaders = normalizeHeaders(this._requestInit?.headers);
		return new Headers({
			...headers,
			...extraHeaders
		});
	}
	_startOrAuth() {
		const fetchImpl = this?._eventSourceInit?.fetch ?? this._fetch ?? fetch;
		return new Promise((resolve, reject) => {
			this._eventSource = new EventSource(this._url.href, {
				...this._eventSourceInit,
				fetch: async (url, init) => {
					const headers = await this._commonHeaders();
					headers.set("Accept", "text/event-stream");
					const response = await fetchImpl(url, {
						...init,
						headers
					});
					if (response.status === 401) {
						this._last401Response = response;
						if (response.headers.has("www-authenticate")) {
							const { resourceMetadataUrl, scope } = extractWWWAuthenticateParams(response);
							this._resourceMetadataUrl = resourceMetadataUrl;
							this._scope = scope;
						}
					}
					return response;
				}
			});
			this._abortController = new AbortController();
			this._eventSource.onerror = (event) => {
				if (event.code === 401 && this._authProvider) {
					if (this._authProvider.onUnauthorized && this._last401Response) {
						const response = this._last401Response;
						this._last401Response = void 0;
						this._eventSource?.close();
						this._authProvider.onUnauthorized({
							response,
							serverUrl: this._url,
							fetchFn: this._fetchWithInit
						}).then(() => this._startOrAuth().then(resolve, reject), (error$2) => {
							markAuthSeamEscape(error$2);
							this.onerror?.(error$2);
							reject(error$2);
						});
						return;
					}
					const error$1 = markAuthSeamEscape(new UnauthorizedError());
					reject(error$1);
					this.onerror?.(error$1);
					return;
				}
				const error = new SseError(event.code, event.message, event);
				reject(error);
				this.onerror?.(error);
			};
			this._eventSource.onopen = () => {};
			this._eventSource.addEventListener("endpoint", (event) => {
				const messageEvent = event;
				try {
					this._endpoint = new URL(messageEvent.data, this._url);
					if (this._endpoint.origin !== this._url.origin) throw new Error(`Endpoint origin does not match connection origin: ${this._endpoint.origin}`);
				} catch (error) {
					reject(error);
					this.onerror?.(error);
					this.close();
					return;
				}
				resolve();
			});
			this._eventSource.onmessage = (event) => {
				const messageEvent = event;
				let message;
				try {
					message = JSONRPCMessageSchema.parse(JSON.parse(messageEvent.data));
				} catch (error) {
					this.onerror?.(error);
					return;
				}
				this.onmessage?.(message);
			};
		});
	}
	async start() {
		if (this._eventSource) throw new Error("SSEClientTransport already started! If using Client class, note that connect() calls start() automatically.");
		return await this._startOrAuth();
	}
	async finishAuth(codeOrParams, iss) {
		if (!this._oauthProvider) throw new UnauthorizedError("finishAuth requires an OAuthClientProvider");
		const { authorizationCode, iss: issParam } = await resolveAuthorizationCallbackParams(codeOrParams, iss, this._oauthProvider, this._url, {
			fetchFn: this._fetchWithInit,
			resourceMetadataUrl: this._resourceMetadataUrl
		});
		if (await auth(this._oauthProvider, {
			serverUrl: this._url,
			authorizationCode,
			iss: issParam,
			resourceMetadataUrl: this._resourceMetadataUrl,
			scope: this._scope,
			fetchFn: this._fetchWithInit,
			skipIssuerMetadataValidation: this._skipIssuerMetadataValidation
		}) !== "AUTHORIZED") throw new UnauthorizedError("Failed to authorize");
	}
	async close() {
		this._abortController?.abort();
		this._eventSource?.close();
		this.onclose?.();
	}
	async send(message) {
		return this._send(message, false);
	}
	async _send(message, isAuthRetry) {
		if (!this._endpoint) throw new SdkError(SdkErrorCode.NotConnected, "Not connected");
		try {
			const headers = await this._commonHeaders();
			headers.set("content-type", "application/json");
			const init = {
				...this._requestInit,
				method: "POST",
				headers,
				body: JSON.stringify(message),
				signal: this._abortController?.signal
			};
			const response = await (this._fetch ?? fetch)(this._endpoint, init);
			if (!response.ok) {
				if (response.status === 401 && this._authProvider) {
					if (response.headers.has("www-authenticate")) {
						const { resourceMetadataUrl, scope } = extractWWWAuthenticateParams(response);
						this._resourceMetadataUrl = resourceMetadataUrl;
						this._scope = scope;
					}
					if (this._authProvider.onUnauthorized && !isAuthRetry) {
						try {
							await this._authProvider.onUnauthorized({
								response,
								serverUrl: this._url,
								fetchFn: this._fetchWithInit
							});
						} catch (error) {
							throw markAuthSeamEscape(error);
						}
						await response.text?.().catch(() => {});
						return this._send(message, true);
					}
					await response.text?.().catch(() => {});
					if (isAuthRetry) throw markAuthSeamEscape(new SdkHttpError(SdkErrorCode.ClientHttpAuthentication, "Server returned 401 after re-authentication", {
						status: 401,
						statusText: response.statusText
					}));
					throw markAuthSeamEscape(new UnauthorizedError());
				}
				const text = await response.text?.().catch(() => null);
				throw new Error(`Error POSTing to endpoint (HTTP ${response.status}): ${text}`);
			}
			await response.text?.().catch(() => {});
		} catch (error) {
			this.onerror?.(error);
			throw error;
		}
	}
	setProtocolVersion(version) {
		this._protocolVersion = version;
	}
};

//#endregion
//#region src/client/streamableHttp.ts
/** Default cap on step-up re-authorization retries within a single send/stream-open. */
const DEFAULT_MAX_STEP_UP_RETRIES = 1;
const DEFAULT_STREAMABLE_HTTP_RECONNECTION_OPTIONS = {
	initialReconnectionDelay: 1e3,
	maxReconnectionDelay: 3e4,
	reconnectionDelayGrowFactor: 1.5,
	maxRetries: 2
};
/**
* Standard/auth header names the transport owns. The per-request
* `TransportSendOptions.headers` carrier MUST NOT be able to override these —
* they are derived from connection state (`authorization`, `mcp-session-id`)
* or from the message body itself (`mcp-protocol-version`, `mcp-method`,
* `mcp-name`), and a per-request override would let a caller produce a
* header/body disagreement the server's SEP-2243 cross-checks reject.
*/
const RESERVED_REQUEST_HEADER_NAMES = new Set([
	"authorization",
	"content-type",
	"mcp-protocol-version",
	"mcp-method",
	"mcp-name",
	"mcp-session-id"
]);
/**
* `AbortSignal.any` with a manual fallback. `AbortSignal.any` landed in
* Node 20.3; this package's `engines` floor is `>=20`, so 20.0–20.2 must be
* served by the fallback combinator (a controller that aborts on the first
* of `a` or `b`). The native path is preferred because it propagates the
* originating signal's `reason` and participates in GC the way the spec
* defines.
*/
function anySignal(a, b) {
	if (typeof AbortSignal.any === "function") return AbortSignal.any([a, b]);
	const controller = new AbortController();
	if (a.aborted) return controller.abort(a.reason), controller.signal;
	if (b.aborted) return controller.abort(b.reason), controller.signal;
	const cleanup = () => {
		a.removeEventListener("abort", onA);
		b.removeEventListener("abort", onB);
	};
	function onA() {
		cleanup();
		controller.abort(a.reason);
	}
	function onB() {
		cleanup();
		controller.abort(b.reason);
	}
	a.addEventListener("abort", onA, { once: true });
	b.addEventListener("abort", onB, { once: true });
	return controller.signal;
}
/**
* Client transport for Streamable HTTP: this implements the MCP Streamable HTTP transport specification.
* It will connect to a server using HTTP `POST` for sending messages and HTTP `GET` with Server-Sent Events
* for receiving messages.
*/
var StreamableHTTPClientTransport = class {
	_abortController;
	_url;
	_resourceMetadataUrl;
	_scope;
	_requestInit;
	_authProvider;
	_oauthProvider;
	_skipIssuerMetadataValidation;
	_fetch;
	_fetchWithInit;
	_sessionId;
	_reconnectionOptions;
	_protocolVersion;
	_onInsufficientScope;
	_maxStepUpRetries;
	_serverRetryMs;
	_reconnectionScheduler;
	_cancelReconnection;
	onclose;
	onerror;
	onmessage;
	/**
	* Streamable HTTP opens one POST (and SSE response stream) per outbound
	* request and honors `TransportSendOptions.requestSignal`. On a 2026-era
	* connection the protocol layer aborts that per-request stream as the
	* spec cancellation signal instead of POSTing `notifications/cancelled`.
	*/
	hasPerRequestStream = true;
	constructor(url, opts) {
		this._url = url;
		this._resourceMetadataUrl = void 0;
		this._scope = void 0;
		this._requestInit = opts?.requestInit;
		this._skipIssuerMetadataValidation = opts?.skipIssuerMetadataValidation;
		if (isOAuthClientProvider(opts?.authProvider)) {
			this._oauthProvider = opts.authProvider;
			this._authProvider = adaptOAuthProvider(opts.authProvider, { skipIssuerMetadataValidation: opts.skipIssuerMetadataValidation });
		} else this._authProvider = opts?.authProvider;
		this._fetch = opts?.fetch;
		this._fetchWithInit = createFetchWithInit(opts?.fetch, opts?.requestInit);
		this._sessionId = opts?.sessionId;
		this._protocolVersion = opts?.protocolVersion;
		this._reconnectionOptions = opts?.reconnectionOptions ?? DEFAULT_STREAMABLE_HTTP_RECONNECTION_OPTIONS;
		this._reconnectionScheduler = opts?.reconnectionScheduler;
		this._onInsufficientScope = opts?.onInsufficientScope ?? "reauthorize";
		this._maxStepUpRetries = Math.max(0, opts?.maxStepUpRetries ?? DEFAULT_MAX_STEP_UP_RETRIES);
	}
	/**
	* SEP-2350 step-up: compute the union scope, decide whether refresh must be
	* bypassed, and run {@linkcode auth}. Returns the auth result so the caller
	* can decide whether to retry. Shared by the POST `_send` path and the GET
	* `_startOrAuthSse` path so both apply the same `'throw'` short-circuit,
	* the same superset-gated refresh bypass, and the same retry cap.
	*/
	async _stepUpAuthorize(challenge, stepUpRetries) {
		try {
			return await this._stepUpAuthorizeInner(challenge, stepUpRetries);
		} catch (error) {
			throw markAuthSeamEscape(error);
		}
	}
	async _stepUpAuthorizeInner(challenge, stepUpRetries) {
		if (this._onInsufficientScope === "throw") throw new InsufficientScopeError({
			requiredScope: challenge.scope,
			resourceMetadataUrl: challenge.resourceMetadataUrl,
			errorDescription: challenge.errorDescription
		});
		if (!this._oauthProvider) throw new InsufficientScopeError({
			requiredScope: challenge.scope,
			resourceMetadataUrl: challenge.resourceMetadataUrl,
			errorDescription: challenge.errorDescription
		});
		if (stepUpRetries >= this._maxStepUpRetries) throw new SdkHttpError(SdkErrorCode.ClientHttpForbidden, `Server returned 403 insufficient_scope after step-up re-authorization (retry limit ${this._maxStepUpRetries} reached)`, {
			status: 403,
			statusText: challenge.statusText ?? "Forbidden",
			text: challenge.text
		});
		if (challenge.resourceMetadataUrl) this._resourceMetadataUrl = challenge.resourceMetadataUrl;
		const tokens = await this._oauthProvider.tokens();
		const unionScope = computeScopeUnion(this._scope, tokens?.scope, challenge.scope);
		this._scope = unionScope;
		const forceReauthorization = isStrictScopeSuperset(unionScope, tokens?.scope);
		return auth(this._oauthProvider, {
			serverUrl: this._url,
			resourceMetadataUrl: this._resourceMetadataUrl,
			scope: unionScope,
			forceReauthorization,
			fetchFn: this._fetchWithInit,
			skipIssuerMetadataValidation: this._skipIssuerMetadataValidation
		});
	}
	async _commonHeaders() {
		const headers = {};
		let token;
		try {
			token = await this._authProvider?.token();
		} catch (error) {
			throw markAuthSeamEscape(error);
		}
		if (token) headers["Authorization"] = `Bearer ${token}`;
		if (this._sessionId) headers["mcp-session-id"] = this._sessionId;
		if (this._protocolVersion) headers["mcp-protocol-version"] = this._protocolVersion;
		const extraHeaders = normalizeHeaders(this._requestInit?.headers);
		return new Headers({
			...headers,
			...extraHeaders
		});
	}
	/**
	* Body-derived per-request headers: when an outgoing request carries a
	* protocol-version claim in its `_meta` envelope (the version negotiation
	* probe is the first such sender), `MCP-Protocol-Version` and `Mcp-Method`
	* derive from the message itself. The connection-level version slot is
	* neither consulted nor mutated; messages without an envelope claim are
	* untouched, so no 2026 header can appear on a legacy exchange.
	*/
	_applyBodyDerivedHeaders(headers, message) {
		if (Array.isArray(message) || !isJSONRPCRequest(message)) return;
		const envelopeVersion = (message.params?._meta)?.[PROTOCOL_VERSION_META_KEY];
		if (typeof envelopeVersion !== "string") return;
		headers.set("mcp-protocol-version", envelopeVersion);
		headers.set("mcp-method", message.method);
		const params = message.params;
		const nameHeader = message.method === "resources/read" ? typeof params?.uri === "string" ? params.uri : void 0 : typeof params?.name === "string" ? params.name : void 0;
		if (nameHeader !== void 0) headers.set("mcp-name", encodeMcpParamValue(nameHeader));
	}
	/**
	* `true` when the outbound message is a single request carrying a
	* modern-era protocol-version envelope claim — the same predicate that
	* gates body-derived `mcp-method`/`mcp-name` emission. Used to confine the
	* 400-body-as-ProtocolError delivery to modern-era exchanges only.
	*/
	_isModernEnvelopedRequest(message) {
		if (Array.isArray(message) || !isJSONRPCRequest(message)) return false;
		const v = (message.params?._meta)?.[PROTOCOL_VERSION_META_KEY];
		return typeof v === "string" && isModernProtocolVersion(v);
	}
	async _startOrAuthSse(options, isAuthRetry = false, stepUpRetries = 0) {
		const { resumptionToken, requestSignal } = options;
		const isIntentionalAbort = () => this._abortController?.signal.aborted === true || requestSignal?.aborted === true;
		try {
			const headers = await this._commonHeaders();
			const types = [...headers.get("accept")?.split(",").map((s) => s.trim().toLowerCase()) ?? [], "text/event-stream"];
			headers.set("accept", [...new Set(types)].join(", "));
			if (resumptionToken) headers.set("last-event-id", resumptionToken);
			const transportSignal = this._abortController?.signal;
			const signal = requestSignal !== void 0 && transportSignal !== void 0 ? anySignal(transportSignal, requestSignal) : requestSignal ?? transportSignal;
			const response = await (this._fetch ?? fetch)(this._url, {
				...this._requestInit,
				method: "GET",
				headers,
				signal
			});
			if (!response.ok) {
				if (response.status === 401 && this._authProvider) {
					if (response.headers.has("www-authenticate")) {
						const { resourceMetadataUrl, scope } = extractWWWAuthenticateParams(response);
						this._resourceMetadataUrl = resourceMetadataUrl;
						this._scope = computeScopeUnion(this._scope, scope);
					}
					if (this._authProvider.onUnauthorized && !isAuthRetry) {
						try {
							await this._authProvider.onUnauthorized({
								response,
								serverUrl: this._url,
								fetchFn: this._fetchWithInit
							});
						} catch (error) {
							throw markAuthSeamEscape(error);
						}
						await response.text?.().catch(() => {});
						return this._startOrAuthSse(options, true, stepUpRetries);
					}
					await response.text?.().catch(() => {});
					if (isAuthRetry) throw markAuthSeamEscape(new SdkHttpError(SdkErrorCode.ClientHttpAuthentication, "Server returned 401 after re-authentication", {
						status: 401,
						statusText: response.statusText
					}));
					throw markAuthSeamEscape(new UnauthorizedError());
				}
				if (response.status === 403) {
					const { resourceMetadataUrl, scope, error, errorDescription } = extractWWWAuthenticateParams(response);
					if (error === "insufficient_scope") {
						const text = await response.text?.().catch(() => null);
						if (await this._stepUpAuthorize({
							scope,
							resourceMetadataUrl,
							errorDescription,
							statusText: response.statusText,
							text
						}, stepUpRetries) !== "AUTHORIZED") throw markAuthSeamEscape(new UnauthorizedError());
						return this._startOrAuthSse(options, isAuthRetry, stepUpRetries + 1);
					}
				}
				await response.text?.().catch(() => {});
				if (response.status === 405) {
					options.onRequestStreamEnd?.();
					return;
				}
				throw new SdkHttpError(SdkErrorCode.ClientHttpFailedToOpenStream, `Failed to open SSE stream: ${response.statusText}`, {
					status: response.status,
					statusText: response.statusText
				});
			}
			this._handleSseStream(response.body, options, true);
		} catch (error) {
			if (!isIntentionalAbort()) this.onerror?.(error);
			throw error;
		}
	}
	/**
	* Calculates the next reconnection delay using a backoff algorithm
	*
	* @param attempt Current reconnection attempt count for the specific stream
	* @returns Time to wait in milliseconds before next reconnection attempt
	*/
	_getNextReconnectionDelay(attempt) {
		if (this._serverRetryMs !== void 0) return this._serverRetryMs;
		const initialDelay = this._reconnectionOptions.initialReconnectionDelay;
		const growFactor = this._reconnectionOptions.reconnectionDelayGrowFactor;
		const maxDelay = this._reconnectionOptions.maxReconnectionDelay;
		return Math.min(initialDelay * Math.pow(growFactor, attempt), maxDelay);
	}
	/**
	* Schedule a reconnection attempt using server-provided retry interval or backoff
	*
	* @param lastEventId The ID of the last received event for resumability
	* @param attemptCount Current reconnection attempt count for this specific stream
	*/
	_scheduleReconnection(options, attemptCount = 0) {
		const maxRetries = this._reconnectionOptions.maxRetries;
		if (attemptCount >= maxRetries) {
			this.onerror?.(/* @__PURE__ */ new Error(`Maximum reconnection attempts (${maxRetries}) exceeded.`));
			options.onRequestStreamEnd?.();
			return;
		}
		const delay = this._getNextReconnectionDelay(attemptCount);
		const reconnect = () => {
			this._cancelReconnection = void 0;
			if (this._abortController?.signal.aborted || options.requestSignal?.aborted) return;
			this._startOrAuthSse(options).catch((error) => {
				if (this._abortController?.signal.aborted || options.requestSignal?.aborted) return;
				this.onerror?.(/* @__PURE__ */ new Error(`Failed to reconnect SSE stream: ${error instanceof Error ? error.message : String(error)}`));
				try {
					this._scheduleReconnection(options, attemptCount + 1);
				} catch (scheduleError) {
					this.onerror?.(scheduleError instanceof Error ? scheduleError : new Error(String(scheduleError)));
				}
			});
		};
		if (this._reconnectionScheduler) {
			const cancel = this._reconnectionScheduler(reconnect, delay, attemptCount);
			this._cancelReconnection = typeof cancel === "function" ? cancel : void 0;
		} else {
			const handle = setTimeout(reconnect, delay);
			this._cancelReconnection = () => clearTimeout(handle);
		}
	}
	_handleSseStream(stream, options, isReconnectable) {
		if (!stream) {
			options.onRequestStreamEnd?.();
			return;
		}
		const { onresumptiontoken, replayMessageId, requestSignal, onRequestStreamEnd } = options;
		const isIntentionalAbort = () => this._abortController?.signal.aborted === true || requestSignal?.aborted === true;
		let lastEventId;
		let hasPrimingEvent = false;
		let receivedResponse = false;
		const processStream = async () => {
			try {
				const reader = stream.pipeThrough(new TextDecoderStream()).pipeThrough(new EventSourceParserStream({ onRetry: (retryMs) => {
					this._serverRetryMs = retryMs;
				} })).getReader();
				while (true) {
					const { value: event, done } = await reader.read();
					if (done) break;
					if (event.id) {
						lastEventId = event.id;
						hasPrimingEvent = true;
						onresumptiontoken?.(event.id);
					}
					if (!event.data) continue;
					if (!event.event || event.event === "message") try {
						const message = JSONRPCMessageSchema.parse(JSON.parse(event.data));
						if (isJSONRPCResultResponse(message) || isJSONRPCErrorResponse(message)) {
							receivedResponse = true;
							if (replayMessageId !== void 0) message.id = replayMessageId;
						}
						this.onmessage?.(message);
					} catch (error) {
						this.onerror?.(error);
					}
				}
				if ((isReconnectable || hasPrimingEvent) && !receivedResponse && this._abortController && !isIntentionalAbort()) this._scheduleReconnection({
					resumptionToken: lastEventId,
					onresumptiontoken,
					replayMessageId,
					requestSignal,
					onRequestStreamEnd
				}, 0);
				else if (!isIntentionalAbort()) onRequestStreamEnd?.();
			} catch (error) {
				if (isIntentionalAbort()) return;
				this.onerror?.(/* @__PURE__ */ new Error(`SSE stream disconnected: ${error}`));
				if ((isReconnectable || hasPrimingEvent) && !receivedResponse && this._abortController && !isIntentionalAbort()) try {
					this._scheduleReconnection({
						resumptionToken: lastEventId,
						onresumptiontoken,
						replayMessageId,
						requestSignal,
						onRequestStreamEnd
					}, 0);
				} catch (error$1) {
					this.onerror?.(/* @__PURE__ */ new Error(`Failed to reconnect: ${error$1 instanceof Error ? error$1.message : String(error$1)}`));
					onRequestStreamEnd?.();
				}
				else onRequestStreamEnd?.();
			}
		};
		processStream();
	}
	async start() {
		if (this._abortController) throw new Error("StreamableHTTPClientTransport already started! If using Client class, note that connect() calls start() automatically.");
		this._abortController = new AbortController();
	}
	async finishAuth(codeOrParams, iss) {
		if (!this._oauthProvider) throw new UnauthorizedError("finishAuth requires an OAuthClientProvider");
		const { authorizationCode, iss: issParam } = await resolveAuthorizationCallbackParams(codeOrParams, iss, this._oauthProvider, this._url, {
			fetchFn: this._fetchWithInit,
			resourceMetadataUrl: this._resourceMetadataUrl
		});
		if (await auth(this._oauthProvider, {
			serverUrl: this._url,
			authorizationCode,
			iss: issParam,
			resourceMetadataUrl: this._resourceMetadataUrl,
			scope: this._scope,
			fetchFn: this._fetchWithInit,
			skipIssuerMetadataValidation: this._skipIssuerMetadataValidation
		}) !== "AUTHORIZED") throw new UnauthorizedError("Failed to authorize");
	}
	async close() {
		try {
			this._cancelReconnection?.();
		} finally {
			this._cancelReconnection = void 0;
			this._abortController?.abort();
			this.onclose?.();
		}
	}
	async send(message, options) {
		return this._send(message, options, false);
	}
	async _send(message, options, isAuthRetry, stepUpRetries = 0) {
		try {
			const { resumptionToken, onresumptiontoken } = options || {};
			if (resumptionToken) {
				this._startOrAuthSse({
					resumptionToken,
					replayMessageId: isJSONRPCRequest(message) ? message.id : void 0,
					requestSignal: options?.requestSignal
				}).catch((error) => this.onerror?.(error));
				return;
			}
			const headers = await this._commonHeaders();
			this._applyBodyDerivedHeaders(headers, message);
			const isHandshake = Array.isArray(message) ? message.some((m) => isInitializeRequest(m)) : isInitializeRequest(message);
			if (isHandshake) headers.delete("mcp-session-id");
			if (options?.headers !== void 0) for (const [name, value] of Object.entries(options.headers)) {
				if (RESERVED_REQUEST_HEADER_NAMES.has(name.toLowerCase())) continue;
				headers.set(name, value);
			}
			headers.set("content-type", "application/json");
			const types = [
				...headers.get("accept")?.split(",").map((s) => s.trim().toLowerCase()) ?? [],
				"application/json",
				"text/event-stream"
			];
			headers.set("accept", [...new Set(types)].join(", "));
			const transportSignal = this._abortController?.signal;
			const signal = options?.requestSignal !== void 0 && transportSignal !== void 0 ? anySignal(transportSignal, options.requestSignal) : options?.requestSignal ?? transportSignal;
			const init = {
				...this._requestInit,
				method: "POST",
				headers,
				body: JSON.stringify(message),
				signal
			};
			const response = await (this._fetch ?? fetch)(this._url, init);
			if (isHandshake && response.ok) this._sessionId = response.headers.get("mcp-session-id") || void 0;
			if (!response.ok) {
				if (response.status === 401 && this._authProvider) {
					if (response.headers.has("www-authenticate")) {
						const { resourceMetadataUrl, scope } = extractWWWAuthenticateParams(response);
						this._resourceMetadataUrl = resourceMetadataUrl;
						this._scope = computeScopeUnion(this._scope, scope);
					}
					if (this._authProvider.onUnauthorized && !isAuthRetry) {
						try {
							await this._authProvider.onUnauthorized({
								response,
								serverUrl: this._url,
								fetchFn: this._fetchWithInit
							});
						} catch (error) {
							throw markAuthSeamEscape(error);
						}
						await response.text?.().catch(() => {});
						return this._send(message, options, true, stepUpRetries);
					}
					await response.text?.().catch(() => {});
					if (isAuthRetry) throw markAuthSeamEscape(new SdkHttpError(SdkErrorCode.ClientHttpAuthentication, "Server returned 401 after re-authentication", {
						status: 401,
						statusText: response.statusText
					}));
					throw markAuthSeamEscape(new UnauthorizedError());
				}
				const text = await response.text?.().catch(() => null);
				if (response.status === 403) {
					const { resourceMetadataUrl, scope, error, errorDescription } = extractWWWAuthenticateParams(response);
					if (error === "insufficient_scope") {
						if (await this._stepUpAuthorize({
							scope,
							resourceMetadataUrl,
							errorDescription,
							statusText: response.statusText,
							text
						}, stepUpRetries) !== "AUTHORIZED") throw markAuthSeamEscape(new UnauthorizedError());
						return this._send(message, options, isAuthRetry, stepUpRetries + 1);
					}
				}
				if (response.status === 400 && typeof text === "string" && this._isModernEnvelopedRequest(message)) try {
					const parsed = JSONRPCMessageSchema.parse(JSON.parse(text));
					const requests = (Array.isArray(message) ? message : [message]).filter((m) => isJSONRPCRequest(m));
					if (isJSONRPCErrorResponse(parsed) && requests.some((r) => r.id === parsed.id)) {
						this.onmessage?.(parsed);
						return;
					}
				} catch {}
				throw new SdkHttpError(SdkErrorCode.ClientHttpNotImplemented, `Error POSTing to endpoint: ${text}`, {
					status: response.status,
					statusText: response.statusText,
					text
				});
			}
			if (response.status === 202) {
				await response.text?.().catch(() => {});
				if (isInitializedNotification(message)) this._startOrAuthSse({ resumptionToken: void 0 }).catch((error) => this.onerror?.(error));
				return;
			}
			const hasRequests = (Array.isArray(message) ? message : [message]).some((msg) => "method" in msg && "id" in msg && msg.id !== void 0);
			const contentType = response.headers.get("content-type");
			const responseMediaType = mediaTypeEssence(contentType);
			if (hasRequests) if (responseMediaType === "text/event-stream") this._handleSseStream(response.body, {
				onresumptiontoken,
				requestSignal: options?.requestSignal,
				onRequestStreamEnd: options?.onRequestStreamEnd
			}, false);
			else if (responseMediaType === "application/json") {
				const data = await response.json();
				const responseMessages = Array.isArray(data) ? data.map((msg) => JSONRPCMessageSchema.parse(msg)) : [JSONRPCMessageSchema.parse(data)];
				for (const msg of responseMessages) this.onmessage?.(msg);
			} else {
				await response.text?.().catch(() => {});
				throw new SdkError(SdkErrorCode.ClientHttpUnexpectedContent, `Unexpected content type: ${contentType}`, { contentType });
			}
			else await response.text?.().catch(() => {});
		} catch (error) {
			if (options?.requestSignal?.aborted !== true) this.onerror?.(error);
			throw error;
		}
	}
	get sessionId() {
		return this._sessionId;
	}
	/**
	* Terminates the current session by sending a `DELETE` request to the server.
	*
	* Clients that no longer need a particular session
	* (e.g., because the user is leaving the client application) SHOULD send an
	* HTTP `DELETE` to the MCP endpoint with the `Mcp-Session-Id` header to explicitly
	* terminate the session.
	*
	* The server MAY respond with HTTP `405 Method Not Allowed`, indicating that
	* the server does not allow clients to terminate sessions.
	*/
	async terminateSession() {
		if (!this._sessionId) return;
		try {
			const headers = await this._commonHeaders();
			const init = {
				...this._requestInit,
				method: "DELETE",
				headers,
				signal: this._abortController?.signal
			};
			const response = await (this._fetch ?? fetch)(this._url, init);
			await response.text?.().catch(() => {});
			if (!response.ok && response.status !== 405) throw new SdkHttpError(SdkErrorCode.ClientHttpFailedToTerminateSession, `Failed to terminate session: ${response.statusText}`, {
				status: response.status,
				statusText: response.statusText
			});
			this._sessionId = void 0;
		} catch (error) {
			this.onerror?.(error);
			throw error;
		}
	}
	setProtocolVersion(version) {
		this._protocolVersion = version;
	}
	get protocolVersion() {
		return this._protocolVersion;
	}
	/**
	* Resume an SSE stream from a previous event ID.
	* Opens a `GET` SSE connection with `Last-Event-ID` header to replay missed events.
	*
	* @param lastEventId The event ID to resume from
	* @param options Optional callback to receive new resumption tokens
	*/
	async resumeStream(lastEventId, options) {
		await this._startOrAuthSse({
			resumptionToken: lastEventId,
			onresumptiontoken: options?.onresumptiontoken
		});
	}
};

//#endregion
//#region src/fromJsonSchema.ts
let _defaultValidator;
function fromJsonSchema(schema, validator) {
	return fromJsonSchema$1(schema, validator ?? (_defaultValidator ??= new DefaultJsonSchemaValidator()));
}

//#endregion
export { AuthorizationServerMismatchError, BAGGAGE_META_KEY, CLIENT_CAPABILITIES_META_KEY, CLIENT_INFO_META_KEY, Client, ClientCredentialsProvider, CrossAppAccessProvider, DEFAULT_NEGOTIATED_PROTOCOL_VERSION, DEFAULT_REQUEST_TIMEOUT_MSEC, INTERNAL_ERROR, INVALID_PARAMS, INVALID_REQUEST, InMemoryResponseCacheStore, InMemoryTransport, InsecureTokenEndpointError, InsufficientScopeError, IssuerMismatchError, JSONRPC_VERSION, LATEST_PROTOCOL_VERSION, LOG_LEVEL_META_KEY, MAX_CACHE_TTL_MS, METHOD_NOT_FOUND, MissingRequiredClientCapabilityError, OAuthClientFlowError, OAuthError, OAuthErrorCode, PARSE_ERROR, PROTOCOL_VERSION_META_KEY, PrivateKeyJwtProvider, Protocol, ProtocolError, ProtocolErrorCode, RELATED_TASK_META_KEY, ReadBuffer, RegistrationRejectedError, ResourceNotFoundError, SERVER_INFO_META_KEY, SSEClientTransport, STDIO_DEFAULT_MAX_BUFFER_SIZE, SUBSCRIPTION_ID_META_KEY, SUPPORTED_PROTOCOL_VERSIONS, SdkError, SdkErrorCode, SdkHttpError, SseError, StaticPrivateKeyJwtProvider, StreamableHTTPClientTransport, TRACEPARENT_META_KEY, TRACESTATE_META_KEY, UnauthorizedError, UnsupportedProtocolVersionError, UriTemplate, UrlElicitationRequiredError, applyMiddlewares, assertCompleteRequestPrompt, assertCompleteRequestResourceTemplate, assertSecureTokenEndpoint, auth, buildDiscoveryUrls, checkResourceAllowed, computeScopeUnion, createFetchWithInit, createMiddleware, createPrivateKeyJwtAuth, deserializeMessage, discoverAndRequestJwtAuthGrant, discoverAuthorizationServerMetadata, discoverOAuthMetadata, discoverOAuthProtectedResourceMetadata, discoverOAuthServerInfo, exchangeAuthorization, exchangeJwtAuthGrant, extractResourceMetadataUrl, extractWWWAuthenticateParams, fetchToken, fromJsonSchema, getDisplayName, getSupportedElicitationModes, isCallToolResult, isHttpsUrl, isInitializeRequest, isInitializedNotification, isInputRequiredResult, isJSONRPCErrorResponse, isJSONRPCNotification, isJSONRPCRequest, isJSONRPCResponse, isJSONRPCResultResponse, isJsonContentType, isSpecType, isStrictScopeSuperset, isTaskAugmentedRequestParams, mergeCapabilities, parseErrorResponse, parseJSONRPCMessage, preloadSchemas, prepareAuthorizationCodeRequest, refreshAuthorization, registerClient, requestJwtAuthorizationGrant, resolveClientMetadata, resourceUrlFromServerUrl, selectClientAuthMethod, selectResourceURL, serializeMessage, specTypeSchemas, startAuthorization, validateAuthorizationResponseIssuer, validateClientMetadataUrl, withInputRequired, withLogging, withOAuth };
//# sourceMappingURL=index.mjs.map