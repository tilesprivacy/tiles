import { $ as ContentBlockSchema, $n as ServerTasksCapabilitySchema, $t as ListTasksResultSchema, A as StoredOAuthTokens, An as RequestMetaSchema, Ar as ToolUseContentSchema, At as InitializeRequestSchema, B as CancelTaskRequestSchema, Bn as ResourceUpdatedNotificationSchema, Bt as JSONRPCResultResponseSchema, C as OpenIdProviderDiscoveryMetadata, Cn as PromptReferenceSchema, Cr as TitledSingleSelectEnumSchemaSchema, Ct as GetTaskRequestSchema, D as OptionalSafeUrlSchema, Dn as ReadResourceResultSchema, Dr as ToolListChangedNotificationSchema, Dt as ImageContentSchema, E as OpenIdProviderMetadataSchema, En as ReadResourceRequestSchema, Er as ToolExecutionSchema, Et as IconsSchema, F as BlobResourceContentsSchema, Fn as ResourceRequestParamsSchema, Fr as JSONArray, Ft as JSONRPCErrorResponseSchema, G as ClientNotificationSchema, Gn as RootsListChangedNotificationSchema, Gt as ListPromptsResultSchema, H as CancelledNotificationParamsSchema, Hn as ResultSchema, Ht as LegacyTitledEnumSchemaSchema, I as BooleanSchemaSchema, In as ResourceSchema, Ir as JSONObject, It as JSONRPCMessageSchema, J as ClientTasksCapabilitySchema, Jn as SamplingMessageSchema, Jt as ListResourcesRequestSchema, K as ClientRequestSchema, Kn as SamplingContentSchema, Kt as ListResourceTemplatesRequestSchema, L as CallToolRequestParamsSchema, Ln as ResourceTemplateReferenceSchema, Lr as JSONValue, Lt as JSONRPCNotificationSchema, M as AudioContentSchema, Mn as ResourceContentsSchema, Mr as UnsubscribeRequestSchema, Mt as InitializedNotificationSchema, N as BaseMetadataSchema, Nn as ResourceLinkSchema, Nr as UntitledMultiSelectEnumSchemaSchema, Nt as JSONArraySchema, O as SafeUrlSchema, On as RelatedTaskMetadataSchema, Or as ToolResultContentSchema, Ot as ImplementationSchema, P as BaseRequestParamsSchema, Pn as ResourceListChangedNotificationSchema, Pr as UntitledSingleSelectEnumSchemaSchema, Pt as JSONObjectSchema, Q as CompleteResultSchema, Qn as ServerResultSchema, Qt as ListTasksRequestSchema, R as CallToolRequestSchema, Rn as ResourceTemplateSchema, Rt as JSONRPCRequestSchema, S as OAuthTokensSchema, Sn as PromptMessageSchema, Sr as TitledMultiSelectEnumSchemaSchema, St as GetTaskPayloadResultSchema, T as OpenIdProviderMetadata, Tn as ReadResourceRequestParamsSchema, Tr as ToolChoiceSchema, Tt as IconSchema, U as CancelledNotificationSchema, Un as RoleSchema, Ut as ListChangedOptionsBaseSchema, V as CancelTaskResultSchema, Vn as ResultMetaObjectSchema, Vt as JSONValueSchema, W as ClientCapabilitiesSchema, Wn as RootSchema, Wt as ListPromptsRequestSchema, X as CompleteRequestParamsSchema, Xn as ServerNotificationSchema, Xt as ListRootsRequestSchema, Y as CompatibilityCallToolResultSchema, Yn as ServerCapabilitiesSchema, Yt as ListResourcesResultSchema, Z as CompleteRequestSchema, Zn as ServerRequestSchema, Zt as ListRootsResultSchema, _ as OAuthProtectedResourceMetadata, _n as ProgressNotificationSchema, _r as TaskStatusNotificationParamsSchema, _t as EnumSchemaSchema, a as OAuthClientInformationFull, an as ModelHintSchema, ar as SubscribeRequestSchema, at as CursorSchema, b as OAuthTokenRevocationRequestSchema, bn as PromptArgumentSchema, br as TextContentSchema, bt as GetPromptResultSchema, c as OAuthClientInformationSchema, cn as NotificationSchema, cr as SubscriptionsAcknowledgedNotificationSchema, ct as ElicitRequestFormParamsSchema, d as OAuthClientRegistrationError, dn as PaginatedRequestParamsSchema, dr as SubscriptionsListenResultMetaSchema, dt as ElicitRequestURLParamsSchema, en as ListToolsRequestSchema, er as SetLevelRequestParamsSchema, et as CreateMessageRequestParamsSchema, f as OAuthClientRegistrationErrorSchema, fn as PaginatedRequestSchema, fr as SubscriptionsListenResultSchema, ft as ElicitResultSchema, g as OAuthMetadataSchema, gn as ProgressNotificationParamsSchema, gr as TaskSchema, gt as EmptyResultSchema, h as OAuthMetadata, hn as PrimitiveSchemaDefinitionSchema, hr as TaskMetadataSchema, ht as EmbeddedResourceSchema, i as OAuthClientInformation, in as LoggingMessageNotificationSchema, ir as SubscribeRequestParamsSchema, it as CreateTaskResultSchema, j as AnnotationsSchema, jn as RequestSchema, jr as UnsubscribeRequestParamsSchema, jt as InitializeResultSchema, k as StoredOAuthClientInformation, kn as RequestIdSchema, kr as ToolSchema, kt as InitializeRequestParamsSchema, l as OAuthClientMetadata, ln as NotificationsParamsSchema, lr as SubscriptionsListenRequestParamsSchema, lt as ElicitRequestParamsSchema, m as OAuthErrorResponseSchema, mn as PingRequestSchema, mr as TaskCreationParamsSchema, mt as ElicitationCompleteNotificationSchema, n as IdJagTokenExchangeResponse, nn as LoggingLevelSchema, nr as SingleSelectEnumSchemaSchema, nt as CreateMessageResultSchema, o as OAuthClientInformationFullSchema, on as ModelPreferencesSchema, or as SubscriptionFilterSchema, ot as DiscoverRequestSchema, p as OAuthErrorResponse, pn as PaginatedResultSchema, pr as TaskAugmentedRequestParamsSchema, pt as ElicitationCompleteNotificationParamsSchema, q as ClientResultSchema, qn as SamplingMessageContentBlockSchema, qt as ListResourceTemplatesResultSchema, r as IdJagTokenExchangeResponseSchema, rn as LoggingMessageNotificationParamsSchema, rr as StringSchemaSchema, rt as CreateMessageResultWithToolsSchema, s as OAuthClientInformationMixed, sn as MultiSelectEnumSchemaSchema, sr as SubscriptionsAcknowledgedNotificationParamsSchema, st as DiscoverResultSchema, t as AuthorizationServerMetadata, tn as ListToolsResultSchema, tr as SetLevelRequestSchema, tt as CreateMessageRequestSchema, u as OAuthClientMetadataSchema, un as NumberSchemaSchema, ur as SubscriptionsListenRequestSchema, ut as ElicitRequestSchema, v as OAuthProtectedResourceMetadataSchema, vn as ProgressSchema, vr as TaskStatusNotificationSchema, vt as GetPromptRequestParamsSchema, w as OpenIdProviderDiscoveryMetadataSchema, wn as PromptSchema, wr as ToolAnnotationsSchema, wt as GetTaskResultSchema, x as OAuthTokens, xn as PromptListChangedNotificationSchema, xr as TextResourceContentsSchema, xt as GetTaskPayloadRequestSchema, y as OAuthTokenRevocationRequest, yn as ProgressTokenSchema, yr as TaskStatusSchema, yt as GetPromptRequestSchema, z as CallToolResultSchema, zn as ResourceUpdatedNotificationParamsSchema, zt as JSONRPCResponseSchema } from "./auth-hYtaIUZm.cjs";

//#region src/constants.d.ts
declare const LATEST_PROTOCOL_VERSION = "2025-11-25";
declare const DEFAULT_NEGOTIATED_PROTOCOL_VERSION = "2025-03-26";
declare const SUPPORTED_PROTOCOL_VERSIONS: string[];
/**
 * `_meta` key associating a message with a 2025-11-25 task.
 *
 * @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only.
 */
declare const RELATED_TASK_META_KEY = "io.modelcontextprotocol/related-task";
/**
 * `_meta` key carrying the MCP protocol version governing a request.
 *
 * For the HTTP transport, the value must match the `MCP-Protocol-Version` header.
 */
declare const PROTOCOL_VERSION_META_KEY = "io.modelcontextprotocol/protocolVersion";
/**
 * `_meta` key identifying the client software making a request.
 *
 * Clients SHOULD include it on every request; the value is self-reported and
 * intended for display, logging, and debugging — servers should not rely on
 * it for behavior or security decisions.
 */
declare const CLIENT_INFO_META_KEY = "io.modelcontextprotocol/clientInfo";
/**
 * `_meta` key identifying the server software producing a response.
 *
 * Servers SHOULD include it on every response; the value is self-reported and
 * intended for display, logging, and debugging — clients should not rely on
 * it for behavior or security decisions.
 */
declare const SERVER_INFO_META_KEY = "io.modelcontextprotocol/serverInfo";
/**
 * `_meta` key carrying the client's capabilities for a request.
 *
 * Capabilities are declared per request rather than once at initialization;
 * servers must not infer capabilities from prior requests.
 */
declare const CLIENT_CAPABILITIES_META_KEY = "io.modelcontextprotocol/clientCapabilities";
/**
 * `_meta` key carrying the JSON-RPC ID of the `subscriptions/listen` request
 * that opened the stream a notification was delivered on.
 *
 * Stamped by the server on every notification delivered via a
 * `subscriptions/listen` stream (including the leading
 * `notifications/subscriptions/acknowledged`); on stdio, where all messages
 * share one channel, clients use it to correlate notifications with their
 * originating subscription. The value is the listen request's JSON-RPC ID
 * verbatim.
 */
declare const SUBSCRIPTION_ID_META_KEY = "io.modelcontextprotocol/subscriptionId";
/**
 * `_meta` key carrying the desired log level for a request.
 *
 * When absent, the server must not send `notifications/message` notifications
 * for the request.
 *
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months.
 */
declare const LOG_LEVEL_META_KEY = "io.modelcontextprotocol/logLevel";
/**
 * `_meta` key carrying W3C Trace Context for distributed tracing (SEP-414).
 *
 * When present, the value MUST follow the W3C `traceparent` header format,
 * e.g. `00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01`.
 *
 * @see https://www.w3.org/TR/trace-context/#traceparent-header
 */
declare const TRACEPARENT_META_KEY = "traceparent";
/**
 * `_meta` key carrying vendor-specific trace state for distributed tracing (SEP-414).
 *
 * When present, the value MUST follow the W3C `tracestate` header format,
 * e.g. `vendor1=value1,vendor2=value2`.
 *
 * @see https://www.w3.org/TR/trace-context/#tracestate-header
 */
declare const TRACESTATE_META_KEY = "tracestate";
/**
 * `_meta` key carrying cross-cutting propagation values for distributed tracing (SEP-414).
 *
 * When present, the value MUST follow the W3C Baggage header format,
 * e.g. `userId=alice,serverRegion=us-east-1`.
 *
 * @see https://www.w3.org/TR/baggage/
 */
declare const BAGGAGE_META_KEY = "baggage";
declare const JSONRPC_VERSION = "2.0";
declare const PARSE_ERROR = -32700;
declare const INVALID_REQUEST = -32600;
declare const METHOD_NOT_FOUND = -32601;
declare const INVALID_PARAMS = -32602;
declare const INTERNAL_ERROR = -32603;
//#endregion
export { AnnotationsSchema, AudioContentSchema, AuthorizationServerMetadata, BAGGAGE_META_KEY, BaseMetadataSchema, BaseRequestParamsSchema, BlobResourceContentsSchema, BooleanSchemaSchema, CLIENT_CAPABILITIES_META_KEY, CLIENT_INFO_META_KEY, CallToolRequestParamsSchema, CallToolRequestSchema, CallToolResultSchema, CancelTaskRequestSchema, CancelTaskResultSchema, CancelledNotificationParamsSchema, CancelledNotificationSchema, ClientCapabilitiesSchema, ClientNotificationSchema, ClientRequestSchema, ClientResultSchema, ClientTasksCapabilitySchema, CompatibilityCallToolResultSchema, CompleteRequestParamsSchema, CompleteRequestSchema, CompleteResultSchema, ContentBlockSchema, CreateMessageRequestParamsSchema, CreateMessageRequestSchema, CreateMessageResultSchema, CreateMessageResultWithToolsSchema, CreateTaskResultSchema, CursorSchema, DEFAULT_NEGOTIATED_PROTOCOL_VERSION, DiscoverRequestSchema, DiscoverResultSchema, ElicitRequestFormParamsSchema, ElicitRequestParamsSchema, ElicitRequestSchema, ElicitRequestURLParamsSchema, ElicitResultSchema, ElicitationCompleteNotificationParamsSchema, ElicitationCompleteNotificationSchema, EmbeddedResourceSchema, EmptyResultSchema, EnumSchemaSchema, GetPromptRequestParamsSchema, GetPromptRequestSchema, GetPromptResultSchema, GetTaskPayloadRequestSchema, GetTaskPayloadResultSchema, GetTaskRequestSchema, GetTaskResultSchema, INTERNAL_ERROR, INVALID_PARAMS, INVALID_REQUEST, IconSchema, IconsSchema, IdJagTokenExchangeResponse, IdJagTokenExchangeResponseSchema, ImageContentSchema, ImplementationSchema, InitializeRequestParamsSchema, InitializeRequestSchema, InitializeResultSchema, InitializedNotificationSchema, type JSONArray, JSONArraySchema, type JSONObject, JSONObjectSchema, JSONRPCErrorResponseSchema, JSONRPCMessageSchema, JSONRPCNotificationSchema, JSONRPCRequestSchema, JSONRPCResponseSchema, JSONRPCResultResponseSchema, JSONRPC_VERSION, type JSONValue, JSONValueSchema, LATEST_PROTOCOL_VERSION, LOG_LEVEL_META_KEY, LegacyTitledEnumSchemaSchema, ListChangedOptionsBaseSchema, ListPromptsRequestSchema, ListPromptsResultSchema, ListResourceTemplatesRequestSchema, ListResourceTemplatesResultSchema, ListResourcesRequestSchema, ListResourcesResultSchema, ListRootsRequestSchema, ListRootsResultSchema, ListTasksRequestSchema, ListTasksResultSchema, ListToolsRequestSchema, ListToolsResultSchema, LoggingLevelSchema, LoggingMessageNotificationParamsSchema, LoggingMessageNotificationSchema, METHOD_NOT_FOUND, ModelHintSchema, ModelPreferencesSchema, MultiSelectEnumSchemaSchema, NotificationSchema, NotificationsParamsSchema, NumberSchemaSchema, OAuthClientInformation, OAuthClientInformationFull, OAuthClientInformationFullSchema, OAuthClientInformationMixed, OAuthClientInformationSchema, OAuthClientMetadata, OAuthClientMetadataSchema, OAuthClientRegistrationError, OAuthClientRegistrationErrorSchema, OAuthErrorResponse, OAuthErrorResponseSchema, OAuthMetadata, OAuthMetadataSchema, OAuthProtectedResourceMetadata, OAuthProtectedResourceMetadataSchema, OAuthTokenRevocationRequest, OAuthTokenRevocationRequestSchema, OAuthTokens, OAuthTokensSchema, OpenIdProviderDiscoveryMetadata, OpenIdProviderDiscoveryMetadataSchema, OpenIdProviderMetadata, OpenIdProviderMetadataSchema, OptionalSafeUrlSchema, PARSE_ERROR, PROTOCOL_VERSION_META_KEY, PaginatedRequestParamsSchema, PaginatedRequestSchema, PaginatedResultSchema, PingRequestSchema, PrimitiveSchemaDefinitionSchema, ProgressNotificationParamsSchema, ProgressNotificationSchema, ProgressSchema, ProgressTokenSchema, PromptArgumentSchema, PromptListChangedNotificationSchema, PromptMessageSchema, PromptReferenceSchema, PromptSchema, RELATED_TASK_META_KEY, ReadResourceRequestParamsSchema, ReadResourceRequestSchema, ReadResourceResultSchema, RelatedTaskMetadataSchema, RequestIdSchema, RequestMetaSchema, RequestSchema, ResourceContentsSchema, ResourceLinkSchema, ResourceListChangedNotificationSchema, ResourceRequestParamsSchema, ResourceSchema, ResourceTemplateReferenceSchema, ResourceTemplateSchema, ResourceUpdatedNotificationParamsSchema, ResourceUpdatedNotificationSchema, ResultMetaObjectSchema, ResultSchema, RoleSchema, RootSchema, RootsListChangedNotificationSchema, SERVER_INFO_META_KEY, SUBSCRIPTION_ID_META_KEY, SUPPORTED_PROTOCOL_VERSIONS, SafeUrlSchema, SamplingContentSchema, SamplingMessageContentBlockSchema, SamplingMessageSchema, ServerCapabilitiesSchema, ServerNotificationSchema, ServerRequestSchema, ServerResultSchema, ServerTasksCapabilitySchema, SetLevelRequestParamsSchema, SetLevelRequestSchema, SingleSelectEnumSchemaSchema, StoredOAuthClientInformation, StoredOAuthTokens, StringSchemaSchema, SubscribeRequestParamsSchema, SubscribeRequestSchema, SubscriptionFilterSchema, SubscriptionsAcknowledgedNotificationParamsSchema, SubscriptionsAcknowledgedNotificationSchema, SubscriptionsListenRequestParamsSchema, SubscriptionsListenRequestSchema, SubscriptionsListenResultMetaSchema, SubscriptionsListenResultSchema, TRACEPARENT_META_KEY, TRACESTATE_META_KEY, TaskAugmentedRequestParamsSchema, TaskCreationParamsSchema, TaskMetadataSchema, TaskSchema, TaskStatusNotificationParamsSchema, TaskStatusNotificationSchema, TaskStatusSchema, TextContentSchema, TextResourceContentsSchema, TitledMultiSelectEnumSchemaSchema, TitledSingleSelectEnumSchemaSchema, ToolAnnotationsSchema, ToolChoiceSchema, ToolExecutionSchema, ToolListChangedNotificationSchema, ToolResultContentSchema, ToolSchema, ToolUseContentSchema, UnsubscribeRequestParamsSchema, UnsubscribeRequestSchema, UntitledMultiSelectEnumSchemaSchema, UntitledSingleSelectEnumSchemaSchema };
//# sourceMappingURL=internal.d.cts.map