import { $ as withInputRequired, $i as TRACESTATE_META_KEY, $n as Notification, $r as SetLevelRequest, $t as GetTaskResult, A as SpecTypes, Ai as UnsubscribeRequest, An as ListPromptsRequest, Ar as RequestTypeMap, At as CreateMessageRequestParamsWithTools, B as isJSONRPCNotification, Bi as INVALID_PARAMS, Bn as ListToolsRequest, Br as Result, Bt as ElicitRequestURLParams, C as ServerContext, Ca as StoredOAuthTokens, Ci as Tool, Cn as JSONRPCResponse, Cr as Request, Ct as CompleteRequestPrompt, D as TransportSendOptions, Di as ToolListChangedNotification, Dn as ListChangedCallback, Dr as RequestMetaObject, Dt as CreateMessageRequest, E as Transport, Ei as ToolExecution, En as LegacyTitledEnumSchema, Er as RequestMetaEnvelope, Et as ContentBlock, F as isCallToolResult, Fi as BAGGAGE_META_KEY, Fn as ListResourcesResult, Fr as ResourceRequestParams, Ft as DiscoverRequest, G as parseJSONRPCMessage, Gi as METHOD_NOT_FOUND, Gn as MessageClassification, Gr as RootsListChangedNotification, Gt as EmptyResult, H as isJSONRPCResponse, Hi as JSONRPC_VERSION, Hn as LoggingLevel, Hr as ResultTypeMap, Ht as ElicitationCompleteNotification, I as isInitializeRequest, Ii as CLIENT_CAPABILITIES_META_KEY, In as ListRootsRequest, Ir as ResourceTemplateReference, It as DiscoverResult, J as ResourceNotFoundError, Ji as RELATED_TASK_META_KEY, Jn as MethodNotFoundError, Jr as SamplingMessageContentBlock, Jt as GetPromptRequestParams, K as MissingRequiredClientCapabilityError, Ki as PARSE_ERROR, Kn as MessageExtraInfo, Kr as SamplingContent, Kt as EnumSchema, L as isInitializedNotification, Li as CLIENT_INFO_META_KEY, Ln as ListRootsResult, Lr as ResourceTemplateType, Lt as ElicitRequest, M as specTypeSchemas, Mi as UnsupportedProtocolVersionErrorData, Mn as ListResourceTemplatesRequest, Mr as ResourceContents, Mt as CreateMessageResultWithTools, N as assertCompleteRequestPrompt, Ni as UntitledMultiSelectEnumSchema, Nn as ListResourceTemplatesResult, Nr as ResourceLink, Nt as CreateTaskResult, O as createFetchWithInit, Oi as ToolResultContent, On as ListChangedHandlers, Or as RequestMethod, Ot as CreateMessageRequestParams, P as assertCompleteRequestResourceTemplate, Pi as UntitledSingleSelectEnumSchema, Pn as ListResourcesRequest, Pr as ResourceListChangedNotification, Pt as Cursor, Q as InputRequiredOptions, Qi as TRACEPARENT_META_KEY, Qn as MultiSelectEnumSchema, Qr as ServerResult, Qt as GetTaskRequest, R as isInputRequiredResult, Ri as DEFAULT_NEGOTIATED_PROTOCOL_VERSION, Rn as ListTasksRequest, Rr as ResourceUpdatedNotification, Rt as ElicitRequestFormParams, S as RequestStateAccessor, Sa as StoredOAuthClientInformation, Si as TitledSingleSelectEnumSchema, Sn as JSONRPCRequest, Sr as RelatedTaskMetadata, St as CompleteRequestParams, T as FetchLike, Ti as ToolChoice, Tn as JSONValue, Tr as RequestMeta, Tt as CompleteResult, U as isJSONRPCResultResponse, Ui as LATEST_PROTOCOL_VERSION, Un as LoggingMessageNotification, Ur as Role, Ut as ElicitationCompleteNotificationParams, V as isJSONRPCRequest, Vi as INVALID_REQUEST, Vn as ListToolsResult, Vr as ResultMetaObject, Vt as ElicitResult, W as isTaskAugmentedRequestParams, Wi as LOG_LEVEL_META_KEY, Wn as LoggingMessageNotificationParams, Wr as Root, Wt as EmbeddedResource, X as UrlElicitationRequiredError, Xi as SUBSCRIPTION_ID_META_KEY, Xn as ModelHint, Xr as ServerNotification, Xt as GetTaskPayloadRequest, Y as UnsupportedProtocolVersionError, Yi as SERVER_INFO_META_KEY, Yn as MissingRequiredClientCapabilityErrorData, Yr as ServerCapabilities, Yt as GetPromptResult, Z as ProtocolErrorCode, Zi as SUPPORTED_PROTOCOL_VERSIONS, Zn as ModelPreferences, Zr as ServerRequest, Zt as GetTaskPayloadResult, _ as ProgressCallback, _a as OAuthProtectedResourceMetadata, _i as TaskStatusNotification, _n as JSONArray, _r as PromptMessage, _t as ClientNotification, a as ReadBuffer, aa as SdkHttpErrorData, ai as SubscriptionFilter, an as InitializeRequest, ar as PaginatedRequestParams, at as AuthInfo, b as RequestHandlerSchemas, ba as OpenIdProviderDiscoveryMetadata, bi as TextResourceContents, bn as JSONRPCMessage, br as ReadResourceRequestParams, bt as CompatibilityCallToolResult, c as serializeMessage, ca as AuthorizationServerMetadata, ci as SubscriptionsListenRequest, cn as InitializedNotification, cr as PingRequest, ct as BooleanSchema, d as isJsonContentType, da as OAuthClientInformationFull, di as SubscriptionsListenResultMeta, dn as InputRequiredResult, dr as ProgressNotification, dt as CallToolResult, ea as checkResourceAllowed, ei as SetLevelRequestParams, en as HandlerResultTypeMap, er as NotificationMethod, et as StandardSchemaV1, f as BaseContext, fa as OAuthClientInformationMixed, fi as Task, fn as InputResponse, fr as ProgressNotificationParams, ft as CancelTaskRequest, g as NotificationOptions, ga as OAuthMetadata, gi as TaskStatus, gn as InvalidRequestError, gr as PromptListChangedNotification, gt as ClientCapabilities, h as NonCompleteResultFlow, ha as OAuthErrorResponse, hi as TaskMetadata, hn as InvalidParamsError, hr as PromptArgument, ht as CancelledNotificationParams, i as Variables, ia as SdkHttpError, ii as SubscribeRequestParams, in as Implementation, ir as PaginatedRequest, it as AudioContent, j as isSpecType, ji as UnsubscribeRequestParams, jn as ListPromptsResult, jr as Resource, jt as CreateMessageResult, k as SpecTypeName, ki as ToolUseContent, kn as ListChangedOptions, kr as RequestParams, kt as CreateMessageRequestParamsBase, l as ProtocolEra, la as IdJagTokenExchangeResponse, li as SubscriptionsListenRequestParams, ln as InputRequest, lr as PrimitiveSchemaDefinition, lt as CallToolRequest, m as DEFAULT_REQUEST_TIMEOUT_MSEC, ma as OAuthClientRegistrationError, mi as TaskCreationParams, mn as InternalError, mr as Prompt, mt as CancelledNotification, n as InMemoryTransport, na as SdkError, ni as StringSchema, nn as Icons, nr as NotificationTypeMap, nt as StandardSchemaWithJSON, o as STDIO_DEFAULT_MAX_BUFFER_SIZE, oa as OAuthError, oi as SubscriptionsAcknowledgedNotification, on as InitializeRequestParams, or as PaginatedResult, ot as BaseMetadata, p as ClientContext, pa as OAuthClientMetadata, pi as TaskAugmentedRequestParams, pn as InputResponses, pr as ProgressToken, pt as CancelTaskResult, q as ProtocolError, qi as PROTOCOL_VERSION_META_KEY, qn as MetaObject, qr as SamplingMessage, qt as GetPromptRequest, r as UriTemplate, ra as SdkErrorCode, ri as SubscribeRequest, rn as ImageContent, rr as NumberSchema, rt as Annotations, s as deserializeMessage, sa as OAuthErrorCode, si as SubscriptionsAcknowledgedNotificationParams, sn as InitializeResult, sr as ParseError, st as BlobResourceContents, t as preloadSchemas, ta as resourceUrlFromServerUrl, ti as SingleSelectEnumSchema, tn as Icon, tr as NotificationParams, tt as StandardSchemaV1Sync, u as getDisplayName, ua as OAuthClientInformation, ui as SubscriptionsListenResult, un as InputRequests, ur as Progress, ut as CallToolRequestParams, v as Protocol, va as OAuthTokenRevocationRequest, vi as TaskStatusNotificationParams, vn as JSONObject, vr as PromptReference, vt as ClientRequest, w as mergeCapabilities, wi as ToolAnnotations, wn as JSONRPCResultResponse, wr as RequestId, wt as CompleteRequestResourceTemplate, x as RequestOptions, xa as OpenIdProviderMetadata, xi as TitledMultiSelectEnumSchema, xn as JSONRPCNotification, xr as ReadResourceResult, xt as CompleteRequest, y as ProtocolOptions, ya as OAuthTokens, yi as TextContent, yn as JSONRPCErrorResponse, yr as ReadResourceRequest, yt as ClientResult, z as isJSONRPCErrorResponse, zi as INTERNAL_ERROR, zn as ListTasksResult, zr as ResourceUpdatedNotificationParams, zt as ElicitRequestParams } from "./index-CHhhk6PQ.cjs";
import { i as jsonSchemaValidator, n as JsonSchemaValidator, r as JsonSchemaValidatorResult, t as JsonSchemaType } from "./types-DsbMXNFL.cjs";
import { ErrorEvent, EventSourceInit } from "eventsource";

//#region src/client/authErrors.d.ts

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
declare class OAuthClientFlowError extends Error {
  static [Symbol.hasInstance](value: unknown): boolean;
  /**
   * Brand-based type guard: equivalent to `value instanceof this`, as an
   * explicit static predicate (the axios/AWS-SDK `isInstance` style). Reads
   * the caller's own brand via `this`, so every branded subclass gets a
   * correctly-scoped guard by inheritance. Must be invoked on the class —
   * in callback position write `v => SdkError.isInstance(v)`, not
   * `.filter(SdkError.isInstance)` (detached calls throw rather than
   * silently matching nothing).
   */
  static isInstance<T extends abstract new (...args: never[]) => unknown>(this: T, value: unknown): value is InstanceType<T>;
  constructor(message: string);
}
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
declare class IssuerMismatchError extends OAuthClientFlowError {
  /** Which check failed — metadata echo (RFC 8414 §3.3) or authorization-response `iss` (RFC 9207). */
  readonly kind: 'metadata' | 'authorization_response';
  /** The issuer the client expected (from validated metadata / discovery input). */
  readonly expected: string | undefined;
  /** The issuer value that was received. Attacker-controllable on the `'authorization_response'` path. */
  readonly received: string | undefined;
  constructor(kind: 'metadata' | 'authorization_response', expected: string | undefined, received: string | undefined);
}
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
declare class RegistrationRejectedError extends OAuthClientFlowError {
  /** HTTP status code returned by the registration endpoint. */
  readonly status: number;
  /** Raw response body text (typically an RFC 7591 error JSON document). */
  readonly body: string;
  /** The exact client metadata that was POSTed (after SDK defaults were applied). */
  readonly submittedMetadata: OAuthClientMetadata;
  constructor(args: {
    status: number;
    body: string;
    submittedMetadata: OAuthClientMetadata;
  });
}
/**
 * Thrown by the token-exchange and refresh paths when the resolved token
 * endpoint is not `https:` and is not a loopback host (SEP-2207). This is a
 * configuration error — re-authorizing cannot fix it — so it intentionally does
 * **not** extend `OAuthError` and `auth()`'s refresh branch rethrows it instead
 * of falling through to a fresh `/authorize` redirect.
 */
declare class InsecureTokenEndpointError extends OAuthClientFlowError {
  /** The token endpoint URL that was rejected. */
  readonly tokenEndpoint: string;
  constructor(tokenEndpoint: string);
}
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
declare class AuthorizationServerMismatchError extends OAuthClientFlowError {
  /** The issuer recorded in `discoveryState()` when the authorization redirect was issued. */
  readonly recordedIssuer: string;
  /** The issuer resolved by discovery on this call. */
  readonly currentIssuer: string;
  constructor(/** The issuer recorded in `discoveryState()` when the authorization redirect was issued. */
  recordedIssuer: string, /** The issuer resolved by discovery on this call. */
  currentIssuer: string);
}
declare class InsufficientScopeError extends OAuthClientFlowError {
  /** The `scope` value from the `WWW-Authenticate` challenge — the scopes the resource server says are required. */
  readonly requiredScope?: string;
  /** The `resource_metadata` URL from the `WWW-Authenticate` challenge, if present. */
  readonly resourceMetadataUrl?: URL;
  /** The `error_description` from the `WWW-Authenticate` challenge, if present. */
  readonly errorDescription?: string;
  constructor(init: {
    requiredScope?: string;
    resourceMetadataUrl?: URL;
    errorDescription?: string;
  });
}
//#endregion
//#region src/client/auth.d.ts
/**
 * Function type for adding client authentication to token requests.
 */
type AddClientAuthentication = (headers: Headers, params: URLSearchParams, url: string | URL, metadata?: AuthorizationServerMetadata) => void | Promise<void>;
/**
 * Context passed to {@linkcode AuthProvider.onUnauthorized} when the server
 * responds with 401. Provides everything needed to refresh credentials.
 */
interface UnauthorizedContext {
  /** The 401 response — inspect `WWW-Authenticate` for resource metadata, scope, etc. */
  response: Response;
  /** The MCP server URL, for passing to {@linkcode auth} or discovery helpers. */
  serverUrl: URL;
  /** Fetch function configured with the transport's `requestInit`, for making auth requests. */
  fetchFn: FetchLike;
}
/**
 * Minimal interface for authenticating MCP client transports with bearer tokens.
 *
 * Transports call {@linkcode AuthProvider.token | token()} before every request
 * to obtain the current token, and {@linkcode AuthProvider.onUnauthorized | onUnauthorized()}
 * (if provided) when the server responds with 401, giving the provider a chance
 * to refresh credentials before the transport retries once.
 *
 * For simple cases (API keys, gateway-managed tokens), implement only `token()`:
 * ```typescript
 * const authProvider: AuthProvider = { token: async () => process.env.API_KEY };
 * ```
 *
 * For OAuth flows, pass an {@linkcode OAuthClientProvider} directly — transports
 * accept either shape and adapt OAuth providers automatically via {@linkcode adaptOAuthProvider}.
 */
interface AuthProvider {
  /**
   * Returns the current bearer token, or `undefined` if no token is available.
   * Called before every request.
   */
  token(): Promise<string | undefined>;
  /**
   * Called when the server responds with 401. If provided, the transport will
   * await this, then retry the request once. If the retry also gets 401, or if
   * this method is not provided, the transport throws {@linkcode UnauthorizedError}.
   *
   * Implementations should refresh tokens, re-authenticate, etc. — whatever is
   * needed so the next `token()` call returns a valid token.
   */
  onUnauthorized?(ctx: UnauthorizedContext): Promise<void>;
}
/**
 * Context passed to the credential-persistence methods on
 * {@linkcode OAuthClientProvider} — `clientInformation` / `saveClientInformation`
 * and `tokens` / `saveTokens`. Carries the resolved authorization-server `issuer`
 * so provider implementations can key persisted credentials per authorization
 * server (RFC 6749 §2.2 — client identifiers are unique to the AS that issued
 * them). Providers that store a single credential set may ignore it.
 */
interface OAuthClientInformationContext {
  /**
   * The authorization server's `issuer` identifier from its validated metadata
   * document, used as the binding key for persisted credentials.
   */
  issuer: string;
}
/**
 * Implements an end-to-end OAuth client to be used with one MCP server.
 *
 * This client relies upon a concept of an authorized "session," the exact
 * meaning of which is application-defined. Tokens, authorization codes, and
 * code verifiers should not cross different sessions.
 *
 * Transports accept `OAuthClientProvider` directly via the `authProvider` option —
 * they adapt it to {@linkcode AuthProvider} internally via {@linkcode adaptOAuthProvider}.
 * No changes are needed to existing implementations.
 */
interface OAuthClientProvider {
  /**
   * The URL to redirect the user agent to after authorization.
   * Return `undefined` for non-interactive flows that don't require user interaction
   * (e.g., `client_credentials`, `jwt-bearer`).
   */
  get redirectUrl(): string | URL | undefined;
  /**
   * External URL the server should use to fetch client metadata document
   */
  clientMetadataUrl?: string;
  /**
   * Metadata about this OAuth client.
   */
  get clientMetadata(): OAuthClientMetadata;
  /**
   * Returns an OAuth2 state parameter.
   */
  state?(): string | Promise<string>;
  /**
   * Loads information about this OAuth client, as registered already with the
   * server, or returns `undefined` if the client is not registered with the
   * server.
   *
   * @param ctx - Carries the resolved authorization-server `issuer`. Providers
   *   that persist credentials per authorization server should return the entry
   *   keyed by `ctx.issuer`. Providers with a single credential set may ignore it.
   */
  clientInformation(ctx?: OAuthClientInformationContext): StoredOAuthClientInformation | undefined | Promise<StoredOAuthClientInformation | undefined>;
  /**
   * If implemented, this permits the OAuth client to dynamically register with
   * the server. Client information saved this way should later be read via
   * {@linkcode OAuthClientProvider.clientInformation | clientInformation()}.
   *
   * This method is not required to be implemented if client information is
   * statically known (e.g., pre-registered).
   *
   * @param ctx - Carries the resolved authorization-server `issuer`. Providers
   *   that persist credentials per authorization server should store the entry
   *   keyed by `ctx.issuer`.
   */
  saveClientInformation?(clientInformation: StoredOAuthClientInformation, ctx?: OAuthClientInformationContext): void | Promise<void>;
  /**
   * Loads any existing OAuth tokens for the current session, or returns
   * `undefined` if there are no saved tokens.
   *
   * @param ctx - Carries the resolved authorization-server `issuer`. Providers
   *   that persist tokens per authorization server should return the entry
   *   keyed by `ctx.issuer`. Providers with a single token set may ignore it.
   *   When called with no `ctx` — the transport's per-request bearer-token
   *   read — return the most-recently-saved token set; do not return
   *   `undefined` for `ctx === undefined`.
   */
  tokens(ctx?: OAuthClientInformationContext): StoredOAuthTokens | undefined | Promise<StoredOAuthTokens | undefined>;
  /**
   * Stores new OAuth tokens for the current session, after a successful
   * authorization.
   *
   * @param ctx - Carries the resolved authorization-server `issuer`. Providers
   *   that persist tokens per authorization server should store the entry
   *   keyed by `ctx.issuer`.
   */
  saveTokens(tokens: StoredOAuthTokens, ctx?: OAuthClientInformationContext): void | Promise<void>;
  /**
   * Invoked to redirect the user agent to the given URL to begin the authorization flow.
   */
  redirectToAuthorization(authorizationUrl: URL): void | Promise<void>;
  /**
   * Saves a PKCE code verifier for the current session, before redirecting to
   * the authorization flow.
   */
  saveCodeVerifier(codeVerifier: string): void | Promise<void>;
  /**
   * Loads the PKCE code verifier for the current session, necessary to validate
   * the authorization result.
   */
  codeVerifier(): string | Promise<string>;
  /**
   * Adds custom client authentication to OAuth token requests.
   *
   * This optional method allows implementations to customize how client credentials
   * are included in token exchange and refresh requests. When provided, this method
   * is called instead of the default authentication logic, giving full control over
   * the authentication mechanism.
   *
   * Common use cases include:
   * - Supporting authentication methods beyond the standard OAuth 2.0 methods
   * - Adding custom headers for proprietary authentication schemes
   * - Implementing client assertion-based authentication (e.g., JWT bearer tokens)
   *
   * @param headers - The request headers (can be modified to add authentication)
   * @param params - The request body parameters (can be modified to add credentials)
   * @param url - The token endpoint URL being called
   * @param metadata - Optional OAuth metadata for the server, which may include supported authentication methods
   */
  addClientAuthentication?: AddClientAuthentication;
  /**
   * If defined, overrides the selection and validation of the
   * RFC 8707 Resource Indicator. If left undefined, default
   * validation behavior will be used.
   *
   * Implementations must verify the returned resource matches the MCP server.
   */
  validateResourceURL?(serverUrl: string | URL, resource?: string): Promise<URL | undefined>;
  /**
   * If implemented, provides a way for the client to invalidate (e.g. delete) the specified
   * credentials, in the case where the server has indicated that they are no longer valid.
   * This avoids requiring the user to intervene manually.
   */
  invalidateCredentials?(scope: 'all' | 'client' | 'tokens' | 'verifier' | 'discovery'): void | Promise<void>;
  /**
   * Prepares grant-specific parameters for a token request.
   *
   * This optional method allows providers to customize the token request based on
   * the grant type they support. When implemented, it returns the grant type and
   * any grant-specific parameters needed for the token exchange.
   *
   * If not implemented, the default behavior depends on the flow:
   * - For authorization code flow: uses `code`, `code_verifier`, and `redirect_uri`
   * - For `client_credentials`: detected via `grant_types` in {@linkcode OAuthClientProvider.clientMetadata | clientMetadata}
   *
   * @param scope - Optional scope to request
   * @returns Grant type and parameters, or `undefined` to use default behavior
   *
   * @example
   * // For client_credentials grant:
   * prepareTokenRequest(scope) {
   *   return {
   *     grantType: 'client_credentials',
   *     params: scope ? { scope } : {}
   *   };
   * }
   *
   * @example
   * // For authorization_code grant (default behavior):
   * async prepareTokenRequest() {
   *   return {
   *     grantType: 'authorization_code',
   *     params: {
   *       code: this.authorizationCode,
   *       code_verifier: await this.codeVerifier(),
   *       redirect_uri: String(this.redirectUrl)
   *     }
   *   };
   * }
   */
  prepareTokenRequest?(scope?: string): URLSearchParams | Promise<URLSearchParams | undefined> | undefined;
  /**
   * Saves the resolved authorization-server **issuer**. Called after a successful
   * token exchange (timing changed in v2: was post-discovery, now post-`saveTokens`).
   *
   * @deprecated Superseded by the `issuer` stamp on stored tokens / client credentials
   * (SEP-2352). {@linkcode auth} still **writes** this for back-compat with providers
   * that read it (e.g. Cross-App Access), but the SDK never reads it. Prefer reading
   * the `issuer` field on the value passed to {@linkcode saveTokens} /
   * {@linkcode saveClientInformation}, or the `ctx.issuer` argument.
   */
  saveAuthorizationServerUrl?(authorizationServerUrl: string): void | Promise<void>;
  /**
   * Returns the previously saved authorization server URL, if available.
   *
   * @deprecated Superseded by the `issuer` stamp on stored tokens / client credentials
   * (SEP-2352). The SDK never reads this method; it remains for provider implementations
   * that consume the value internally (e.g. Cross-App Access).
   */
  authorizationServerUrl?(): string | undefined | Promise<string | undefined>;
  /**
   * Saves the resource URL after RFC 9728 discovery.
   * This method is called by {@linkcode auth} after successful discovery of the
   * resource metadata.
   *
   * Providers implementing Cross-App Access or other flows that need access to
   * the discovered resource URL should implement this method.
   *
   * @param resourceUrl - The resource URL discovered via RFC 9728
   */
  saveResourceUrl?(resourceUrl: string): void | Promise<void>;
  /**
   * Returns the previously saved resource URL, if available.
   *
   * Providers implementing Cross-App Access can use this to access the
   * resource URL discovered during the OAuth flow.
   *
   * @returns The resource URL, or `undefined` if not available
   */
  resourceUrl?(): string | undefined | Promise<string | undefined>;
  /**
   * Saves the OAuth discovery state after RFC 9728 and authorization server metadata
   * discovery. Providers can persist this state to avoid redundant discovery requests
   * on subsequent {@linkcode auth} calls.
   *
   * This state can also be provided out-of-band (e.g., from a previous session or
   * external configuration) to bootstrap the OAuth flow without discovery.
   *
   * Called by {@linkcode auth} after successful discovery.
   *
   * MUST persist with the same durability as `codeVerifier` (survives the redirect
   * round-trip).
   */
  saveDiscoveryState?(state: OAuthDiscoveryState): void | Promise<void>;
  /**
   * Returns previously saved discovery state, or `undefined` if none is cached.
   *
   * When available, {@linkcode auth} restores the discovery state (authorization server
   * URL, resource metadata, etc.) instead of performing RFC 9728 discovery, reducing
   * latency on subsequent calls.
   *
   * Hosts should call {@linkcode invalidateCredentials} with scope `'discovery'`
   * on repeated 401s so a changed `authorization_servers` list is picked up; the
   * SDK does not invoke that scope itself.
   *
   * MUST persist with the same durability as `codeVerifier` (survives the redirect
   * round-trip).
   */
  discoveryState?(): OAuthDiscoveryState | undefined | Promise<OAuthDiscoveryState | undefined>;
}
/**
 * Discovery state that can be persisted across sessions by an {@linkcode OAuthClientProvider}.
 *
 * Contains the results of RFC 9728 protected resource metadata discovery and
 * authorization server metadata discovery. Persisting this state avoids
 * redundant discovery HTTP requests on subsequent {@linkcode auth} calls.
 */
interface OAuthDiscoveryState extends OAuthServerInfo {
  /** The URL at which the protected resource metadata was found, if available. */
  resourceMetadataUrl?: string;
}
type AuthResult = 'AUTHORIZED' | 'REDIRECT';
declare class UnauthorizedError extends Error {
  static [Symbol.hasInstance](value: unknown): boolean;
  /**
   * Brand-based type guard: equivalent to `value instanceof this`, as an
   * explicit static predicate (the axios/AWS-SDK `isInstance` style). Reads
   * the caller's own brand via `this`, so every branded subclass gets a
   * correctly-scoped guard by inheritance. Must be invoked on the class —
   * in callback position write `v => SdkError.isInstance(v)`, not
   * `.filter(SdkError.isInstance)` (detached calls throw rather than
   * silently matching nothing).
   */
  static isInstance<T extends abstract new (...args: never[]) => unknown>(this: T, value: unknown): value is InstanceType<T>;
  constructor(message?: string);
}
declare function validateAuthorizationResponseIssuer({
  iss,
  expectedIssuer,
  issParameterSupported
}: {
  /** The form-urldecoded `iss` query parameter from the authorization callback, or `undefined` if absent. */
  iss: string | undefined;
  /** The `issuer` value from the authorization server's validated metadata document. */
  expectedIssuer: string | undefined;
  /** Whether the metadata advertised `authorization_response_iss_parameter_supported: true`. */
  issParameterSupported: boolean;
}): void;
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
declare function computeScopeUnion(...scopes: ReadonlyArray<string | undefined>): string | undefined;
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
declare function isStrictScopeSuperset(union: string | undefined, current: string | undefined): boolean;
type ClientAuthMethod = 'client_secret_basic' | 'client_secret_post' | 'none';
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
declare function selectClientAuthMethod(clientInformation: OAuthClientInformationMixed, supportedMethods: string[]): ClientAuthMethod;
/**
 * SEP-2207: refuse to send credentials to a non-TLS, non-loopback token endpoint.
 * Throws {@linkcode InsecureTokenEndpointError}. Loopback hosts are exempt.
 */
declare function assertSecureTokenEndpoint(tokenEndpoint: string | URL): URL;
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
declare function resolveClientMetadata(provider: Pick<OAuthClientProvider, 'clientMetadata' | 'redirectUrl'>): OAuthClientMetadata;
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
declare function parseErrorResponse(input: Response | string): Promise<OAuthError>;
/**
 * Options for {@linkcode auth}. The full OAuth flow orchestrator's input.
 */
interface AuthOptions {
  /** The MCP server URL — the protected resource the flow authorizes against. */
  serverUrl: string | URL;
  /**
   * The authorization code returned by the authorization server on the redirect
   * callback. When set, {@linkcode auth} exchanges it for tokens; when unset,
   * {@linkcode auth} runs discovery and either refreshes or initiates redirect.
   */
  authorizationCode?: string;
  /**
   * The form-urldecoded `iss` query parameter from the authorization callback,
   * if present. Passed through to RFC 9207 §2.4 issuer validation alongside
   * `authorizationCode`. Validated against the recorded issuer per RFC 9207
   * §2.4 before the code is redeemed — see
   * {@linkcode validateAuthorizationResponseIssuer} and the migration guide's
   * *Authorization-server mix-up defense* section.
   */
  iss?: string;
  /** Scope to request; computed by Scope Selection Strategy when omitted. */
  scope?: string;
  /** Explicit `resource_metadata` URL from a `WWW-Authenticate` challenge. */
  resourceMetadataUrl?: URL;
  /** Custom `fetch` implementation. */
  fetchFn?: FetchLike;
  /**
   * Opt-out for the RFC 8414 §3.3 issuer-echo check during authorization
   * server discovery. Disabling it is **security-weakening** and intended only
   * for authorization servers known to publish a mismatched `issuer`.
   *
   * @default false
   */
  skipIssuerMetadataValidation?: boolean;
  /**
   * When `true`, {@linkcode auth} skips the refresh-token branch even when a
   * `refresh_token` is available, and proceeds directly to a fresh
   * authorization request ({@linkcode startAuthorization}).
   *
   * Set by the transport's `403 insufficient_scope` step-up path when the
   * required scope is a strict superset of the current token's granted scope:
   * the refresh grant cannot widen scope (RFC 6749 §6), so refreshing would
   * silently drop the new scope and the next request would 403 again. Forcing
   * a fresh authorization request ensures the widened scope reaches the
   * authorization server.
   *
   * Hosts driving step-up themselves (with `onInsufficientScope: 'throw'`)
   * should set this when {@linkcode isStrictScopeSuperset} of the union over
   * the current token's `scope` is `true`.
   *
   * @default false
   */
  forceReauthorization?: boolean;
}
/**
 * Orchestrates the full auth flow with a server.
 *
 * This can be used as a single entry point for all authorization functionality,
 * instead of linking together the other lower-level functions in this module.
 */
declare function auth(provider: OAuthClientProvider, options: AuthOptions): Promise<AuthResult>;
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
declare function validateClientMetadataUrl(url: string | undefined): void;
/**
 * SEP-991: URL-based Client IDs
 * Validate that the `client_id` is a valid URL with `https` scheme
 */
declare function isHttpsUrl(value?: string): boolean;
declare function selectResourceURL(serverUrl: string | URL, provider: OAuthClientProvider, resourceMetadata?: OAuthProtectedResourceMetadata): Promise<URL | undefined>;
/**
 * Extract `resource_metadata`, `scope`, `error`, and `error_description` from a
 * `WWW-Authenticate` header.
 */
declare function extractWWWAuthenticateParams(res: Response): {
  resourceMetadataUrl?: URL;
  scope?: string;
  error?: string;
  errorDescription?: string;
};
/**
 * Extract `resource_metadata` from response header.
 * @deprecated Use {@linkcode extractWWWAuthenticateParams} instead.
 */
declare function extractResourceMetadataUrl(res: Response): URL | undefined;
/**
 * Looks up {@link https://datatracker.ietf.org/doc/html/rfc9728 | RFC 9728}
 * OAuth 2.0 Protected Resource Metadata.
 *
 * If the server returns a 404 for the well-known endpoint, this function will
 * return `undefined`. Any other errors will be thrown as exceptions.
 */
declare function discoverOAuthProtectedResourceMetadata(serverUrl: string | URL, opts?: {
  protocolVersion?: string;
  resourceMetadataUrl?: string | URL;
}, fetchFn?: FetchLike): Promise<OAuthProtectedResourceMetadata>;
/**
 * Looks up RFC 8414 OAuth 2.0 Authorization Server Metadata.
 *
 * If the server returns a 404 for the well-known endpoint, this function will
 * return `undefined`. Any other errors will be thrown as exceptions.
 *
 * @deprecated This function is deprecated in favor of {@linkcode discoverAuthorizationServerMetadata}.
 */
declare function discoverOAuthMetadata(issuer: string | URL, {
  authorizationServerUrl,
  protocolVersion
}?: {
  authorizationServerUrl?: string | URL;
  protocolVersion?: string;
}, fetchFn?: FetchLike): Promise<OAuthMetadata | undefined>;
/**
 * Builds a list of discovery URLs to try for authorization server metadata.
 * URLs are returned in priority order:
 * 1. OAuth metadata at the given URL
 * 2. OIDC metadata endpoints at the given URL
 */
declare function buildDiscoveryUrls(authorizationServerUrl: string | URL): {
  url: URL;
  type: 'oauth' | 'oidc';
}[];
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
declare function discoverAuthorizationServerMetadata(authorizationServerUrl: string | URL, {
  fetchFn,
  protocolVersion,
  skipIssuerValidation
}?: {
  fetchFn?: FetchLike;
  protocolVersion?: string;
  skipIssuerValidation?: boolean;
}): Promise<AuthorizationServerMetadata | undefined>;
/**
 * Result of {@linkcode discoverOAuthServerInfo}.
 */
interface OAuthServerInfo {
  /**
   * The authorization server URL, either discovered via RFC 9728
   * or derived from the MCP server URL as a fallback.
   */
  authorizationServerUrl: string;
  /**
   * The authorization server metadata (endpoints, capabilities),
   * or `undefined` if metadata discovery failed.
   */
  authorizationServerMetadata?: AuthorizationServerMetadata;
  /**
   * The OAuth 2.0 Protected Resource Metadata from RFC 9728,
   * or `undefined` if the server does not support it.
   */
  resourceMetadata?: OAuthProtectedResourceMetadata;
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
declare function discoverOAuthServerInfo(serverUrl: string | URL, opts?: {
  resourceMetadataUrl?: URL;
  fetchFn?: FetchLike;
  /**
   * Forwarded to {@linkcode discoverAuthorizationServerMetadata} as
   * `skipIssuerValidation`. **Security-weakening** — see {@linkcode AuthOptions.skipIssuerMetadataValidation}.
   */
  skipIssuerMetadataValidation?: boolean;
}): Promise<OAuthServerInfo>;
/**
 * Begins the authorization flow with the given server, by generating a PKCE challenge and constructing the authorization URL.
 */
declare function startAuthorization(authorizationServerUrl: string | URL, {
  metadata,
  clientInformation,
  redirectUrl,
  scope,
  state,
  resource
}: {
  metadata?: AuthorizationServerMetadata;
  clientInformation: OAuthClientInformationMixed;
  redirectUrl: string | URL;
  scope?: string;
  state?: string;
  resource?: URL;
}): Promise<{
  authorizationUrl: URL;
  codeVerifier: string;
}>;
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
declare function prepareAuthorizationCodeRequest(authorizationCode: string, codeVerifier: string, redirectUri: string | URL): URLSearchParams;
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
declare function exchangeAuthorization(authorizationServerUrl: string | URL, {
  metadata,
  clientInformation,
  authorizationCode,
  iss,
  codeVerifier,
  redirectUri,
  resource,
  addClientAuthentication,
  fetchFn
}: {
  metadata?: AuthorizationServerMetadata;
  clientInformation: OAuthClientInformationMixed;
  authorizationCode: string;
  /**
   * The form-urldecoded `iss` query parameter from the authorization callback.
   * Validated per RFC 9207 §2.4 against `metadata.issuer` before the code is
   * redeemed; see {@linkcode validateAuthorizationResponseIssuer}.
   */
  iss?: string;
  codeVerifier: string;
  redirectUri: string | URL;
  resource?: URL;
  addClientAuthentication?: OAuthClientProvider['addClientAuthentication'];
  fetchFn?: FetchLike;
}): Promise<OAuthTokens>;
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
declare function refreshAuthorization(authorizationServerUrl: string | URL, {
  metadata,
  clientInformation,
  refreshToken,
  resource,
  addClientAuthentication,
  fetchFn
}: {
  metadata?: AuthorizationServerMetadata;
  clientInformation: OAuthClientInformationMixed;
  refreshToken: string;
  resource?: URL;
  addClientAuthentication?: OAuthClientProvider['addClientAuthentication'];
  fetchFn?: FetchLike;
}): Promise<OAuthTokens>;
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
declare function fetchToken(provider: OAuthClientProvider, authorizationServerUrl: string | URL, {
  metadata,
  resource,
  authorizationCode,
  iss,
  scope,
  fetchFn
}?: {
  metadata?: AuthorizationServerMetadata;
  resource?: URL;
  /** Authorization code for the default `authorization_code` grant flow */
  authorizationCode?: string;
  /**
   * The form-urldecoded `iss` query parameter from the authorization callback.
   * Validated per RFC 9207 §2.4 when `authorizationCode` is present;
   * see {@linkcode validateAuthorizationResponseIssuer}.
   */
  iss?: string;
  /** Optional scope parameter from auth() options */
  scope?: string;
  fetchFn?: FetchLike;
}): Promise<OAuthTokens>;
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
declare function registerClient(authorizationServerUrl: string | URL, {
  metadata,
  clientMetadata,
  scope,
  fetchFn
}: {
  metadata?: AuthorizationServerMetadata;
  clientMetadata: OAuthClientMetadata;
  scope?: string;
  fetchFn?: FetchLike;
}): Promise<OAuthClientInformationFull>;
//#endregion
//#region src/client/authExtensions.d.ts
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
declare function createPrivateKeyJwtAuth(options: {
  issuer: string;
  subject: string;
  privateKey: string | Uint8Array | Record<string, unknown>;
  alg: string;
  audience?: string | URL;
  lifetimeSeconds?: number;
  claims?: Record<string, unknown>;
}): AddClientAuthentication;
/**
 * Options for creating a {@linkcode ClientCredentialsProvider}.
 */
interface ClientCredentialsProviderOptions {
  /**
   * The `client_id` for this OAuth client.
   */
  clientId: string;
  /**
   * The `client_secret` for `client_secret_basic` authentication.
   */
  clientSecret: string;
  /**
   * Optional client name for metadata.
   */
  clientName?: string;
  /**
   * Space-separated scopes values requested by the client.
   */
  scope?: string;
  /**
   * The authorization server's `issuer` identifier these credentials were registered with.
   * Stamped onto the stored client information so `auth()`'s SEP-2352 issuer check
   * refuses to send the credential to any other authorization server. Hosts supplying
   * static client credentials SHOULD set this; omitting it preserves the legacy
   * (no-binding) behaviour for back-compat.
   */
  expectedIssuer?: string;
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
declare class ClientCredentialsProvider implements OAuthClientProvider {
  private _tokens?;
  private _clientInfo;
  private _clientMetadata;
  constructor(options: ClientCredentialsProviderOptions);
  get redirectUrl(): undefined;
  get clientMetadata(): OAuthClientMetadata;
  clientInformation(): StoredOAuthClientInformation;
  tokens(): StoredOAuthTokens | undefined;
  saveTokens(tokens: StoredOAuthTokens): void;
  redirectToAuthorization(): void;
  saveCodeVerifier(): void;
  codeVerifier(): string;
  prepareTokenRequest(scope?: string): URLSearchParams;
}
/**
 * Options for creating a {@linkcode PrivateKeyJwtProvider}.
 */
interface PrivateKeyJwtProviderOptions {
  /**
   * The `client_id` for this OAuth client.
   */
  clientId: string;
  /**
   * The private key for signing JWT assertions.
   * Can be a PEM string, Uint8Array, or JWK object.
   */
  privateKey: string | Uint8Array | Record<string, unknown>;
  /**
   * The algorithm to use for signing (e.g., 'RS256', 'ES256').
   */
  algorithm: string;
  /**
   * Optional client name for metadata.
   */
  clientName?: string;
  /**
   * Optional JWT lifetime in seconds (default: 300).
   */
  jwtLifetimeSeconds?: number;
  /**
   * Space-separated scopes values requested by the client.
   */
  scope?: string;
  /**
   * Optional custom claims to include in the JWT assertion.
   * These are merged with the standard claims (`iss`, `sub`, `aud`, `exp`, `iat`, `jti`),
   * with custom claims taking precedence for any overlapping keys.
   *
   * Useful for including additional claims that help scope the access token
   * with finer granularity than what scopes alone allow.
   */
  claims?: Record<string, unknown>;
  /**
   * The authorization server's `issuer` identifier these credentials were registered with.
   * Seeds the SEP-2352 issuer stamp — see {@linkcode ClientCredentialsProviderOptions.expectedIssuer}.
   */
  expectedIssuer?: string;
}
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
declare class PrivateKeyJwtProvider implements OAuthClientProvider {
  private _tokens?;
  private _clientInfo;
  private _clientMetadata;
  addClientAuthentication: AddClientAuthentication;
  constructor(options: PrivateKeyJwtProviderOptions);
  get redirectUrl(): undefined;
  get clientMetadata(): OAuthClientMetadata;
  clientInformation(): StoredOAuthClientInformation;
  tokens(): StoredOAuthTokens | undefined;
  saveTokens(tokens: StoredOAuthTokens): void;
  redirectToAuthorization(): void;
  saveCodeVerifier(): void;
  codeVerifier(): string;
  prepareTokenRequest(scope?: string): URLSearchParams;
}
/**
 * Options for creating a {@linkcode StaticPrivateKeyJwtProvider}.
 */
interface StaticPrivateKeyJwtProviderOptions {
  /**
   * The `client_id` for this OAuth client.
   */
  clientId: string;
  /**
   * A pre-built JWT client assertion to use for authentication.
   *
   * This token should already contain the appropriate claims
   * (`iss`, `sub`, `aud`, `exp`, etc.) and be signed by the client's key.
   */
  jwtBearerAssertion: string;
  /**
   * Optional client name for metadata.
   */
  clientName?: string;
  /**
   * Space-separated scopes values requested by the client.
   */
  scope?: string;
  /**
   * The authorization server's `issuer` identifier this assertion was minted for.
   * Seeds the SEP-2352 issuer stamp — see {@linkcode ClientCredentialsProviderOptions.expectedIssuer}.
   */
  expectedIssuer?: string;
}
/**
 * OAuth provider for `client_credentials` grant with a static `private_key_jwt` assertion.
 *
 * This provider mirrors {@linkcode PrivateKeyJwtProvider} but instead of constructing and
 * signing a JWT on each request, it accepts a pre-built JWT assertion string and
 * uses it directly for authentication.
 */
declare class StaticPrivateKeyJwtProvider implements OAuthClientProvider {
  private _tokens?;
  private _clientInfo;
  private _clientMetadata;
  addClientAuthentication: AddClientAuthentication;
  constructor(options: StaticPrivateKeyJwtProviderOptions);
  get redirectUrl(): undefined;
  get clientMetadata(): OAuthClientMetadata;
  clientInformation(): StoredOAuthClientInformation;
  tokens(): StoredOAuthTokens | undefined;
  saveTokens(tokens: StoredOAuthTokens): void;
  redirectToAuthorization(): void;
  saveCodeVerifier(): void;
  codeVerifier(): string;
  prepareTokenRequest(scope?: string): URLSearchParams;
}
/**
 * Context provided to the assertion callback in {@linkcode CrossAppAccessProvider}.
 * Contains orchestrator-discovered information needed for JWT Authorization Grant requests.
 */
interface CrossAppAccessContext {
  /**
   * The authorization server URL of the target MCP server.
   * Discovered via RFC 9728 protected resource metadata.
   */
  authorizationServerUrl: string;
  /**
   * The resource URL of the target MCP server.
   * Discovered via RFC 9728 protected resource metadata.
   */
  resourceUrl: string;
  /**
   * Optional scope being requested for the MCP server.
   */
  scope?: string;
  /**
   * Fetch function to use for HTTP requests (e.g., for IdP token exchange).
   */
  fetchFn: FetchLike;
}
/**
 * Callback function type that provides a JWT Authorization Grant (ID-JAG).
 *
 * The callback receives context about the target MCP server (authorization server URL,
 * resource URL, scope) and should return a JWT Authorization Grant that will be used
 * to obtain an access token from the MCP server.
 */
type AssertionCallback = (context: CrossAppAccessContext) => string | Promise<string>;
/**
 * Options for creating a {@linkcode CrossAppAccessProvider}.
 */
interface CrossAppAccessProviderOptions {
  /**
   * Callback function that provides a JWT Authorization Grant (ID-JAG).
   *
   * The callback receives the MCP server's authorization server URL, resource URL,
   * and requested scope, and should return a JWT Authorization Grant obtained from
   * the enterprise IdP via RFC 8693 token exchange.
   *
   * You can use the utility functions from the `crossAppAccess` module
   * for standard flows, or implement custom logic.
   *
   * @example
   * ```ts
   * assertion: async (ctx) => {
   *     const result = await discoverAndRequestJwtAuthGrant({
   *         idpUrl: 'https://idp.example.com',
   *         audience: ctx.authorizationServerUrl,
   *         resource: ctx.resourceUrl,
   *         idToken: await getIdToken(),
   *         clientId: 'my-idp-client',
   *         clientSecret: 'my-idp-secret',
   *         scope: ctx.scope,
   *         fetchFn: ctx.fetchFn
   *     });
   *     return result.jwtAuthGrant;
   * }
   * ```
   */
  assertion: AssertionCallback;
  /**
   * The `client_id` registered with the MCP server's authorization server.
   */
  clientId: string;
  /**
   * The `client_secret` for authenticating with the MCP server's authorization server.
   */
  clientSecret: string;
  /**
   * Optional client name for metadata.
   */
  clientName?: string;
  /**
   * Custom fetch implementation. Defaults to global fetch.
   */
  fetchFn?: FetchLike;
  /**
   * The MCP authorization server's `issuer` identifier these credentials were registered with.
   * Seeds the SEP-2352 issuer stamp — see {@linkcode ClientCredentialsProviderOptions.expectedIssuer}.
   */
  expectedIssuer?: string;
}
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
declare class CrossAppAccessProvider implements OAuthClientProvider {
  private _tokens?;
  private _clientInfo;
  private _clientMetadata;
  private _assertionCallback;
  private _fetchFn;
  private _authorizationServerUrl?;
  private _resourceUrl?;
  private _scope?;
  constructor(options: CrossAppAccessProviderOptions);
  get redirectUrl(): undefined;
  get clientMetadata(): OAuthClientMetadata;
  clientInformation(): StoredOAuthClientInformation;
  tokens(): StoredOAuthTokens | undefined;
  saveTokens(tokens: StoredOAuthTokens): void;
  redirectToAuthorization(): void;
  saveCodeVerifier(): void;
  codeVerifier(): string;
  /**
   * Saves the authorization server URL discovered during OAuth flow.
   * This is called by the auth() function after RFC 9728 discovery.
   */
  saveAuthorizationServerUrl(authorizationServerUrl: string): void;
  /**
   * Returns the cached authorization server URL if available.
   */
  authorizationServerUrl(): string | undefined;
  /**
   * Saves the resource URL discovered during OAuth flow.
   * This is called by the auth() function after RFC 9728 discovery.
   */
  saveResourceUrl?(resourceUrl: string): void;
  /**
   * Returns the cached resource URL if available.
   */
  resourceUrl?(): string | undefined;
  prepareTokenRequest(scope?: string): Promise<URLSearchParams>;
}
//#endregion
//#region src/client/probeClassifier.d.ts
/**
 * A cached era verdict for `connect({ prior })` — the persistable subset of
 * the {@linkcode ProbeVerdict} vocabulary (`'modern'` = speaks 2026-07-28+,
 * `'legacy'` = 2025-era `initialize` only). Freshness is the supplying host's
 * responsibility: a stale modern verdict fails loudly at the first request,
 * but a stale legacy verdict succeeds silently forever (an upgraded server
 * still answers `initialize`) — date cached legacy verdicts in your own
 * storage and stop supplying them past your policy horizon. Full recipe:
 * the gateway guide (`docs/advanced/gateway.md`).
 */
type PriorDiscovery = /** Adopt `discover` directly: zero round trips. */
{
  kind: 'modern';
  discover: DiscoverResult;
}
/** Known-legacy server: skip the `server/discover` probe and run the plain legacy `initialize` handshake. */ | {
  kind: 'legacy';
};
//#endregion
//#region src/client/responseCache.d.ts
/**
 * Client-side response cache for SEP-2549 (`CacheableResult`) freshness hints.
 *
 * The store is a dumb keyed-value carrier: every freshness, scope and
 * invalidation decision lives in the {@linkcode ClientResponseCache} (the
 * `Client`'s single cache-coordination collaborator). The `stamp` field is
 * mcp.d's re-derivation key — a derived view (e.g. the `name → Tool` index)
 * re-computes only when the backing entry's stamp changes.
 *
 * Reference design: mcp.d `client/cache.d` / `client/client.d` (`CacheStore`,
 * `cachedTool`, `cachedFetch`, `invalidateLogical`).
 */
/** A value or a promise of one. The store interface is async-ready; the in-memory default returns plain values. */
type MaybePromise<T> = T | Promise<T>;
/** The freshness scope of a cached entry (SEP-2549 `cacheHints.scope`). */
type CacheScope = 'public' | 'private';
/**
 * Per-call cache disposition for the cacheable verbs (`listTools()` /
 * `listPrompts()` / `listResources()` / `listResourceTemplates()` /
 * `readResource()`):
 *
 * - `'use'` (the default) — serve a still-fresh cached entry without a round
 *   trip; on miss/stale, fetch and write.
 * - `'refresh'` — always fetch (ignore any held entry) and write the fresh
 *   result.
 * - `'bypass'` — fetch without consulting OR writing the cache (the result is
 *   not stored). The `tools/list`-derived index (mirroring / output
 *   validation) is therefore unaffected by a `'bypass'` call.
 */
type CacheMode = 'use' | 'refresh' | 'bypass';
/**
 * A logical cache address. `params` is the canonical result-affecting params
 * key (`''` for the four list ops, the `uri` for `resources/read`); omitted is
 * equivalent to `''`. `partition` namespaces the entry by connected-server
 * identity AND per-principal scope: the `Client` writes a JSON-encoded
 * `[serverIdentity, principal]` pair (so a server-controlled `serverInfo`
 * string cannot bleed into the principal slot regardless of what characters
 * it contains). A `'public'`-scoped entry lives at `[serverIdentity, '']`; a
 * `'private'`-scoped entry at `[serverIdentity, cachePartition]`. Omitted is
 * equivalent to `''`.
 */
interface CacheKey {
  readonly method: string;
  readonly params?: string;
  readonly partition?: string;
}
/**
 * One cached response body. `value` is the JSON-serialized result document —
 * a store is a dumb string carrier and persists it verbatim; the cache owns
 * both codec halves. `stamp` is the store-generated monotonically increasing write counter —
 * opaque to callers. Derived views (e.g. a `name → Tool` index) memoize
 * against it and re-derive only when it changes. `expiresAt` (absolute ms
 * epoch, `now + ttlMs`) and `scope` are the client-computed freshness
 * metadata; the store MUST persist them and hand them back on `get` so the
 * read path can decide freshness and gate the shared-partition fallback on
 * `scope === 'public'`.
 */
interface CacheEntry {
  readonly value: string;
  readonly stamp: number;
  readonly expiresAt?: number;
  readonly scope?: CacheScope;
}
/**
 * The pluggable response-cache store. The interface is intentionally narrow;
 * the in-memory default is the only implementation the SDK ships.
 *
 * Every method is async-ready ({@linkcode MaybePromise}) so a Redis-style
 * store can implement the same interface without a later breaking change; the
 * in-memory default stays synchronous (plain values are valid under
 * `MaybePromise`). The `Client` `await`s every call site.
 *
 * Entries are keyed by `{method, params, partition}` where `partition` is the
 * `Client`-derived `[serverIdentity, principal]` JSON pair, so one store
 * instance is safe to share across `Client` instances connected to different
 * servers and/or principals: writes from distinct connections never collide,
 * the shared-partition read fallback is gated on the stored
 * `scope === 'public'`, and `list_changed` / `HEADER_MISMATCH` evictions are
 * scoped to the connected server's two partitions — co-tenants on a shared
 * store are unaffected. The `Client` constructor still allocates a fresh
 * {@linkcode InMemoryResponseCacheStore} per instance by default; supply your
 * own to share or persist.
 */
interface ResponseCacheStore {
  get(key: CacheKey): MaybePromise<CacheEntry | undefined>;
  /**
   * Writes `entry` under `key` and returns the store-generated stamp the
   * resulting {@linkcode CacheEntry} carries. The store owns the stamp
   * counter; callers do not supply one. The caller owns `expiresAt` and
   * `scope` (the client-computed freshness metadata); the store MUST persist
   * them and hand them back on `get`.
   */
  set(key: CacheKey, entry: {
    value: string;
    expiresAt?: number;
    scope?: CacheScope;
  }): MaybePromise<number>;
  /**
   * Drop the single entry under `key` (no-op if absent). Called for both
   * `notifications/resources/updated` (per-URI) and the `list_changed`
   * notifications (the list singletons live at `{method, params: '', partition}`).
   */
  delete(key: CacheKey): MaybePromise<void>;
  /**
   * Drop every entry for `method` across every partition. The `Client` does
   * NOT call this (its `list_changed` path issues two partition-scoped
   * `delete()` calls so co-tenants on a shared store keep their entries);
   * kept on the interface for callers that want a method-wide bulk-clear.
   */
  evict(method: string): MaybePromise<void>;
  /** Drop every entry (connection reset). */
  clear(): MaybePromise<void>;
}
/** Options for {@linkcode InMemoryResponseCacheStore}. */
interface InMemoryResponseCacheStoreOptions {
  /**
   * Maximum number of held `resources/read` entries (the only
   * unbounded-keyspace method). When inserting a new `resources/read` key
   * would exceed this, the oldest such entry (by insertion order) is
   * evicted first. The list-singleton methods (`tools/list`,
   * `prompts/list`, `resources/list`, `resources/templates/list`,
   * `server/discover`) are **exempt** — they hold at most one entry per
   * partition and back the `tools/list`-derived index, so an unbounded URI
   * working set never displaces them. The default of `512` bounds growth on
   * a long-lived client against template-expanded URIs. `0` disables the
   * bound.
   */
  maxEntries?: number;
}
/**
 * In-memory default. Bounded by an insertion-ordered size cap (default `512`;
 * see {@linkcode InMemoryResponseCacheStoreOptions.maxEntries}) on the
 * `resources/read` keyspace so an unbounded stream of distinct URIs cannot
 * grow it without limit; the list-singleton methods are exempt and never
 * evicted by the cap. `Map` preserves insertion order, so the oldest live
 * capped key is the first matching iteration entry.
 */
declare class InMemoryResponseCacheStore implements ResponseCacheStore {
  private readonly _entries;
  private readonly _maxEntries;
  private _stamp;
  /** Count of held entries that are subject to the cap (i.e. not in {@linkcode CAP_EXEMPT_METHODS}). */
  private _cappedSize;
  constructor(options?: InMemoryResponseCacheStoreOptions);
  /** Number of held entries (for diagnostics / bounding tests). */
  get size(): number;
  get(key: CacheKey): CacheEntry | undefined;
  set(key: CacheKey, entry: {
    value: string;
    expiresAt?: number;
    scope?: CacheScope;
  }): number;
  delete(key: CacheKey): void;
  evict(method: string): void;
  clear(): void;
}
/**
 * Upper bound on the server-supplied `ttlMs` honoured by
 * {@linkcode ClientResponseCache} (24h). A server cannot pin an entry
 * indefinitely.
 */
declare const MAX_CACHE_TTL_MS = 86400000;
//#endregion
//#region src/client/versionNegotiation.d.ts
/**
 * Probe policy for `'auto'` and pinned negotiation modes.
 *
 * There is no special probe timeout opinion: the probe inherits the client's
 * STANDARD request timeout unless `timeoutMs` overrides it.
 */
interface VersionNegotiationProbeOptions {
  /**
   * Timeout for the probe exchange, in milliseconds.
   *
   * The timeout verdict is transport-aware: on stdio, a probe that gets no
   * response within the timeout indicates a legacy server and falls back to
   * the `initialize` handshake (measured on the disposable sibling for the
   * SDK's own stdio transport, with the fallback running on the session
   * child's fresh pipe; in place for custom stdio-shaped transports); on
   * HTTP, where a deployed server answers and silence means an outage,
   * `connect()` rejects with the standard typed timeout error instead.
   *
   * @default the standard request timeout (`DEFAULT_REQUEST_TIMEOUT_MSEC`, or the `timeout` passed to `connect()`)
   */
  timeoutMs?: number;
  /**
   * Number of times to re-send the probe after a timeout before reaching the
   * timeout verdict. Governs timeout re-sends only — the spec-mandated
   * `-32022` corrective continuation (select-and-continue with a mutual
   * version) is a separate negotiation step and is never counted against
   * `maxRetries`.
   *
   * @default 0 (no retries)
   */
  maxRetries?: number;
}
/**
 * Negotiation mode:
 *
 * - `'legacy'` — no negotiation: the plain 2025 connect sequence, byte-identical
 *   to a client without this option.
 * - `'auto'` — probe with `server/discover` at connect; conservative fallback to
 *   the plain legacy `initialize` handshake unless the outcome is definitive
 *   modern evidence. Network outage rejects with a typed connect error; a
 *   probe timeout falls back to `initialize` on stdio (a silent server on a
 *   local pipe is a legacy server) and rejects with a typed timeout error on
 *   HTTP (silence there is an outage). On the SDK's stdio transport (the base
 *   `StdioClientTransport` exactly; subclasses probe in place) the probe
 *   runs on a short-lived sibling process spawned from the same parameters
 *   (its stderr is discarded) and the caller's transport starts once, after
 *   the era is known — so a child that exits on the unrecognized probe, the
 *   shape of servers built on SDKs that terminate on any pre-`initialize`
 *   request, is simply a legacy server. A mid-probe connection close on HTTP
 *   (or on a custom stdio-shaped transport, which probes in place) rejects
 *   with the typed connect error.
 * - `{ pin: '<version>' }` — modern era at exactly the pinned revision: the
 *   connect-time `server/discover` must offer it. No fallback — anything else
 *   fails loudly with a typed error.
 */
type VersionNegotiationMode = 'legacy' | 'auto' | {
  pin: string;
};
/**
 * Opt-in protocol version negotiation, configured on
 * `ClientOptions.versionNegotiation`.
 */
interface VersionNegotiationOptions {
  /**
   * @default 'legacy'
   */
  mode?: VersionNegotiationMode;
  /**
   * Probe timeout/retry policy (only consulted by the probing modes).
   */
  probe?: VersionNegotiationProbeOptions;
}
//#endregion
//#region src/client/client.d.ts
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
declare function getSupportedElicitationModes(capabilities: ClientCapabilities['elicitation']): {
  supportsFormMode: boolean;
  supportsUrlMode: boolean;
};
type ClientOptions = ProtocolOptions & {
  /**
   * Capabilities to advertise as being supported by this client.
   */
  capabilities?: ClientCapabilities;
  /**
   * JSON Schema validator for tool output validation.
   *
   * The validator is used to validate structured content returned by tools
   * against their declared output schemas.
   *
   * @default Runtime-selected validator (AJV-backed on Node.js, `@cfworker/json-schema`-backed on browser/workerd runtimes)
   */
  jsonSchemaValidator?: jsonSchemaValidator;
  /**
   * Opt-in protocol version negotiation (protocol revision 2026-07-28 and later).
   *
   * **The default is `'legacy'`**: absent (or `mode: 'legacy'`), `connect()`
   * runs the plain 2025 sequence, byte-identical to today's behavior (no
   * probe, no new headers). Opt into `'auto'` or pin to talk to a 2026-07-28
   * server.
   *
   * - `mode: 'auto'` — `connect()` probes the server with `server/discover` first:
   *   definitive modern evidence selects the modern era; definitive legacy signals
   *   (and anything unrecognized) fall back to the plain legacy `initialize`
   *   handshake, byte-equivalent to a 2025 client. On the SDK's own stdio
   *   transport (the base `StdioClientTransport` exactly; subclasses probe in
   *   place) the probe runs on a short-lived sibling process spawned from the
   *   same parameters (one extra spawn per connect; its stderr is discarded) and
   *   the caller's transport starts once, after the era is known; HTTP — and
   *   custom or subclassed stdio-shaped transports — probe on the connection itself. A
   *   network outage rejects with a typed connect error. A probe timeout is
   *   transport-aware: on stdio it indicates a legacy server (some legacy servers
   *   never answer unknown pre-`initialize` requests) and falls back to
   *   `initialize`; on HTTP it rejects with a typed timeout
   *   error (silence on a deployed server is an outage, not a legacy signal).
   * - `mode: { pin: '2026-07-28' }` — modern era at exactly the pinned revision;
   *   no probe-and-fallback: anything else fails loudly.
   *
   * Probe policy lives under `probe: { timeoutMs?, maxRetries? }`; the probe
   * inherits the client's standard request timeout unless overridden, and
   * `maxRetries` (default `0`) governs timeout re-sends only — the
   * spec-mandated `-32022` corrective continuation is never counted against it.
   *
   * Once a modern era is negotiated, the client automatically attaches the
   * per-request `_meta` envelope (the reserved protocol-version / client-info /
   * client-capabilities keys) to every outgoing request and notification;
   * user-supplied `_meta` keys take precedence over the auto-attached ones.
   */
  versionNegotiation?: VersionNegotiationOptions;
  /**
   * Multi-round-trip auto-fulfilment (protocol revision 2026-07-28).
   *
   * On the 2026-07-28 era, servers obtain client input (elicitation,
   * sampling, roots) by answering `tools/call`, `prompts/get`, or
   * `resources/read` with an `input_required` result instead of sending a
   * server→client request. By default the client fulfils those embedded
   * requests automatically through the SAME handlers registered via
   * {@linkcode Client.setRequestHandler | setRequestHandler} (e.g.
   * `elicitation/create`), then retries the original call with the
   * collected `inputResponses` and a byte-exact echo of the opaque
   * `requestState`, on a fresh request id, up to `maxRounds` rounds.
   * `client.callTool()` (and its siblings) keep returning their plain
   * result type — the interactive rounds happen inside the call.
   *
   * Set `autoFulfill: false` for manual mode: an `input_required` response
   * then surfaces as a typed error unless the individual call passes
   * `allowInputRequired: true` (pair it with `withInputRequired()` on the
   * explicit-schema path to type both outcomes).
   *
   * Has no effect on 2025-era connections, which have no `input_required`
   * vocabulary.
   */
  inputRequired?: InputRequiredOptions;
  /**
   * Configure handlers for list changed notifications (tools, prompts, resources).
   *
   * @example
   * ```ts source="./client.examples.ts#ClientOptions_listChanged"
   * const client = new Client(
   *     { name: 'my-client', version: '1.0.0' },
   *     {
   *         listChanged: {
   *             tools: {
   *                 onChanged: (error, tools) => {
   *                     if (error) {
   *                         console.error('Failed to refresh tools:', error);
   *                         return;
   *                     }
   *                     console.log('Tools updated:', tools);
   *                 }
   *             },
   *             prompts: {
   *                 onChanged: (error, prompts) => console.log('Prompts updated:', prompts)
   *             }
   *         }
   *     }
   * );
   * ```
   */
  listChanged?: ListChangedHandlers;
  /**
   * Cap on the number of pages the auto-aggregating
   * {@linkcode Client.listTools | listTools()} /
   * {@linkcode Client.listPrompts | listPrompts()} /
   * {@linkcode Client.listResources | listResources()} /
   * {@linkcode Client.listResourceTemplates | listResourceTemplates()} walk
   * fetches before throwing (a defence against a server whose `nextCursor`
   * never converges). `0` disables the cap. Default: `64`. Applies only to
   * the no-argument auto-aggregate path; an explicit-`cursor` per-page call
   * is never capped.
   */
  listMaxPages?: number;
  /**
   * The response-cache store backing the client's derived views (the cached
   * `tools/list` result that {@linkcode Client.callTool | callTool}'s output
   * validation and SEP-2243 header mirroring read) and the SEP-2549
   * cache-hint serving on the cacheable verbs. Defaults to a fresh
   * {@linkcode InMemoryResponseCacheStore} per client.
   *
   * Entries are automatically scoped by connected-server identity (derived
   * from `serverInfo` after connect) AND, for `'private'`-scoped results,
   * by `cachePartition` — encoded collision-free via `JSON.stringify`, so a
   * server cannot craft a `serverInfo` that bleeds into another server's
   * namespace or another principal's slot. One store may therefore back
   * several clients (e.g. a host pool against the same server, or one
   * persistent KV across servers); `list_changed` evictions are scoped to
   * the connected server's partitions, so co-tenants are unaffected. Set
   * `cachePartition` to your principal identifier (e.g. the auth subject)
   * when sharing across principals. Note `serverInfo` is self-reported — a
   * server that deliberately impersonates
   * another's `name`/`version` shares its `'public'` slot; the
   * per-principal isolation via `cachePartition` holds regardless.
   */
  responseCacheStore?: ResponseCacheStore;
  /**
   * Opaque per-principal identifier for response-cache writes whose
   * server-reported `cacheScope` is `'private'` (the spec's "MUST NOT share
   * across authorization contexts"). Within the connected server's
   * namespace, `'public'`-scoped entries live at the shared
   * `[serverIdentity, '']` partition and `'private'`-scoped entries at
   * `[serverIdentity, cachePartition]`. Set this to a stable identity of
   * the authorization context (e.g. the auth subject) when one
   * `responseCacheStore` backs several principals; with the
   * default `''` every entry — public or private — lives at the server's
   * shared partition, which is the safe single-tenant posture.
   */
  cachePartition?: string;
  /**
   * TTL (ms) applied when a cacheable result arrives without a `ttlMs`
   * field. Default `0` — a result without an explicit hint is never served
   * from cache (every call refetches), but it is still **stored** so the
   * `tools/list`-derived index that {@linkcode Client.callTool | callTool}'s
   * SEP-2243 mirroring and output-schema validation read keeps working
   * regardless. The spec defines absent-or-≤0 as "immediately stale".
   */
  defaultCacheTtlMs?: number;
};
/**
 * Options for {@linkcode Client.connect}. Extends {@linkcode RequestOptions}
 * (the timeout/signal apply to the connect-time handshake or probe) with the
 * cached-era-verdict knob.
 */
type ConnectOptions = RequestOptions & {
  /**
   * A cached era verdict, taking precedence over `versionNegotiation`:
   * `{ kind: 'modern', discover }` adopts a prior {@linkcode DiscoverResult}
   * with zero round trips (throws `SdkError(EraNegotiationFailed)` on no
   * 2026-07-28+ overlap); `{ kind: 'legacy' }` skips the probe and runs the
   * plain legacy `initialize` handshake. Freshness is the supplying host's
   * responsibility — see {@linkcode PriorDiscovery}. Reuse only within one
   * authorization context.
   */
  prior?: PriorDiscovery;
};
/**
 * {@linkcode RequestOptions} extended with the per-call cache disposition for
 * the cacheable verbs (`listTools()` / `listPrompts()` / `listResources()` /
 * `listResourceTemplates()` / `readResource()`). See {@linkcode CacheMode}.
 */
type CacheableRequestOptions = RequestOptions & {
  /**
   * `'use'` (default) serves a still-fresh cached entry without a round
   * trip; `'refresh'` always fetches and re-stores; `'bypass'` fetches
   * without consulting or writing the cache. Applies to the no-`cursor`
   * auto-aggregate path on the list verbs and to `readResource`; ignored
   * elsewhere.
   */
  cacheMode?: CacheMode;
};
/**
 * Options for {@linkcode Client.callTool}. Extends {@linkcode RequestOptions}
 * with an escape hatch for callers that already hold the tool definition
 * (e.g. from a previous session or configuration) — pass it via
 * `toolDefinition` so SEP-2243 `Mcp-Param-*` header mirroring can run without a
 * prior `tools/list`.
 */
type CallToolRequestOptions = RequestOptions & {
  /**
   * The tool definition to use for SEP-2243 `Mcp-Param-*` header mirroring on
   * a 2026-07-28 connection over Streamable HTTP, AND for output-schema
   * validation of the result. When set, the client uses this definition's
   * `inputSchema` and `outputSchema` instead of (and without consulting) the
   * cached `tools/list` result, so the two derived views agree.
   */
  toolDefinition?: Tool;
};
/**
 * A handle to an open `subscriptions/listen` stream (protocol revision
 * 2026-07-28). Change notifications delivered on the stream dispatch to the
 * existing {@linkcode Client.setNotificationHandler} registrations.
 */
interface McpSubscription {
  /**
   * The subset of the requested filter the server agreed to honor (from
   * `notifications/subscriptions/acknowledged`).
   */
  readonly honoredFilter: SubscriptionFilter;
  /**
   * Tears the subscription down. Idempotent. Aborts the listen request's
   * stream (where the transport supports it) AND sends
   * `notifications/cancelled` referencing the listen request id — both,
   * always, so close works on any transport.
   */
  close(): Promise<void>;
  /**
   * Resolves exactly once when the subscription has terminated. Never
   * rejects — this is an observation, not an operation.
   *
   * - `'local'` — you called {@linkcode close} (or aborted the
   *   `RequestOptions.signal` you passed to `listen()`).
   * - `'graceful'` — the server ended the subscription deliberately by
   *   sending the empty `subscriptions/listen` response (e.g. on shutdown).
   * - `'remote'` — the stream ended without a response, or the transport
   *   dropped — an unexpected disconnect. Re-listen if you still want
   *   events.
   */
  readonly closed: Promise<'local' | 'graceful' | 'remote'>;
}
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
declare class Client extends Protocol<ClientContext> {
  private _clientInfo;
  private _serverCapabilities?;
  private _serverVersion?;
  private _capabilities;
  private _instructions?;
  private _jsonSchemaValidator;
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
  private readonly _cache;
  private readonly _defaultCacheTtlMs;
  private readonly _listMaxPages;
  private _listChangedDebounceTimers;
  /**
   * The constructor `listChanged` configuration. Durable across reconnects:
   * read fresh on every connect (legacy or modern), never consumed.
   */
  private readonly _listChangedConfig?;
  private _enforceStrictCapabilities;
  private _versionNegotiation?;
  private _supportedProtocolVersionsOption?;
  private _inputRequiredDriverConfig;
  /**
   * Active subscriptions/listen state, keyed by subscription id (= the
   * listen request's JSON-RPC id verbatim). The id is a STRING from a
   * Client-owned counter (`'listen:' + N`) — JSON-RPC permits string ids,
   * and Protocol's numeric `_requestMessageId` counter only ever issues
   * numbers, so listen ids cannot collide with ordinary request ids.
   */
  private _listenState;
  private _nextListenId;
  /** The auto-opened subscription backing ClientOptions.listChanged on a modern connection. */
  private _autoOpenedSubscription?;
  /** Backing store for {@linkcode getDiscoverResult}. Per-connection. */
  private _discoverResult?;
  /**
   * Clears every per-connection field in one place. Called at the start of
   * each fresh (non-resuming) connect and from `close()`, so a stale
   * negotiated era / server identity / auto-opened subscription cannot
   * survive a reconnect.
   */
  private _resetConnectionState;
  close(): Promise<void>;
  /**
   * Initializes this client with the given name and version information.
   */
  constructor(_clientInfo: Implementation, options?: ClientOptions);
  protected buildContext(ctx: BaseContext, _transportInfo?: MessageExtraInfo): ClientContext;
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
  protected _shouldDropInbound(message: JSONRPCRequest | JSONRPCNotification): 'drop' | undefined;
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
  protected _outboundMetaEnvelope(): Readonly<Record<string, unknown>> | undefined;
  /**
   * Wires the multi-round-trip auto-fulfilment engine (protocol revision
   * 2026-07-28) into the response funnel: an `input_required` answer is
   * fulfilled through the registered elicitation/sampling/roots handlers
   * and the original request retried via `flow.retry`, up to
   * `inputRequired.maxRounds` rounds. With auto-fulfilment disabled the
   * response surfaces as a typed error steering to manual mode.
   */
  protected _resolveNonCompleteResult<T extends StandardSchemaV1>(decoded: {
    kind: 'input_required';
    inputRequests: Record<string, unknown>;
    requestState?: string;
  }, flow: NonCompleteResultFlow<T>): Promise<unknown>;
  /**
   * Set up handlers for list changed notifications based on config and server capabilities.
   * This should only be called after initialization when server capabilities are known.
   * Handlers are silently skipped if the server doesn't advertise the corresponding listChanged capability.
   * @internal
   */
  private _setupListChangedHandlers;
  /**
   * Registers new capabilities. This can only be called before connecting to a transport.
   *
   * The new capabilities will be merged with any existing capabilities previously given (e.g., at initialization).
   */
  registerCapabilities(capabilities: ClientCapabilities): void;
  /**
   * Configure protocol version negotiation before connecting (equivalent to
   * passing `versionNegotiation` at construction time). Can only be called
   * before connecting to a transport. Passing `undefined` clears a previously
   * configured negotiation, restoring the default `'legacy'` posture.
   *
   * See {@linkcode ClientOptions | ClientOptions.versionNegotiation} for the mode semantics.
   */
  setVersionNegotiation(options: VersionNegotiationOptions | undefined): void;
  /**
   * Enforces client-side validation for `elicitation/create` and `sampling/createMessage`
   * regardless of how the handler was registered.
   */
  protected _wrapHandler(method: string, handler: (request: JSONRPCRequest, ctx: ClientContext) => Promise<Result>): (request: JSONRPCRequest, ctx: ClientContext) => Promise<Result>;
  protected assertCapability(capability: keyof ServerCapabilities, method: string): void;
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
  connect(transport: Transport, options?: ConnectOptions): Promise<void>;
  /**
   * Plain legacy connect — the pinned 2025 sequence, byte-untouched. The
   * `mode: 'legacy'` connect body, shared with the `prior` legacy verdict.
   */
  private _connectPlainLegacy;
  /**
   * The 2025 `initialize` handshake — the body of the plain legacy connect and
   * the `'auto'`-mode fallback path (same `initialize` body, zero 2026 headers;
   * on the stdio sibling path it opens the session child's fresh pipe, in the
   * in-place modes it rides the probed connection). Callers clear the negotiated protocol version before
   * the handshake; its completion sets the negotiated (legacy) version.
   */
  private _legacyHandshake;
  /**
   * Negotiated connect (mode `'auto'` or `{ pin }`): probe with `server/discover`
   * before the Protocol machinery attaches — on a disposable sibling process for
   * the SDK's stdio transport, in place otherwise — then either establish the
   * modern era or perform the plain legacy handshake.
   */
  private _connectNegotiated;
  /**
   * Connect from a validated {@linkcode PriorDiscovery}: the modern arm
   * adopts the `DiscoverResult` (zero round trips; `EraNegotiationFailed`
   * on no 2026-07-28+ overlap), the legacy arm runs the plain legacy connect.
   */
  private _connectFromPrior;
  /**
   * After initialization has completed, this will be populated with the server's reported capabilities.
   */
  getServerCapabilities(): ServerCapabilities | undefined;
  /**
   * The connected server's self-reported name and version, when it
   * identified itself: required on the legacy `initialize` result; a spec
   * SHOULD in the discover result's `_meta` on 2026-07-28, so a successful
   * modern connect against an anonymous server leaves this `undefined`.
   */
  getServerVersion(): Implementation | undefined;
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
  private _deriveServerIdentity;
  /**
   * After initialization has completed, this will be populated with the protocol version negotiated
   * during the initialize handshake. When manually reconstructing a transport for reconnection, pass this
   * value to the new transport so it continues sending the required `mcp-protocol-version` header.
   */
  getNegotiatedProtocolVersion(): string | undefined;
  /**
   * After initialization has completed, this returns the protocol era of the
   * connection: `'modern'` when the connection negotiated a 2026-07-28+
   * revision (via `server/discover`), `'legacy'` for the 2025-era
   * `initialize` handshake, or `undefined` before the connection is
   * established.
   */
  getProtocolEra(): ProtocolEra | undefined;
  /**
   * After initialization has completed, this may be populated with information about the server's instructions.
   */
  getInstructions(): string | undefined;
  /**
   * The {@linkcode DiscoverResult} from the last `'auto'`/pinned probe,
   * {@linkcode discover} call, or `connect({ prior })` that adopted a
   * modern verdict (a legacy verdict leaves this `undefined` — there is no
   * `DiscoverResult` on that path). Persistable via `JSON.stringify`; wrap
   * as `{ kind: 'modern', discover }` and feed to {@linkcode ConnectOptions}
   * `prior`.
   */
  getDiscoverResult(): DiscoverResult | undefined;
  protected assertCapabilityForMethod(method: RequestMethod | string): void;
  protected assertNotificationCapability(method: NotificationMethod | string): void;
  protected assertRequestHandlerCapability(method: string): void;
  ping(options?: RequestOptions): Promise<EmptyResult>;
  /**
   * Send `server/discover` (2026-07-28+) and record the result for
   * {@linkcode getDiscoverResult}.
   */
  discover(options?: RequestOptions): Promise<DiscoverResult>;
  /** Requests argument autocompletion suggestions from the server for a prompt or resource. */
  complete(params: CompleteRequest['params'], options?: RequestOptions): Promise<CompleteResult>;
  /**
   * Sets the minimum severity level for log messages sent by the server.
   *
   * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577).
   * Remains functional during the deprecation window (at least twelve months).
   * Migrate to stderr logging (STDIO servers) or OpenTelemetry.
   */
  setLoggingLevel(level: LoggingLevel, options?: RequestOptions): Promise<EmptyResult>;
  /** Retrieves a prompt by name from the server, passing the given arguments for template substitution. */
  getPrompt(params: GetPromptRequest['params'], options?: RequestOptions): Promise<GetPromptResult>;
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
  listPrompts(params?: ListPromptsRequest['params'], options?: CacheableRequestOptions): Promise<ListPromptsResult>;
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
  listResources(params?: ListResourcesRequest['params'], options?: CacheableRequestOptions): Promise<ListResourcesResult>;
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
  listResourceTemplates(params?: ListResourceTemplatesRequest['params'], options?: CacheableRequestOptions): Promise<ListResourceTemplatesResult>;
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
  private _listAllPages;
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
  private _freshness;
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
  private _serveFromCache;
  /** Route a custom-store failure to `onerror` without aborting the surrounding dispatch. */
  private _reportStoreError;
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
  private _compileOutputValidator;
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
  private _resolveXMcpHeaderScan;
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
  readResource(params: ReadResourceRequest['params'], options?: CacheableRequestOptions): Promise<ReadResourceResult>;
  /** Subscribes to change notifications for a resource. The server must support resource subscriptions. */
  subscribeResource(params: SubscribeRequest['params'], options?: RequestOptions): Promise<EmptyResult>;
  /** Unsubscribes from change notifications for a resource. */
  unsubscribeResource(params: UnsubscribeRequest['params'], options?: RequestOptions): Promise<EmptyResult>;
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
  listen(filter: SubscriptionFilter, options?: RequestOptions): Promise<McpSubscription>;
  /**
   * The subscription auto-opened by `ClientOptions.listChanged` on a modern
   * connection — the listen filter is the intersection of the configured
   * sub-options and the server-advertised `listChanged` capabilities.
   * `undefined` on a legacy connection, before connect, or when that
   * intersection is empty (auto-open skipped). Exposed so the consumer can
   * `close()` it.
   */
  get autoOpenedSubscription(): McpSubscription | undefined;
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
  protected _onnotification(raw: JSONRPCNotification, extra?: MessageExtraInfo): void;
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
  protected _onresponse(response: JSONRPCResponse): void;
  /**
   * Settle every live per-listen state machine on a transport-initiated
   * close (the server dropping the connection on stdio/InMemory) before
   * Protocol's `_onclose` tears the transport down. The base
   * `_responseHandlers` settlement does not reach `_listenState` (listen
   * ids are never registered there), so without this override a remote
   * close would leave an in-flight `listen()` / open `McpSubscription`
   * hanging.
   */
  protected _onclose(): void;
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
  callTool(params: CallToolRequest['params'], options?: CallToolRequestOptions): Promise<CallToolResult>;
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
  listTools(params?: ListToolsRequest['params'], options?: CacheableRequestOptions): Promise<ListToolsResult>;
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
  private _excludeInvalidXMcpHeaderTools;
  /**
   * Set up a single list changed handler.
   * @internal
   */
  private _setupListChangedHandler;
  /**
   * Notifies the server that the client's root list has changed. Requires the `roots.listChanged` capability.
   *
   * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577).
   * Remains functional during the deprecation window (at least twelve months).
   * Migrate to passing paths via tool parameters, resource URIs, or configuration.
   */
  sendRootsListChanged(): Promise<void>;
}
//#endregion
//#region src/client/crossAppAccess.d.ts
/**
 * Options for requesting a JWT Authorization Grant via RFC 8693 Token Exchange.
 */
interface RequestJwtAuthGrantOptions {
  /**
   * The IdP's token endpoint URL where the token exchange request will be sent.
   */
  tokenEndpoint: string | URL;
  /**
   * The authorization server URL of the target MCP server (used as `audience` in the token exchange request).
   */
  audience: string | URL;
  /**
   * The resource identifier of the target MCP server (RFC 9728).
   */
  resource: string | URL;
  /**
   * The identity assertion (ID Token) from the enterprise IdP.
   * This should be the OpenID Connect ID Token obtained during user authentication.
   */
  idToken: string;
  /**
   * The client ID registered with the IdP for token exchange.
   */
  clientId: string;
  /**
   * The client secret for authenticating with the IdP.
   *
   * Optional: the IdP may register the MCP client as a public client. RFC 8693 does
   * not mandate confidential clients for token exchange. Omitting this parameter
   * omits `client_secret` from the request body.
   */
  clientSecret?: string;
  /**
   * Optional space-separated list of scopes to request for the target MCP server.
   */
  scope?: string;
  /**
   * Custom fetch implementation. Defaults to global fetch.
   */
  fetchFn?: FetchLike;
}
/**
 * Options for discovering the IdP's token endpoint and requesting a JWT Authorization Grant.
 * Extends {@linkcode RequestJwtAuthGrantOptions} with IdP discovery.
 */
interface DiscoverAndRequestJwtAuthGrantOptions extends Omit<RequestJwtAuthGrantOptions, 'tokenEndpoint'> {
  /**
   * The IdP's issuer URL for OAuth metadata discovery.
   * Will be used to discover the token endpoint via `.well-known/oauth-authorization-server`.
   */
  idpUrl: string | URL;
}
/**
 * Result from a successful JWT Authorization Grant token exchange.
 */
interface JwtAuthGrantResult {
  /**
   * The JWT Authorization Grant (ID-JAG) that can be used to request an access token from the MCP server.
   */
  jwtAuthGrant: string;
  /**
   * Optional expiration time in seconds for the JWT Authorization Grant.
   */
  expiresIn?: number;
  /**
   * Optional scope granted by the IdP (may differ from requested scope).
   */
  scope?: string;
}
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
declare function requestJwtAuthorizationGrant(options: RequestJwtAuthGrantOptions): Promise<JwtAuthGrantResult>;
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
declare function discoverAndRequestJwtAuthGrant(options: DiscoverAndRequestJwtAuthGrantOptions): Promise<JwtAuthGrantResult>;
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
declare function exchangeJwtAuthGrant(options: {
  tokenEndpoint: string | URL;
  jwtAuthGrant: string;
  clientId: string;
  clientSecret?: string;
  /**
   * Client authentication method. Defaults to `'client_secret_basic'` to align with
   * `CrossAppAccessProvider` and SEP-990 conformance requirements.
   * Callers with no `clientSecret` should pass `'none'` for public-client auth.
   */
  authMethod?: ClientAuthMethod;
  fetchFn?: FetchLike;
}): Promise<{
  access_token: string;
  token_type: string;
  expires_in?: number;
  scope?: string;
}>;
//#endregion
//#region src/client/middleware.d.ts
/**
 * Middleware function that wraps and enhances fetch functionality.
 * Takes a fetch handler and returns an enhanced fetch handler.
 */
type Middleware = (next: FetchLike) => FetchLike;
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
declare const withOAuth: (provider: OAuthClientProvider, baseUrl?: string | URL) => Middleware;
/**
 * Logger function type for HTTP requests
 */
type RequestLogger = (input: {
  method: string;
  url: string | URL;
  status: number;
  statusText: string;
  duration: number;
  requestHeaders?: Headers;
  responseHeaders?: Headers;
  error?: Error;
}) => void;
/**
 * Configuration options for the logging middleware
 */
type LoggingOptions = {
  /**
   * Custom logger function, defaults to console logging
   */
  logger?: RequestLogger;
  /**
   * Whether to include request headers in logs
   * @default false
   */
  includeRequestHeaders?: boolean;
  /**
   * Whether to include response headers in logs
   * @default false
   */
  includeResponseHeaders?: boolean;
  /**
   * Status level filter - only log requests with status >= this value
   * Set to `0` to log all requests, `400` to log only errors
   * @default 0
   */
  statusLevel?: number;
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
declare const withLogging: (options?: LoggingOptions) => Middleware;
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
declare const applyMiddlewares: (...middleware: Middleware[]) => Middleware;
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
declare const createMiddleware: (handler: (next: FetchLike, input: string | URL, init?: RequestInit) => Promise<Response>) => Middleware;
//#endregion
//#region src/client/sse.d.ts
declare class SseError extends Error {
  readonly code: number | undefined;
  readonly event: ErrorEvent;
  static [Symbol.hasInstance](value: unknown): boolean;
  /**
   * Brand-based type guard: equivalent to `value instanceof this`, as an
   * explicit static predicate (the axios/AWS-SDK `isInstance` style). Reads
   * the caller's own brand via `this`, so every branded subclass gets a
   * correctly-scoped guard by inheritance. Must be invoked on the class —
   * in callback position write `v => SdkError.isInstance(v)`, not
   * `.filter(SdkError.isInstance)` (detached calls throw rather than
   * silently matching nothing).
   */
  static isInstance<T extends abstract new (...args: never[]) => unknown>(this: T, value: unknown): value is InstanceType<T>;
  constructor(code: number | undefined, message: string | undefined, event: ErrorEvent);
}
/**
 * Configuration options for the {@linkcode SSEClientTransport}.
 */
type SSEClientTransportOptions = {
  /**
   * An OAuth client provider to use for authentication.
   *
   * {@linkcode AuthProvider.token | token()} is called before every request to obtain the
   * bearer token. When the server responds with 401, {@linkcode AuthProvider.onUnauthorized | onUnauthorized()}
   * is called (if provided) to refresh credentials, then the request is retried once. If
   * the retry also gets 401, or `onUnauthorized` is not provided, {@linkcode UnauthorizedError}
   * is thrown.
   *
   * For simple bearer tokens: `{ token: async () => myApiKey }`.
   *
   * For OAuth flows, pass an {@linkcode index.OAuthClientProvider | OAuthClientProvider} implementation.
   * Interactive flows: after {@linkcode UnauthorizedError}, redirect the user, then call
   * {@linkcode SSEClientTransport.finishAuth | finishAuth} with the authorization code before reconnecting.
   */
  authProvider?: AuthProvider | OAuthClientProvider;
  /**
   * Opt-out for the RFC 8414 §3.3 issuer-echo check during authorization-server
   * metadata discovery. **Security-weakening** — see
   * {@linkcode index.AuthOptions.skipIssuerMetadataValidation | AuthOptions.skipIssuerMetadataValidation}.
   * Only honoured when {@linkcode SSEClientTransportOptions.authProvider | authProvider}
   * is an `OAuthClientProvider`.
   */
  skipIssuerMetadataValidation?: boolean;
  /**
   * Customizes the initial SSE request to the server (the request that begins the stream).
   *
   * NOTE: Setting this property will prevent an `Authorization` header from
   * being automatically attached to the SSE request, if an {@linkcode SSEClientTransportOptions.authProvider | authProvider} is
   * also given. This can be worked around by setting the `Authorization` header
   * manually.
   */
  eventSourceInit?: EventSourceInit;
  /**
   * Customizes recurring `POST` requests to the server.
   */
  requestInit?: RequestInit;
  /**
   * Custom fetch implementation used for all network requests.
   */
  fetch?: FetchLike;
};
/**
 * Client transport for SSE: this will connect to a server using Server-Sent Events for receiving
 * messages and make separate `POST` requests for sending messages.
 * @deprecated SSEClientTransport is deprecated. Prefer to use {@linkcode index.StreamableHTTPClientTransport | StreamableHTTPClientTransport} where possible instead. Note that because some servers are still using SSE, clients may need to support both transports during the migration period.
 */
declare class SSEClientTransport implements Transport {
  private _eventSource?;
  private _endpoint?;
  private _abortController?;
  private _url;
  private _resourceMetadataUrl?;
  private _scope?;
  private _eventSourceInit?;
  private _requestInit?;
  private _authProvider?;
  private _oauthProvider?;
  private _skipIssuerMetadataValidation?;
  private _fetch?;
  private _fetchWithInit;
  private _protocolVersion?;
  onclose?: () => void;
  onerror?: (error: Error) => void;
  onmessage?: (message: JSONRPCMessage) => void;
  constructor(url: URL, opts?: SSEClientTransportOptions);
  private _last401Response?;
  private _commonHeaders;
  private _startOrAuth;
  start(): Promise<void>;
  /**
   * Call this method after the user has finished authorizing via their user agent and is redirected back to the MCP client application. This will exchange the authorization code for an access token, enabling the next connection attempt to successfully auth.
   *
   * **Preferred:** pass the callback URL's `searchParams` directly. The SDK extracts `code`
   * and `iss`, validates `iss` against the recorded issuer (RFC 9207) **before** reading any
   * other parameter, and on mismatch throws an {@linkcode IssuerMismatchError} that carries
   * none of the callback's `error`/`error_description`/`error_uri` text. The `(code, iss?)`
   * positional form remains supported for back-compat.
   *
   * The SDK does **not** validate `state`; compare it to your stored value before calling
   * `finishAuth`.
   *
   * @param callbackParams - The `URLSearchParams` from the authorization callback URL
   *   (e.g. `new URL(callbackUrl).searchParams`). `code` and `iss` are read from it.
   */
  finishAuth(callbackParams: URLSearchParams): Promise<void>;
  /**
   * @param authorizationCode - The `code` query parameter from the authorization callback URL.
   * @param iss - The form-urldecoded `iss` query parameter from the same callback URL, if
   *   present. Validated per RFC 9207 against the recorded issuer before the code is redeemed.
   */
  finishAuth(authorizationCode: string, iss?: string): Promise<void>;
  close(): Promise<void>;
  send(message: JSONRPCMessage): Promise<void>;
  private _send;
  setProtocolVersion(version: string): void;
}
//#endregion
//#region src/client/streamableHttp.d.ts
/**
 * Options for starting or authenticating an SSE connection
 */
interface StartSSEOptions {
  /**
   * The resumption token used to continue long-running requests that were interrupted.
   *
   * This allows clients to reconnect and continue from where they left off.
   */
  resumptionToken?: string;
  /**
   * A callback that is invoked when the resumption token changes.
   *
   * This allows clients to persist the latest token for potential reconnection.
   */
  onresumptiontoken?: (token: string) => void;
  /**
   * Override Message ID to associate with the replay message
   * so that the response can be associated with the new resumed request.
   */
  replayMessageId?: string | number;
  /**
   * The per-request abort signal supplied by the caller via
   * `TransportSendOptions.requestSignal`. When this signal is aborted the
   * originating POST and its SSE response stream are torn down
   * intentionally — `_handleSseStream` treats it exactly like the
   * transport-level abort: no `onerror`, no reconnect.
   */
  requestSignal?: AbortSignal;
  /**
   * The per-request stream-end callback supplied via
   * `TransportSendOptions.onRequestStreamEnd`. Fired when the SSE response
   * stream for the originating POST ends or errors for any non-deliberate
   * reason (server closed, network dropped, reconnection exhausted) — never
   * when `requestSignal` was aborted.
   */
  onRequestStreamEnd?: () => void;
}
/**
 * Configuration options for reconnection behavior of the {@linkcode StreamableHTTPClientTransport}.
 */
interface StreamableHTTPReconnectionOptions {
  /**
   * Maximum backoff time between reconnection attempts in milliseconds.
   * Default is 30000 (30 seconds).
   */
  maxReconnectionDelay: number;
  /**
   * Initial backoff time between reconnection attempts in milliseconds.
   * Default is 1000 (1 second).
   */
  initialReconnectionDelay: number;
  /**
   * The factor by which the reconnection delay increases after each attempt.
   * Default is 1.5.
   */
  reconnectionDelayGrowFactor: number;
  /**
   * Maximum number of reconnection attempts before giving up.
   * Default is 2.
   */
  maxRetries: number;
}
/**
 * Custom scheduler for SSE stream reconnection attempts.
 *
 * Called instead of `setTimeout` when the transport needs to schedule a reconnection.
 * Useful in environments where `setTimeout` is unsuitable (serverless functions that
 * terminate before the timer fires, mobile apps that need platform background scheduling,
 * desktop apps handling sleep/wake).
 *
 * @param reconnect - Call this to perform the reconnection attempt.
 * @param delay - Suggested delay in milliseconds (from backoff calculation).
 * @param attemptCount - Zero-indexed retry attempt number.
 * @returns An optional cancel function. If returned, it will be called on
 * {@linkcode StreamableHTTPClientTransport.close | transport.close()} to abort the
 * pending reconnection.
 *
 * @example
 * ```ts source="./streamableHttp.examples.ts#ReconnectionScheduler_basicUsage"
 * const scheduler: ReconnectionScheduler = (reconnect, delay) => {
 *     const id = platformBackgroundTask.schedule(reconnect, delay);
 *     return () => platformBackgroundTask.cancel(id);
 * };
 * ```
 */
type ReconnectionScheduler = (reconnect: () => void, delay: number, attemptCount: number) => (() => void) | void;
/**
 * Configuration options for the {@linkcode StreamableHTTPClientTransport}.
 */
type StreamableHTTPClientTransportOptions = {
  /**
   * An OAuth client provider to use for authentication.
   *
   * {@linkcode AuthProvider.token | token()} is called before every request to obtain the
   * bearer token. When the server responds with 401, {@linkcode AuthProvider.onUnauthorized | onUnauthorized()}
   * is called (if provided) to refresh credentials, then the request is retried once. If
   * the retry also gets 401, or `onUnauthorized` is not provided, {@linkcode UnauthorizedError}
   * is thrown.
   *
   * For simple bearer tokens: `{ token: async () => myApiKey }`.
   *
   * For OAuth flows, pass an {@linkcode index.OAuthClientProvider | OAuthClientProvider} implementation
   * directly — the transport adapts it to `AuthProvider` internally. Interactive flows: after
   * {@linkcode UnauthorizedError}, redirect the user, then call
   * {@linkcode StreamableHTTPClientTransport.finishAuth | finishAuth} with the authorization code before
   * reconnecting.
   */
  authProvider?: AuthProvider | OAuthClientProvider;
  /**
   * Opt-out for the RFC 8414 §3.3 issuer-echo check during authorization-server
   * metadata discovery. **Security-weakening** — see
   * {@linkcode index.AuthOptions.skipIssuerMetadataValidation | AuthOptions.skipIssuerMetadataValidation}.
   * Only honoured when {@linkcode StreamableHTTPClientTransportOptions.authProvider | authProvider}
   * is an `OAuthClientProvider`.
   */
  skipIssuerMetadataValidation?: boolean;
  /**
   * Customizes HTTP requests to the server.
   */
  requestInit?: RequestInit;
  /**
   * Custom fetch implementation used for all network requests.
   */
  fetch?: FetchLike;
  /**
   * Options to configure the reconnection behavior.
   */
  reconnectionOptions?: StreamableHTTPReconnectionOptions;
  /**
   * Custom scheduler for reconnection attempts. If not provided, `setTimeout` is used.
   * See {@linkcode ReconnectionScheduler}.
   */
  reconnectionScheduler?: ReconnectionScheduler;
  /**
   * Session ID for the connection. This is used to identify the session on the server.
   * When not provided and connecting to a server that supports session IDs, the server will generate a new session ID.
   */
  sessionId?: string;
  /**
   * The MCP protocol version to include in the `mcp-protocol-version` header on all requests.
   * When reconnecting with a preserved `sessionId`, set this to the version negotiated during the original
   * handshake so the reconnected transport continues sending the required header.
   */
  protocolVersion?: string;
  /**
   * How the transport reacts to a `403 Forbidden` response carrying
   * `WWW-Authenticate: Bearer error="insufficient_scope"`.
   *
   * - `'reauthorize'` (default): the transport runs the step-up authorization
   *   flow — computes the union of the previously-requested scope and the
   *   challenged scope, calls {@linkcode index.auth | auth()} (forcing a
   *   fresh authorization request when the union strictly exceeds the current
   *   token's granted scope, since refresh cannot widen scope per RFC 6749
   *   §6), and retries the request once. Retries are bounded by
   *   {@linkcode StreamableHTTPClientTransportOptions.maxStepUpRetries | maxStepUpRetries}.
   *   If no {@linkcode index.OAuthClientProvider | OAuthClientProvider} is
   *   configured, step-up cannot run and the transport throws
   *   {@linkcode index.InsufficientScopeError | InsufficientScopeError} instead.
   * - `'throw'`: the transport throws {@linkcode index.InsufficientScopeError | InsufficientScopeError}
   *   carrying the challenge parameters and does not re-authorize. Use this
   *   for `client_credentials` / m2m clients where re-authorization cannot
   *   widen scope, or for interactive clients that want to gate the consent
   *   prompt behind UX.
   *
   * @default 'reauthorize'
   */
  onInsufficientScope?: 'reauthorize' | 'throw';
  /**
   * Maximum number of step-up re-authorization attempts the transport makes
   * per send (and per GET stream open) before giving up. Only consulted when
   * {@linkcode StreamableHTTPClientTransportOptions.onInsufficientScope | onInsufficientScope}
   * is `'reauthorize'`. Cross-request tracking ("this resource+operation
   * already failed N times across the session") is host responsibility.
   *
   * @default 1
   */
  maxStepUpRetries?: number;
};
/**
 * Client transport for Streamable HTTP: this implements the MCP Streamable HTTP transport specification.
 * It will connect to a server using HTTP `POST` for sending messages and HTTP `GET` with Server-Sent Events
 * for receiving messages.
 */
declare class StreamableHTTPClientTransport implements Transport {
  private _abortController?;
  private _url;
  private _resourceMetadataUrl?;
  private _scope?;
  private _requestInit?;
  private _authProvider?;
  private _oauthProvider?;
  private _skipIssuerMetadataValidation?;
  private _fetch?;
  private _fetchWithInit;
  private _sessionId?;
  private _reconnectionOptions;
  private _protocolVersion?;
  private _onInsufficientScope;
  private _maxStepUpRetries;
  private _serverRetryMs?;
  private readonly _reconnectionScheduler?;
  private _cancelReconnection?;
  onclose?: () => void;
  onerror?: (error: Error) => void;
  onmessage?: (message: JSONRPCMessage) => void;
  /**
   * Streamable HTTP opens one POST (and SSE response stream) per outbound
   * request and honors `TransportSendOptions.requestSignal`. On a 2026-era
   * connection the protocol layer aborts that per-request stream as the
   * spec cancellation signal instead of POSTing `notifications/cancelled`.
   */
  readonly hasPerRequestStream = true;
  constructor(url: URL, opts?: StreamableHTTPClientTransportOptions);
  /**
   * SEP-2350 step-up: compute the union scope, decide whether refresh must be
   * bypassed, and run {@linkcode auth}. Returns the auth result so the caller
   * can decide whether to retry. Shared by the POST `_send` path and the GET
   * `_startOrAuthSse` path so both apply the same `'throw'` short-circuit,
   * the same superset-gated refresh bypass, and the same retry cap.
   */
  private _stepUpAuthorize;
  private _stepUpAuthorizeInner;
  private _commonHeaders;
  /**
   * Body-derived per-request headers: when an outgoing request carries a
   * protocol-version claim in its `_meta` envelope (the version negotiation
   * probe is the first such sender), `MCP-Protocol-Version` and `Mcp-Method`
   * derive from the message itself. The connection-level version slot is
   * neither consulted nor mutated; messages without an envelope claim are
   * untouched, so no 2026 header can appear on a legacy exchange.
   */
  private _applyBodyDerivedHeaders;
  /**
   * `true` when the outbound message is a single request carrying a
   * modern-era protocol-version envelope claim — the same predicate that
   * gates body-derived `mcp-method`/`mcp-name` emission. Used to confine the
   * 400-body-as-ProtocolError delivery to modern-era exchanges only.
   */
  private _isModernEnvelopedRequest;
  private _startOrAuthSse;
  /**
   * Calculates the next reconnection delay using a backoff algorithm
   *
   * @param attempt Current reconnection attempt count for the specific stream
   * @returns Time to wait in milliseconds before next reconnection attempt
   */
  private _getNextReconnectionDelay;
  /**
   * Schedule a reconnection attempt using server-provided retry interval or backoff
   *
   * @param lastEventId The ID of the last received event for resumability
   * @param attemptCount Current reconnection attempt count for this specific stream
   */
  private _scheduleReconnection;
  private _handleSseStream;
  start(): Promise<void>;
  /**
   * Call this method after the user has finished authorizing via their user agent and is redirected back to the MCP client application. This will exchange the authorization code for an access token, enabling the next connection attempt to successfully auth.
   *
   * **Preferred:** pass the callback URL's `searchParams` directly. The SDK extracts `code`
   * and `iss`, validates `iss` against the recorded issuer (RFC 9207) **before** reading any
   * other parameter, and on mismatch throws an {@linkcode IssuerMismatchError} that carries
   * none of the callback's `error`/`error_description`/`error_uri` text — those are
   * attacker-controlled in a mix-up attack and MUST NOT be displayed. The `(code, iss?)`
   * positional form remains supported for back-compat.
   *
   * The SDK does **not** validate `state`; compare it to your stored value before calling
   * `finishAuth`.
   *
   * @param callbackParams - The `URLSearchParams` from the authorization callback URL
   *   (e.g. `new URL(callbackUrl).searchParams`). `code` and `iss` are read from it.
   */
  finishAuth(callbackParams: URLSearchParams): Promise<void>;
  /**
   * @param authorizationCode - The `code` query parameter from the authorization callback URL.
   * @param iss - The form-urldecoded `iss` query parameter from the same callback URL, if
   *   present. Validated per RFC 9207 against the recorded issuer before the code is redeemed.
   *   When the authorization server advertises `authorization_response_iss_parameter_supported: true`,
   *   omitting this causes the exchange to be **rejected** with {@linkcode IssuerMismatchError}.
   */
  finishAuth(authorizationCode: string, iss?: string): Promise<void>;
  close(): Promise<void>;
  send(message: JSONRPCMessage | JSONRPCMessage[], options?: {
    resumptionToken?: string;
    onresumptiontoken?: (token: string) => void;
    requestSignal?: AbortSignal;
    onRequestStreamEnd?: () => void;
    headers?: Readonly<Record<string, string>>;
  }): Promise<void>;
  private _send;
  get sessionId(): string | undefined;
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
  terminateSession(): Promise<void>;
  setProtocolVersion(version: string): void;
  get protocolVersion(): string | undefined;
  /**
   * Resume an SSE stream from a previous event ID.
   * Opens a `GET` SSE connection with `Last-Event-ID` header to replay missed events.
   *
   * @param lastEventId The event ID to resume from
   * @param options Optional callback to receive new resumption tokens
   */
  resumeStream(lastEventId: string, options?: {
    onresumptiontoken?: (token: string) => void;
  }): Promise<void>;
}
//#endregion
//#region src/fromJsonSchema.d.ts
declare function fromJsonSchema<T = unknown>(schema: JsonSchemaType, validator?: jsonSchemaValidator): StandardSchemaWithJSON<T, T>;
//#endregion
export { type AddClientAuthentication, Annotations, type AssertionCallback, AudioContent, AuthInfo, type AuthOptions, type AuthProvider, type AuthResult, type AuthorizationServerMetadata, AuthorizationServerMismatchError, BAGGAGE_META_KEY, type BaseContext, BaseMetadata, BlobResourceContents, BooleanSchema, CLIENT_CAPABILITIES_META_KEY, CLIENT_INFO_META_KEY, type CacheEntry, type CacheKey, type CacheMode, type CacheScope, type CacheableRequestOptions, CallToolRequest, type CallToolRequestOptions, CallToolRequestParams, CallToolResult, CancelTaskRequest, CancelTaskResult, CancelledNotification, CancelledNotificationParams, Client, type ClientAuthMethod, ClientCapabilities, type ClientContext, ClientCredentialsProvider, type ClientCredentialsProviderOptions, ClientNotification, type ClientOptions, ClientRequest, ClientResult, CompatibilityCallToolResult, CompleteRequest, CompleteRequestParams, CompleteRequestPrompt, CompleteRequestResourceTemplate, CompleteResult, type ConnectOptions, ContentBlock, CreateMessageRequest, CreateMessageRequestParams, CreateMessageRequestParamsBase, CreateMessageRequestParamsWithTools, CreateMessageResult, CreateMessageResultWithTools, CreateTaskResult, type CrossAppAccessContext, CrossAppAccessProvider, type CrossAppAccessProviderOptions, Cursor, DEFAULT_NEGOTIATED_PROTOCOL_VERSION, DEFAULT_REQUEST_TIMEOUT_MSEC, type DiscoverAndRequestJwtAuthGrantOptions, DiscoverRequest, DiscoverResult, ElicitRequest, ElicitRequestFormParams, ElicitRequestParams, ElicitRequestURLParams, ElicitResult, ElicitationCompleteNotification, ElicitationCompleteNotificationParams, EmbeddedResource, EmptyResult, EnumSchema, type FetchLike, GetPromptRequest, GetPromptRequestParams, GetPromptResult, GetTaskPayloadRequest, GetTaskPayloadResult, GetTaskRequest, GetTaskResult, HandlerResultTypeMap, INTERNAL_ERROR, INVALID_PARAMS, INVALID_REQUEST, Icon, Icons, type IdJagTokenExchangeResponse, ImageContent, Implementation, InMemoryResponseCacheStore, type InMemoryResponseCacheStoreOptions, InMemoryTransport, InitializeRequest, InitializeRequestParams, InitializeResult, InitializedNotification, InputRequest, InputRequests, type InputRequiredOptions, InputRequiredResult, InputResponse, InputResponses, InsecureTokenEndpointError, InsufficientScopeError, InternalError, InvalidParamsError, InvalidRequestError, IssuerMismatchError, JSONArray, JSONObject, JSONRPCErrorResponse, JSONRPCMessage, JSONRPCNotification, JSONRPCRequest, JSONRPCResponse, JSONRPCResultResponse, JSONRPC_VERSION, JSONValue, type JsonSchemaType, type JsonSchemaValidator, type JsonSchemaValidatorResult, type JwtAuthGrantResult, LATEST_PROTOCOL_VERSION, LOG_LEVEL_META_KEY, LegacyTitledEnumSchema, ListChangedCallback, ListChangedHandlers, ListChangedOptions, ListPromptsRequest, ListPromptsResult, ListResourceTemplatesRequest, ListResourceTemplatesResult, ListResourcesRequest, ListResourcesResult, ListRootsRequest, ListRootsResult, ListTasksRequest, ListTasksResult, ListToolsRequest, ListToolsResult, LoggingLevel, LoggingMessageNotification, LoggingMessageNotificationParams, type LoggingOptions, MAX_CACHE_TTL_MS, METHOD_NOT_FOUND, type MaybePromise, type McpSubscription, MessageClassification, MessageExtraInfo, MetaObject, MethodNotFoundError, type Middleware, MissingRequiredClientCapabilityError, MissingRequiredClientCapabilityErrorData, ModelHint, ModelPreferences, MultiSelectEnumSchema, Notification, NotificationMethod, type NotificationOptions, NotificationParams, NotificationTypeMap, NumberSchema, OAuthClientFlowError, type OAuthClientInformation, type OAuthClientInformationContext, type OAuthClientInformationFull, type OAuthClientInformationMixed, type OAuthClientMetadata, type OAuthClientProvider, type OAuthClientRegistrationError, type OAuthDiscoveryState, OAuthError, OAuthErrorCode, type OAuthErrorResponse, type OAuthMetadata, type OAuthProtectedResourceMetadata, type OAuthServerInfo, type OAuthTokenRevocationRequest, type OAuthTokens, type OpenIdProviderDiscoveryMetadata, type OpenIdProviderMetadata, PARSE_ERROR, PROTOCOL_VERSION_META_KEY, PaginatedRequest, PaginatedRequestParams, PaginatedResult, ParseError, PingRequest, PrimitiveSchemaDefinition, type PriorDiscovery, PrivateKeyJwtProvider, type PrivateKeyJwtProviderOptions, Progress, type ProgressCallback, ProgressNotification, ProgressNotificationParams, ProgressToken, Prompt, PromptArgument, PromptListChangedNotification, PromptMessage, PromptReference, Protocol, type ProtocolEra, ProtocolError, ProtocolErrorCode, type ProtocolOptions, RELATED_TASK_META_KEY, ReadBuffer, ReadResourceRequest, ReadResourceRequestParams, ReadResourceResult, type ReconnectionScheduler, RegistrationRejectedError, RelatedTaskMetadata, Request, type RequestHandlerSchemas, RequestId, type RequestJwtAuthGrantOptions, type RequestLogger, RequestMeta, RequestMetaEnvelope, RequestMetaObject, RequestMethod, type RequestOptions, RequestParams, type RequestStateAccessor, RequestTypeMap, Resource, ResourceContents, ResourceLink, ResourceListChangedNotification, ResourceNotFoundError, ResourceRequestParams, ResourceTemplateReference, ResourceTemplateType, ResourceUpdatedNotification, ResourceUpdatedNotificationParams, type ResponseCacheStore, Result, ResultMetaObject, ResultTypeMap, Role, Root, RootsListChangedNotification, SERVER_INFO_META_KEY, SSEClientTransport, type SSEClientTransportOptions, STDIO_DEFAULT_MAX_BUFFER_SIZE, SUBSCRIPTION_ID_META_KEY, SUPPORTED_PROTOCOL_VERSIONS, SamplingContent, SamplingMessage, SamplingMessageContentBlock, SdkError, SdkErrorCode, SdkHttpError, type SdkHttpErrorData, ServerCapabilities, type ServerContext, ServerNotification, ServerRequest, ServerResult, SetLevelRequest, SetLevelRequestParams, SingleSelectEnumSchema, type SpecTypeName, type SpecTypes, SseError, type StandardSchemaV1, type StandardSchemaV1Sync, type StandardSchemaWithJSON, type StartSSEOptions, StaticPrivateKeyJwtProvider, type StaticPrivateKeyJwtProviderOptions, type StoredOAuthClientInformation, type StoredOAuthTokens, StreamableHTTPClientTransport, type StreamableHTTPClientTransportOptions, type StreamableHTTPReconnectionOptions, StringSchema, SubscribeRequest, SubscribeRequestParams, SubscriptionFilter, SubscriptionsAcknowledgedNotification, SubscriptionsAcknowledgedNotificationParams, SubscriptionsListenRequest, SubscriptionsListenRequestParams, SubscriptionsListenResult, SubscriptionsListenResultMeta, TRACEPARENT_META_KEY, TRACESTATE_META_KEY, Task, TaskAugmentedRequestParams, TaskCreationParams, TaskMetadata, TaskStatus, TaskStatusNotification, TaskStatusNotificationParams, TextContent, TextResourceContents, TitledMultiSelectEnumSchema, TitledSingleSelectEnumSchema, Tool, ToolAnnotations, ToolChoice, ToolExecution, ToolListChangedNotification, ToolResultContent, ToolUseContent, type Transport, type TransportSendOptions, UnauthorizedError, UnsubscribeRequest, UnsubscribeRequestParams, UnsupportedProtocolVersionError, UnsupportedProtocolVersionErrorData, UntitledMultiSelectEnumSchema, UntitledSingleSelectEnumSchema, UriTemplate, UrlElicitationRequiredError, type Variables, type VersionNegotiationMode, type VersionNegotiationOptions, type VersionNegotiationProbeOptions, applyMiddlewares, assertCompleteRequestPrompt, assertCompleteRequestResourceTemplate, assertSecureTokenEndpoint, auth, buildDiscoveryUrls, checkResourceAllowed, computeScopeUnion, createFetchWithInit, createMiddleware, createPrivateKeyJwtAuth, deserializeMessage, discoverAndRequestJwtAuthGrant, discoverAuthorizationServerMetadata, discoverOAuthMetadata, discoverOAuthProtectedResourceMetadata, discoverOAuthServerInfo, exchangeAuthorization, exchangeJwtAuthGrant, extractResourceMetadataUrl, extractWWWAuthenticateParams, fetchToken, fromJsonSchema, getDisplayName, getSupportedElicitationModes, isCallToolResult, isHttpsUrl, isInitializeRequest, isInitializedNotification, isInputRequiredResult, isJSONRPCErrorResponse, isJSONRPCNotification, isJSONRPCRequest, isJSONRPCResponse, isJSONRPCResultResponse, isJsonContentType, isSpecType, isStrictScopeSuperset, isTaskAugmentedRequestParams, type jsonSchemaValidator, mergeCapabilities, parseErrorResponse, parseJSONRPCMessage, preloadSchemas, prepareAuthorizationCodeRequest, refreshAuthorization, registerClient, requestJwtAuthorizationGrant, resolveClientMetadata, resourceUrlFromServerUrl, selectClientAuthMethod, selectResourceURL, serializeMessage, specTypeSchemas, startAuthorization, validateAuthorizationResponseIssuer, validateClientMetadataUrl, withInputRequired, withLogging, withOAuth };
//# sourceMappingURL=index.d.cts.map