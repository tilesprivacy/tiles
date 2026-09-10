import { AnnotationsSchema, AudioContentSchema, AuthorizationServerMetadata, BAGGAGE_META_KEY, BaseMetadataSchema, BaseRequestParamsSchema, BlobResourceContentsSchema, BooleanSchemaSchema, CLIENT_CAPABILITIES_META_KEY, CLIENT_INFO_META_KEY, CallToolRequestParamsSchema, CallToolRequestSchema, CallToolResultSchema, CancelTaskRequestSchema, CancelTaskResultSchema, CancelledNotificationParamsSchema, CancelledNotificationSchema, ClientCapabilitiesSchema, ClientNotificationSchema, ClientRequestSchema, ClientResultSchema, ClientTasksCapabilitySchema, CompatibilityCallToolResultSchema, CompleteRequestParamsSchema, CompleteRequestSchema, CompleteResultSchema, ContentBlockSchema, CreateMessageRequestParamsSchema, CreateMessageRequestSchema, CreateMessageResultSchema, CreateMessageResultWithToolsSchema, CreateTaskResultSchema, CursorSchema, DEFAULT_NEGOTIATED_PROTOCOL_VERSION, DiscoverRequestSchema, DiscoverResultSchema, ElicitRequestFormParamsSchema, ElicitRequestParamsSchema, ElicitRequestSchema, ElicitRequestURLParamsSchema, ElicitResultSchema, ElicitationCompleteNotificationParamsSchema, ElicitationCompleteNotificationSchema, EmbeddedResourceSchema, EmptyResultSchema, EnumSchemaSchema, GetPromptRequestParamsSchema, GetPromptRequestSchema, GetPromptResultSchema, GetTaskPayloadRequestSchema, GetTaskPayloadResultSchema, GetTaskRequestSchema, GetTaskResultSchema, INTERNAL_ERROR, INVALID_PARAMS, INVALID_REQUEST, IconSchema, IconsSchema, IdJagTokenExchangeResponse, ImageContentSchema, ImplementationSchema, InitializeRequestParamsSchema, InitializeRequestSchema, InitializeResultSchema, InitializedNotificationSchema, JSONArray, JSONArraySchema, JSONObject, JSONObjectSchema, JSONRPCErrorResponseSchema, JSONRPCMessageSchema, JSONRPCNotificationSchema, JSONRPCRequestSchema, JSONRPCResponseSchema, JSONRPCResultResponseSchema, JSONRPC_VERSION, JSONValue, JSONValueSchema, LATEST_PROTOCOL_VERSION, LOG_LEVEL_META_KEY, LegacyTitledEnumSchemaSchema, ListChangedOptionsBaseSchema, ListPromptsRequestSchema, ListPromptsResultSchema, ListResourceTemplatesRequestSchema, ListResourceTemplatesResultSchema, ListResourcesRequestSchema, ListResourcesResultSchema, ListRootsRequestSchema, ListRootsResultSchema, ListTasksRequestSchema, ListTasksResultSchema, ListToolsRequestSchema, ListToolsResultSchema, LoggingLevelSchema, LoggingMessageNotificationParamsSchema, LoggingMessageNotificationSchema, METHOD_NOT_FOUND, ModelHintSchema, ModelPreferencesSchema, MultiSelectEnumSchemaSchema, NotificationSchema, NotificationsParamsSchema, NumberSchemaSchema, OAuthClientInformation, OAuthClientInformationFull, OAuthClientInformationMixed, OAuthClientMetadata, OAuthClientRegistrationError, OAuthErrorResponse, OAuthMetadata, OAuthProtectedResourceMetadata, OAuthTokenRevocationRequest, OAuthTokens, OpenIdProviderDiscoveryMetadata, OpenIdProviderMetadata, PARSE_ERROR, PROTOCOL_VERSION_META_KEY, PaginatedRequestParamsSchema, PaginatedRequestSchema, PaginatedResultSchema, PingRequestSchema, PrimitiveSchemaDefinitionSchema, ProgressNotificationParamsSchema, ProgressNotificationSchema, ProgressSchema, ProgressTokenSchema, PromptArgumentSchema, PromptListChangedNotificationSchema, PromptMessageSchema, PromptReferenceSchema, PromptSchema, RELATED_TASK_META_KEY, ReadResourceRequestParamsSchema, ReadResourceRequestSchema, ReadResourceResultSchema, RelatedTaskMetadataSchema, RequestIdSchema, RequestMetaSchema, RequestSchema, ResourceContentsSchema, ResourceLinkSchema, ResourceListChangedNotificationSchema, ResourceRequestParamsSchema, ResourceSchema, ResourceTemplateReferenceSchema, ResourceTemplateSchema, ResourceUpdatedNotificationParamsSchema, ResourceUpdatedNotificationSchema, ResultMetaObjectSchema, ResultSchema, RoleSchema, RootSchema, RootsListChangedNotificationSchema, SERVER_INFO_META_KEY, SUBSCRIPTION_ID_META_KEY, SUPPORTED_PROTOCOL_VERSIONS, SamplingContentSchema, SamplingMessageContentBlockSchema, SamplingMessageSchema, ServerCapabilitiesSchema, ServerNotificationSchema, ServerRequestSchema, ServerResultSchema, ServerTasksCapabilitySchema, SetLevelRequestParamsSchema, SetLevelRequestSchema, SingleSelectEnumSchemaSchema, StoredOAuthClientInformation, StoredOAuthTokens, StringSchemaSchema, SubscribeRequestParamsSchema, SubscribeRequestSchema, SubscriptionFilterSchema, SubscriptionsAcknowledgedNotificationParamsSchema, SubscriptionsAcknowledgedNotificationSchema, SubscriptionsListenRequestParamsSchema, SubscriptionsListenRequestSchema, SubscriptionsListenResultMetaSchema, SubscriptionsListenResultSchema, TRACEPARENT_META_KEY, TRACESTATE_META_KEY, TaskAugmentedRequestParamsSchema, TaskCreationParamsSchema, TaskMetadataSchema, TaskSchema, TaskStatusNotificationParamsSchema, TaskStatusNotificationSchema, TaskStatusSchema, TextContentSchema, TextResourceContentsSchema, TitledMultiSelectEnumSchemaSchema, TitledSingleSelectEnumSchemaSchema, ToolAnnotationsSchema, ToolChoiceSchema, ToolExecutionSchema, ToolListChangedNotificationSchema, ToolResultContentSchema, ToolSchema, ToolUseContentSchema, UnsubscribeRequestParamsSchema, UnsubscribeRequestSchema, UntitledMultiSelectEnumSchemaSchema, UntitledSingleSelectEnumSchemaSchema } from "@modelcontextprotocol/core/internal";
import * as z from "zod/v4";

//#region rolldown:runtime

//#endregion
//#region ../core-internal/src/auth/errors.d.ts
/**
 * OAuth error codes as defined by {@link https://datatracker.ietf.org/doc/html/rfc6749#section-5.2 | RFC 6749}
 * and extensions.
 */
declare enum OAuthErrorCode {
  /**
   * The request is missing a required parameter, includes an invalid parameter value,
   * includes a parameter more than once, or is otherwise malformed.
   */
  InvalidRequest = "invalid_request",
  /**
   * Client authentication failed (e.g., unknown client, no client authentication included,
   * or unsupported authentication method).
   */
  InvalidClient = "invalid_client",
  /**
   * The provided authorization grant or refresh token is invalid, expired, revoked,
   * does not match the redirection URI used in the authorization request, or was issued to another client.
   */
  InvalidGrant = "invalid_grant",
  /**
   * The authenticated client is not authorized to use this authorization grant type.
   */
  UnauthorizedClient = "unauthorized_client",
  /**
   * The authorization grant type is not supported by the authorization server.
   */
  UnsupportedGrantType = "unsupported_grant_type",
  /**
   * The requested scope is invalid, unknown, malformed, or exceeds the scope granted by the resource owner.
   */
  InvalidScope = "invalid_scope",
  /**
   * The resource owner or authorization server denied the request.
   */
  AccessDenied = "access_denied",
  /**
   * The authorization server encountered an unexpected condition that prevented it from fulfilling the request.
   */
  ServerError = "server_error",
  /**
   * The authorization server is currently unable to handle the request due to temporary overloading or maintenance.
   */
  TemporarilyUnavailable = "temporarily_unavailable",
  /**
   * The authorization server does not support obtaining an authorization code using this method.
   */
  UnsupportedResponseType = "unsupported_response_type",
  /**
   * The authorization server does not support the requested token type.
   */
  UnsupportedTokenType = "unsupported_token_type",
  /**
   * The access token provided is expired, revoked, malformed, or invalid for other reasons.
   */
  InvalidToken = "invalid_token",
  /**
   * The HTTP method used is not allowed for this endpoint. (Custom, non-standard error)
   */
  MethodNotAllowed = "method_not_allowed",
  /**
   * Rate limit exceeded. (Custom, non-standard error based on RFC 6585)
   */
  TooManyRequests = "too_many_requests",
  /**
   * The client metadata is invalid. (Custom error for dynamic client registration - RFC 7591)
   */
  InvalidClientMetadata = "invalid_client_metadata",
  /**
   * The value of one or more redirection URIs is invalid. (Dynamic client registration - RFC 7591 §3.2.2)
   */
  InvalidRedirectUri = "invalid_redirect_uri",
  /**
   * The request requires higher privileges than provided by the access token.
   */
  InsufficientScope = "insufficient_scope",
  /**
   * The requested resource is invalid, missing, unknown, or malformed. (Custom error for resource indicators - RFC 8707)
   */
  InvalidTarget = "invalid_target",
}
/**
 * OAuth error class for all OAuth-related errors.
 */
declare class OAuthError extends Error {
  readonly code: OAuthErrorCode | string;
  readonly errorUri?: string | undefined;
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
  static isInstance<T$1 extends abstract new (...args: never[]) => unknown>(this: T$1, value: unknown): value is InstanceType<T$1>;
  constructor(code: OAuthErrorCode | string, message: string, errorUri?: string | undefined);
  /**
   * Converts the error to a standard OAuth error response object.
   */
  toResponseObject(): OAuthErrorResponse;
  /**
   * Creates an {@linkcode OAuthError} from an OAuth error response.
   */
  static fromResponse(response: OAuthErrorResponse): OAuthError;
}
//#endregion
//#region ../core-internal/src/errors/sdkErrors.d.ts
/**
 * Error codes for SDK errors (local errors that never cross the wire).
 * Unlike {@linkcode ProtocolErrorCode} which uses numeric JSON-RPC codes, `SdkErrorCode` uses
 * descriptive string values for better developer experience.
 *
 * These errors are thrown locally by the SDK and are never serialized as
 * JSON-RPC error responses.
 */
declare enum SdkErrorCode {
  /** Transport is not connected */
  NotConnected = "NOT_CONNECTED",
  /** Transport is already connected */
  AlreadyConnected = "ALREADY_CONNECTED",
  /** Protocol is not initialized */
  NotInitialized = "NOT_INITIALIZED",
  /** Required capability is not supported by the remote side */
  CapabilityNotSupported = "CAPABILITY_NOT_SUPPORTED",
  /** Request timed out waiting for response */
  RequestTimeout = "REQUEST_TIMEOUT",
  /** Connection was closed */
  ConnectionClosed = "CONNECTION_CLOSED",
  /** Failed to send message */
  SendFailed = "SEND_FAILED",
  /** Response result failed local schema validation */
  InvalidResult = "INVALID_RESULT",
  /**
   * The response carried a `resultType` discriminator (protocol revision
   * 2026-07-28) naming a result kind this client cannot consume yet, e.g.
   * `input_required`. The kind is carried in `data.resultType`.
   */
  UnsupportedResultType = "UNSUPPORTED_RESULT_TYPE",
  /**
   * The multi-round-trip auto-fulfilment driver exhausted its round cap
   * (`inputRequired.maxRounds`) without the server returning a complete
   * result. `data.rounds` carries the cap that was hit and
   * `data.lastResult` carries the last `input_required` payload received
   * (`{ inputRequests, requestState? }`), so callers can inspect or resume
   * the flow manually.
   */
  InputRequiredRoundsExceeded = "INPUT_REQUIRED_ROUNDS_EXCEEDED",
  /**
   * The auto-aggregating no-`cursor` `listTools()` / `listPrompts()` /
   * `listResources()` / `listResourceTemplates()` walk hit the
   * `ClientOptions.listMaxPages` cap without the server's pagination
   * converging. `data.method` carries the list verb and
   * `data.listMaxPages` the cap that was hit; raise the cap or fall back to
   * explicit per-page `{ cursor }` calls.
   */
  ListPaginationExceeded = "LIST_PAGINATION_EXCEEDED",
  /**
   * The spec method being sent does not exist on the negotiated protocol
   * version's wire era (e.g. `tasks/get` toward a 2026-07-28 peer, or
   * `server/discover` toward a 2025-era peer). Raised locally, before
   * anything reaches the transport. The method and era are carried in
   * `data.method` / `data.era`.
   */
  MethodNotSupportedByProtocolVersion = "METHOD_NOT_SUPPORTED_BY_PROTOCOL_VERSION",
  /**
   * Protocol-era negotiation at connect time failed without producing either a
   * usable modern (2026-07-28+) era or a definitive legacy fallback signal —
   * e.g. the negotiation mode forbids falling back (`pin`), the probe hit a
   * network failure, or the server answered the probe with a 5xx (a typed
   * connect error, never an era verdict).
   *
   * Negotiation-phase only: this code is never used once an era is
   * established. Auth walls never carry it: a 401/403 rejecting the probe
   * uses {@linkcode ClientHttpAuthentication} / {@linkcode ClientHttpForbidden}
   * instead, so era-recovery flows keyed on this code (e.g. cached-verdict
   * gateways) can never persist a verdict for an unauthorized exchange.
   */
  EraNegotiationFailed = "ERA_NEGOTIATION_FAILED",
  ClientHttpNotImplemented = "CLIENT_HTTP_NOT_IMPLEMENTED",
  /**
   * HTTP 401 authentication failure: the transport's re-auth retry still got
   * 401 (`Server returned 401 after re-authentication`), or the version
   * negotiation probe was rejected 401 with no `authProvider` configured
   * (`Version negotiation failed: the server requires authorization (HTTP 401)`).
   * Carried on an {@linkcode SdkHttpError} with `status: 401`.
   */
  ClientHttpAuthentication = "CLIENT_HTTP_AUTHENTICATION",
  /**
   * HTTP 403 denial: the step-up re-authorization retry limit was reached,
   * or the version negotiation probe was rejected 403
   * (`Version negotiation failed: the server denied access (HTTP 403)`).
   * Carried on an {@linkcode SdkHttpError} with `status: 403`.
   */
  ClientHttpForbidden = "CLIENT_HTTP_FORBIDDEN",
  ClientHttpUnexpectedContent = "CLIENT_HTTP_UNEXPECTED_CONTENT",
  ClientHttpFailedToOpenStream = "CLIENT_HTTP_FAILED_TO_OPEN_STREAM",
  ClientHttpFailedToTerminateSession = "CLIENT_HTTP_FAILED_TO_TERMINATE_SESSION",
}
/**
 * SDK errors are local errors that never cross the wire.
 * They are distinct from {@linkcode ProtocolError} which represents JSON-RPC protocol errors
 * that are serialized and sent as error responses.
 *
 * @example
 * ```ts source="./sdkErrors.examples.ts#SdkError_basicUsage"
 * try {
 *     // Throwing an SDK error
 *     throw new SdkError(SdkErrorCode.NotConnected, 'Transport is not connected');
 * } catch (error) {
 *     // Checking error type by code
 *     if (error instanceof SdkError && error.code === SdkErrorCode.RequestTimeout) {
 *         // Handle timeout
 *     }
 * }
 * ```
 */
declare class SdkError extends Error {
  readonly code: SdkErrorCode;
  readonly data?: unknown | undefined;
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
  static isInstance<T$1 extends abstract new (...args: never[]) => unknown>(this: T$1, value: unknown): value is InstanceType<T$1>;
  constructor(code: SdkErrorCode, message: string, data?: unknown | undefined);
}
/**
 * Typed shape for HTTP error data carried by {@linkcode SdkHttpError}.
 */
interface SdkHttpErrorData {
  status: number;
  statusText?: string;
  [key: string]: unknown;
}
/**
 * An {@linkcode SdkError} subclass for HTTP transport failures.
 *
 * Thrown by the streamable HTTP transport when the server responds with a
 * non-OK status code. Narrows {@linkcode SdkError.data | data} to
 * {@linkcode SdkHttpErrorData} so consumers can inspect the HTTP status
 * without unsafe casting.
 *
 * @example
 * ```ts source="./sdkErrors.examples.ts#SdkHttpError_basicUsage"
 * if (error instanceof SdkHttpError) {
 *     console.log(error.status); // number
 *     console.log(error.statusText); // string | undefined
 * }
 * ```
 */
declare class SdkHttpError extends SdkError {
  readonly data: SdkHttpErrorData;
  constructor(code: SdkErrorCode, message: string, data: SdkHttpErrorData);
  get status(): number;
  get statusText(): string | undefined;
}
//#endregion
//#region ../core-internal/src/shared/authUtils.d.ts
/**
 * Utilities for handling OAuth resource URIs.
 */
/**
 * Converts a server URL to a resource URL by removing the fragment.
 * {@link https://datatracker.ietf.org/doc/html/rfc8707#section-2 | RFC 8707 section 2}
 * states that resource URIs "MUST NOT include a fragment component".
 * Keeps everything else unchanged (scheme, domain, port, path, query).
 */
declare function resourceUrlFromServerUrl(url: URL | string): URL;
/**
 * Checks if a requested resource URL matches a configured resource URL.
 * A requested resource matches if it has the same scheme, domain, port,
 * and its path starts with the configured resource's path.
 *
 * @param options - The options object
 * @param options.requestedResource - The resource URL being requested
 * @param options.configuredResource - The resource URL that has been configured
 * @returns true if the requested resource matches the configured resource, false otherwise
 */
declare function checkResourceAllowed({
  requestedResource,
  configuredResource
}: {
  requestedResource: URL | string;
  configuredResource: URL | string;
}): boolean;
declare namespace schemas_d_exports {
  export { AnnotationsSchema, AudioContentSchema, BaseMetadataSchema, BaseRequestParamsSchema, BlobResourceContentsSchema, BooleanSchemaSchema, CallToolRequestParamsSchema, CallToolRequestSchema, CallToolResultSchema, CancelTaskRequestSchema, CancelTaskResultSchema, CancelledNotificationParamsSchema, CancelledNotificationSchema, ClientCapabilitiesSchema, ClientNotificationSchema, ClientRequestSchema, ClientResultSchema, ClientTasksCapabilitySchema, CompatibilityCallToolResultSchema, CompleteRequestParamsSchema, CompleteRequestSchema, CompleteResultSchema, ContentBlockSchema, CreateMessageRequestParamsSchema, CreateMessageRequestSchema, CreateMessageResultSchema, CreateMessageResultWithToolsSchema, CreateTaskResultSchema, CursorSchema, DiscoverRequestSchema, DiscoverResultSchema, ElicitRequestFormParamsSchema, ElicitRequestParamsSchema, ElicitRequestSchema, ElicitRequestURLParamsSchema, ElicitResultSchema, ElicitationCompleteNotificationParamsSchema, ElicitationCompleteNotificationSchema, EmbeddedResourceSchema, EmptyResultSchema, EnumSchemaSchema, GetPromptRequestParamsSchema, GetPromptRequestSchema, GetPromptResultSchema, GetTaskPayloadRequestSchema, GetTaskPayloadResultSchema, GetTaskRequestSchema, GetTaskResultSchema, IconSchema, IconsSchema, ImageContentSchema, ImplementationSchema, InitializeRequestParamsSchema, InitializeRequestSchema, InitializeResultSchema, InitializedNotificationSchema, JSONArraySchema, JSONObjectSchema, JSONRPCErrorResponseSchema, JSONRPCMessageSchema, JSONRPCNotificationSchema, JSONRPCRequestSchema, JSONRPCResponseSchema, JSONRPCResultResponseSchema, JSONValueSchema, LegacyTitledEnumSchemaSchema, ListChangedOptionsBaseSchema, ListPromptsRequestSchema, ListPromptsResultSchema, ListResourceTemplatesRequestSchema, ListResourceTemplatesResultSchema, ListResourcesRequestSchema, ListResourcesResultSchema, ListRootsRequestSchema, ListRootsResultSchema, ListTasksRequestSchema, ListTasksResultSchema, ListToolsRequestSchema, ListToolsResultSchema, LoggingLevelSchema, LoggingMessageNotificationParamsSchema, LoggingMessageNotificationSchema, ModelHintSchema, ModelPreferencesSchema, MultiSelectEnumSchemaSchema, NotificationSchema, NotificationsParamsSchema, NumberSchemaSchema, PaginatedRequestParamsSchema, PaginatedRequestSchema, PaginatedResultSchema, PingRequestSchema, PrimitiveSchemaDefinitionSchema, ProgressNotificationParamsSchema, ProgressNotificationSchema, ProgressSchema, ProgressTokenSchema, PromptArgumentSchema, PromptListChangedNotificationSchema, PromptMessageSchema, PromptReferenceSchema, PromptSchema, ReadResourceRequestParamsSchema, ReadResourceRequestSchema, ReadResourceResultSchema, RelatedTaskMetadataSchema, RequestIdSchema, RequestMetaSchema, RequestSchema, ResourceContentsSchema, ResourceLinkSchema, ResourceListChangedNotificationSchema, ResourceRequestParamsSchema, ResourceSchema, ResourceTemplateReferenceSchema, ResourceTemplateSchema, ResourceUpdatedNotificationParamsSchema, ResourceUpdatedNotificationSchema, ResultMetaObjectSchema, ResultSchema, RoleSchema, RootSchema, RootsListChangedNotificationSchema, SamplingContentSchema, SamplingMessageContentBlockSchema, SamplingMessageSchema, ServerCapabilitiesSchema, ServerNotificationSchema, ServerRequestSchema, ServerResultSchema, ServerTasksCapabilitySchema, SetLevelRequestParamsSchema, SetLevelRequestSchema, SingleSelectEnumSchemaSchema, StringSchemaSchema, SubscribeRequestParamsSchema, SubscribeRequestSchema, SubscriptionFilterSchema, SubscriptionsAcknowledgedNotificationParamsSchema, SubscriptionsAcknowledgedNotificationSchema, SubscriptionsListenRequestParamsSchema, SubscriptionsListenRequestSchema, SubscriptionsListenResultMetaSchema, SubscriptionsListenResultSchema, TaskAugmentedRequestParamsSchema, TaskCreationParamsSchema, TaskMetadataSchema, TaskSchema, TaskStatusNotificationParamsSchema, TaskStatusNotificationSchema, TaskStatusSchema, TextContentSchema, TextResourceContentsSchema, TitledMultiSelectEnumSchemaSchema, TitledSingleSelectEnumSchemaSchema, ToolAnnotationsSchema, ToolChoiceSchema, ToolExecutionSchema, ToolListChangedNotificationSchema, ToolResultContentSchema, ToolSchema, ToolUseContentSchema, UnsubscribeRequestParamsSchema, UnsubscribeRequestSchema, UntitledMultiSelectEnumSchemaSchema, UntitledSingleSelectEnumSchemaSchema };
}
//#endregion
//#region ../core-internal/src/types/types.d.ts
/**
 * Utility types
 */
type ExpandRecursively<T$1> = T$1 extends object ? (T$1 extends infer O ? { [K in keyof O]: ExpandRecursively<O[K]> } : never) : T$1;
type Primitive = string | number | boolean | bigint | null | undefined;
type Flatten<T$1> = T$1 extends Primitive ? T$1 : T$1 extends Array<infer U> ? Array<Flatten<U>> : T$1 extends Set<infer U> ? Set<Flatten<U>> : T$1 extends Map<infer K, infer V> ? Map<Flatten<K>, Flatten<V>> : T$1 extends object ? { [K in keyof T$1]: Flatten<T$1[K]> } : T$1;
type Infer<Schema extends z.ZodTypeAny> = Flatten<z.infer<Schema>>;
/**
 * Wire-only members hidden from the public types.
 *
 * `resultType` is the protocol-revision-2026-07-28 wire discrimination field
 * on results. It is consumed by the SDK's protocol layer (and stripped before
 * results reach consumers), so the public result types do not declare it.
 * The wire schemas continue to model it internally.
 */
type WireOnlyResultKey = 'resultType';
/**
 * Removes wire-only members from a (possibly union) schema-inferred type
 * while preserving every other declared member, optionality, and the loose
 * index signature.
 */
type StripWireOnly<T$1> = T$1 extends unknown ? { [K in keyof T$1 as K extends WireOnlyResultKey ? never : K]: T$1[K] } : never;
type ProgressToken = Infer<typeof ProgressTokenSchema>;
type Cursor = Infer<typeof CursorSchema>;
type Request = Infer<typeof RequestSchema>;
/** @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only. */
type TaskAugmentedRequestParams = Infer<typeof TaskAugmentedRequestParamsSchema>;
type RequestMeta = Infer<typeof RequestMetaSchema>;
type Notification = Infer<typeof NotificationSchema>;
type Result$1 = StripWireOnly<Infer<typeof ResultSchema>>;
type RequestId = Infer<typeof RequestIdSchema>;
type JSONRPCRequest = Infer<typeof JSONRPCRequestSchema>;
type JSONRPCNotification = Infer<typeof JSONRPCNotificationSchema>;
type JSONRPCErrorResponse = Infer<typeof JSONRPCErrorResponseSchema>;
type JSONRPCResultResponse = Omit<Infer<typeof JSONRPCResultResponseSchema>, 'result'> & {
  result: Result$1;
};
type JSONRPCResponse = JSONRPCResultResponse | JSONRPCErrorResponse;
type JSONRPCMessage = JSONRPCRequest | JSONRPCNotification | JSONRPCResultResponse | JSONRPCErrorResponse;
type RequestParams = Infer<typeof BaseRequestParamsSchema>;
type NotificationParams = Infer<typeof NotificationsParamsSchema>;
/**
 * The per-request `_meta` envelope carried by every request under protocol revision
 * 2026-07-28 (protocol version, client info, client capabilities, optional log level).
 *
 * Neutral hand-written shape keyed by the public meta-key constants — never
 * inferred from a wire-module schema (this neutral layer does not import from
 * `wire/rev*`). A `type` alias rather than an interface so it stays assignable
 * to `_meta`'s string-indexed object slot.
 */
type RequestMetaEnvelope = {};
type EmptyResult = StripWireOnly<Infer<typeof EmptyResultSchema>>;
type CancelledNotificationParams = Infer<typeof CancelledNotificationParamsSchema>;
type CancelledNotification = Infer<typeof CancelledNotificationSchema>;
type Icon = Infer<typeof IconSchema>;
type Icons = Infer<typeof IconsSchema>;
type BaseMetadata = Infer<typeof BaseMetadataSchema>;
type Annotations = Infer<typeof AnnotationsSchema>;
type Role = Infer<typeof RoleSchema>;
type Implementation = Infer<typeof ImplementationSchema>;
/**
 * Capabilities a client may support.
 *
 * Note: the `roots` and `sampling` capabilities are deprecated as of protocol
 * version 2026-07-28 (SEP-2577); they remain in the specification for at least
 * twelve months. See `ClientCapabilitiesSchema`.
 */
type ClientCapabilities = Infer<typeof ClientCapabilitiesSchema>;
type InitializeRequestParams = Infer<typeof InitializeRequestParamsSchema>;
type InitializeRequest = Infer<typeof InitializeRequestSchema>;
/**
 * Capabilities a server may support.
 *
 * Note: the `logging` capability is deprecated as of protocol version
 * 2026-07-28 (SEP-2577); it remains in the specification for at least twelve
 * months. See `ServerCapabilitiesSchema`.
 */
type ServerCapabilities = Infer<typeof ServerCapabilitiesSchema>;
type InitializeResult = StripWireOnly<Infer<typeof InitializeResultSchema>>;
type InitializedNotification = Infer<typeof InitializedNotificationSchema>;
type DiscoverRequest = Infer<typeof DiscoverRequestSchema>;
type DiscoverResult = StripWireOnly<Infer<typeof DiscoverResultSchema>>;
type PingRequest = Infer<typeof PingRequestSchema>;
type Progress = Infer<typeof ProgressSchema>;
type ProgressNotificationParams = Infer<typeof ProgressNotificationParamsSchema>;
type ProgressNotification = Infer<typeof ProgressNotificationSchema>;
/** @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only. */
type Task = Infer<typeof TaskSchema>;
/** @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only. */
type TaskStatus = Infer<typeof TaskStatusSchema>;
/** @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only. */
type TaskCreationParams = Infer<typeof TaskCreationParamsSchema>;
/** @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only. */
type TaskMetadata = Infer<typeof TaskMetadataSchema>;
/** @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only. */
type RelatedTaskMetadata = Infer<typeof RelatedTaskMetadataSchema>;
/** @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only. */
type CreateTaskResult = StripWireOnly<Infer<typeof CreateTaskResultSchema>>;
/** @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only. */
type TaskStatusNotificationParams = Infer<typeof TaskStatusNotificationParamsSchema>;
/** @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only. */
type TaskStatusNotification = Infer<typeof TaskStatusNotificationSchema>;
/** @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only. */
type GetTaskRequest = Infer<typeof GetTaskRequestSchema>;
/** @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only. */
type GetTaskResult = StripWireOnly<Infer<typeof GetTaskResultSchema>>;
/** @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only. */
type GetTaskPayloadRequest = Infer<typeof GetTaskPayloadRequestSchema>;
/** @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only. */
type ListTasksRequest = Infer<typeof ListTasksRequestSchema>;
/** @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only. */
type ListTasksResult = StripWireOnly<Infer<typeof ListTasksResultSchema>>;
/** @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only. */
type CancelTaskRequest = Infer<typeof CancelTaskRequestSchema>;
/** @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only. */
type CancelTaskResult = StripWireOnly<Infer<typeof CancelTaskResultSchema>>;
/** @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only. */
type GetTaskPayloadResult = StripWireOnly<Infer<typeof GetTaskPayloadResultSchema>>;
type PaginatedRequestParams = Infer<typeof PaginatedRequestParamsSchema>;
type PaginatedRequest = Infer<typeof PaginatedRequestSchema>;
type PaginatedResult = StripWireOnly<Infer<typeof PaginatedResultSchema>>;
type ResourceContents = Infer<typeof ResourceContentsSchema>;
type TextResourceContents = Infer<typeof TextResourceContentsSchema>;
type BlobResourceContents = Infer<typeof BlobResourceContentsSchema>;
type Resource = Infer<typeof ResourceSchema>;
type ResourceTemplateType = Infer<typeof ResourceTemplateSchema>;
type ListResourcesRequest = Infer<typeof ListResourcesRequestSchema>;
type ListResourcesResult = StripWireOnly<Infer<typeof ListResourcesResultSchema>>;
type ListResourceTemplatesRequest = Infer<typeof ListResourceTemplatesRequestSchema>;
type ListResourceTemplatesResult = StripWireOnly<Infer<typeof ListResourceTemplatesResultSchema>>;
type ResourceRequestParams = Infer<typeof ResourceRequestParamsSchema>;
type ReadResourceRequestParams = Infer<typeof ReadResourceRequestParamsSchema>;
type ReadResourceRequest = Infer<typeof ReadResourceRequestSchema>;
type ReadResourceResult = StripWireOnly<Infer<typeof ReadResourceResultSchema>>;
type ResourceListChangedNotification = Infer<typeof ResourceListChangedNotificationSchema>;
type SubscribeRequestParams = Infer<typeof SubscribeRequestParamsSchema>;
type SubscribeRequest = Infer<typeof SubscribeRequestSchema>;
type UnsubscribeRequestParams = Infer<typeof UnsubscribeRequestParamsSchema>;
type UnsubscribeRequest = Infer<typeof UnsubscribeRequestSchema>;
type ResourceUpdatedNotificationParams = Infer<typeof ResourceUpdatedNotificationParamsSchema>;
type ResourceUpdatedNotification = Infer<typeof ResourceUpdatedNotificationSchema>;
type SubscriptionFilter = Infer<typeof SubscriptionFilterSchema>;
type SubscriptionsListenRequestParams = Infer<typeof SubscriptionsListenRequestParamsSchema>;
type SubscriptionsListenRequest = Infer<typeof SubscriptionsListenRequestSchema>;
type SubscriptionsAcknowledgedNotificationParams = Infer<typeof SubscriptionsAcknowledgedNotificationParamsSchema>;
type SubscriptionsAcknowledgedNotification = Infer<typeof SubscriptionsAcknowledgedNotificationSchema>;
type SubscriptionsListenResultMeta = Infer<typeof SubscriptionsListenResultMetaSchema>;
type SubscriptionsListenResult = StripWireOnly<Infer<typeof SubscriptionsListenResultSchema>>;
type PromptArgument = Infer<typeof PromptArgumentSchema>;
type Prompt = Infer<typeof PromptSchema>;
type ListPromptsRequest = Infer<typeof ListPromptsRequestSchema>;
type ListPromptsResult = StripWireOnly<Infer<typeof ListPromptsResultSchema>>;
type GetPromptRequestParams = Infer<typeof GetPromptRequestParamsSchema>;
type GetPromptRequest = Infer<typeof GetPromptRequestSchema>;
type TextContent = Infer<typeof TextContentSchema>;
type ImageContent = Infer<typeof ImageContentSchema>;
type AudioContent = Infer<typeof AudioContentSchema>;
/**
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to calling LLM
 * provider APIs directly.
 */
type ToolUseContent = Infer<typeof ToolUseContentSchema>;
/**
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to calling LLM
 * provider APIs directly.
 */
type ToolResultContent = Infer<typeof ToolResultContentSchema>;
type EmbeddedResource = Infer<typeof EmbeddedResourceSchema>;
type ResourceLink = Infer<typeof ResourceLinkSchema>;
type ContentBlock = Infer<typeof ContentBlockSchema>;
type PromptMessage = Infer<typeof PromptMessageSchema>;
type GetPromptResult = StripWireOnly<Infer<typeof GetPromptResultSchema>>;
type PromptListChangedNotification = Infer<typeof PromptListChangedNotificationSchema>;
type ToolAnnotations = Infer<typeof ToolAnnotationsSchema>;
type ToolExecution = Infer<typeof ToolExecutionSchema>;
type Tool = Infer<typeof ToolSchema>;
type ListToolsRequest = Infer<typeof ListToolsRequestSchema>;
type ListToolsResult = StripWireOnly<Infer<typeof ListToolsResultSchema>>;
type CallToolRequestParams = Infer<typeof CallToolRequestParamsSchema>;
type CallToolResult = StripWireOnly<Infer<typeof CallToolResultSchema>>;
type CompatibilityCallToolResult = StripWireOnly<Infer<typeof CompatibilityCallToolResultSchema>>;
type CallToolRequest = Infer<typeof CallToolRequestSchema>;
type ToolListChangedNotification = Infer<typeof ToolListChangedNotificationSchema>;
/**
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to stderr logging
 * (STDIO servers) or OpenTelemetry.
 */
type LoggingLevel = Infer<typeof LoggingLevelSchema>;
/**
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to stderr logging
 * (STDIO servers) or OpenTelemetry.
 */
type SetLevelRequestParams = Infer<typeof SetLevelRequestParamsSchema>;
/**
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to stderr logging
 * (STDIO servers) or OpenTelemetry.
 */
type SetLevelRequest = Infer<typeof SetLevelRequestSchema>;
/**
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to stderr logging
 * (STDIO servers) or OpenTelemetry.
 */
type LoggingMessageNotificationParams = Infer<typeof LoggingMessageNotificationParamsSchema>;
/**
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to stderr logging
 * (STDIO servers) or OpenTelemetry.
 */
type LoggingMessageNotification = Infer<typeof LoggingMessageNotificationSchema>;
/**
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to calling LLM
 * provider APIs directly.
 */
type ToolChoice = Infer<typeof ToolChoiceSchema>;
/**
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to calling LLM
 * provider APIs directly.
 */
type ModelHint = Infer<typeof ModelHintSchema>;
/**
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to calling LLM
 * provider APIs directly.
 */
type ModelPreferences = Infer<typeof ModelPreferencesSchema>;
/**
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to calling LLM
 * provider APIs directly.
 */
type SamplingContent = Infer<typeof SamplingContentSchema>;
/**
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to calling LLM
 * provider APIs directly.
 */
type SamplingMessageContentBlock = Infer<typeof SamplingMessageContentBlockSchema>;
/**
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to calling LLM
 * provider APIs directly.
 */
type SamplingMessage = Infer<typeof SamplingMessageSchema>;
/**
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to calling LLM
 * provider APIs directly.
 */
type CreateMessageRequestParams = Infer<typeof CreateMessageRequestParamsSchema>;
/**
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to calling LLM
 * provider APIs directly.
 */
type CreateMessageRequest = Infer<typeof CreateMessageRequestSchema>;
/**
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to calling LLM
 * provider APIs directly.
 */
type CreateMessageResult = StripWireOnly<Infer<typeof CreateMessageResultSchema>>;
/**
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to calling LLM
 * provider APIs directly.
 */
type CreateMessageResultWithTools = StripWireOnly<Infer<typeof CreateMessageResultWithToolsSchema>>;
type BooleanSchema = Infer<typeof BooleanSchemaSchema>;
type StringSchema = Infer<typeof StringSchemaSchema>;
type NumberSchema = Infer<typeof NumberSchemaSchema>;
type EnumSchema = Infer<typeof EnumSchemaSchema>;
type UntitledSingleSelectEnumSchema = Infer<typeof UntitledSingleSelectEnumSchemaSchema>;
type TitledSingleSelectEnumSchema = Infer<typeof TitledSingleSelectEnumSchemaSchema>;
type LegacyTitledEnumSchema = Infer<typeof LegacyTitledEnumSchemaSchema>;
type UntitledMultiSelectEnumSchema = Infer<typeof UntitledMultiSelectEnumSchemaSchema>;
type TitledMultiSelectEnumSchema = Infer<typeof TitledMultiSelectEnumSchemaSchema>;
type SingleSelectEnumSchema = Infer<typeof SingleSelectEnumSchemaSchema>;
type MultiSelectEnumSchema = Infer<typeof MultiSelectEnumSchemaSchema>;
type PrimitiveSchemaDefinition = Infer<typeof PrimitiveSchemaDefinitionSchema>;
type ElicitRequestParams = Infer<typeof ElicitRequestParamsSchema>;
type ElicitRequestFormParams = Infer<typeof ElicitRequestFormParamsSchema>;
type ElicitRequestURLParams = Infer<typeof ElicitRequestURLParamsSchema>;
type ElicitRequest = Infer<typeof ElicitRequestSchema>;
/** @deprecated Removed from the spec by #2891 (2026-07-28). 2025-era only; the 2026-07-28 wire codec excludes this notification. */
type ElicitationCompleteNotificationParams = Infer<typeof ElicitationCompleteNotificationParamsSchema>;
/** @deprecated Removed from the spec by #2891 (2026-07-28). 2025-era only; the 2026-07-28 wire codec excludes this notification. */
type ElicitationCompleteNotification = Infer<typeof ElicitationCompleteNotificationSchema>;
type ElicitResult = StripWireOnly<Infer<typeof ElicitResultSchema>>;
type ResourceTemplateReference = Infer<typeof ResourceTemplateReferenceSchema>;
type PromptReference = Infer<typeof PromptReferenceSchema>;
type CompleteRequestParams = Infer<typeof CompleteRequestParamsSchema>;
type CompleteRequest = Infer<typeof CompleteRequestSchema>;
type CompleteResult = StripWireOnly<Infer<typeof CompleteResultSchema>>;
/**
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to passing paths via
 * tool parameters, resource URIs, or configuration.
 */
type Root = Infer<typeof RootSchema>;
/**
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to passing paths via
 * tool parameters, resource URIs, or configuration.
 */
type ListRootsRequest = Infer<typeof ListRootsRequestSchema>;
/**
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to passing paths via
 * tool parameters, resource URIs, or configuration.
 */
type ListRootsResult = StripWireOnly<Infer<typeof ListRootsResultSchema>>;
/**
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to passing paths via
 * tool parameters, resource URIs, or configuration.
 */
type RootsListChangedNotification = Infer<typeof RootsListChangedNotificationSchema>;
/**
 * A single embedded (de-JSON-RPC'd) input request inside an
 * {@linkcode InputRequiredResult}: an elicitation, sampling, or roots request
 * object carried in-band rather than sent as a server→client JSON-RPC request.
 */
type InputRequest = CreateMessageRequest | ListRootsRequest | ElicitRequest;
/**
 * A single embedded (de-JSON-RPC'd) input response inside a retried request's
 * `inputResponses`: the bare result object for the corresponding
 * {@linkcode InputRequest} (never wrapped in a `{method, result}` envelope).
 */
type InputResponse = CreateMessageResult | ListRootsResult | ElicitResult;
/**
 * A map of embedded input requests, keyed by server-assigned identifiers that
 * are unique within the scope of the request.
 */
interface InputRequests {
  [key: string]: InputRequest;
}
/**
 * A map of embedded input responses. Keys correspond to the keys of the
 * {@linkcode InputRequests} map the server sent; values are the client's bare
 * result for each request.
 */
interface InputResponses {
  [key: string]: InputResponse;
}
/**
 * The input-required result a handler for a multi-round-trip method
 * (`tools/call`, `prompts/get`, `resources/read`) returns to request more
 * input from the client (protocol revision 2026-07-28). Build it with the
 * `inputRequired()` builder; hand-built literals are equally legal —
 * `resultType: 'input_required'` is the discriminator, and the SDK re-checks
 * the at-least-one rule at the seam.
 *
 * This is the one place the wire discriminator `resultType` appears on the
 * neutral surface: the handler authors it, the 2026-07-28 codec passes it
 * through to the wire, and consumers receiving results never see it (complete
 * results are lifted).
 *
 * At least one of `inputRequests` or `requestState` must be present.
 *
 * `requestState` is an opaque, server-minted string echoed back verbatim by
 * the client on retry. It travels through the client and MUST be treated by
 * the server as attacker-controlled input on re-entry: if it influences
 * authorization, resource access, or business logic, the server MUST protect
 * its integrity (e.g. HMAC or AEAD) and MUST reject state that fails
 * verification (spec: basic/patterns/mrtr §Server Requirements). The SDK
 * applies no integrity protection by default — without a configured
 * `ServerOptions.requestState.verify` hook, `ctx.mcpReq.requestState()`
 * returns the raw, unverified string; with one, the seam rejects state the
 * hook refuses and the accessor returns the hook's decoded payload.
 */
interface InputRequiredResult extends Result$1 {
  resultType: 'input_required';
  /** Embedded requests the client must fulfil before retrying. */
  inputRequests?: InputRequests;
  /** Opaque server state the client echoes back verbatim on retry. */
  requestState?: string;
}
type ClientRequest = Infer<typeof ClientRequestSchema>;
type ClientNotification = Infer<typeof ClientNotificationSchema>;
type ClientResult = StripWireOnly<Infer<typeof ClientResultSchema>>;
type ServerRequest = Infer<typeof ServerRequestSchema>;
type ServerNotification = Infer<typeof ServerNotificationSchema>;
type ServerResult = StripWireOnly<Infer<typeof ServerResultSchema>>;
type MethodToTypeMap<U$1> = { [T in U$1 as T extends {
  method: infer M extends string;
} ? M : never]: T };
/**
 * Task methods are 2025-11-25 wire vocabulary with no SDK runtime: the task
 * wire types stay importable (see the Tasks section above), but the typed
 * method surface — `request()`, `setRequestHandler()`, `ctx.mcpReq.send()` —
 * does not offer them. The wire schemas keep parsing task vocabulary for
 * interoperability with 2025-11-25 peers.
 */
type TaskRequestMethod = 'tasks/get' | 'tasks/result' | 'tasks/list' | 'tasks/cancel';
type TaskNotificationMethod = 'notifications/tasks/status';
type RequestMethod = Exclude<ClientRequest['method'] | ServerRequest['method'], TaskRequestMethod>;
type NotificationMethod = Exclude<ClientNotification['method'] | ServerNotification['method'], TaskNotificationMethod>;
type RequestTypeMap = MethodToTypeMap<Exclude<ClientRequest | ServerRequest, {
  method: TaskRequestMethod;
}>>;
type NotificationTypeMap = MethodToTypeMap<Exclude<ClientNotification | ServerNotification, {
  method: TaskNotificationMethod;
}>>;
type ResultTypeMap = {
  ping: EmptyResult;
  initialize: InitializeResult;
  'server/discover': DiscoverResult;
  'completion/complete': CompleteResult;
  'logging/setLevel': EmptyResult;
  'prompts/get': GetPromptResult;
  'prompts/list': ListPromptsResult;
  'resources/list': ListResourcesResult;
  'resources/templates/list': ListResourceTemplatesResult;
  'resources/read': ReadResourceResult;
  'resources/subscribe': EmptyResult;
  'resources/unsubscribe': EmptyResult;
  'subscriptions/listen': SubscriptionsListenResult;
  'tools/call': CallToolResult;
  'tools/list': ListToolsResult;
  'sampling/createMessage': CreateMessageResult | CreateMessageResultWithTools;
  'elicitation/create': ElicitResult;
  'roots/list': ListRootsResult;
};
/**
 * The handler-return counterpart of {@linkcode ResultTypeMap}: what a
 * registered request handler may RETURN for each method. Identical to
 * `ResultTypeMap` except that the multi-round-trip methods (`tools/call`,
 * `prompts/get`, `resources/read`) additionally accept an
 * {@linkcode InputRequiredResult} (protocol revision 2026-07-28).
 *
 * `ResultTypeMap` itself — what a *requester* receives — is deliberately NOT
 * widened: `client.callTool()` returns a plain {@linkcode CallToolResult} on
 * both protocol eras.
 */
type HandlerResultTypeMap = { [M in keyof ResultTypeMap]: M extends 'tools/call' | 'prompts/get' | 'resources/read' ? ResultTypeMap[M] | InputRequiredResult : ResultTypeMap[M] };
/**
 * Information about a validated access token, provided to request handlers.
 */
interface AuthInfo {
  /**
   * The access token.
   */
  token: string;
  /**
   * The client ID associated with this token.
   */
  clientId: string;
  /**
   * Scopes associated with this token.
   */
  scopes: string[];
  /**
   * When the token expires (in seconds since epoch).
   */
  expiresAt?: number;
  /**
   * The RFC 8707 resource server identifier for which this token is valid.
   * If set, this MUST match the MCP server's resource identifier (minus hash fragment).
   */
  resource?: URL;
  /**
   * Additional data associated with the token.
   * This field should be used for any additional data that needs to be attached to the auth info.
   */
  extra?: Record<string, unknown>;
}
type JSONRPCErrorObject = {
  code: number;
  message: string;
  data?: unknown;
};
interface ParseError extends JSONRPCErrorObject {
  code: typeof PARSE_ERROR;
}
interface InvalidRequestError extends JSONRPCErrorObject {
  code: typeof INVALID_REQUEST;
}
interface MethodNotFoundError extends JSONRPCErrorObject {
  code: typeof METHOD_NOT_FOUND;
}
interface InvalidParamsError extends JSONRPCErrorObject {
  code: typeof INVALID_PARAMS;
}
interface InternalError extends JSONRPCErrorObject {
  code: typeof INTERNAL_ERROR;
}
/**
 * Data carried by a `-32021` MissingRequiredClientCapability protocol error
 * (protocol revision 2026-07-28).
 */
interface MissingRequiredClientCapabilityErrorData {
  /**
   * The capabilities the server requires from the client to process the
   * request, in the `ClientCapabilities` shape (only the missing
   * capabilities are listed).
   */
  requiredCapabilities: ClientCapabilities;
}
/**
 * Data carried by a `-32022` UnsupportedProtocolVersion protocol error
 * (protocol revision 2026-07-28).
 */
interface UnsupportedProtocolVersionErrorData {
  /**
   * Protocol versions the receiver supports. The sender should choose a
   * mutually supported version from this list and retry.
   */
  supported: string[];
  /**
   * The protocol version that was requested.
   */
  requested: string;
}
/**
 * Callback type for list changed notifications.
 */
type ListChangedCallback<T$1> = (error: Error | null, items: T$1[] | null) => void;
/**
 * Options for subscribing to list changed notifications.
 *
 * @typeParam T - The type of items in the list (`Tool`, `Prompt`, or `Resource`)
 */
type ListChangedOptions<T$1> = {
  /**
   * If `true`, the list will be refreshed automatically when a list changed notification is received.
   * @default true
   */
  autoRefresh?: boolean;
  /**
   * Debounce time in milliseconds. Set to `0` to disable.
   * @default 300
   */
  debounceMs?: number;
  /**
   * Callback invoked when the list changes.
   *
   * If `autoRefresh` is `true`, `items` contains the updated list.
   * If `autoRefresh` is `false`, `items` is `null` (caller should refresh manually).
   */
  onChanged: ListChangedCallback<T$1>;
};
/**
 * Configuration for list changed notification handlers.
 *
 * Use this to configure handlers for tools, prompts, and resources list changes
 * when creating a client.
 *
 * Note: Handlers are only activated if the server advertises the corresponding
 * `listChanged` capability (e.g., `tools.listChanged: true`). If the server
 * doesn't advertise this capability, the handler will not be set up.
 */
type ListChangedHandlers = {
  /**
   * Handler for tool list changes.
   */
  tools?: ListChangedOptions<Tool>;
  /**
   * Handler for prompt list changes.
   */
  prompts?: ListChangedOptions<Prompt>;
  /**
   * Handler for resource list changes.
   */
  resources?: ListChangedOptions<Resource>;
};
/**
 * Protocol-era classification of an inbound message.
 *
 * Populated by transports that classify messages at the edge (e.g. an HTTP
 * entry distinguishing 2025-era from 2026-era traffic). The wire era itself
 * is connection state (the negotiated protocol version held by the
 * `Client`/`Server` instance); the protocol layer validates a classified
 * message against that instance era at dispatch — a mismatch is treated as
 * an entry/routing error, never a per-message era switch. Unclassified
 * traffic is dispatched on the instance era unchanged.
 */
interface MessageClassification {
  /**
   * The wire era the message was classified into: `legacy` for the
   * 2025-11-25 family of revisions, `modern` for 2026-07-28 and later.
   */
  era: 'legacy' | 'modern';
  /**
   * The exact protocol revision, when the classifier derived one.
   */
  revision?: string;
}
/**
 * Extra information about a message.
 */
interface MessageExtraInfo {
  /**
   * The original HTTP request.
   */
  request?: globalThis.Request;
  /**
   * Protocol-era classification of the message, when the transport
   * classified it at the edge. Validated by the protocol layer against the
   * instance's negotiated era at dispatch (the edge→instance handoff
   * check); it does not select the era itself.
   */
  classification?: MessageClassification;
  /**
   * The authentication information.
   */
  authInfo?: AuthInfo;
  /**
   * Callback to close the SSE stream for this request, triggering client reconnection.
   * Only available when using {@linkcode @modelcontextprotocol/node!streamableHttp.NodeStreamableHTTPServerTransport | NodeStreamableHTTPServerTransport} with eventStore configured.
   */
  closeSSEStream?: () => void;
  /**
   * Callback to close the standalone GET SSE stream, triggering client reconnection.
   * Only available when using {@linkcode @modelcontextprotocol/node!streamableHttp.NodeStreamableHTTPServerTransport | NodeStreamableHTTPServerTransport} with eventStore configured.
   */
  closeStandaloneSSEStream?: () => void;
}
type MetaObject = Record<string, unknown>;
type RequestMetaObject = RequestMeta;
/**
 * The contents of a result's `_meta` field (the 2026-07-28 `ResultMetaObject`):
 * loose, with the optional self-reported serverInfo key typed when present.
 */
type ResultMetaObject = Infer<typeof ResultMetaObjectSchema>;
/**
 * {@linkcode CreateMessageRequestParams} without tools - for backwards-compatible overload.
 * Excludes tools/toolChoice to indicate they should not be provided.
 *
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to calling LLM
 * provider APIs directly.
 */
type CreateMessageRequestParamsBase = Omit<CreateMessageRequestParams, 'tools' | 'toolChoice'>;
/**
 * {@linkcode CreateMessageRequestParams} with required tools - for tool-enabled overload.
 *
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to calling LLM
 * provider APIs directly.
 */
interface CreateMessageRequestParamsWithTools extends CreateMessageRequestParams {
  tools: Tool[];
}
type CompleteRequestResourceTemplate = ExpandRecursively<CompleteRequest & {
  params: CompleteRequestParams & {
    ref: ResourceTemplateReference;
  };
}>;
type CompleteRequestPrompt = ExpandRecursively<CompleteRequest & {
  params: CompleteRequestParams & {
    ref: PromptReference;
  };
}>;
//#endregion
//#region ../core-internal/src/wire/codec.d.ts
/** Wire eras with distinct vocabulary. */
type WireEra = '2025-11-25' | '2026-07-28';
/**
 * Wire-only material lifted off an inbound message by the protocol layer
 * before dispatch (the V-3 seam): the reserved `_meta` envelope keys and the
 * multi-round-trip driver fields. This is the typed driver-material channel
 * of the codec contract — handlers never see it; the protocol layer surfaces
 * it via `ctx.mcpReq.envelope` / `.inputResponses` / `.requestState`, and the
 * MRTR driver (M4.1) consumes the retry fields from here.
 */
interface LiftedWireMaterial {
  envelope?: Partial<RequestMetaEnvelope>;
  inputResponses?: Record<string, unknown>;
  requestState?: string;
}
/**
 * Tri-state validation outcome — the function-only contract for what the
 * schema-getter pair (`hasRequestMethod` ⇒ −32601-by-absence; `…Schema(m)
 * .parse` throw ⇒ −32602) used to encode in two pieces. Preserving the split
 * is the point: collapsing 'absent from this era's registry' into 'invalid'
 * would make the in-band fallback chain (validate → on `not-in-era` fall to
 * validateInputRequest) treat absence as failure and never fall through.
 */
type ValidateOutcome<T$1> = {
  readonly ok: true;
  readonly value: T$1;
}
/**
 * Method is spec vocabulary but absent from THIS era's registry. Callers
 * map to −32601 (inbound) or a typed local SdkError (outbound), or fall
 * through to the in-band validator.
 */ | {
  readonly ok: false;
  readonly reason: 'not-in-era';
}
/**
 * Method is in this era's registry; payload failed the era-exact schema.
 * Callers map to −32602.
 */ | {
  readonly ok: false;
  readonly reason: 'invalid';
  readonly message: string;
};
/** A single self-identifying problem found while validating a per-request `_meta` envelope. */
interface EnvelopeIssue {
  /**
   * The envelope key the problem is about: one of the reserved `_meta`
   * keys, or a dotted path inside one.
   */
  readonly key: string;
  /** A short description of what is wrong with that key (`missing`, or a validation message). */
  readonly problem: string;
}
/** Material a Client supplies for an era to build its per-request `_meta` envelope from. */
interface OutboundEnvelopeMaterial {
  readonly protocolVersion: string;
  readonly clientInfo: Implementation;
  readonly clientCapabilities: ClientCapabilities;
  readonly logLevel?: LoggingLevel;
}
/** Result decode outcomes — the raw-first discrimination (V-1) lives in `decodeResult`. */
type DecodedResult = {
  kind: 'complete';
  /** The neutral result value: wire-only material consumed/stripped. */
  result: Result$1;
} | {
  kind: 'input_required';
  /**
   * Driver-only material (never consumer-visible). The full
   * multi-round-trip driver is M4.1 scope; this seam carries the
   * discriminated payload to it.
   */
  inputRequests: Record<string, unknown>;
  requestState?: string;
} | {
  kind: 'invalid';
  error: SdkError;
};
/**
 * The per-era wire codec contract (design C §3, adapted to the live funnel
 * layout: the universal wire-only LIFT runs once in the protocol layer for
 * every message — spec, custom, and fallback paths alike — and codecs consume
 * the lifted material rather than re-implementing the strip per era).
 */
interface WireCodec {
  readonly era: WireEra;
  /** Registry membership — the deletion story (inbound −32601 by absence; outbound typed local error). */
  hasRequestMethod(method: string): boolean;
  hasNotificationMethod(method: string): boolean;
  hasInputRequestMethod(method: string): boolean;
  /** Era-exact request validation. `not-in-era` ≡ −32601-by-absence; `invalid` ≡ −32602. */
  validateRequest<M$1 extends RequestMethod>(method: M$1, raw: unknown): ValidateOutcome<RequestTypeMap[M$1]>;
  validateRequest(method: string, raw: unknown): ValidateOutcome<unknown>;
  /** Era-exact result validation (same registry as `validateRequest`). */
  validateResult<M$1 extends RequestMethod>(method: M$1, raw: unknown): ValidateOutcome<ResultTypeMap[M$1]>;
  validateResult(method: string, raw: unknown): ValidateOutcome<unknown>;
  /** Era-exact notification validation. */
  validateNotification<M$1 extends NotificationMethod>(method: M$1, raw: unknown): ValidateOutcome<NotificationTypeMap[M$1]>;
  validateNotification(method: string, raw: unknown): ValidateOutcome<unknown>;
  /**
   * In-band (de-JSON-RPC'd) input-request validation — the embedded
   * requests a multi-round-trip `input_required` result may carry. Always
   * `not-in-era` on the 2025 era (elicitation/sampling/roots are wire
   * request methods there). Does NOT grant registry membership.
   */
  validateInputRequest<M$1 extends RequestMethod>(method: M$1, raw: unknown): ValidateOutcome<RequestTypeMap[M$1]>;
  validateInputRequest(method: string, raw: unknown): ValidateOutcome<unknown>;
  /** In-band bare-response validation answering an embedded input request. */
  validateInputResponse<M$1 extends RequestMethod>(method: M$1, raw: unknown): ValidateOutcome<ResultTypeMap[M$1]>;
  validateInputResponse(method: string, raw: unknown): ValidateOutcome<unknown>;
  /**
   * Param-conditional `sampling/createMessage` result validation — the one
   * spec result whose schema depends on REQUEST params (tools vs no tools).
   * The 2025 era owns the with-tools/plain frozen schemas; the 2026 era
   * returns `not-in-era` (sampling is in-band there — callers fall through
   * to `validateInputResponse`).
   */
  samplingResultVariant(hasTools: true, raw: unknown): ValidateOutcome<CreateMessageResultWithTools>;
  samplingResultVariant(hasTools: false, raw: unknown): ValidateOutcome<CreateMessageResult>;
  samplingResultVariant(hasTools: boolean, raw: unknown): ValidateOutcome<CreateMessageResult | CreateMessageResultWithTools>;
  /**
   * Outbound per-request `_meta` envelope encode. Returns the keyed object
   * to merge into `params._meta` on this era, or `undefined` when this era
   * carries no per-request envelope (the 2025 era — legacy wire stays
   * byte-identical).
   */
  outboundEnvelope(material: OutboundEnvelopeMaterial): Readonly<Record<string, unknown>> | undefined;
  /**
   * Structured envelope validation: maps a `_meta` object to
   * self-identifying issues. The 2025 era never requires an envelope and
   * always returns `[]`; the 2026 era owns the required-key pre-pass plus
   * the wire-exact `RequestMetaEnvelopeSchema` parse.
   */
  validateEnvelopeMeta(meta: Readonly<Record<string, unknown>>): EnvelopeIssue[];
  /**
   * Per-registration `tools/call` result projection. Two independent
   * decisions, both owned here so server-side code never re-derives them:
   *
   * - SEP-2106 §4.3 TextContent auto-append (EVERY era, value-shape-based):
   *   when `structuredContent` is a non-object value (array/primitive/
   *   `null`) and the handler authored no `type:'text'` block, append
   *   `{type:'text', text: JSON.stringify(value)}` so consumers that read
   *   only `content` still receive a rendering. The author opts out by
   *   returning any `text` block themselves.
   *
   * - `{result:…}` wrap (2025 era only): wrap as `{result:<value>}` when the
   *   value is non-object (the 2025 wire shape requires `structuredContent`
   *   to be an object — a schema-less tool returning `[1,2,3]` would
   *   otherwise ship wire-illegal bytes) OR when the tool's ADVERTISED
   *   `outputSchema` has a non-object root (so the result matches the
   *   `encodeResult('tools/list')` projection of the same tool). Identity on
   *   the 2026 era — the wire shape carries the natural value directly.
   */
  projectCallToolResult(result: CallToolResult, advertisedOutputSchema: Readonly<Record<string, unknown>> | undefined): CallToolResult;
  /**
   * Step 1 of result decoding: RAW `resultType` handling BEFORE any schema
   * validation (V-1's structural home). Era postures (Q1-SD3):
   * - 2026 era: required discriminator — absent ⇒ typed error naming the
   *   spec violation; `input_required` ⇒ driver payload; unknown ⇒ invalid,
   *   no retry; `complete` ⇒ consume + lift.
   * - 2025 era: `resultType` is foreign vocabulary ⇒ strip-on-lift.
   */
  decodeResult(method: string, raw: unknown): DecodedResult;
  /**
   * Outbound result mapping (the stamp seam). The 2025-era codec is the
   * identity — it has NO stamp code path (the never-stamp guarantee; it
   * never reads `serverInfo`). The 2026-era codec strictly enforces the
   * 2026 wire shape for the known deleted-field set
   * (`execution.taskSupport`, `capabilities.tasks` — Q1-SD3 iii), stamps
   * `resultType`, fills the required `ttlMs`/`cacheScope` fields on
   * cacheable results, and — when the caller supplies its identity —
   * stamps `_meta['io.modelcontextprotocol/serverInfo']` on every result
   * (spec PR #3002 SHOULD; a handler-authored value wins). `Server`
   * instances pass their identity; clients and hand-constructed protocol
   * objects pass nothing and no identity is ever invented.
   */
  encodeResult(method: string, result: Result$1, serverInfo?: Implementation): Result$1;
  /**
   * Outbound error-code mapping (the error half of the stamp seam). A
   * handler-thrown `ProtocolError`'s numeric code passes through here on
   * its way to the JSON-RPC error response, so per-era wire-code selection
   * lives in the codec rather than in handler/funnel code. The current
   * mapping is identical on both eras (the `-32002` resource-not-found
   * domain code maps to `-32602` Invalid Params on the wire — the
   * 2026-07-28 spec MUST, and what the deployed v1.x SDK already emits on
   * earlier revisions); the seam is the structural home for any future
   * per-era divergence. Unknown codes pass through unchanged.
   */
  encodeErrorCode(code: number): number;
  /**
   * @deprecated Use {@link validateEnvelopeMeta}. Inbound envelope
   * enforcement for era-classified traffic: validates the lifted envelope
   * material of a request. Returns an error message when the era requires
   * an envelope and it is missing/invalid (→ −32602 at the dispatch
   * layer); `undefined` when acceptable. The 2025 era never requires an
   * envelope.
   */
  checkInboundEnvelope(material: LiftedWireMaterial): string | undefined;
}
//#endregion
//#region ../core-internal/src/util/standardSchema.d.ts
interface StandardTypedV1<Input = unknown, Output = Input> {
  readonly '~standard': StandardTypedV1.Props<Input, Output>;
}
declare namespace StandardTypedV1 {
  interface Props<Input = unknown, Output = Input> {
    readonly version: 1;
    readonly vendor: string;
    readonly types?: Types<Input, Output> | undefined;
  }
  interface Types<Input = unknown, Output = Input> {
    readonly input: Input;
    readonly output: Output;
  }
  type InferInput<Schema extends StandardTypedV1> = NonNullable<Schema['~standard']['types']>['input'];
  type InferOutput<Schema extends StandardTypedV1> = NonNullable<Schema['~standard']['types']>['output'];
}
interface StandardSchemaV1<Input = unknown, Output = Input> {
  readonly '~standard': StandardSchemaV1.Props<Input, Output>;
}
declare namespace StandardSchemaV1 {
  interface Props<Input = unknown, Output = Input> extends StandardTypedV1.Props<Input, Output> {
    readonly validate: (value: unknown, options?: Options | undefined) => Result<Output> | Promise<Result<Output>>;
  }
  interface Options {
    readonly libraryOptions?: Record<string, unknown> | undefined;
  }
  type Result<Output> = SuccessResult<Output> | FailureResult;
  interface SuccessResult<Output> {
    readonly value: Output;
    readonly issues?: undefined;
  }
  interface FailureResult {
    readonly issues: ReadonlyArray<Issue>;
  }
  interface Issue {
    readonly message: string;
    readonly path?: ReadonlyArray<PropertyKey | PathSegment> | undefined;
  }
  interface PathSegment {
    readonly key: PropertyKey;
  }
  type InferInput<Schema extends StandardTypedV1> = StandardTypedV1.InferInput<Schema>;
  type InferOutput<Schema extends StandardTypedV1> = StandardTypedV1.InferOutput<Schema>;
}
interface StandardJSONSchemaV1<Input = unknown, Output = Input> {
  readonly '~standard': StandardJSONSchemaV1.Props<Input, Output>;
}
declare namespace StandardJSONSchemaV1 {
  interface Props<Input = unknown, Output = Input> extends StandardTypedV1.Props<Input, Output> {
    readonly jsonSchema: Converter;
  }
  interface Converter {
    readonly input: (options: Options) => Record<string, unknown>;
    readonly output: (options: Options) => Record<string, unknown>;
  }
  type Target = 'draft-2020-12' | 'draft-07' | 'openapi-3.0' | (object & string);
  interface Options {
    readonly target: Target;
    readonly libraryOptions?: Record<string, unknown> | undefined;
  }
  type InferInput<Schema extends StandardTypedV1> = StandardTypedV1.InferInput<Schema>;
  type InferOutput<Schema extends StandardTypedV1> = StandardTypedV1.InferOutput<Schema>;
}
/**
 * Combined interface for schemas with both validation and JSON Schema conversion —
 * the intersection of {@linkcode StandardSchemaV1} and {@linkcode StandardJSONSchemaV1}.
 *
 * This is the type accepted by `registerTool` / `registerPrompt`. The SDK needs
 * `~standard.jsonSchema` to advertise the tool's argument shape in `tools/list`, and
 * `~standard.validate` to check incoming arguments when a `tools/call` arrives.
 *
 * Zod v4, ArkType, and Valibot (via `@valibot/to-json-schema`'s `toStandardJsonSchema`)
 * all implement both interfaces.
 *
 * @see https://standardschema.dev/ for the Standard Schema specification
 */
interface StandardSchemaWithJSON<Input = unknown, Output = Input> {
  readonly '~standard': StandardSchemaV1.Props<Input, Output> & StandardJSONSchemaV1.Props<Input, Output>;
}
declare namespace StandardSchemaWithJSON {
  type InferInput<Schema extends StandardTypedV1> = StandardTypedV1.InferInput<Schema>;
  type InferOutput<Schema extends StandardTypedV1> = StandardTypedV1.InferOutput<Schema>;
}
/**
 * Narrowing of {@linkcode StandardSchemaV1} whose `validate` is guaranteed synchronous.
 *
 * The Zod schemas backing `specTypeSchemas` contain no async refinements or transforms,
 * so every entry satisfies this interface. Consumers can call `validate()` and access
 * `.issues` / `.value` on the result without `await`.
 *
 * `StandardSchemaV1Sync` is assignable to `StandardSchemaV1` — it is a strict subtype.
 */
interface StandardSchemaV1Sync<Input = unknown, Output = Input> extends StandardSchemaV1<Input, Output> {
  readonly '~standard': StandardSchemaV1Sync.Props<Input, Output>;
}
declare namespace StandardSchemaV1Sync {
  interface Props<Input = unknown, Output = Input> extends StandardSchemaV1.Props<Input, Output> {
    readonly validate: (value: unknown, options?: StandardSchemaV1.Options | undefined) => StandardSchemaV1.Result<Output>;
  }
  type InferInput<Schema extends StandardTypedV1> = StandardTypedV1.InferInput<Schema>;
  type InferOutput<Schema extends StandardTypedV1> = StandardTypedV1.InferOutput<Schema>;
}
//#endregion
//#region ../core-internal/src/shared/inputRequired.d.ts
/**
 * Wraps a result schema so a request issued through `client.request()` /
 * `ctx.mcpReq.send()` with `allowInputRequired: true` is typed as either the
 * schema's result or an {@linkcode InputRequiredResult}.
 *
 * The manual multi-round-trip path: pass `{ allowInputRequired: true }` in the
 * request options so an `input_required` response is handed back to the
 * caller instead of being auto-fulfilled (or rejected), and wrap the result
 * schema with `withInputRequired()` so the returned value is typed and
 * validated correctly for both outcomes — `input_required` values pass
 * through as-is, complete results validate against the wrapped schema.
 */
declare function withInputRequired<S extends StandardSchemaV1>(schema: S): StandardSchemaV1<unknown, StandardSchemaV1.InferOutput<S> | InputRequiredResult>;
//#endregion
//#region ../core-internal/src/shared/inputRequiredDriver.d.ts
/**
 * Multi-round-trip driver options (`inputRequired` on the client options bag).
 */
interface InputRequiredOptions {
  /**
   * Fulfil `input_required` results automatically by dispatching the
   * embedded requests to the registered handlers and retrying.
   *
   * Set to `false` for manual mode: an `input_required` response then
   * surfaces as a typed error unless the individual call opts in with
   * `allowInputRequired: true` (and, for typed results on the explicit
   * schema path, `withInputRequired()`).
   *
   * @default true
   */
  autoFulfill?: boolean;
  /**
   * Maximum number of rounds (retries) the driver performs for a single
   * call before failing with a typed
   * {@linkcode SdkErrorCode.InputRequiredRoundsExceeded} error.
   *
   * @default 10
   */
  maxRounds?: number;
}
//#endregion
//#region ../core-internal/src/types/enums.d.ts
/**
 * Error codes for protocol errors that cross the wire as JSON-RPC error responses.
 * These follow the JSON-RPC specification and MCP-specific extensions.
 */
declare enum ProtocolErrorCode {
  ParseError = -32700,
  InvalidRequest = -32600,
  MethodNotFound = -32601,
  InvalidParams = -32602,
  InternalError = -32603,
  /**
   * Resource not found.
   *
   * Receive-tolerated only: the SDK never EMITS `-32002` — `resources/read`
   * misses answer `-32602` (Invalid Params) on every protocol revision per
   * the 2026-07-28 spec MUST, and a handler-thrown `-32002` is mapped to
   * `-32602` at the era encode seam. The member stays importable so clients
   * can recognise `-32002` from peers built on earlier SDK releases (the
   * spec's "clients SHOULD also accept `-32002`" backwards-compatibility
   * clause). Throw `ResourceNotFoundError` instead.
   */
  ResourceNotFound = -32002,
  /**
   * Processing the request requires a capability the client did not declare
   * in the request's `clientCapabilities` (protocol revision 2026-07-28).
   */
  MissingRequiredClientCapability = -32021,
  /**
   * The request's protocol version is unknown to the server or unsupported
   * by it (protocol revision 2026-07-28).
   */
  UnsupportedProtocolVersion = -32022,
  UrlElicitationRequired = -32042,
}
//#endregion
//#region ../core-internal/src/types/errors.d.ts
/**
 * Protocol errors are JSON-RPC errors that cross the wire as error responses.
 * They use numeric error codes from the {@linkcode ProtocolErrorCode} enum.
 *
 * `instanceof` on this class (and its subclasses) is brand-matched, so it works
 * across separately bundled copies of the SDK — e.g. an error constructed by
 * `@modelcontextprotocol/client` matches the class re-exported by
 * `@modelcontextprotocol/server` in the same process.
 */
declare class ProtocolError extends Error {
  readonly code: number;
  readonly data?: unknown | undefined;
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
  static isInstance<T$1 extends abstract new (...args: never[]) => unknown>(this: T$1, value: unknown): value is InstanceType<T$1>;
  constructor(code: number, message: string, data?: unknown | undefined);
  /**
   * Factory method to create the appropriate error type based on the error code and data
   */
  static fromError(code: number, message: string, data?: unknown): ProtocolError;
}
/**
 * Error type for a `resources/read` miss: the requested resource does not
 * exist. The wire code is `-32602` (Invalid Params) on every protocol
 * revision — the spec MUST for revision 2026-07-28, and the value the v1.x
 * SDK has always emitted on earlier revisions. The error data echoes the
 * requested URI.
 *
 * Recognise this error by checking `error.data` is exactly `{ uri: string }`
 * (a `-32602` whose data carries `uri` and nothing else is resource-not-found;
 * any other `-32602` is an ordinary Invalid Params). For backwards compatibility, clients should also
 * accept `-32002` as resource not found — earlier SDK builds emitted that
 * code, and {@linkcode ProtocolError.fromError} reconstructs this class for
 * either code **when `error.data` carries `uri`** (a bare `-32002` without
 * `data.uri` stays a generic {@linkcode ProtocolError}). `instanceof` checks
 * are brand-matched and work across separately bundled copies of the SDK.
 */
declare class ResourceNotFoundError extends ProtocolError {
  constructor(uri: string, message?: string);
  /** The URI that was requested and not found. */
  get uri(): string;
}
/**
 * Specialized error type when a tool requires a URL mode elicitation.
 * This makes it nicer for the client to handle since there is specific data to work with instead of just a code to check against.
 */
declare class UrlElicitationRequiredError extends ProtocolError {
  constructor(elicitations: ElicitRequestURLParams[], message?: string);
  get elicitations(): ElicitRequestURLParams[];
}
/**
 * Error type for the `-32022` UnsupportedProtocolVersion protocol error (protocol
 * revision 2026-07-28): the request's protocol version is unknown to the server or
 * unsupported by it.
 *
 * The error data lists the protocol versions the receiver supports (`supported`),
 * so the sender can choose a mutually supported version and retry, and echoes the
 * version that was requested (`requested`).
 */
declare class UnsupportedProtocolVersionError extends ProtocolError {
  constructor(data: UnsupportedProtocolVersionErrorData, message?: string);
  /**
   * Protocol versions the receiver supports.
   */
  get supported(): string[];
  /**
   * The protocol version that was requested.
   */
  get requested(): string;
}
/**
 * Error type for the `-32021` MissingRequiredClientCapability protocol error
 * (protocol revision 2026-07-28): processing the request requires a capability
 * the client did not declare in the request's `clientCapabilities`.
 *
 * The error data lists the missing capabilities (`requiredCapabilities`) in
 * the `ClientCapabilities` shape, so the client can see exactly what it would
 * have to declare for the request to be served. On HTTP, the response status
 * is `400 Bad Request`.
 *
 * Recognize this error by its code and `data.requiredCapabilities`, or by
 * `instanceof` — checks are brand-matched and work across separately bundled
 * copies of the SDK.
 */
declare class MissingRequiredClientCapabilityError extends ProtocolError {
  constructor(data: MissingRequiredClientCapabilityErrorData, message?: string);
  /**
   * The capabilities the server requires from the client to process the
   * request (only the missing capabilities are listed).
   */
  get requiredCapabilities(): ClientCapabilities;
}
//#endregion
//#region ../core-internal/src/types/guards.d.ts
/**
 * Validates and parses an unknown value as a JSON-RPC message.
 *
 * Use this to validate incoming messages in custom transport implementations.
 * Throws if the value does not conform to the JSON-RPC message schema.
 *
 * @param value - The value to validate (typically a parsed JSON object).
 * @returns The validated {@linkcode JSONRPCMessage}.
 * @throws If validation fails.
 */
declare function parseJSONRPCMessage(value: unknown): JSONRPCMessage;
declare const isJSONRPCRequest: (value: unknown) => value is JSONRPCRequest;
declare const isJSONRPCNotification: (value: unknown) => value is JSONRPCNotification;
/**
 * Checks if a value is a valid {@linkcode JSONRPCResultResponse}.
 * @param value - The value to check.
 *
 * @returns True if the value is a valid {@linkcode JSONRPCResultResponse}, false otherwise.
 */
declare const isJSONRPCResultResponse: (value: unknown) => value is JSONRPCResultResponse;
/**
 * Checks if a value is a valid {@linkcode JSONRPCErrorResponse}.
 * @param value - The value to check.
 *
 * @returns True if the value is a valid {@linkcode JSONRPCErrorResponse}, false otherwise.
 */
declare const isJSONRPCErrorResponse: (value: unknown) => value is JSONRPCErrorResponse;
/**
 * Checks if a value is a valid {@linkcode JSONRPCResponse} (either a result or error response).
 * @param value - The value to check.
 *
 * @returns True if the value is a valid {@linkcode JSONRPCResponse}, false otherwise.
 */
declare const isJSONRPCResponse: (value: unknown) => value is JSONRPCResponse;
/**
 * Checks if a value is a valid {@linkcode CallToolResult}.
 *
 * This is a consumer-side VALUE check against the neutral model, not a wire
 * validator: a raw wire object that additionally carries wire-only members
 * (e.g. `resultType`) still passes through the loose index signature. Use a
 * transport-level parse to validate raw wire traffic.
 *
 * @param value - The value to check.
 *
 * @returns True if the value is a valid {@linkcode CallToolResult}, false otherwise.
 */
declare const isCallToolResult: (value: unknown) => value is CallToolResult;
/**
 * Checks whether a value is an input-required result (protocol revision
 * 2026-07-28): the multi-round-trip return shape discriminated by
 * `resultType: 'input_required'`.
 *
 * This is a discriminator check, not a full validator — the at-least-one rule
 * (`inputRequests` or `requestState`) is enforced by the `inputRequired()`
 * builder and re-checked by the server seam for hand-built values.
 *
 * @param value - The value to check.
 * @returns True if the value carries the `input_required` discriminator.
 */
declare const isInputRequiredResult: (value: unknown) => value is InputRequiredResult;
/**
 * Checks if a value is a valid {@linkcode TaskAugmentedRequestParams}.
 * @param value - The value to check.
 *
 * @returns True if the value is a valid {@linkcode TaskAugmentedRequestParams}, false otherwise.
 *
 * @deprecated Recognizes 2025-11-25 task wire vocabulary, which has no SDK
 * runtime; kept importable for interoperability only.
 */
declare const isTaskAugmentedRequestParams: (value: unknown) => value is TaskAugmentedRequestParams;
declare const isInitializeRequest: (value: unknown) => value is InitializeRequest;
declare const isInitializedNotification: (value: unknown) => value is InitializedNotification;
declare function assertCompleteRequestPrompt(request: CompleteRequest): asserts request is CompleteRequestPrompt;
declare function assertCompleteRequestResourceTemplate(request: CompleteRequest): asserts request is CompleteRequestResourceTemplate;
//#endregion
//#region ../core-internal/src/types/specTypeSchema.d.ts
/**
 * Explicit allowlist of protocol Zod schemas that correspond to a public spec type in `types.ts`.
 *
 * This intentionally excludes internal helper schemas exported from `schemas.ts` that have no
 * matching public type (e.g. `ListChangedOptionsBaseSchema`, `BaseRequestParamsSchema`,
 * `NotificationsParamsSchema`, `ClientTasksCapabilitySchema`, `ServerTasksCapabilitySchema`).
 * Keeping the list explicit means new public spec types must be added here deliberately, and
 * internals never leak into `SpecTypeName`.
 *
 * `ResourceTemplateSchema` is included; its public type is exported as `ResourceTemplateType`
 * (the bare name collides with the server package's `ResourceTemplate` class), so
 * `SpecTypes['ResourceTemplate']` is structurally equal to `ResourceTemplateType` rather than to
 * a type literally named `ResourceTemplate`.
 */
declare const SPEC_SCHEMA_KEYS: readonly ["AnnotationsSchema", "AudioContentSchema", "BaseMetadataSchema", "BlobResourceContentsSchema", "BooleanSchemaSchema", "CallToolRequestSchema", "CallToolRequestParamsSchema", "CallToolResultSchema", "CancelledNotificationSchema", "CancelledNotificationParamsSchema", "CancelTaskRequestSchema", "CancelTaskResultSchema", "ClientCapabilitiesSchema", "ClientNotificationSchema", "ClientRequestSchema", "ClientResultSchema", "CompatibilityCallToolResultSchema", "CompleteRequestSchema", "CompleteRequestParamsSchema", "CompleteResultSchema", "ContentBlockSchema", "CreateMessageRequestSchema", "CreateMessageRequestParamsSchema", "CreateMessageResultSchema", "CreateMessageResultWithToolsSchema", "CreateTaskResultSchema", "CursorSchema", "DiscoverRequestSchema", "DiscoverResultSchema", "ElicitationCompleteNotificationSchema", "ElicitationCompleteNotificationParamsSchema", "ElicitRequestSchema", "ElicitRequestFormParamsSchema", "ElicitRequestParamsSchema", "ElicitRequestURLParamsSchema", "ElicitResultSchema", "EmbeddedResourceSchema", "EmptyResultSchema", "EnumSchemaSchema", "GetPromptRequestSchema", "GetPromptRequestParamsSchema", "GetPromptResultSchema", "GetTaskPayloadRequestSchema", "GetTaskPayloadResultSchema", "GetTaskRequestSchema", "GetTaskResultSchema", "IconSchema", "IconsSchema", "ImageContentSchema", "ImplementationSchema", "InitializedNotificationSchema", "InitializeRequestSchema", "InitializeRequestParamsSchema", "InitializeResultSchema", "JSONArraySchema", "JSONObjectSchema", "JSONRPCErrorResponseSchema", "JSONRPCMessageSchema", "JSONRPCNotificationSchema", "JSONRPCRequestSchema", "JSONRPCResponseSchema", "JSONRPCResultResponseSchema", "JSONValueSchema", "LegacyTitledEnumSchemaSchema", "ListPromptsRequestSchema", "ListPromptsResultSchema", "ListResourcesRequestSchema", "ListResourcesResultSchema", "ListResourceTemplatesRequestSchema", "ListResourceTemplatesResultSchema", "ListRootsRequestSchema", "ListRootsResultSchema", "ListTasksRequestSchema", "ListTasksResultSchema", "ListToolsRequestSchema", "ListToolsResultSchema", "LoggingLevelSchema", "LoggingMessageNotificationSchema", "LoggingMessageNotificationParamsSchema", "ModelHintSchema", "ModelPreferencesSchema", "MultiSelectEnumSchemaSchema", "NotificationSchema", "NumberSchemaSchema", "PaginatedRequestSchema", "PaginatedRequestParamsSchema", "PaginatedResultSchema", "PingRequestSchema", "PrimitiveSchemaDefinitionSchema", "ProgressSchema", "ProgressNotificationSchema", "ProgressNotificationParamsSchema", "ProgressTokenSchema", "PromptSchema", "PromptArgumentSchema", "PromptListChangedNotificationSchema", "PromptMessageSchema", "PromptReferenceSchema", "ReadResourceRequestSchema", "ReadResourceRequestParamsSchema", "ReadResourceResultSchema", "RelatedTaskMetadataSchema", "RequestSchema", "RequestIdSchema", "RequestMetaSchema", "ResourceSchema", "ResourceContentsSchema", "ResourceLinkSchema", "ResourceListChangedNotificationSchema", "ResourceRequestParamsSchema", "ResourceTemplateSchema", "ResourceTemplateReferenceSchema", "ResourceUpdatedNotificationSchema", "ResourceUpdatedNotificationParamsSchema", "ResultMetaObjectSchema", "ResultSchema", "RoleSchema", "RootSchema", "RootsListChangedNotificationSchema", "SamplingContentSchema", "SamplingMessageSchema", "SamplingMessageContentBlockSchema", "ServerCapabilitiesSchema", "ServerNotificationSchema", "ServerRequestSchema", "ServerResultSchema", "SetLevelRequestSchema", "SetLevelRequestParamsSchema", "SingleSelectEnumSchemaSchema", "StringSchemaSchema", "SubscribeRequestSchema", "SubscribeRequestParamsSchema", "SubscriptionFilterSchema", "SubscriptionsAcknowledgedNotificationSchema", "SubscriptionsAcknowledgedNotificationParamsSchema", "SubscriptionsListenRequestSchema", "SubscriptionsListenRequestParamsSchema", "SubscriptionsListenResultSchema", "SubscriptionsListenResultMetaSchema", "TaskAugmentedRequestParamsSchema", "TaskCreationParamsSchema", "TaskMetadataSchema", "TaskSchema", "TaskStatusSchema", "TaskStatusNotificationSchema", "TaskStatusNotificationParamsSchema", "TextContentSchema", "TextResourceContentsSchema", "TitledMultiSelectEnumSchemaSchema", "TitledSingleSelectEnumSchemaSchema", "ToolSchema", "ToolAnnotationsSchema", "ToolChoiceSchema", "ToolExecutionSchema", "ToolListChangedNotificationSchema", "ToolResultContentSchema", "ToolUseContentSchema", "UnsubscribeRequestSchema", "UnsubscribeRequestParamsSchema", "UntitledMultiSelectEnumSchemaSchema", "UntitledSingleSelectEnumSchemaSchema"];
declare const authSchemas: {
  readonly IdJagTokenExchangeResponseSchema: any;
  readonly OAuthClientInformationFullSchema: any;
  readonly OAuthClientInformationSchema: any;
  readonly OAuthClientMetadataSchema: any;
  readonly OAuthClientRegistrationErrorSchema: any;
  readonly OAuthErrorResponseSchema: any;
  readonly OAuthMetadataSchema: any;
  readonly OAuthProtectedResourceMetadataSchema: any;
  readonly OAuthTokenRevocationRequestSchema: any;
  readonly OAuthTokensSchema: any;
  readonly OpenIdProviderDiscoveryMetadataSchema: any;
  readonly OpenIdProviderMetadataSchema: any;
};
type ProtocolSchemaKey = (typeof SPEC_SCHEMA_KEYS)[number];
type AuthSchemaKey = keyof typeof authSchemas;
type SchemaKey = ProtocolSchemaKey | AuthSchemaKey;
type SchemaFor<K$1 extends SchemaKey> = K$1 extends ProtocolSchemaKey ? (typeof schemas_d_exports)[K$1] : K$1 extends AuthSchemaKey ? (typeof authSchemas)[K$1] : never;
type StripSchemaSuffix<K$1> = K$1 extends `${infer N}Schema` ? N : never;
/**
 * Union of every named type in the SDK's protocol and OAuth schemas (e.g. `'CallToolResult'`,
 * `'ContentBlock'`, `'Tool'`, `'OAuthTokens'`). Derived from the internal Zod schemas, so it stays
 * in sync with the spec.
 */
type SpecTypeName = StripSchemaSuffix<SchemaKey>;
/**
 * Maps each {@linkcode SpecTypeName} to its TypeScript type.
 *
 * `SpecTypes['Tool']` is equivalent to importing the `Tool` type directly.
 * These validators cover the NEUTRAL model — the consumer-facing shapes with
 * no wire-only members (`resultType`, the reserved `_meta` envelope keys).
 * Per-revision WIRE validators are deliberately not public surface; they are
 * planned to return as versioned `zod-schemas/<revision>` exports for
 * consumers who validate raw wire traffic themselves.
 */
type SpecTypes = { [K in SchemaKey as StripSchemaSuffix<K>]: SchemaFor<K> extends z.ZodType ? z.output<SchemaFor<K>> : never };
/**
 * Input shape for each {@linkcode SpecTypeName}. For most types this equals {@linkcode SpecTypes},
 * but a few schemas apply defaults/preprocessing, so the accepted input may be looser than the
 * resulting output type.
 */
type SpecTypeInputs = { [K in SchemaKey as StripSchemaSuffix<K>]: SchemaFor<K> extends z.ZodType ? z.input<SchemaFor<K>> : never };
type SchemaRecord = { readonly [K in SpecTypeName]: StandardSchemaV1Sync<SpecTypeInputs[K], SpecTypes[K]> };
type GuardRecord = { readonly [K in SpecTypeName]: (value: unknown) => value is SpecTypeInputs[K] };
/**
 * Runtime validators for every MCP spec type, keyed by type name.
 *
 * Use this when you need to validate a spec-defined shape at a boundary the SDK does not own, for
 * example an extension's custom-method payload that embeds a `CallToolResult`, or a value read from
 * storage that should be a `Tool`.
 *
 * Each entry implements the Standard Schema interface, so it composes with any
 * Standard-Schema-aware library. For a simple boolean check, use {@linkcode isSpecType} instead.
 *
 * @example
 * ```ts source="./specTypeSchema.examples.ts#specTypeSchemas_basicUsage"
 * const result = specTypeSchemas.CallToolResult['~standard'].validate(untrusted);
 * if (result.issues === undefined) {
 *     // result.value is CallToolResult
 * }
 * ```
 */
declare const specTypeSchemas: SchemaRecord;
/**
 * Type predicates for every MCP spec type, keyed by type name.
 *
 * Returns `true` if the value satisfies the schema's input type (`z.input<>`, before defaults and
 * transforms are applied), and narrows to that input type. For schemas with `.default()` or
 * `.preprocess()`, this may accept values that do not structurally match the named output type;
 * for example `isSpecType.CallToolResult({})` is `true` because `content` has a default. Use
 * `specTypeSchemas.X['~standard'].validate(value)` when you need the validated output value.
 *
 * Each guard is a standalone function, so it can be passed directly as a callback.
 *
 * @example
 * ```ts source="./specTypeSchema.examples.ts#isSpecType_basicUsage"
 * if (isSpecType.ContentBlock(value)) {
 *     // value is ContentBlock
 * }
 *
 * const blocks = mixed.filter(isSpecType.ContentBlock);
 * ```
 */
declare const isSpecType: GuardRecord;
//#endregion
//#region ../core-internal/src/shared/transport.d.ts
type FetchLike = (url: string | URL, init?: RequestInit) => Promise<Response>;
/**
 * Creates a fetch function that includes base `RequestInit` options.
 * This ensures requests inherit settings like credentials, mode, headers, etc. from the base init.
 *
 * @param baseFetch - The base fetch function to wrap (defaults to global `fetch`)
 * @param baseInit - The base `RequestInit` to merge with each request
 * @returns A wrapped fetch function that merges base options with call-specific options
 */
declare function createFetchWithInit(baseFetch?: FetchLike, baseInit?: RequestInit): FetchLike;
/**
 * Options for sending a JSON-RPC message.
 */
type TransportSendOptions = {
  /**
   * If present, `relatedRequestId` is used to indicate to the transport which incoming request to associate this outgoing message with.
   */
  relatedRequestId?: RequestId | undefined;
  /**
   * The resumption token used to continue long-running requests that were interrupted.
   *
   * This allows clients to reconnect and continue from where they left off, if supported by the transport.
   */
  resumptionToken?: string | undefined;
  /**
   * A callback that is invoked when the resumption token changes, if supported by the transport.
   *
   * This allows clients to persist the latest token for potential reconnection.
   */
  onresumptiontoken?: ((token: string) => void) | undefined;
  /**
   * An abort signal for THIS outbound message's underlying request, when the
   * transport sends one outbound message per underlying request (the
   * Streamable HTTP transport's POST-per-request model). Aborting it cancels
   * the underlying request (and its SSE response stream) without closing the
   * transport. Transports that share a single channel (stdio, in-memory)
   * ignore it.
   */
  requestSignal?: AbortSignal | undefined;
  /**
   * Fired by transports that open a per-request stream (the Streamable HTTP
   * transport's POST-per-request SSE response) when that stream ends or
   * errors for any reason OTHER than a deliberate `requestSignal` abort —
   * i.e. the server closed the stream, the network dropped it, or
   * reconnection was exhausted. Transports that share a single channel
   * (stdio, in-memory) ignore it.
   */
  onRequestStreamEnd?: (() => void) | undefined;
  /**
   * Additional HTTP headers to send with THIS outbound message, when the
   * transport sends one outbound message per underlying HTTP request (the
   * Streamable HTTP transport's POST-per-request model). Transports that
   * share a single channel (stdio, in-memory) ignore it.
   *
   * The Client uses this to attach SEP-2243 `Mcp-Param-{Name}` headers to a
   * `tools/call` request on a 2026-07-28 connection. Values are sent
   * verbatim — encode anything that is not a safe RFC 9110 field value
   * before passing it here.
   */
  headers?: Readonly<Record<string, string>> | undefined;
};
/**
 * Describes the minimal contract for an MCP transport that a client or server can communicate over.
 */
interface Transport {
  /**
   * Starts processing messages on the transport, including any connection steps that might need to be taken.
   *
   * This method should only be called after callbacks are installed, or else messages may be lost.
   *
   * NOTE: This method should not be called explicitly when using {@linkcode @modelcontextprotocol/client!client/client.Client | Client} or {@linkcode @modelcontextprotocol/server!server/server.Server | Server} classes, as they will implicitly call {@linkcode Transport.start | start()}.
   */
  start(): Promise<void>;
  /**
   * Sends a JSON-RPC message (request or response).
   *
   * If present, `relatedRequestId` is used to indicate to the transport which incoming request to associate this outgoing message with.
   */
  send(message: JSONRPCMessage, options?: TransportSendOptions): Promise<void>;
  /**
   * Closes the connection.
   */
  close(): Promise<void>;
  /**
   * `true` when this transport opens one underlying request per outbound
   * JSON-RPC request (the Streamable HTTP POST-per-request model) and
   * therefore honors {@linkcode TransportSendOptions.requestSignal}. The
   * 2026-07-28 spec makes closing that per-request stream the cancellation
   * signal — the protocol layer aborts `requestSignal` instead of POSTing
   * `notifications/cancelled` when this flag is set on a 2026-era
   * connection. Transports that share a single channel (stdio, in-memory)
   * leave it `undefined`.
   */
  readonly hasPerRequestStream?: boolean;
  /**
   * Callback for when the connection is closed for any reason.
   *
   * This should be invoked when {@linkcode Transport.close | close()} is called as well.
   */
  onclose?: (() => void) | undefined;
  /**
   * Callback for when an error occurs.
   *
   * Note that errors are not necessarily fatal; they are used for reporting any kind of exceptional condition out of band.
   */
  onerror?: ((error: Error) => void) | undefined;
  /**
   * Callback for when a message (request or response) is received over the connection.
   *
   * Includes the {@linkcode MessageExtraInfo.request | request} and {@linkcode MessageExtraInfo.authInfo | authInfo} if the transport is authenticated.
   *
   * The {@linkcode MessageExtraInfo.request | request} can be used to get the original request information (headers, etc.)
   */
  onmessage?: (<T$1 extends JSONRPCMessage>(message: T$1, extra?: MessageExtraInfo) => void) | undefined;
  /**
   * The session ID generated for this connection.
   */
  sessionId?: string | undefined;
  /**
   * Sets the protocol version used for the connection (called when the initialize response is received).
   */
  setProtocolVersion?: ((version: string) => void) | undefined;
  /**
   * Sets the supported protocol versions for header validation (called during connect).
   * This allows the server to pass its supported versions to the transport.
   */
  setSupportedProtocolVersions?: ((versions: string[]) => void) | undefined;
}
//#endregion
//#region ../core-internal/src/shared/protocol.d.ts
/**
 * Callback for progress notifications.
 */
type ProgressCallback = (progress: Progress) => void;
/**
 * Additional initialization options.
 */
type ProtocolOptions = {
  /**
   * Protocol versions supported. The legacy `initialize` handshake offers and
   * falls back to the first 2025-era entry in the list (the client sends it,
   * the server counter-offers it); 2026-era entries are only ever selected via
   * `server/discover`. Passed to transport during {@linkcode Protocol.connect | connect()}.
   *
   * @default {@linkcode SUPPORTED_PROTOCOL_VERSIONS}
   */
  supportedProtocolVersions?: string[];
  /**
   * Whether to restrict emitted requests to only those that the remote side has indicated that they can handle, through their advertised capabilities.
   *
   * Note that this DOES NOT affect checking of _local_ side capabilities, as it is considered a logic error to mis-specify those.
   *
   * Currently this defaults to `false`, for backwards compatibility with SDK versions that did not advertise capabilities correctly. In future, this will default to `true`.
   */
  enforceStrictCapabilities?: boolean;
  /**
   * An array of notification method names that should be automatically debounced.
   * Any notifications with a method in this list will be coalesced if they
   * occur in the same tick of the event loop.
   * e.g., `['notifications/tools/list_changed']`
   */
  debouncedNotificationMethods?: string[];
};
/**
 * The default request timeout, in milliseconds.
 */
declare const DEFAULT_REQUEST_TIMEOUT_MSEC = 60000;
/**
 * Options that can be given per request.
 */
type RequestOptions = {
  /**
   * If set, requests progress notifications from the remote end (if supported). When progress notifications are received, this callback will be invoked.
   */
  onprogress?: ProgressCallback;
  /**
   * Can be used to cancel an in-flight request. This will cause an `AbortError` to be raised from {@linkcode Protocol.request | request()}.
   */
  signal?: AbortSignal;
  /**
   * A timeout (in milliseconds) for this request. If exceeded, an {@linkcode SdkError} with code {@linkcode SdkErrorCode.RequestTimeout} will be raised from {@linkcode Protocol.request | request()}.
   *
   * If not specified, {@linkcode DEFAULT_REQUEST_TIMEOUT_MSEC} will be used as the timeout.
   */
  timeout?: number;
  /**
   * If `true`, receiving a progress notification will reset the request timeout.
   * This is useful for long-running operations that send periodic progress updates.
   * Default: `false`
   */
  resetTimeoutOnProgress?: boolean;
  /**
   * Maximum total time (in milliseconds) to wait for a response.
   * If exceeded, an {@linkcode SdkError} with code {@linkcode SdkErrorCode.RequestTimeout} will be raised, regardless of progress notifications.
   * If not specified, there is no maximum total timeout.
   *
   * For multi-round-trip requests fulfilled by the auto-fulfilment driver
   * (protocol revision 2026-07-28), the budget bounds the WHOLE flow: every
   * retry leg is given only the time remaining.
   */
  maxTotalTimeout?: number;
  /**
   * Manual multi-round-trip mode for this call (protocol revision
   * 2026-07-28): when the response is an `input_required` result, hand it
   * back to the caller instead of auto-fulfilling it (or raising a typed
   * error). The resolved value is the neutral input-required shape
   * (`resultType: 'input_required'`, `inputRequests?`, `requestState?`);
   * wrap the result schema with `withInputRequired()` on the explicit
   * schema path to type both outcomes. The caller is then responsible for
   * gathering the requested input and retrying the original request with
   * `inputResponses` / `requestState` params and a fresh request.
   *
   * Default: `false`.
   */
  allowInputRequired?: boolean;
} & TransportSendOptions;
/**
 * Flow context handed to {@linkcode Protocol._resolveNonCompleteResult}: the
 * originating request, its options, the wire codec that decoded the response,
 * the timestamp the originating leg was issued at (for whole-flow timeout
 * accounting), and a `retry` closure that re-enters the request funnel with
 * fresh params on a fresh request id.
 */
interface NonCompleteResultFlow<T$1 extends StandardSchemaV1 = StandardSchemaV1> {
  codec: WireCodec;
  request: Request;
  resultSchema: T$1;
  options: RequestOptions | undefined;
  flowStartedAt: number;
  /** Re-issue the originating request with the given params and per-leg options. */
  retry(params: Record<string, unknown> | undefined, legOptions: RequestOptions): Promise<unknown>;
}
/**
 * Options that can be given per notification.
 */
type NotificationOptions = {
  /**
   * May be used to indicate to the transport which incoming request to associate this outgoing notification with.
   */
  relatedRequestId?: RequestId;
};
/**
 * The type of `ctx.mcpReq.requestState`. The type parameter is
 * caller-asserted (no validation, like the two-argument `acceptedContent<T>`)
 * — pair with `createRequestStateCodec<T>` so the claim is backed by the
 * codec's verification.
 */
type RequestStateAccessor = <T$1 = unknown>() => T$1 | undefined;
/**
 * Base context provided to all request handlers.
 */
type BaseContext = {
  /**
   * The session ID from the transport, if available.
   */
  sessionId?: string;
  /**
   * Information about the MCP request being handled.
   */
  mcpReq: {
    /**
     * The JSON-RPC ID of the request being handled.
     */
    id: RequestId;
    /**
     * The method name of the request (e.g., 'tools/call', 'ping').
     */
    method: string;
    /**
     * Metadata from the original request, with the reserved
     * `io.modelcontextprotocol/*` envelope keys already lifted out
     * (readable via `ctx.mcpReq.envelope`).
     */
    _meta?: RequestMeta;
    /**
     * The per-request `_meta` envelope (protocol revision 2026-07-28):
     * the reserved `io.modelcontextprotocol/*` keys carried by the
     * request, lifted out of the `_meta` the handler sees. Surfaced as
     * received — `Partial` because only the keys the request actually
     * carried are present (envelope requiredness is enforced per request
     * at dispatch time, not by the lift); only present at all when the
     * request carried envelope keys.
     */
    envelope?: Partial<RequestMetaEnvelope>;
    /**
     * Multi-round-trip input responses carried by a retried request
     * (protocol revision 2026-07-28), lifted out of the params the
     * handler sees. Entries are the BARE response objects keyed by the
     * identifiers the server assigned in `inputRequests`; entries that do
     * not look like bare responses (e.g. a `{method, result}` wrapper)
     * are dropped and their keys recorded in `droppedInputResponseKeys`.
     *
     * The values arrive from the client and are NOT validated by the SDK
     * — treat them as untrusted input.
     */
    inputResponses?: Record<string, unknown>;
    /**
     * Keys of `inputResponses` entries the SDK dropped because they were
     * not bare response objects (for example the wrapped `{method,
     * result}` shape some peers emit). Surfaced so a handler can re-issue
     * the corresponding input request rather than hard-fail.
     */
    droppedInputResponseKeys?: string[];
    /**
     * Reads the multi-round-trip request state for the current round:
     * the value the configured `ServerOptions.requestState.verify` hook
     * resolved with (e.g. `createRequestStateCodec.verify`'s decoded
     * payload — `mint<T>`/`requestState<T>()` are the typed pair), the
     * raw wire string when no hook is configured, or `undefined` when
     * the round carried no state. The type parameter is a compile-time
     * cast only.
     *
     * SECURITY: `requestState` round-trips through the client and MUST
     * be treated as attacker-controlled input. The SDK applies no
     * integrity protection by default — servers whose state influences
     * authorization or business logic MUST integrity-protect it and
     * verify via the `requestState.verify` hook (spec:
     * basic/patterns/mrtr, server requirements 4–5).
     */
    requestState: RequestStateAccessor;
    /**
     * An abort signal used to communicate if the request was cancelled from the sender's side.
     */
    signal: AbortSignal;
    /**
     * Sends a request that relates to the current request being handled.
     *
     * This is used by certain transports to correctly associate related messages.
     *
     * For spec methods the result type is inferred from the method name.
     * For custom (non-spec) methods, pass a result schema as the second argument.
     */
    send: {
      <M$1 extends RequestMethod>(request: {
        method: M$1;
        params?: Record<string, unknown>;
      }, options?: RequestOptions): Promise<ResultTypeMap[M$1]>;
      <T$1 extends StandardSchemaV1>(request: Request, resultSchema: T$1, options?: RequestOptions): Promise<StandardSchemaV1.InferOutput<T$1>>;
    };
    /**
     * Sends a notification that relates to the current request being handled.
     *
     * This is used by certain transports to correctly associate related messages.
     */
    notify: (notification: Notification) => Promise<void>;
  };
  /**
   * HTTP transport information, only available when using an HTTP-based transport.
   */
  http?: {
    /**
     * Information about a validated access token, provided to request handlers.
     */
    authInfo?: AuthInfo;
  };
};
/**
 * Context provided to server-side request handlers, extending {@linkcode BaseContext} with server-specific fields.
 */
type ServerContext = BaseContext & {
  mcpReq: {
    /**
     * Send a log message notification to the client.
     * Respects the client's log level filter set via logging/setLevel.
     *
     * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577).
     * Remains functional during the deprecation window (at least twelve months).
     * Migrate to stderr logging (STDIO servers) or OpenTelemetry.
     */
    log: (level: LoggingLevel, data: unknown, logger?: string) => Promise<void>;
    /**
     * Send an elicitation request to the client, requesting user input.
     *
     * @deprecated Throws on a 2026-07-28-era request — return `inputRequired(...)`
     * (multi-round-trip) from the handler instead. The 2025 push-style server-to-client request model is
     * replaced by input_required results in the 2026-07-28 protocol. If your factory serves
     * both eras, this only works on the legacy path.
     */
    elicitInput: (params: ElicitRequestFormParams | ElicitRequestURLParams, options?: RequestOptions) => Promise<ElicitResult>;
    /**
     * Request LLM sampling from the client.
     *
     * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577).
     * Throws on a 2026-07-28-era request — return `inputRequired(...)` (multi-round-trip)
     * from the handler instead, or migrate to calling LLM provider APIs directly. The 2025 push-style
     * server-to-client request model is replaced by input_required results in the 2026-07-28
     * protocol. If your factory serves both eras, this only works on the legacy path.
     */
    requestSampling: (params: CreateMessageRequest['params'], options?: RequestOptions) => Promise<CreateMessageResult | CreateMessageResultWithTools>;
  };
  http?: {
    /**
     * The original HTTP request.
     */
    req?: globalThis.Request;
    /**
     * Closes the SSE stream for this request, triggering client reconnection.
     * Only available when using a StreamableHTTPServerTransport with eventStore configured.
     */
    closeSSE?: () => void;
    /**
     * Closes the standalone GET SSE stream, triggering client reconnection.
     * Only available when using a StreamableHTTPServerTransport with eventStore configured.
     */
    closeStandaloneSSE?: () => void;
  };
};
/**
 * Context provided to client-side request handlers.
 */
type ClientContext = BaseContext;
/**
 * Implements MCP protocol framing on top of a pluggable transport, including
 * features like request/response linking, notifications, and progress.
 *
 * `Protocol` is abstract; `Client` and `Server` are the concrete role-specific
 * implementations most code should use.
 */
declare abstract class Protocol<ContextT extends BaseContext> {
  private _options?;
  private _transport?;
  private _requestMessageId;
  private _requestHandlers;
  private _requestHandlerAbortControllers;
  private _notificationHandlers;
  private _responseHandlers;
  private _progressHandlers;
  private _timeoutInfo;
  private _pendingDebouncedNotifications;
  /**
   * The protocol version negotiated for the current connection (`undefined`
   * before negotiation completes), which determines the wire era this
   * instance speaks. Set by the SDK's negotiation and initialize paths
   * (`Client.connect`, `Server._oninitialize`).
   */
  protected _negotiatedProtocolVersion?: string;
  protected _supportedProtocolVersions: string[];
  /**
   * Callback for when the connection is closed for any reason.
   *
   * This is invoked when {@linkcode Protocol.close | close()} is called as well.
   */
  onclose?: () => void;
  /**
   * Callback for when an error occurs.
   *
   * Note that errors are not necessarily fatal; they are used for reporting any kind of exceptional condition out of band.
   */
  onerror?: (error: Error) => void;
  /**
   * A handler to invoke for any request types that do not have their own handler installed.
   */
  fallbackRequestHandler?: (request: JSONRPCRequest, ctx: ContextT) => Promise<Result$1>;
  /**
   * A handler to invoke for any notification types that do not have their own handler installed.
   */
  fallbackNotificationHandler?: (notification: Notification) => Promise<void>;
  constructor(_options?: ProtocolOptions | undefined);
  /**
   * Builds the context object for request handlers. Subclasses must override
   * to return the appropriate context type (e.g., ServerContext adds HTTP request info).
   */
  protected abstract buildContext(ctx: BaseContext, transportInfo?: MessageExtraInfo): ContextT;
  /**
   * Drop consult for inbound messages whose transport did not classify them
   * at the edge — long-lived channels such as stdio, where a role class may
   * need to decline traffic the negotiated era has no answer for (the
   * client-side inbound-request drop on modern-era connections: the
   * 2026-07-28 era has no server→client request channel, and on stdio the
   * client must never write JSON-RPC responses).
   *
   * Consulted ONLY when the transport supplied no
   * {@linkcode MessageExtraInfo.classification}: edge-classified traffic
   * never reaches the hook. Returning `'drop'` discards the message without
   * writing any response (requests are surfaced via `onerror`). The base
   * implementation returns `undefined`: unclassified traffic keeps today's
   * dispatch path unchanged. Era selection never happens here — era is
   * instance state, owned by the serving entry that constructed and
   * connected the instance.
   */
  protected _shouldDropInbound(_message: JSONRPCRequest | JSONRPCNotification): 'drop' | undefined;
  /**
   * The per-request `_meta` envelope this instance attaches to every outgoing
   * request and notification, when one applies. The base implementation
   * returns `undefined` (no envelope — the 2025-era posture, so legacy-era
   * outbound traffic is byte-identical to a build without this seam).
   * `Client` overrides it on a connection that negotiated a modern (2026-07-28+)
   * era to return the reserved protocol-version / client-info /
   * client-capabilities keys. User-supplied `_meta` keys take precedence over
   * the auto-attached ones.
   */
  protected _outboundMetaEnvelope(): Readonly<Record<string, unknown>> | undefined;
  /**
   * Attach this instance's outbound `_meta` envelope (when one is configured)
   * to a request or notification. A no-op when the seam returns `undefined`
   * — the message returns by reference, so the legacy-era wire stays
   * byte-identical. User-supplied `_meta` keys are spread last so they win
   * over the auto-attached envelope keys.
   */
  private _envelopeOutbound;
  /**
   * Extension point for non-`complete` decoded results in the response
   * funnel: a result the wire codec discriminated into a kind other than
   * `'complete'` or `'invalid'` is handed here for the role class to
   * resolve. The base default surfaces it as a typed
   * {@linkcode SdkErrorCode.UnsupportedResultType} error (no retry).
   *
   * Intended consumers (named so the seam stays accountable):
   * - the `Client`'s multi-round-trip auto-fulfilment engine, which fulfils
   *   `'input_required'` results through the registered
   *   elicitation/sampling/roots handlers and retries via `flow.retry`;
   * - a future client-side terminal-result handler for
   *   `subscriptions/listen`, when the spec defines one.
   *
   * `Server` instances never receive `input_required` responses on their
   * outbound legs and leave the base behavior in place.
   */
  protected _resolveNonCompleteResult<T$1 extends StandardSchemaV1>(decoded: ReturnType<WireCodec['decodeResult']> & {
    kind: 'input_required';
  }, flow: NonCompleteResultFlow<T$1>): Promise<unknown>;
  /**
   * Protected accessor for a registered request handler. Used by role
   * classes that dispatch synthesized requests through the same stored
   * handler chain (e.g. the `Client` fulfilling an embedded multi-round-trip
   * input request).
   */
  protected _getRequestHandler(method: string): ((request: JSONRPCRequest, ctx: ContextT) => Promise<Result$1>) | undefined;
  private _oncancel;
  private _setupTimeout;
  private _resetTimeout;
  private _cleanupTimeout;
  /**
   * Attaches to the given transport, starts it, and starts listening for messages.
   *
   * The caller assumes ownership of the {@linkcode Transport}, replacing any callbacks that have already been set, and expects that it is the only user of the {@linkcode Transport} instance going forward.
   */
  connect(transport: Transport): Promise<void>;
  /**
   * Transport-close hook. Subclass overrides MUST call `super._onclose()`
   * after their own cleanup — base teardown (response-handler settlement,
   * timeout clearing, in-flight request abort) does not run otherwise.
   */
  protected _onclose(): void;
  private _onerror;
  /**
   * Inbound-notification dispatch. Subclass overrides MUST delegate
   * unmatched traffic to `super._onnotification(rawNotification, extra)` —
   * an override that consumes only what it owns and falls through to base
   * dispatch for everything else.
   */
  protected _onnotification(rawNotification: JSONRPCNotification, extra?: MessageExtraInfo): void;
  private _onrequest;
  private _onprogress;
  /**
   * Inbound-response dispatch. Subclass overrides MUST delegate unmatched
   * traffic to `super._onresponse(response)` — an override that consumes
   * only what it owns and falls through to base dispatch for everything
   * else.
   */
  protected _onresponse(response: JSONRPCResponse | JSONRPCErrorResponse): void;
  get transport(): Transport | undefined;
  /**
   * Closes the connection.
   */
  close(): Promise<void>;
  /**
   * A method to check if a capability is supported by the remote side, for the given method to be called.
   *
   * This should be implemented by subclasses.
   */
  protected abstract assertCapabilityForMethod(method: RequestMethod | string): void;
  /**
   * A method to check if a notification is supported by the local side, for the given method to be sent.
   *
   * This should be implemented by subclasses.
   */
  protected abstract assertNotificationCapability(method: NotificationMethod | string): void;
  /**
   * A method to check if a request handler is supported by the local side, for the given method to be handled.
   *
   * This should be implemented by subclasses.
   */
  protected abstract assertRequestHandlerCapability(method: string): void;
  /**
   * Sends a request and waits for a response.
   *
   * For spec methods the result schema is resolved automatically from the method name
   * and the return type is method-keyed. For custom (non-spec) methods, pass a
   * `resultSchema` as the second argument; the response is validated against it and
   * the return type is inferred from the schema.
   *
   * Do not use this method to emit notifications! Use {@linkcode Protocol.notification | notification()} instead.
   */
  request<M$1 extends RequestMethod>(request: {
    method: M$1;
    params?: Record<string, unknown>;
  }, options?: RequestOptions): Promise<ResultTypeMap[M$1]>;
  request<T$1 extends StandardSchemaV1>(request: Request, resultSchema: T$1, options?: RequestOptions): Promise<StandardSchemaV1.InferOutput<T$1>>;
  /**
   * The wire codec for this instance's negotiated era — the phase-2 truth:
   * everything an established connection sends and receives resolves
   * through it. Legacy until a version has been negotiated.
   */
  private _negotiatedWireCodec;
  /**
   * Protected accessor for the instance's negotiated wire codec, for role
   * classes (Client/Server/McpServer) routing era-dependent behavior
   * through the codec's function-only surface — `samplingResultVariant`,
   * `outboundEnvelope`, `projectCallToolResult` — instead of branching on
   * the protocol version themselves.
   */
  protected _wireCodec(): WireCodec;
  /**
   * Outbound codec resolution: while the negotiated version is still unset
   * (the negotiation window), lifecycle messages are bootstrap-pinned BY
   * METHOD — they self-identify their era (`initialize` IS the legacy
   * handshake, `server/discover` IS the modern probe). Once a version has
   * been negotiated, the instance era is authoritative for everything — a
   * negotiated session never re-routes a method onto the other era.
   */
  private _resolveOutboundCodec;
  /**
   * Era gate for outbound requests — deletions are physical in BOTH
   * directions: sending a spec method that the resolved era does not define
   * dies locally with a typed error before anything reaches the transport.
   * Methods outside the spec universe are consumer-owned extension methods
   * and stay era-blind.
   */
  private _assertOutboundRequestInEra;
  /**
   * Sends a request and waits for a response, using the provided schema for
   * validation instead of the era registry's method-keyed entry.
   *
   * This is the internal implementation used by SDK methods whose result
   * schema cannot be expressed as a method-keyed registry entry — the one
   * surviving case is `server.createMessage`, whose result schema depends
   * on the REQUEST params (tools vs no tools) — and by callers passing
   * explicit compatibility schemas. Spec methods are still era-gated here:
   * an explicit schema never smuggles a deleted method onto the wire.
   */
  protected _requestWithSchema<T$1 extends StandardSchemaV1>(request: Request, resultSchema: T$1, options?: RequestOptions): Promise<StandardSchemaV1.InferOutput<T$1>>;
  /**
   * The request funnel proper, keyed by the resolved era codec: the codec
   * owns result decoding (raw-first `resultType` discrimination — V-1 —
   * and the era's lift posture) before the schema validation step.
   */
  private _requestWithSchemaViaCodec;
  /**
   * Emits a notification, which is a one-way message that does not expect a response.
   */
  notification(notification: Notification, options?: NotificationOptions): Promise<void>;
  /**
   * The notification funnel proper, keyed by the resolved era codec —
   * direct sends and related notifications (`ctx.mcpReq.notify`) alike
   * resolve through the instance's negotiated era at send time.
   */
  private _notificationViaCodec;
  /**
   * Registers a handler to invoke when this protocol object receives a request with the given method.
   *
   * Note that this will replace any previous request handler for the same method.
   *
   * For spec methods, pass `(method, handler)`; the request is parsed with the spec
   * schema and the handler receives the typed `Request`. For custom (non-spec)
   * methods, pass `(method, schemas, handler)`; `params` are validated against
   * `schemas.params` and the handler receives the parsed params object directly.
   * Supplying `schemas.result` types the handler's return value.
   *
   * @example Custom request method
   * ```ts source="./protocol.examples.ts#Protocol_setRequestHandler_customMethod"
   * const SearchParams = z.object({ query: z.string(), limit: z.number().optional() });
   * const SearchResult = z.object({ hits: z.array(z.string()) });
   *
   * protocol.setRequestHandler('acme/search', { params: SearchParams, result: SearchResult }, async (params, _ctx) => {
   *     return { hits: [`result for ${params.query}`] };
   * });
   * ```
   */
  setRequestHandler<M$1 extends RequestMethod>(method: M$1, handler: (request: RequestTypeMap[M$1], ctx: ContextT) => HandlerResultTypeMap[M$1] | Promise<HandlerResultTypeMap[M$1]>): void;
  setRequestHandler<P extends StandardSchemaV1, R extends StandardSchemaV1 | undefined = undefined>(method: string, schemas: {
    params: P;
    result?: R;
  }, handler: (params: StandardSchemaV1.InferOutput<P>, ctx: ContextT) => InferHandlerResult<R> | Promise<InferHandlerResult<R>>): void;
  /**
   * Hook for subclasses to wrap a registered request handler with role-specific
   * validation or behavior (e.g. `Server` validates `tools/call` results, `Client`
   * validates `elicitation/create` mode and result). Runs for both the 2-arg and
   * 3-arg registration paths. The default implementation is identity.
   *
   * Subclasses overriding this hook avoid redeclaring `setRequestHandler`'s overload set.
   */
  protected _wrapHandler(_method: string, handler: (request: JSONRPCRequest, ctx: ContextT) => Promise<Result$1>): (request: JSONRPCRequest, ctx: ContextT) => Promise<Result$1>;
  /**
   * Hook for subclasses to supply the implementation identity the 2026-era
   * encode seam stamps into outbound result `_meta` under
   * `io.modelcontextprotocol/serverInfo` (spec PR #3002: servers SHOULD
   * identify themselves on every response). The default is `undefined` — no
   * stamp. Only `Server` overrides this: the key identifies the software
   * producing a response, and the 2025-era codec never stamps anything
   * regardless (the never-stamp guarantee).
   */
  protected _outboundServerInfo(): Implementation | undefined;
  /**
   * Removes the request handler for the given method.
   */
  removeRequestHandler(method: RequestMethod | string): void;
  /**
   * Asserts that a request handler has not already been set for the given method, in preparation for a new one being automatically installed.
   */
  assertCanSetRequestHandler(method: RequestMethod | string): void;
  /**
   * Registers a handler to invoke when this protocol object receives a notification with the given method.
   *
   * Note that this will replace any previous notification handler for the same method.
   *
   * For spec methods, pass `(method, handler)`; the notification is parsed with the
   * spec schema. For custom (non-spec) methods, pass `(method, schemas, handler)`;
   * `params` are validated against `schemas.params` and the handler receives the
   * parsed params object directly. The raw notification is passed as the second
   * argument; `_meta` is recoverable via `notification.params?._meta` (minus the
   * reserved `io.modelcontextprotocol/*` envelope keys, which the protocol layer
   * lifts out before dispatch).
   */
  setNotificationHandler<M$1 extends NotificationMethod>(method: M$1, handler: (notification: NotificationTypeMap[M$1]) => void | Promise<void>): void;
  setNotificationHandler<P extends StandardSchemaV1>(method: string, schemas: {
    params: P;
  }, handler: (params: StandardSchemaV1.InferOutput<P>, notification: Notification) => void | Promise<void>): void;
  /**
   * Removes the notification handler for the given method.
   */
  removeNotificationHandler(method: NotificationMethod | string): void;
}
/**
 * Schema bundle accepted by {@linkcode Protocol.setRequestHandler | setRequestHandler}'s 3-arg form.
 *
 * `params` is required and validates the inbound `request.params`. `result` is optional;
 * when supplied it types the handler's return value (no runtime validation is performed
 * on the result).
 */
interface RequestHandlerSchemas<P extends StandardSchemaV1 = StandardSchemaV1, R extends StandardSchemaV1 | undefined = StandardSchemaV1 | undefined> {
  params: P;
  result?: R;
}
type InferHandlerResult<R extends StandardSchemaV1 | undefined> = R extends StandardSchemaV1 ? StandardSchemaV1.InferOutput<R> : Result$1;
declare function mergeCapabilities(base: ServerCapabilities, additional: Partial<ServerCapabilities>): ServerCapabilities;
declare function mergeCapabilities(base: ClientCapabilities, additional: Partial<ClientCapabilities>): ClientCapabilities;
//#endregion
//#region ../core-internal/src/shared/mediaType.d.ts
/**
 * Whether a raw `Content-Type` header value denotes `application/json`.
 * Parameters (for example `charset=utf-8`) are allowed and ignored; malformed
 * parameter sections do not reject a header whose media type is unambiguously
 * `application/json` (see `mediaTypeEssence` for the exact grammar).
 */
declare function isJsonContentType(header: string | null | undefined): boolean;
//#endregion
//#region ../core-internal/src/shared/metadataUtils.d.ts
/**
 * Utilities for working with {@linkcode BaseMetadata} objects.
 */
/**
 * Gets the display name for an object with {@linkcode BaseMetadata}.
 * For tools, the precedence is: `title` → {@linkcode index.ToolAnnotations | annotations}.`title` → `name`
 * For other objects: `title` → `name`
 * This implements the spec requirement: "if no title is provided, name should be used for display purposes"
 */
declare function getDisplayName(metadata: BaseMetadata | (BaseMetadata & {
  annotations?: {
    title?: string;
  };
})): string;
//#endregion
//#region ../core-internal/src/shared/protocolEras.d.ts
/**
 * Protocol-era helpers (pure module). The MCP wire protocol splits into two eras:
 * legacy (the 2025-11-25 family and earlier; the version is negotiated via the
 * `initialize` handshake) and modern (2026-07-28 and later; no `initialize` —
 * servers advertise versions via `server/discover` and every request carries a
 * `_meta` envelope).
 *
 * An operation that belongs to one era must only ever consult that era's subset
 * of a supported-versions list: `initialize` never accepts or counter-offers a
 * modern revision, and the `server/discover` advertisement only ever contains
 * modern revisions.
 */
/**
 * The protocol era of a connection: `'legacy'` for the 2025-11-25 family and
 * earlier (negotiated via `initialize`), `'modern'` for 2026-07-28 and later
 * (negotiated via `server/discover`; every request carries a `_meta` envelope).
 */
type ProtocolEra = 'legacy' | 'modern';
//#endregion
//#region ../core-internal/src/shared/stdio.d.ts
declare const STDIO_DEFAULT_MAX_BUFFER_SIZE: number;
/**
 * Buffers a continuous stdio stream into discrete JSON-RPC messages.
 */
declare class ReadBuffer {
  private _buffer?;
  private _maxBufferSize;
  constructor(options?: {
    maxBufferSize?: number;
  });
  append(chunk: Buffer): void;
  readMessage(): JSONRPCMessage | null;
  clear(): void;
}
declare function deserializeMessage(line: string): JSONRPCMessage;
declare function serializeMessage(message: JSONRPCMessage): string;
//#endregion
//#region ../core-internal/src/shared/uriTemplate.d.ts
type Variables = Record<string, string | string[]>;
declare class UriTemplate {
  /**
   * Returns true if the given string contains any URI template expressions.
   * A template expression is a sequence of characters enclosed in curly braces,
   * like `{foo}` or `{?bar}`.
   */
  static isTemplate(str: string): boolean;
  private static validateLength;
  private readonly template;
  private readonly parts;
  get variableNames(): string[];
  constructor(template: string);
  toString(): string;
  private parse;
  private getOperator;
  private getNames;
  private encodeValue;
  private expandPart;
  expand(variables: Variables): string;
  private escapeRegExp;
  private partToRegExp;
  match(uri: string): Variables | null;
}
//#endregion
//#region ../core-internal/src/util/inMemory.d.ts
/**
 * In-memory transport for creating clients and servers that talk to each other within the same process.
 *
 * Intended for testing and development. For production in-process connections, use
 * `StreamableHTTPClientTransport` against a local server URL.
 */
declare class InMemoryTransport implements Transport {
  private _otherTransport?;
  private _messageQueue;
  private _closed;
  onclose?: () => void;
  onerror?: (error: Error) => void;
  onmessage?: (message: JSONRPCMessage, extra?: {
    authInfo?: AuthInfo;
  }) => void;
  sessionId?: string;
  /**
   * Creates a pair of linked in-memory transports that can communicate with each other. One should be passed to a {@linkcode @modelcontextprotocol/client!client/client.Client | Client} and one to a {@linkcode @modelcontextprotocol/server!server/server.Server | Server}.
   */
  static createLinkedPair(): [InMemoryTransport, InMemoryTransport];
  start(): Promise<void>;
  close(): Promise<void>;
  /**
   * Sends a message with optional auth info.
   * This is useful for testing authentication scenarios.
   */
  send(message: JSONRPCMessage, options?: {
    relatedRequestId?: RequestId;
    authInfo?: AuthInfo;
  }): Promise<void>;
}
//#endregion
//#region ../core-internal/src/wire/preload.d.ts
/**
 * Eagerly builds every lazily-constructed wire-schema layer, so that no later
 * validation pays schema-construction cost.
 *
 * Synchronous and idempotent: every layer is a memo, so the first call does
 * all the work and subsequent calls return immediately. Reference identity is
 * unaffected — this forces the same memos every lazy consumer pulls through.
 *
 * Call it at module scope on platforms that bill per-request CPU but not
 * module evaluation (isolate-based edge/serverless runtimes), where deferring
 * construction would move it into the first request of every fresh isolate:
 *
 * ```ts
 * // from '@modelcontextprotocol/server' or '@modelcontextprotocol/client' —
 * // each package bundles its own schema copy, so warm the one(s) you import.
 * preloadSchemas(); // module scope — runs during isolate warm-up
 * ```
 *
 * On Node CLIs and other process-per-invocation runtimes, prefer the lazy
 * default — there, module-scope construction is pure added boot latency.
 */
declare function preloadSchemas(): void;
//#endregion
export { withInputRequired as $, TRACESTATE_META_KEY as $i, Notification as $n, SetLevelRequest as $r, GetTaskResult as $t, SpecTypes as A, UnsubscribeRequest as Ai, ListPromptsRequest as An, RequestTypeMap as Ar, CreateMessageRequestParamsWithTools as At, isJSONRPCNotification as B, INVALID_PARAMS as Bi, ListToolsRequest as Bn, Result$1 as Br, ElicitRequestURLParams as Bt, ServerContext as C, StoredOAuthTokens as Ca, Tool as Ci, JSONRPCResponse as Cn, Request as Cr, CompleteRequestPrompt as Ct, TransportSendOptions as D, ToolListChangedNotification as Di, ListChangedCallback as Dn, RequestMetaObject as Dr, CreateMessageRequest as Dt, Transport as E, ToolExecution as Ei, LegacyTitledEnumSchema as En, RequestMetaEnvelope as Er, ContentBlock as Et, isCallToolResult as F, BAGGAGE_META_KEY as Fi, ListResourcesResult as Fn, ResourceRequestParams as Fr, DiscoverRequest as Ft, parseJSONRPCMessage as G, METHOD_NOT_FOUND as Gi, MessageClassification as Gn, RootsListChangedNotification as Gr, EmptyResult as Gt, isJSONRPCResponse as H, JSONRPC_VERSION as Hi, LoggingLevel as Hn, ResultTypeMap as Hr, ElicitationCompleteNotification as Ht, isInitializeRequest as I, CLIENT_CAPABILITIES_META_KEY as Ii, ListRootsRequest as In, ResourceTemplateReference as Ir, DiscoverResult as It, ResourceNotFoundError as J, RELATED_TASK_META_KEY as Ji, MethodNotFoundError as Jn, SamplingMessageContentBlock as Jr, GetPromptRequestParams as Jt, MissingRequiredClientCapabilityError as K, PARSE_ERROR as Ki, MessageExtraInfo as Kn, SamplingContent as Kr, EnumSchema as Kt, isInitializedNotification as L, CLIENT_INFO_META_KEY as Li, ListRootsResult as Ln, ResourceTemplateType as Lr, ElicitRequest as Lt, specTypeSchemas as M, UnsupportedProtocolVersionErrorData as Mi, ListResourceTemplatesRequest as Mn, ResourceContents as Mr, CreateMessageResultWithTools as Mt, assertCompleteRequestPrompt as N, UntitledMultiSelectEnumSchema as Ni, ListResourceTemplatesResult as Nn, ResourceLink as Nr, CreateTaskResult as Nt, createFetchWithInit as O, ToolResultContent as Oi, ListChangedHandlers as On, RequestMethod as Or, CreateMessageRequestParams as Ot, assertCompleteRequestResourceTemplate as P, UntitledSingleSelectEnumSchema as Pi, ListResourcesRequest as Pn, ResourceListChangedNotification as Pr, Cursor as Pt, InputRequiredOptions as Q, TRACEPARENT_META_KEY as Qi, MultiSelectEnumSchema as Qn, ServerResult as Qr, GetTaskRequest as Qt, isInputRequiredResult as R, DEFAULT_NEGOTIATED_PROTOCOL_VERSION as Ri, ListTasksRequest as Rn, ResourceUpdatedNotification as Rr, ElicitRequestFormParams as Rt, RequestStateAccessor as S, StoredOAuthClientInformation as Sa, TitledSingleSelectEnumSchema as Si, JSONRPCRequest as Sn, RelatedTaskMetadata as Sr, CompleteRequestParams as St, FetchLike as T, ToolChoice as Ti, JSONValue as Tn, RequestMeta as Tr, CompleteResult as Tt, isJSONRPCResultResponse as U, LATEST_PROTOCOL_VERSION as Ui, LoggingMessageNotification as Un, Role as Ur, ElicitationCompleteNotificationParams as Ut, isJSONRPCRequest as V, INVALID_REQUEST as Vi, ListToolsResult as Vn, ResultMetaObject as Vr, ElicitResult as Vt, isTaskAugmentedRequestParams as W, LOG_LEVEL_META_KEY as Wi, LoggingMessageNotificationParams as Wn, Root as Wr, EmbeddedResource as Wt, UrlElicitationRequiredError as X, SUBSCRIPTION_ID_META_KEY as Xi, ModelHint as Xn, ServerNotification as Xr, GetTaskPayloadRequest as Xt, UnsupportedProtocolVersionError as Y, SERVER_INFO_META_KEY as Yi, MissingRequiredClientCapabilityErrorData as Yn, ServerCapabilities as Yr, GetPromptResult as Yt, ProtocolErrorCode as Z, SUPPORTED_PROTOCOL_VERSIONS as Zi, ModelPreferences as Zn, ServerRequest as Zr, GetTaskPayloadResult as Zt, ProgressCallback as _, OAuthProtectedResourceMetadata as _a, TaskStatusNotification as _i, JSONArray as _n, PromptMessage as _r, ClientNotification as _t, ReadBuffer as a, SdkHttpErrorData as aa, SubscriptionFilter as ai, InitializeRequest as an, PaginatedRequestParams as ar, AuthInfo as at, RequestHandlerSchemas as b, OpenIdProviderDiscoveryMetadata as ba, TextResourceContents as bi, JSONRPCMessage as bn, ReadResourceRequestParams as br, CompatibilityCallToolResult as bt, serializeMessage as c, AuthorizationServerMetadata as ca, SubscriptionsListenRequest as ci, InitializedNotification as cn, PingRequest as cr, BooleanSchema as ct, isJsonContentType as d, OAuthClientInformationFull as da, SubscriptionsListenResultMeta as di, InputRequiredResult as dn, ProgressNotification as dr, CallToolResult as dt, checkResourceAllowed as ea, SetLevelRequestParams as ei, HandlerResultTypeMap as en, NotificationMethod as er, StandardSchemaV1 as et, BaseContext as f, OAuthClientInformationMixed as fa, Task as fi, InputResponse as fn, ProgressNotificationParams as fr, CancelTaskRequest as ft, NotificationOptions as g, OAuthMetadata as ga, TaskStatus as gi, InvalidRequestError as gn, PromptListChangedNotification as gr, ClientCapabilities as gt, NonCompleteResultFlow as h, OAuthErrorResponse as ha, TaskMetadata as hi, InvalidParamsError as hn, PromptArgument as hr, CancelledNotificationParams as ht, Variables as i, SdkHttpError as ia, SubscribeRequestParams as ii, Implementation as in, PaginatedRequest as ir, AudioContent as it, isSpecType as j, UnsubscribeRequestParams as ji, ListPromptsResult as jn, Resource as jr, CreateMessageResult as jt, SpecTypeName as k, ToolUseContent as ki, ListChangedOptions as kn, RequestParams as kr, CreateMessageRequestParamsBase as kt, ProtocolEra as l, IdJagTokenExchangeResponse as la, SubscriptionsListenRequestParams as li, InputRequest as ln, PrimitiveSchemaDefinition as lr, CallToolRequest as lt, DEFAULT_REQUEST_TIMEOUT_MSEC as m, OAuthClientRegistrationError as ma, TaskCreationParams as mi, InternalError as mn, Prompt as mr, CancelledNotification as mt, InMemoryTransport as n, SdkError as na, StringSchema as ni, Icons as nn, NotificationTypeMap as nr, StandardSchemaWithJSON as nt, STDIO_DEFAULT_MAX_BUFFER_SIZE as o, OAuthError as oa, SubscriptionsAcknowledgedNotification as oi, InitializeRequestParams as on, PaginatedResult as or, BaseMetadata as ot, ClientContext as p, OAuthClientMetadata as pa, TaskAugmentedRequestParams as pi, InputResponses as pn, ProgressToken as pr, CancelTaskResult as pt, ProtocolError as q, PROTOCOL_VERSION_META_KEY as qi, MetaObject as qn, SamplingMessage as qr, GetPromptRequest as qt, UriTemplate as r, SdkErrorCode as ra, SubscribeRequest as ri, ImageContent as rn, NumberSchema as rr, Annotations as rt, deserializeMessage as s, OAuthErrorCode as sa, SubscriptionsAcknowledgedNotificationParams as si, InitializeResult as sn, ParseError as sr, BlobResourceContents as st, preloadSchemas as t, resourceUrlFromServerUrl as ta, SingleSelectEnumSchema as ti, Icon as tn, NotificationParams as tr, StandardSchemaV1Sync as tt, getDisplayName as u, OAuthClientInformation as ua, SubscriptionsListenResult as ui, InputRequests as un, Progress as ur, CallToolRequestParams as ut, Protocol as v, OAuthTokenRevocationRequest as va, TaskStatusNotificationParams as vi, JSONObject as vn, PromptReference as vr, ClientRequest as vt, mergeCapabilities as w, ToolAnnotations as wi, JSONRPCResultResponse as wn, RequestId as wr, CompleteRequestResourceTemplate as wt, RequestOptions as x, OpenIdProviderMetadata as xa, TitledMultiSelectEnumSchema as xi, JSONRPCNotification as xn, ReadResourceResult as xr, CompleteRequest as xt, ProtocolOptions as y, OAuthTokens as ya, TextContent as yi, JSONRPCErrorResponse as yn, ReadResourceRequest as yr, ClientResult as yt, isJSONRPCErrorResponse as z, INTERNAL_ERROR as zi, ListTasksResult as zn, ResourceUpdatedNotificationParams as zr, ElicitRequestParams as zt };
//# sourceMappingURL=index-CHhhk6PQ.d.cts.map