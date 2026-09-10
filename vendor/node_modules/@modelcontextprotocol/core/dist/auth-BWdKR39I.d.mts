import * as z from "zod/v4";

//#region src/types.d.ts
type JSONValue = string | number | boolean | null | JSONObject | JSONArray;
type JSONObject = {
  [key: string]: JSONValue;
};
type JSONArray = JSONValue[];
//#endregion
//#region src/schemas.d.ts
declare const JSONValueSchema: z.ZodType<JSONValue, JSONValue>;
declare const JSONObjectSchema: z.ZodType<JSONObject, JSONObject>;
declare const JSONArraySchema: z.ZodType<JSONArray, JSONArray>;
/**
 * A progress token, used to associate progress notifications with the original request.
 */
declare const ProgressTokenSchema: z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>;
/**
 * An opaque token used to represent a cursor for pagination.
 */
declare const CursorSchema: z.ZodString;
/** @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only. */
declare const TaskMetadataSchema: z.ZodObject<{
  ttl: z.ZodOptional<z.ZodNumber>;
}, z.core.$strip>;
/**
 * Metadata for associating messages with a task.
 * Include this in the `_meta` field under the key `io.modelcontextprotocol/related-task`.
 *
 * @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only.
 */
declare const RelatedTaskMetadataSchema: z.ZodObject<{
  taskId: z.ZodString;
}, z.core.$strip>;
declare const RequestMetaSchema: z.ZodObject<{
  /**
   * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
   */
  progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
  /**
   * If specified, this request is related to the provided task.
   */
  "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
    taskId: z.ZodString;
  }, z.core.$strip>>;
}, z.core.$loose>;
/**
 * Common params for any request.
 */
declare const BaseRequestParamsSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    /**
     * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
     */
    progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
    /**
     * If specified, this request is related to the provided task.
     */
    "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
      taskId: z.ZodString;
    }, z.core.$strip>>;
  }, z.core.$loose>>;
}, z.core.$strip>;
/**
 * Common params for any task-augmented request.
 *
 * @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only.
 */
declare const TaskAugmentedRequestParamsSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    /**
     * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
     */
    progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
    /**
     * If specified, this request is related to the provided task.
     */
    "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
      taskId: z.ZodString;
    }, z.core.$strip>>;
  }, z.core.$loose>>;
  task: z.ZodOptional<z.ZodObject<{
    ttl: z.ZodOptional<z.ZodNumber>;
  }, z.core.$strip>>;
}, z.core.$strip>;
declare const RequestSchema: z.ZodObject<{
  method: z.ZodString;
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
  }, z.core.$loose>>;
}, z.core.$strip>;
declare const NotificationsParamsSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    /**
     * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
     */
    progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
    /**
     * If specified, this request is related to the provided task.
     */
    "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
      taskId: z.ZodString;
    }, z.core.$strip>>;
  }, z.core.$loose>>;
}, z.core.$strip>;
declare const NotificationSchema: z.ZodObject<{
  method: z.ZodString;
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
  }, z.core.$loose>>;
}, z.core.$strip>;
/**
 * The contents of a result's `_meta` field (the 2026-07-28 `ResultMetaObject`).
 * Loose — implementation-specific keys pass through.
 *
 * The serverInfo key identifies the server software producing the response
 * (servers SHOULD include it on every response; the value is self-reported
 * and intended for display, logging, and debugging). The getter defers the
 * `ImplementationSchema` reference, which is declared later in this file.
 */
declare const ResultMetaObjectSchema: z.ZodObject<{
  readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
    version: z.ZodString;
    websiteUrl: z.ZodOptional<z.ZodString>;
    description: z.ZodOptional<z.ZodString>;
    icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
      src: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
      theme: z.ZodOptional<z.ZodEnum<{
        light: "light";
        dark: "dark";
      }>>;
    }, z.core.$strip>>>;
    name: z.ZodString;
    title: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>>>;
}, z.core.$loose>;
declare const ResultSchema: z.ZodObject<{
  /**
   * See [MCP specification](https://modelcontextprotocol.io/specification/2026-07-28/basic/index#general-fields)
   * for notes on `_meta` usage.
   */
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
}, z.core.$loose>;
/**
 * A uniquely identifying ID for a request in JSON-RPC.
 */
declare const RequestIdSchema: z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>;
/**
 * A request that expects a response.
 */
declare const JSONRPCRequestSchema: z.ZodObject<{
  method: z.ZodString;
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
  }, z.core.$loose>>;
  jsonrpc: z.ZodLiteral<"2.0">;
  id: z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>;
}, z.core.$strict>;
/**
 * A notification which does not expect a response.
 */
declare const JSONRPCNotificationSchema: z.ZodObject<{
  method: z.ZodString;
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
  }, z.core.$loose>>;
  jsonrpc: z.ZodLiteral<"2.0">;
}, z.core.$strict>;
/**
 * A successful (non-error) response to a request.
 */
declare const JSONRPCResultResponseSchema: z.ZodObject<{
  jsonrpc: z.ZodLiteral<"2.0">;
  id: z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>;
  result: z.ZodObject<{
    /**
     * See [MCP specification](https://modelcontextprotocol.io/specification/2026-07-28/basic/index#general-fields)
     * for notes on `_meta` usage.
     */
    _meta: z.ZodOptional<z.ZodObject<{
      readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
        version: z.ZodString;
        websiteUrl: z.ZodOptional<z.ZodString>;
        description: z.ZodOptional<z.ZodString>;
        icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
          src: z.ZodString;
          mimeType: z.ZodOptional<z.ZodString>;
          sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
          theme: z.ZodOptional<z.ZodEnum<{
            light: "light";
            dark: "dark";
          }>>;
        }, z.core.$strip>>>;
        name: z.ZodString;
        title: z.ZodOptional<z.ZodString>;
      }, z.core.$strip>>>;
    }, z.core.$loose>>;
  }, z.core.$loose>;
}, z.core.$strict>;
/**
 * A response to a request that indicates an error occurred.
 */
declare const JSONRPCErrorResponseSchema: z.ZodObject<{
  jsonrpc: z.ZodLiteral<"2.0">;
  id: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
  error: z.ZodObject<{
    code: z.ZodNumber;
    message: z.ZodString;
    data: z.ZodOptional<z.ZodUnknown>;
  }, z.core.$strip>;
}, z.core.$strict>;
declare const JSONRPCMessageSchema: z.ZodUnion<readonly [z.ZodObject<{
  method: z.ZodString;
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
  }, z.core.$loose>>;
  jsonrpc: z.ZodLiteral<"2.0">;
  id: z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>;
}, z.core.$strict>, z.ZodObject<{
  method: z.ZodString;
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
  }, z.core.$loose>>;
  jsonrpc: z.ZodLiteral<"2.0">;
}, z.core.$strict>, z.ZodObject<{
  jsonrpc: z.ZodLiteral<"2.0">;
  id: z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>;
  result: z.ZodObject<{
    /**
     * See [MCP specification](https://modelcontextprotocol.io/specification/2026-07-28/basic/index#general-fields)
     * for notes on `_meta` usage.
     */
    _meta: z.ZodOptional<z.ZodObject<{
      readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
        version: z.ZodString;
        websiteUrl: z.ZodOptional<z.ZodString>;
        description: z.ZodOptional<z.ZodString>;
        icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
          src: z.ZodString;
          mimeType: z.ZodOptional<z.ZodString>;
          sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
          theme: z.ZodOptional<z.ZodEnum<{
            light: "light";
            dark: "dark";
          }>>;
        }, z.core.$strip>>>;
        name: z.ZodString;
        title: z.ZodOptional<z.ZodString>;
      }, z.core.$strip>>>;
    }, z.core.$loose>>;
  }, z.core.$loose>;
}, z.core.$strict>, z.ZodObject<{
  jsonrpc: z.ZodLiteral<"2.0">;
  id: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
  error: z.ZodObject<{
    code: z.ZodNumber;
    message: z.ZodString;
    data: z.ZodOptional<z.ZodUnknown>;
  }, z.core.$strip>;
}, z.core.$strict>]>;
declare const JSONRPCResponseSchema: z.ZodUnion<readonly [z.ZodObject<{
  jsonrpc: z.ZodLiteral<"2.0">;
  id: z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>;
  result: z.ZodObject<{
    /**
     * See [MCP specification](https://modelcontextprotocol.io/specification/2026-07-28/basic/index#general-fields)
     * for notes on `_meta` usage.
     */
    _meta: z.ZodOptional<z.ZodObject<{
      readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
        version: z.ZodString;
        websiteUrl: z.ZodOptional<z.ZodString>;
        description: z.ZodOptional<z.ZodString>;
        icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
          src: z.ZodString;
          mimeType: z.ZodOptional<z.ZodString>;
          sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
          theme: z.ZodOptional<z.ZodEnum<{
            light: "light";
            dark: "dark";
          }>>;
        }, z.core.$strip>>>;
        name: z.ZodString;
        title: z.ZodOptional<z.ZodString>;
      }, z.core.$strip>>>;
    }, z.core.$loose>>;
  }, z.core.$loose>;
}, z.core.$strict>, z.ZodObject<{
  jsonrpc: z.ZodLiteral<"2.0">;
  id: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
  error: z.ZodObject<{
    code: z.ZodNumber;
    message: z.ZodString;
    data: z.ZodOptional<z.ZodUnknown>;
  }, z.core.$strip>;
}, z.core.$strict>]>;
/**
 * A response that indicates success but carries no data.
 */
declare const EmptyResultSchema: z.ZodObject<{
  /**
   * See [MCP specification](https://modelcontextprotocol.io/specification/2026-07-28/basic/index#general-fields)
   * for notes on `_meta` usage.
   */
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
}, z.core.$strict>;
declare const CancelledNotificationParamsSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    /**
     * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
     */
    progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
    /**
     * If specified, this request is related to the provided task.
     */
    "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
      taskId: z.ZodString;
    }, z.core.$strip>>;
  }, z.core.$loose>>;
  requestId: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
  reason: z.ZodOptional<z.ZodString>;
}, z.core.$strip>;
/**
 * This notification can be sent by either side to indicate that it is cancelling a previously-issued request.
 *
 * The request SHOULD still be in-flight, but due to communication latency, it is always possible that this notification MAY arrive after the request has already finished.
 *
 * This notification indicates that the result will be unused, so any associated processing SHOULD cease.
 *
 * A client MUST NOT attempt to cancel its {@linkcode InitializeRequest | initialize} request.
 */
declare const CancelledNotificationSchema: z.ZodObject<{
  method: z.ZodLiteral<"notifications/cancelled">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    requestId: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
    reason: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>;
}, z.core.$strip>;
/**
 * Icon schema for use in {@link Tool | tools}, {@link Prompt | prompts}, {@link Resource | resources}, and {@link Implementation | implementations}.
 */
declare const IconSchema: z.ZodObject<{
  src: z.ZodString;
  mimeType: z.ZodOptional<z.ZodString>;
  sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
  theme: z.ZodOptional<z.ZodEnum<{
    light: "light";
    dark: "dark";
  }>>;
}, z.core.$strip>;
/**
 * Base schema to add `icons` property.
 *
 */
declare const IconsSchema: z.ZodObject<{
  icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
    src: z.ZodString;
    mimeType: z.ZodOptional<z.ZodString>;
    sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
    theme: z.ZodOptional<z.ZodEnum<{
      light: "light";
      dark: "dark";
    }>>;
  }, z.core.$strip>>>;
}, z.core.$strip>;
/**
 * Base metadata interface for common properties across {@link Resource | resources}, {@link Tool | tools}, {@link Prompt | prompts}, and {@link Implementation | implementations}.
 */
declare const BaseMetadataSchema: z.ZodObject<{
  name: z.ZodString;
  title: z.ZodOptional<z.ZodString>;
}, z.core.$strip>;
/**
 * Describes the name and version of an MCP implementation.
 */
declare const ImplementationSchema: z.ZodObject<{
  version: z.ZodString;
  websiteUrl: z.ZodOptional<z.ZodString>;
  description: z.ZodOptional<z.ZodString>;
  icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
    src: z.ZodString;
    mimeType: z.ZodOptional<z.ZodString>;
    sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
    theme: z.ZodOptional<z.ZodEnum<{
      light: "light";
      dark: "dark";
    }>>;
  }, z.core.$strip>>>;
  name: z.ZodString;
  title: z.ZodOptional<z.ZodString>;
}, z.core.$strip>;
/**
 * Task capabilities for clients, indicating which request types support task creation.
 *
 * @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only.
 */
declare const ClientTasksCapabilitySchema: z.ZodObject<{
  /**
   * Present if the client supports listing tasks.
   */
  list: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
  /**
   * Present if the client supports cancelling tasks.
   */
  cancel: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
  /**
   * Capabilities for task creation on specific request types.
   */
  requests: z.ZodOptional<z.ZodObject<{
    /**
     * Task support for sampling requests.
     */
    sampling: z.ZodOptional<z.ZodObject<{
      createMessage: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
    }, z.core.$loose>>;
    /**
     * Task support for elicitation requests.
     */
    elicitation: z.ZodOptional<z.ZodObject<{
      create: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
    }, z.core.$loose>>;
  }, z.core.$loose>>;
}, z.core.$loose>;
/**
 * Task capabilities for servers, indicating which request types support task creation.
 *
 * @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only.
 */
declare const ServerTasksCapabilitySchema: z.ZodObject<{
  /**
   * Present if the server supports listing tasks.
   */
  list: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
  /**
   * Present if the server supports cancelling tasks.
   */
  cancel: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
  /**
   * Capabilities for task creation on specific request types.
   */
  requests: z.ZodOptional<z.ZodObject<{
    /**
     * Task support for tool requests.
     */
    tools: z.ZodOptional<z.ZodObject<{
      call: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
    }, z.core.$loose>>;
  }, z.core.$loose>>;
}, z.core.$loose>;
/**
 * Capabilities a client may support. Known capabilities are defined here, in this schema, but this is not a closed set: any client can define its own, additional capabilities.
 */
declare const ClientCapabilitiesSchema: z.ZodObject<{
  experimental: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>>;
  sampling: z.ZodOptional<z.ZodObject<{
    context: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
    tools: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
  }, z.core.$strip>>;
  elicitation: z.ZodOptional<z.ZodPipe<z.ZodTransform<unknown, unknown>, z.ZodIntersection<z.ZodObject<{
    form: z.ZodOptional<z.ZodIntersection<z.ZodObject<{
      applyDefaults: z.ZodOptional<z.ZodBoolean>;
    }, z.core.$strip>, z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>>;
    url: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
  }, z.core.$strip>, z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>>>>;
  roots: z.ZodOptional<z.ZodObject<{
    listChanged: z.ZodOptional<z.ZodBoolean>;
  }, z.core.$strip>>;
  tasks: z.ZodOptional<z.ZodObject<{
    /**
     * Present if the client supports listing tasks.
     */
    list: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
    /**
     * Present if the client supports cancelling tasks.
     */
    cancel: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
    /**
     * Capabilities for task creation on specific request types.
     */
    requests: z.ZodOptional<z.ZodObject<{
      /**
       * Task support for sampling requests.
       */
      sampling: z.ZodOptional<z.ZodObject<{
        createMessage: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
      }, z.core.$loose>>;
      /**
       * Task support for elicitation requests.
       */
      elicitation: z.ZodOptional<z.ZodObject<{
        create: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
      }, z.core.$loose>>;
    }, z.core.$loose>>;
  }, z.core.$loose>>;
  extensions: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>>;
}, z.core.$strip>;
declare const InitializeRequestParamsSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    /**
     * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
     */
    progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
    /**
     * If specified, this request is related to the provided task.
     */
    "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
      taskId: z.ZodString;
    }, z.core.$strip>>;
  }, z.core.$loose>>;
  protocolVersion: z.ZodString;
  capabilities: z.ZodObject<{
    experimental: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>>;
    sampling: z.ZodOptional<z.ZodObject<{
      context: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
      tools: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
    }, z.core.$strip>>;
    elicitation: z.ZodOptional<z.ZodPipe<z.ZodTransform<unknown, unknown>, z.ZodIntersection<z.ZodObject<{
      form: z.ZodOptional<z.ZodIntersection<z.ZodObject<{
        applyDefaults: z.ZodOptional<z.ZodBoolean>;
      }, z.core.$strip>, z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>>;
      url: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
    }, z.core.$strip>, z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>>>>;
    roots: z.ZodOptional<z.ZodObject<{
      listChanged: z.ZodOptional<z.ZodBoolean>;
    }, z.core.$strip>>;
    tasks: z.ZodOptional<z.ZodObject<{
      /**
       * Present if the client supports listing tasks.
       */
      list: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
      /**
       * Present if the client supports cancelling tasks.
       */
      cancel: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
      /**
       * Capabilities for task creation on specific request types.
       */
      requests: z.ZodOptional<z.ZodObject<{
        /**
         * Task support for sampling requests.
         */
        sampling: z.ZodOptional<z.ZodObject<{
          createMessage: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
        }, z.core.$loose>>;
        /**
         * Task support for elicitation requests.
         */
        elicitation: z.ZodOptional<z.ZodObject<{
          create: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
        }, z.core.$loose>>;
      }, z.core.$loose>>;
    }, z.core.$loose>>;
    extensions: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>>;
  }, z.core.$strip>;
  clientInfo: z.ZodObject<{
    version: z.ZodString;
    websiteUrl: z.ZodOptional<z.ZodString>;
    description: z.ZodOptional<z.ZodString>;
    icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
      src: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
      theme: z.ZodOptional<z.ZodEnum<{
        light: "light";
        dark: "dark";
      }>>;
    }, z.core.$strip>>>;
    name: z.ZodString;
    title: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>;
}, z.core.$strip>;
/**
 * This request is sent from the client to the server when it first connects, asking it to begin initialization.
 */
declare const InitializeRequestSchema: z.ZodObject<{
  method: z.ZodLiteral<"initialize">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    protocolVersion: z.ZodString;
    capabilities: z.ZodObject<{
      experimental: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>>;
      sampling: z.ZodOptional<z.ZodObject<{
        context: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
        tools: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
      }, z.core.$strip>>;
      elicitation: z.ZodOptional<z.ZodPipe<z.ZodTransform<unknown, unknown>, z.ZodIntersection<z.ZodObject<{
        form: z.ZodOptional<z.ZodIntersection<z.ZodObject<{
          applyDefaults: z.ZodOptional<z.ZodBoolean>;
        }, z.core.$strip>, z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>>;
        url: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
      }, z.core.$strip>, z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>>>>;
      roots: z.ZodOptional<z.ZodObject<{
        listChanged: z.ZodOptional<z.ZodBoolean>;
      }, z.core.$strip>>;
      tasks: z.ZodOptional<z.ZodObject<{
        /**
         * Present if the client supports listing tasks.
         */
        list: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
        /**
         * Present if the client supports cancelling tasks.
         */
        cancel: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
        /**
         * Capabilities for task creation on specific request types.
         */
        requests: z.ZodOptional<z.ZodObject<{
          /**
           * Task support for sampling requests.
           */
          sampling: z.ZodOptional<z.ZodObject<{
            createMessage: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
          }, z.core.$loose>>;
          /**
           * Task support for elicitation requests.
           */
          elicitation: z.ZodOptional<z.ZodObject<{
            create: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
          }, z.core.$loose>>;
        }, z.core.$loose>>;
      }, z.core.$loose>>;
      extensions: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>>;
    }, z.core.$strip>;
    clientInfo: z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>;
  }, z.core.$strip>;
}, z.core.$strip>;
/**
 * Capabilities that a server may support. Known capabilities are defined here, in this schema, but this is not a closed set: any server can define its own, additional capabilities.
 */
declare const ServerCapabilitiesSchema: z.ZodObject<{
  experimental: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>>;
  logging: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
  completions: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
  prompts: z.ZodOptional<z.ZodObject<{
    listChanged: z.ZodOptional<z.ZodBoolean>;
  }, z.core.$strip>>;
  resources: z.ZodOptional<z.ZodObject<{
    subscribe: z.ZodOptional<z.ZodBoolean>;
    listChanged: z.ZodOptional<z.ZodBoolean>;
  }, z.core.$strip>>;
  tools: z.ZodOptional<z.ZodObject<{
    listChanged: z.ZodOptional<z.ZodBoolean>;
  }, z.core.$strip>>;
  tasks: z.ZodOptional<z.ZodObject<{
    /**
     * Present if the server supports listing tasks.
     */
    list: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
    /**
     * Present if the server supports cancelling tasks.
     */
    cancel: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
    /**
     * Capabilities for task creation on specific request types.
     */
    requests: z.ZodOptional<z.ZodObject<{
      /**
       * Task support for tool requests.
       */
      tools: z.ZodOptional<z.ZodObject<{
        call: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
      }, z.core.$loose>>;
    }, z.core.$loose>>;
  }, z.core.$loose>>;
  extensions: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>>;
}, z.core.$strip>;
/**
 * After receiving an initialize request from the client, the server sends this response.
 */
declare const InitializeResultSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  protocolVersion: z.ZodString;
  capabilities: z.ZodObject<{
    experimental: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>>;
    logging: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
    completions: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
    prompts: z.ZodOptional<z.ZodObject<{
      listChanged: z.ZodOptional<z.ZodBoolean>;
    }, z.core.$strip>>;
    resources: z.ZodOptional<z.ZodObject<{
      subscribe: z.ZodOptional<z.ZodBoolean>;
      listChanged: z.ZodOptional<z.ZodBoolean>;
    }, z.core.$strip>>;
    tools: z.ZodOptional<z.ZodObject<{
      listChanged: z.ZodOptional<z.ZodBoolean>;
    }, z.core.$strip>>;
    tasks: z.ZodOptional<z.ZodObject<{
      /**
       * Present if the server supports listing tasks.
       */
      list: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
      /**
       * Present if the server supports cancelling tasks.
       */
      cancel: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
      /**
       * Capabilities for task creation on specific request types.
       */
      requests: z.ZodOptional<z.ZodObject<{
        /**
         * Task support for tool requests.
         */
        tools: z.ZodOptional<z.ZodObject<{
          call: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
        }, z.core.$loose>>;
      }, z.core.$loose>>;
    }, z.core.$loose>>;
    extensions: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>>;
  }, z.core.$strip>;
  serverInfo: z.ZodObject<{
    version: z.ZodString;
    websiteUrl: z.ZodOptional<z.ZodString>;
    description: z.ZodOptional<z.ZodString>;
    icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
      src: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
      theme: z.ZodOptional<z.ZodEnum<{
        light: "light";
        dark: "dark";
      }>>;
    }, z.core.$strip>>>;
    name: z.ZodString;
    title: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>;
  instructions: z.ZodOptional<z.ZodString>;
}, z.core.$loose>;
/**
 * This notification is sent from the client to the server after initialization has finished.
 */
declare const InitializedNotificationSchema: z.ZodObject<{
  method: z.ZodLiteral<"notifications/initialized">;
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
  }, z.core.$strip>>;
}, z.core.$strip>;
/**
 * A request from the client asking the server to advertise its supported protocol
 * versions, capabilities, and other metadata (protocol revision 2026-07-28). Servers
 * MUST implement `server/discover`. Clients MAY call it but are not required to —
 * version negotiation can also happen inline via the per-request `_meta` envelope.
 */
declare const DiscoverRequestSchema: z.ZodObject<{
  method: z.ZodLiteral<"server/discover">;
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
  }, z.core.$strip>>;
}, z.core.$strip>;
/**
 * The result returned by the server for a `server/discover` request.
 */
declare const DiscoverResultSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  supportedVersions: z.ZodArray<z.ZodString>;
  capabilities: z.ZodObject<{
    experimental: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>>;
    logging: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
    completions: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
    prompts: z.ZodOptional<z.ZodObject<{
      listChanged: z.ZodOptional<z.ZodBoolean>;
    }, z.core.$strip>>;
    resources: z.ZodOptional<z.ZodObject<{
      subscribe: z.ZodOptional<z.ZodBoolean>;
      listChanged: z.ZodOptional<z.ZodBoolean>;
    }, z.core.$strip>>;
    tools: z.ZodOptional<z.ZodObject<{
      listChanged: z.ZodOptional<z.ZodBoolean>;
    }, z.core.$strip>>;
    tasks: z.ZodOptional<z.ZodObject<{
      /**
       * Present if the server supports listing tasks.
       */
      list: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
      /**
       * Present if the server supports cancelling tasks.
       */
      cancel: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
      /**
       * Capabilities for task creation on specific request types.
       */
      requests: z.ZodOptional<z.ZodObject<{
        /**
         * Task support for tool requests.
         */
        tools: z.ZodOptional<z.ZodObject<{
          call: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
        }, z.core.$loose>>;
      }, z.core.$loose>>;
    }, z.core.$loose>>;
    extensions: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>>;
  }, z.core.$strip>;
  instructions: z.ZodOptional<z.ZodString>;
}, z.core.$loose>;
/**
 * A ping, issued by either the server or the client, to check that the other party is still alive. The receiver must promptly respond, or else may be disconnected.
 */
declare const PingRequestSchema: z.ZodObject<{
  method: z.ZodLiteral<"ping">;
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
  }, z.core.$strip>>;
}, z.core.$strip>;
declare const ProgressSchema: z.ZodObject<{
  progress: z.ZodNumber;
  total: z.ZodOptional<z.ZodNumber>;
  message: z.ZodOptional<z.ZodString>;
}, z.core.$strip>;
declare const ProgressNotificationParamsSchema: z.ZodObject<{
  progressToken: z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>;
  progress: z.ZodNumber;
  total: z.ZodOptional<z.ZodNumber>;
  message: z.ZodOptional<z.ZodString>;
  _meta: z.ZodOptional<z.ZodObject<{
    /**
     * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
     */
    progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
    /**
     * If specified, this request is related to the provided task.
     */
    "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
      taskId: z.ZodString;
    }, z.core.$strip>>;
  }, z.core.$loose>>;
}, z.core.$strip>;
/**
 * An out-of-band notification used to inform the receiver of a progress update for a long-running request.
 *
 * @category notifications/progress
 */
declare const ProgressNotificationSchema: z.ZodObject<{
  method: z.ZodLiteral<"notifications/progress">;
  params: z.ZodObject<{
    progressToken: z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>;
    progress: z.ZodNumber;
    total: z.ZodOptional<z.ZodNumber>;
    message: z.ZodOptional<z.ZodString>;
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
  }, z.core.$strip>;
}, z.core.$strip>;
declare const PaginatedRequestParamsSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    /**
     * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
     */
    progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
    /**
     * If specified, this request is related to the provided task.
     */
    "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
      taskId: z.ZodString;
    }, z.core.$strip>>;
  }, z.core.$loose>>;
  cursor: z.ZodOptional<z.ZodString>;
}, z.core.$strip>;
declare const PaginatedRequestSchema: z.ZodObject<{
  method: z.ZodString;
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    cursor: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>>;
}, z.core.$strip>;
declare const PaginatedResultSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  nextCursor: z.ZodOptional<z.ZodString>;
}, z.core.$loose>;
/**
 * The contents of a specific resource or sub-resource.
 */
declare const ResourceContentsSchema: z.ZodObject<{
  uri: z.ZodString;
  mimeType: z.ZodOptional<z.ZodString>;
  _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
}, z.core.$strip>;
declare const TextResourceContentsSchema: z.ZodObject<{
  uri: z.ZodString;
  mimeType: z.ZodOptional<z.ZodString>;
  _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  text: z.ZodString;
}, z.core.$strip>;
declare const BlobResourceContentsSchema: z.ZodObject<{
  uri: z.ZodString;
  mimeType: z.ZodOptional<z.ZodString>;
  _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  blob: z.ZodString;
}, z.core.$strip>;
/**
 * The sender or recipient of messages and data in a conversation.
 */
declare const RoleSchema: z.ZodEnum<{
  user: "user";
  assistant: "assistant";
}>;
/**
 * Optional annotations providing clients additional context about a resource.
 */
declare const AnnotationsSchema: z.ZodObject<{
  audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
    user: "user";
    assistant: "assistant";
  }>>>;
  priority: z.ZodOptional<z.ZodNumber>;
  lastModified: z.ZodOptional<z.ZodISODateTime>;
}, z.core.$strip>;
/**
 * A known resource that the server is capable of reading.
 */
declare const ResourceSchema: z.ZodObject<{
  uri: z.ZodString;
  description: z.ZodOptional<z.ZodString>;
  mimeType: z.ZodOptional<z.ZodString>;
  size: z.ZodOptional<z.ZodNumber>;
  annotations: z.ZodOptional<z.ZodObject<{
    audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
      user: "user";
      assistant: "assistant";
    }>>>;
    priority: z.ZodOptional<z.ZodNumber>;
    lastModified: z.ZodOptional<z.ZodISODateTime>;
  }, z.core.$strip>>;
  _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
  icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
    src: z.ZodString;
    mimeType: z.ZodOptional<z.ZodString>;
    sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
    theme: z.ZodOptional<z.ZodEnum<{
      light: "light";
      dark: "dark";
    }>>;
  }, z.core.$strip>>>;
  name: z.ZodString;
  title: z.ZodOptional<z.ZodString>;
}, z.core.$strip>;
/**
 * A template description for resources available on the server.
 */
declare const ResourceTemplateSchema: z.ZodObject<{
  uriTemplate: z.ZodString;
  description: z.ZodOptional<z.ZodString>;
  mimeType: z.ZodOptional<z.ZodString>;
  annotations: z.ZodOptional<z.ZodObject<{
    audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
      user: "user";
      assistant: "assistant";
    }>>>;
    priority: z.ZodOptional<z.ZodNumber>;
    lastModified: z.ZodOptional<z.ZodISODateTime>;
  }, z.core.$strip>>;
  _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
  icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
    src: z.ZodString;
    mimeType: z.ZodOptional<z.ZodString>;
    sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
    theme: z.ZodOptional<z.ZodEnum<{
      light: "light";
      dark: "dark";
    }>>;
  }, z.core.$strip>>>;
  name: z.ZodString;
  title: z.ZodOptional<z.ZodString>;
}, z.core.$strip>;
/**
 * Sent from the client to request a list of resources the server has.
 */
declare const ListResourcesRequestSchema: z.ZodObject<{
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    cursor: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>>;
  method: z.ZodLiteral<"resources/list">;
}, z.core.$strip>;
/**
 * The server's response to a {@linkcode ListResourcesRequest | resources/list} request from the client.
 */
declare const ListResourcesResultSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  nextCursor: z.ZodOptional<z.ZodString>;
  resources: z.ZodArray<z.ZodObject<{
    uri: z.ZodString;
    description: z.ZodOptional<z.ZodString>;
    mimeType: z.ZodOptional<z.ZodString>;
    size: z.ZodOptional<z.ZodNumber>;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
    icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
      src: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
      theme: z.ZodOptional<z.ZodEnum<{
        light: "light";
        dark: "dark";
      }>>;
    }, z.core.$strip>>>;
    name: z.ZodString;
    title: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>>;
}, z.core.$loose>;
/**
 * Sent from the client to request a list of resource templates the server has.
 */
declare const ListResourceTemplatesRequestSchema: z.ZodObject<{
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    cursor: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>>;
  method: z.ZodLiteral<"resources/templates/list">;
}, z.core.$strip>;
/**
 * The server's response to a {@linkcode ListResourceTemplatesRequest | resources/templates/list} request from the client.
 */
declare const ListResourceTemplatesResultSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  nextCursor: z.ZodOptional<z.ZodString>;
  resourceTemplates: z.ZodArray<z.ZodObject<{
    uriTemplate: z.ZodString;
    description: z.ZodOptional<z.ZodString>;
    mimeType: z.ZodOptional<z.ZodString>;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
    icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
      src: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
      theme: z.ZodOptional<z.ZodEnum<{
        light: "light";
        dark: "dark";
      }>>;
    }, z.core.$strip>>>;
    name: z.ZodString;
    title: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>>;
}, z.core.$loose>;
declare const ResourceRequestParamsSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    /**
     * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
     */
    progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
    /**
     * If specified, this request is related to the provided task.
     */
    "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
      taskId: z.ZodString;
    }, z.core.$strip>>;
  }, z.core.$loose>>;
  uri: z.ZodString;
}, z.core.$strip>;
/**
 * Parameters for a {@linkcode ReadResourceRequest | resources/read} request.
 */
declare const ReadResourceRequestParamsSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    /**
     * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
     */
    progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
    /**
     * If specified, this request is related to the provided task.
     */
    "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
      taskId: z.ZodString;
    }, z.core.$strip>>;
  }, z.core.$loose>>;
  uri: z.ZodString;
}, z.core.$strip>;
/**
 * Sent from the client to the server, to read a specific resource URI.
 */
declare const ReadResourceRequestSchema: z.ZodObject<{
  method: z.ZodLiteral<"resources/read">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    uri: z.ZodString;
  }, z.core.$strip>;
}, z.core.$strip>;
/**
 * The server's response to a {@linkcode ReadResourceRequest | resources/read} request from the client.
 */
declare const ReadResourceResultSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  contents: z.ZodArray<z.ZodUnion<readonly [z.ZodObject<{
    uri: z.ZodString;
    mimeType: z.ZodOptional<z.ZodString>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    text: z.ZodString;
  }, z.core.$strip>, z.ZodObject<{
    uri: z.ZodString;
    mimeType: z.ZodOptional<z.ZodString>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    blob: z.ZodString;
  }, z.core.$strip>]>>;
}, z.core.$loose>;
/**
 * An optional notification from the server to the client, informing it that the list of resources it can read from has changed. This may be issued by servers without any previous subscription from the client.
 */
declare const ResourceListChangedNotificationSchema: z.ZodObject<{
  method: z.ZodLiteral<"notifications/resources/list_changed">;
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
  }, z.core.$strip>>;
}, z.core.$strip>;
declare const SubscribeRequestParamsSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    /**
     * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
     */
    progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
    /**
     * If specified, this request is related to the provided task.
     */
    "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
      taskId: z.ZodString;
    }, z.core.$strip>>;
  }, z.core.$loose>>;
  uri: z.ZodString;
}, z.core.$strip>;
/**
 * Sent from the client to request `resources/updated` notifications from the server whenever a particular resource changes.
 */
declare const SubscribeRequestSchema: z.ZodObject<{
  method: z.ZodLiteral<"resources/subscribe">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    uri: z.ZodString;
  }, z.core.$strip>;
}, z.core.$strip>;
declare const UnsubscribeRequestParamsSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    /**
     * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
     */
    progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
    /**
     * If specified, this request is related to the provided task.
     */
    "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
      taskId: z.ZodString;
    }, z.core.$strip>>;
  }, z.core.$loose>>;
  uri: z.ZodString;
}, z.core.$strip>;
/**
 * Sent from the client to request cancellation of {@linkcode ResourceUpdatedNotification | resources/updated} notifications from the server. This should follow a previous {@linkcode SubscribeRequest | resources/subscribe} request.
 */
declare const UnsubscribeRequestSchema: z.ZodObject<{
  method: z.ZodLiteral<"resources/unsubscribe">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    uri: z.ZodString;
  }, z.core.$strip>;
}, z.core.$strip>;
/**
 * The set of notification types a client opts in to on a `subscriptions/listen`
 * request. Each type is opt-in; the server MUST NOT send a notification type
 * the client has not explicitly requested here.
 */
declare const SubscriptionFilterSchema: z.ZodObject<{
  toolsListChanged: z.ZodOptional<z.ZodBoolean>;
  promptsListChanged: z.ZodOptional<z.ZodBoolean>;
  resourcesListChanged: z.ZodOptional<z.ZodBoolean>;
  resourceSubscriptions: z.ZodOptional<z.ZodArray<z.ZodString>>;
}, z.core.$strip>;
declare const SubscriptionsListenRequestParamsSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    /**
     * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
     */
    progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
    /**
     * If specified, this request is related to the provided task.
     */
    "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
      taskId: z.ZodString;
    }, z.core.$strip>>;
  }, z.core.$loose>>;
  notifications: z.ZodObject<{
    toolsListChanged: z.ZodOptional<z.ZodBoolean>;
    promptsListChanged: z.ZodOptional<z.ZodBoolean>;
    resourcesListChanged: z.ZodOptional<z.ZodBoolean>;
    resourceSubscriptions: z.ZodOptional<z.ZodArray<z.ZodString>>;
  }, z.core.$strip>;
}, z.core.$strip>;
/**
 * Sent from the client to open a long-lived channel for receiving notifications
 * outside the context of a specific request (protocol revision 2026-07-28).
 * Replaces the previous HTTP GET endpoint and `resources/subscribe`.
 */
declare const SubscriptionsListenRequestSchema: z.ZodObject<{
  method: z.ZodLiteral<"subscriptions/listen">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    notifications: z.ZodObject<{
      toolsListChanged: z.ZodOptional<z.ZodBoolean>;
      promptsListChanged: z.ZodOptional<z.ZodBoolean>;
      resourcesListChanged: z.ZodOptional<z.ZodBoolean>;
      resourceSubscriptions: z.ZodOptional<z.ZodArray<z.ZodString>>;
    }, z.core.$strip>;
  }, z.core.$strip>;
}, z.core.$strip>;
declare const SubscriptionsAcknowledgedNotificationParamsSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    /**
     * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
     */
    progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
    /**
     * If specified, this request is related to the provided task.
     */
    "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
      taskId: z.ZodString;
    }, z.core.$strip>>;
  }, z.core.$loose>>;
  notifications: z.ZodObject<{
    toolsListChanged: z.ZodOptional<z.ZodBoolean>;
    promptsListChanged: z.ZodOptional<z.ZodBoolean>;
    resourcesListChanged: z.ZodOptional<z.ZodBoolean>;
    resourceSubscriptions: z.ZodOptional<z.ZodArray<z.ZodString>>;
  }, z.core.$strip>;
}, z.core.$strip>;
/**
 * Sent by the server as the first message on a `subscriptions/listen` stream
 * to acknowledge that the subscription has been established and report which
 * notification types it agreed to honor (protocol revision 2026-07-28).
 */
declare const SubscriptionsAcknowledgedNotificationSchema: z.ZodObject<{
  method: z.ZodLiteral<"notifications/subscriptions/acknowledged">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    notifications: z.ZodObject<{
      toolsListChanged: z.ZodOptional<z.ZodBoolean>;
      promptsListChanged: z.ZodOptional<z.ZodBoolean>;
      resourcesListChanged: z.ZodOptional<z.ZodBoolean>;
      resourceSubscriptions: z.ZodOptional<z.ZodArray<z.ZodString>>;
    }, z.core.$strip>;
  }, z.core.$strip>;
}, z.core.$strip>;
/**
 * `_meta` for a {@linkcode SubscriptionsListenResult}: the listen request's
 * JSON-RPC ID under the canonical subscription-id key (mirroring the same key
 * on every notification delivered on the stream). Extends
 * {@linkcode ResultMetaObjectSchema}, so the optional serverInfo key is typed
 * here too.
 */
declare const SubscriptionsListenResultMetaSchema: z.ZodObject<{
  readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
    version: z.ZodString;
    websiteUrl: z.ZodOptional<z.ZodString>;
    description: z.ZodOptional<z.ZodString>;
    icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
      src: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
      theme: z.ZodOptional<z.ZodEnum<{
        light: "light";
        dark: "dark";
      }>>;
    }, z.core.$strip>>>;
    name: z.ZodString;
    title: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>>>;
  "io.modelcontextprotocol/subscriptionId": z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>;
}, z.core.$loose>;
/**
 * The response to a `subscriptions/listen` request, signalling that the
 * subscription has ended gracefully (for example, during server shutdown).
 * Because the listen stream is long-lived, this result is sent only when the
 * server tears the subscription down; an abrupt transport close carries no
 * response. The result body is otherwise empty.
 */
declare const SubscriptionsListenResultSchema: z.ZodObject<{
  _meta: z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
    "io.modelcontextprotocol/subscriptionId": z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>;
  }, z.core.$loose>;
}, z.core.$loose>;
/**
 * Parameters for a {@linkcode ResourceUpdatedNotification | notifications/resources/updated} notification.
 */
declare const ResourceUpdatedNotificationParamsSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    /**
     * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
     */
    progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
    /**
     * If specified, this request is related to the provided task.
     */
    "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
      taskId: z.ZodString;
    }, z.core.$strip>>;
  }, z.core.$loose>>;
  uri: z.ZodString;
}, z.core.$strip>;
/**
 * A notification from the server to the client, informing it that a resource has changed and may need to be read again. This should only be sent if the client previously sent a {@linkcode SubscribeRequest | resources/subscribe} request.
 */
declare const ResourceUpdatedNotificationSchema: z.ZodObject<{
  method: z.ZodLiteral<"notifications/resources/updated">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    uri: z.ZodString;
  }, z.core.$strip>;
}, z.core.$strip>;
/**
 * Describes an argument that a prompt can accept.
 */
declare const PromptArgumentSchema: z.ZodObject<{
  name: z.ZodString;
  description: z.ZodOptional<z.ZodString>;
  required: z.ZodOptional<z.ZodBoolean>;
}, z.core.$strip>;
/**
 * A prompt or prompt template that the server offers.
 */
declare const PromptSchema: z.ZodObject<{
  description: z.ZodOptional<z.ZodString>;
  arguments: z.ZodOptional<z.ZodArray<z.ZodObject<{
    name: z.ZodString;
    description: z.ZodOptional<z.ZodString>;
    required: z.ZodOptional<z.ZodBoolean>;
  }, z.core.$strip>>>;
  _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
  icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
    src: z.ZodString;
    mimeType: z.ZodOptional<z.ZodString>;
    sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
    theme: z.ZodOptional<z.ZodEnum<{
      light: "light";
      dark: "dark";
    }>>;
  }, z.core.$strip>>>;
  name: z.ZodString;
  title: z.ZodOptional<z.ZodString>;
}, z.core.$strip>;
/**
 * Sent from the client to request a list of prompts and prompt templates the server has.
 */
declare const ListPromptsRequestSchema: z.ZodObject<{
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    cursor: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>>;
  method: z.ZodLiteral<"prompts/list">;
}, z.core.$strip>;
/**
 * The server's response to a {@linkcode ListPromptsRequest | prompts/list} request from the client.
 */
declare const ListPromptsResultSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  nextCursor: z.ZodOptional<z.ZodString>;
  prompts: z.ZodArray<z.ZodObject<{
    description: z.ZodOptional<z.ZodString>;
    arguments: z.ZodOptional<z.ZodArray<z.ZodObject<{
      name: z.ZodString;
      description: z.ZodOptional<z.ZodString>;
      required: z.ZodOptional<z.ZodBoolean>;
    }, z.core.$strip>>>;
    _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
    icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
      src: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
      theme: z.ZodOptional<z.ZodEnum<{
        light: "light";
        dark: "dark";
      }>>;
    }, z.core.$strip>>>;
    name: z.ZodString;
    title: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>>;
}, z.core.$loose>;
/**
 * Parameters for a {@linkcode GetPromptRequest | prompts/get} request.
 */
declare const GetPromptRequestParamsSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    /**
     * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
     */
    progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
    /**
     * If specified, this request is related to the provided task.
     */
    "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
      taskId: z.ZodString;
    }, z.core.$strip>>;
  }, z.core.$loose>>;
  name: z.ZodString;
  arguments: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodString>>;
}, z.core.$strip>;
/**
 * Used by the client to get a prompt provided by the server.
 */
declare const GetPromptRequestSchema: z.ZodObject<{
  method: z.ZodLiteral<"prompts/get">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    name: z.ZodString;
    arguments: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodString>>;
  }, z.core.$strip>;
}, z.core.$strip>;
/**
 * Text provided to or from an LLM.
 */
declare const TextContentSchema: z.ZodObject<{
  type: z.ZodLiteral<"text">;
  text: z.ZodString;
  annotations: z.ZodOptional<z.ZodObject<{
    audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
      user: "user";
      assistant: "assistant";
    }>>>;
    priority: z.ZodOptional<z.ZodNumber>;
    lastModified: z.ZodOptional<z.ZodISODateTime>;
  }, z.core.$strip>>;
  _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
}, z.core.$strip>;
/**
 * An image provided to or from an LLM.
 */
declare const ImageContentSchema: z.ZodObject<{
  type: z.ZodLiteral<"image">;
  data: z.ZodString;
  mimeType: z.ZodString;
  annotations: z.ZodOptional<z.ZodObject<{
    audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
      user: "user";
      assistant: "assistant";
    }>>>;
    priority: z.ZodOptional<z.ZodNumber>;
    lastModified: z.ZodOptional<z.ZodISODateTime>;
  }, z.core.$strip>>;
  _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
}, z.core.$strip>;
/**
 * Audio content provided to or from an LLM.
 */
declare const AudioContentSchema: z.ZodObject<{
  type: z.ZodLiteral<"audio">;
  data: z.ZodString;
  mimeType: z.ZodString;
  annotations: z.ZodOptional<z.ZodObject<{
    audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
      user: "user";
      assistant: "assistant";
    }>>>;
    priority: z.ZodOptional<z.ZodNumber>;
    lastModified: z.ZodOptional<z.ZodISODateTime>;
  }, z.core.$strip>>;
  _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
}, z.core.$strip>;
/**
 * A tool call request from an assistant (LLM).
 * Represents the assistant's request to use a tool.
 *
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to calling LLM
 * provider APIs directly.
 */
declare const ToolUseContentSchema: z.ZodObject<{
  type: z.ZodLiteral<"tool_use">;
  name: z.ZodString;
  id: z.ZodString;
  input: z.ZodRecord<z.ZodString, z.ZodUnknown>;
  _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
}, z.core.$strip>;
/**
 * The contents of a resource, embedded into a prompt or tool call result.
 */
declare const EmbeddedResourceSchema: z.ZodObject<{
  type: z.ZodLiteral<"resource">;
  resource: z.ZodUnion<readonly [z.ZodObject<{
    uri: z.ZodString;
    mimeType: z.ZodOptional<z.ZodString>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    text: z.ZodString;
  }, z.core.$strip>, z.ZodObject<{
    uri: z.ZodString;
    mimeType: z.ZodOptional<z.ZodString>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    blob: z.ZodString;
  }, z.core.$strip>]>;
  annotations: z.ZodOptional<z.ZodObject<{
    audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
      user: "user";
      assistant: "assistant";
    }>>>;
    priority: z.ZodOptional<z.ZodNumber>;
    lastModified: z.ZodOptional<z.ZodISODateTime>;
  }, z.core.$strip>>;
  _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
}, z.core.$strip>;
/**
 * A resource that the server is capable of reading, included in a prompt or tool call result.
 *
 * Note: resource links returned by tools are not guaranteed to appear in the results of {@linkcode ListResourcesRequest | resources/list} requests.
 */
declare const ResourceLinkSchema: z.ZodObject<{
  uri: z.ZodString;
  description: z.ZodOptional<z.ZodString>;
  mimeType: z.ZodOptional<z.ZodString>;
  size: z.ZodOptional<z.ZodNumber>;
  annotations: z.ZodOptional<z.ZodObject<{
    audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
      user: "user";
      assistant: "assistant";
    }>>>;
    priority: z.ZodOptional<z.ZodNumber>;
    lastModified: z.ZodOptional<z.ZodISODateTime>;
  }, z.core.$strip>>;
  _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
  icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
    src: z.ZodString;
    mimeType: z.ZodOptional<z.ZodString>;
    sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
    theme: z.ZodOptional<z.ZodEnum<{
      light: "light";
      dark: "dark";
    }>>;
  }, z.core.$strip>>>;
  name: z.ZodString;
  title: z.ZodOptional<z.ZodString>;
  type: z.ZodLiteral<"resource_link">;
}, z.core.$strip>;
/**
 * A content block that can be used in prompts and tool results.
 */
declare const ContentBlockSchema: z.ZodUnion<readonly [z.ZodObject<{
  type: z.ZodLiteral<"text">;
  text: z.ZodString;
  annotations: z.ZodOptional<z.ZodObject<{
    audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
      user: "user";
      assistant: "assistant";
    }>>>;
    priority: z.ZodOptional<z.ZodNumber>;
    lastModified: z.ZodOptional<z.ZodISODateTime>;
  }, z.core.$strip>>;
  _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
}, z.core.$strip>, z.ZodObject<{
  type: z.ZodLiteral<"image">;
  data: z.ZodString;
  mimeType: z.ZodString;
  annotations: z.ZodOptional<z.ZodObject<{
    audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
      user: "user";
      assistant: "assistant";
    }>>>;
    priority: z.ZodOptional<z.ZodNumber>;
    lastModified: z.ZodOptional<z.ZodISODateTime>;
  }, z.core.$strip>>;
  _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
}, z.core.$strip>, z.ZodObject<{
  type: z.ZodLiteral<"audio">;
  data: z.ZodString;
  mimeType: z.ZodString;
  annotations: z.ZodOptional<z.ZodObject<{
    audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
      user: "user";
      assistant: "assistant";
    }>>>;
    priority: z.ZodOptional<z.ZodNumber>;
    lastModified: z.ZodOptional<z.ZodISODateTime>;
  }, z.core.$strip>>;
  _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
}, z.core.$strip>, z.ZodObject<{
  uri: z.ZodString;
  description: z.ZodOptional<z.ZodString>;
  mimeType: z.ZodOptional<z.ZodString>;
  size: z.ZodOptional<z.ZodNumber>;
  annotations: z.ZodOptional<z.ZodObject<{
    audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
      user: "user";
      assistant: "assistant";
    }>>>;
    priority: z.ZodOptional<z.ZodNumber>;
    lastModified: z.ZodOptional<z.ZodISODateTime>;
  }, z.core.$strip>>;
  _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
  icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
    src: z.ZodString;
    mimeType: z.ZodOptional<z.ZodString>;
    sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
    theme: z.ZodOptional<z.ZodEnum<{
      light: "light";
      dark: "dark";
    }>>;
  }, z.core.$strip>>>;
  name: z.ZodString;
  title: z.ZodOptional<z.ZodString>;
  type: z.ZodLiteral<"resource_link">;
}, z.core.$strip>, z.ZodObject<{
  type: z.ZodLiteral<"resource">;
  resource: z.ZodUnion<readonly [z.ZodObject<{
    uri: z.ZodString;
    mimeType: z.ZodOptional<z.ZodString>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    text: z.ZodString;
  }, z.core.$strip>, z.ZodObject<{
    uri: z.ZodString;
    mimeType: z.ZodOptional<z.ZodString>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    blob: z.ZodString;
  }, z.core.$strip>]>;
  annotations: z.ZodOptional<z.ZodObject<{
    audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
      user: "user";
      assistant: "assistant";
    }>>>;
    priority: z.ZodOptional<z.ZodNumber>;
    lastModified: z.ZodOptional<z.ZodISODateTime>;
  }, z.core.$strip>>;
  _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
}, z.core.$strip>]>;
/**
 * Describes a message returned as part of a prompt.
 */
declare const PromptMessageSchema: z.ZodObject<{
  role: z.ZodEnum<{
    user: "user";
    assistant: "assistant";
  }>;
  content: z.ZodUnion<readonly [z.ZodObject<{
    type: z.ZodLiteral<"text">;
    text: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"image">;
    data: z.ZodString;
    mimeType: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"audio">;
    data: z.ZodString;
    mimeType: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    uri: z.ZodString;
    description: z.ZodOptional<z.ZodString>;
    mimeType: z.ZodOptional<z.ZodString>;
    size: z.ZodOptional<z.ZodNumber>;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
    icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
      src: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
      theme: z.ZodOptional<z.ZodEnum<{
        light: "light";
        dark: "dark";
      }>>;
    }, z.core.$strip>>>;
    name: z.ZodString;
    title: z.ZodOptional<z.ZodString>;
    type: z.ZodLiteral<"resource_link">;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"resource">;
    resource: z.ZodUnion<readonly [z.ZodObject<{
      uri: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      text: z.ZodString;
    }, z.core.$strip>, z.ZodObject<{
      uri: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      blob: z.ZodString;
    }, z.core.$strip>]>;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>]>;
}, z.core.$strip>;
/**
 * The server's response to a {@linkcode GetPromptRequest | prompts/get} request from the client.
 */
declare const GetPromptResultSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  description: z.ZodOptional<z.ZodString>;
  messages: z.ZodArray<z.ZodObject<{
    role: z.ZodEnum<{
      user: "user";
      assistant: "assistant";
    }>;
    content: z.ZodUnion<readonly [z.ZodObject<{
      type: z.ZodLiteral<"text">;
      text: z.ZodString;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"image">;
      data: z.ZodString;
      mimeType: z.ZodString;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"audio">;
      data: z.ZodString;
      mimeType: z.ZodString;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      uri: z.ZodString;
      description: z.ZodOptional<z.ZodString>;
      mimeType: z.ZodOptional<z.ZodString>;
      size: z.ZodOptional<z.ZodNumber>;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
      type: z.ZodLiteral<"resource_link">;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"resource">;
      resource: z.ZodUnion<readonly [z.ZodObject<{
        uri: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        text: z.ZodString;
      }, z.core.$strip>, z.ZodObject<{
        uri: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        blob: z.ZodString;
      }, z.core.$strip>]>;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>]>;
  }, z.core.$strip>>;
}, z.core.$loose>;
/**
 * An optional notification from the server to the client, informing it that the list of prompts it offers has changed. This may be issued by servers without any previous subscription from the client.
 */
declare const PromptListChangedNotificationSchema: z.ZodObject<{
  method: z.ZodLiteral<"notifications/prompts/list_changed">;
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
  }, z.core.$strip>>;
}, z.core.$strip>;
/**
 * Additional properties describing a `Tool` to clients.
 *
 * NOTE: all properties in {@linkcode ToolAnnotations} are **hints**.
 * They are not guaranteed to provide a faithful description of
 * tool behavior (including descriptive properties like `title`).
 *
 * Clients should never make tool use decisions based on `ToolAnnotations`
 * received from untrusted servers.
 */
declare const ToolAnnotationsSchema: z.ZodObject<{
  title: z.ZodOptional<z.ZodString>;
  readOnlyHint: z.ZodOptional<z.ZodBoolean>;
  destructiveHint: z.ZodOptional<z.ZodBoolean>;
  idempotentHint: z.ZodOptional<z.ZodBoolean>;
  openWorldHint: z.ZodOptional<z.ZodBoolean>;
}, z.core.$strip>;
/**
 * Execution-related properties for a tool.
 */
declare const ToolExecutionSchema: z.ZodObject<{
  taskSupport: z.ZodOptional<z.ZodEnum<{
    optional: "optional";
    required: "required";
    forbidden: "forbidden";
  }>>;
}, z.core.$strip>;
/**
 * Definition for a tool the client can call.
 */
declare const ToolSchema: z.ZodObject<{
  description: z.ZodOptional<z.ZodString>;
  inputSchema: z.ZodObject<{
    type: z.ZodLiteral<"object">;
    properties: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodType<JSONValue, JSONValue, z.core.$ZodTypeInternals<JSONValue, JSONValue>>>>;
    required: z.ZodOptional<z.ZodArray<z.ZodString>>;
  }, z.core.$catchall<z.ZodUnknown>>;
  outputSchema: z.ZodOptional<z.ZodObject<{
    $schema: z.ZodOptional<z.ZodString>;
  }, z.core.$loose>>;
  annotations: z.ZodOptional<z.ZodObject<{
    title: z.ZodOptional<z.ZodString>;
    readOnlyHint: z.ZodOptional<z.ZodBoolean>;
    destructiveHint: z.ZodOptional<z.ZodBoolean>;
    idempotentHint: z.ZodOptional<z.ZodBoolean>;
    openWorldHint: z.ZodOptional<z.ZodBoolean>;
  }, z.core.$strip>>;
  execution: z.ZodOptional<z.ZodObject<{
    taskSupport: z.ZodOptional<z.ZodEnum<{
      optional: "optional";
      required: "required";
      forbidden: "forbidden";
    }>>;
  }, z.core.$strip>>;
  _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
    src: z.ZodString;
    mimeType: z.ZodOptional<z.ZodString>;
    sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
    theme: z.ZodOptional<z.ZodEnum<{
      light: "light";
      dark: "dark";
    }>>;
  }, z.core.$strip>>>;
  name: z.ZodString;
  title: z.ZodOptional<z.ZodString>;
}, z.core.$strip>;
/**
 * Sent from the client to request a list of tools the server has.
 */
declare const ListToolsRequestSchema: z.ZodObject<{
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    cursor: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>>;
  method: z.ZodLiteral<"tools/list">;
}, z.core.$strip>;
/**
 * The server's response to a {@linkcode ListToolsRequest | tools/list} request from the client.
 */
declare const ListToolsResultSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  nextCursor: z.ZodOptional<z.ZodString>;
  tools: z.ZodArray<z.ZodObject<{
    description: z.ZodOptional<z.ZodString>;
    inputSchema: z.ZodObject<{
      type: z.ZodLiteral<"object">;
      properties: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodType<JSONValue, JSONValue, z.core.$ZodTypeInternals<JSONValue, JSONValue>>>>;
      required: z.ZodOptional<z.ZodArray<z.ZodString>>;
    }, z.core.$catchall<z.ZodUnknown>>;
    outputSchema: z.ZodOptional<z.ZodObject<{
      $schema: z.ZodOptional<z.ZodString>;
    }, z.core.$loose>>;
    annotations: z.ZodOptional<z.ZodObject<{
      title: z.ZodOptional<z.ZodString>;
      readOnlyHint: z.ZodOptional<z.ZodBoolean>;
      destructiveHint: z.ZodOptional<z.ZodBoolean>;
      idempotentHint: z.ZodOptional<z.ZodBoolean>;
      openWorldHint: z.ZodOptional<z.ZodBoolean>;
    }, z.core.$strip>>;
    execution: z.ZodOptional<z.ZodObject<{
      taskSupport: z.ZodOptional<z.ZodEnum<{
        optional: "optional";
        required: "required";
        forbidden: "forbidden";
      }>>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
      src: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
      theme: z.ZodOptional<z.ZodEnum<{
        light: "light";
        dark: "dark";
      }>>;
    }, z.core.$strip>>>;
    name: z.ZodString;
    title: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>>;
}, z.core.$loose>;
/**
 * The server's response to a tool call.
 */
declare const CallToolResultSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  content: z.ZodDefault<z.ZodArray<z.ZodUnion<readonly [z.ZodObject<{
    type: z.ZodLiteral<"text">;
    text: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"image">;
    data: z.ZodString;
    mimeType: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"audio">;
    data: z.ZodString;
    mimeType: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    uri: z.ZodString;
    description: z.ZodOptional<z.ZodString>;
    mimeType: z.ZodOptional<z.ZodString>;
    size: z.ZodOptional<z.ZodNumber>;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
    icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
      src: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
      theme: z.ZodOptional<z.ZodEnum<{
        light: "light";
        dark: "dark";
      }>>;
    }, z.core.$strip>>>;
    name: z.ZodString;
    title: z.ZodOptional<z.ZodString>;
    type: z.ZodLiteral<"resource_link">;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"resource">;
    resource: z.ZodUnion<readonly [z.ZodObject<{
      uri: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      text: z.ZodString;
    }, z.core.$strip>, z.ZodObject<{
      uri: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      blob: z.ZodString;
    }, z.core.$strip>]>;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>]>>>;
  structuredContent: z.ZodOptional<z.ZodUnknown>;
  isError: z.ZodOptional<z.ZodBoolean>;
}, z.core.$loose>;
/**
 * {@linkcode CallToolResultSchema} extended with backwards compatibility to protocol version 2024-10-07.
 */
declare const CompatibilityCallToolResultSchema: z.ZodUnion<[z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  content: z.ZodDefault<z.ZodArray<z.ZodUnion<readonly [z.ZodObject<{
    type: z.ZodLiteral<"text">;
    text: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"image">;
    data: z.ZodString;
    mimeType: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"audio">;
    data: z.ZodString;
    mimeType: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    uri: z.ZodString;
    description: z.ZodOptional<z.ZodString>;
    mimeType: z.ZodOptional<z.ZodString>;
    size: z.ZodOptional<z.ZodNumber>;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
    icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
      src: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
      theme: z.ZodOptional<z.ZodEnum<{
        light: "light";
        dark: "dark";
      }>>;
    }, z.core.$strip>>>;
    name: z.ZodString;
    title: z.ZodOptional<z.ZodString>;
    type: z.ZodLiteral<"resource_link">;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"resource">;
    resource: z.ZodUnion<readonly [z.ZodObject<{
      uri: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      text: z.ZodString;
    }, z.core.$strip>, z.ZodObject<{
      uri: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      blob: z.ZodString;
    }, z.core.$strip>]>;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>]>>>;
  structuredContent: z.ZodOptional<z.ZodUnknown>;
  isError: z.ZodOptional<z.ZodBoolean>;
}, z.core.$loose>, z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  toolResult: z.ZodUnknown;
}, z.core.$loose>]>;
/**
 * Parameters for a `tools/call` request.
 */
declare const CallToolRequestParamsSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    /**
     * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
     */
    progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
    /**
     * If specified, this request is related to the provided task.
     */
    "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
      taskId: z.ZodString;
    }, z.core.$strip>>;
  }, z.core.$loose>>;
  task: z.ZodOptional<z.ZodObject<{
    ttl: z.ZodOptional<z.ZodNumber>;
  }, z.core.$strip>>;
  name: z.ZodString;
  arguments: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
}, z.core.$strip>;
/**
 * Used by the client to invoke a tool provided by the server.
 */
declare const CallToolRequestSchema: z.ZodObject<{
  method: z.ZodLiteral<"tools/call">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    task: z.ZodOptional<z.ZodObject<{
      ttl: z.ZodOptional<z.ZodNumber>;
    }, z.core.$strip>>;
    name: z.ZodString;
    arguments: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>;
}, z.core.$strip>;
/**
 * An optional notification from the server to the client, informing it that the list of tools it offers has changed. This may be issued by servers without any previous subscription from the client.
 */
declare const ToolListChangedNotificationSchema: z.ZodObject<{
  method: z.ZodLiteral<"notifications/tools/list_changed">;
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
  }, z.core.$strip>>;
}, z.core.$strip>;
/**
 * Base schema for list changed subscription options (without callback).
 * Used internally for Zod validation of `autoRefresh` and `debounceMs`.
 */
declare const ListChangedOptionsBaseSchema: z.ZodObject<{
  autoRefresh: z.ZodDefault<z.ZodBoolean>;
  debounceMs: z.ZodDefault<z.ZodNumber>;
}, z.core.$strip>;
/**
 * The severity of a log message.
 *
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to stderr logging
 * (STDIO servers) or OpenTelemetry.
 */
declare const LoggingLevelSchema: z.ZodEnum<{
  error: "error";
  debug: "debug";
  info: "info";
  notice: "notice";
  warning: "warning";
  critical: "critical";
  alert: "alert";
  emergency: "emergency";
}>;
/**
 * Parameters for a `logging/setLevel` request.
 *
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to stderr logging
 * (STDIO servers) or OpenTelemetry.
 */
declare const SetLevelRequestParamsSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    /**
     * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
     */
    progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
    /**
     * If specified, this request is related to the provided task.
     */
    "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
      taskId: z.ZodString;
    }, z.core.$strip>>;
  }, z.core.$loose>>;
  level: z.ZodEnum<{
    error: "error";
    debug: "debug";
    info: "info";
    notice: "notice";
    warning: "warning";
    critical: "critical";
    alert: "alert";
    emergency: "emergency";
  }>;
}, z.core.$strip>;
/**
 * A request from the client to the server, to enable or adjust logging.
 *
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to stderr logging
 * (STDIO servers) or OpenTelemetry.
 */
declare const SetLevelRequestSchema: z.ZodObject<{
  method: z.ZodLiteral<"logging/setLevel">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    level: z.ZodEnum<{
      error: "error";
      debug: "debug";
      info: "info";
      notice: "notice";
      warning: "warning";
      critical: "critical";
      alert: "alert";
      emergency: "emergency";
    }>;
  }, z.core.$strip>;
}, z.core.$strip>;
/**
 * Parameters for a `notifications/message` notification.
 *
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to stderr logging
 * (STDIO servers) or OpenTelemetry.
 */
declare const LoggingMessageNotificationParamsSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    /**
     * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
     */
    progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
    /**
     * If specified, this request is related to the provided task.
     */
    "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
      taskId: z.ZodString;
    }, z.core.$strip>>;
  }, z.core.$loose>>;
  level: z.ZodEnum<{
    error: "error";
    debug: "debug";
    info: "info";
    notice: "notice";
    warning: "warning";
    critical: "critical";
    alert: "alert";
    emergency: "emergency";
  }>;
  logger: z.ZodOptional<z.ZodString>;
  data: z.ZodUnknown;
}, z.core.$strip>;
/**
 * Notification of a log message passed from server to client. If no `logging/setLevel` request has been sent from the client, the server MAY decide which messages to send automatically.
 *
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to stderr logging
 * (STDIO servers) or OpenTelemetry.
 */
declare const LoggingMessageNotificationSchema: z.ZodObject<{
  method: z.ZodLiteral<"notifications/message">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    level: z.ZodEnum<{
      error: "error";
      debug: "debug";
      info: "info";
      notice: "notice";
      warning: "warning";
      critical: "critical";
      alert: "alert";
      emergency: "emergency";
    }>;
    logger: z.ZodOptional<z.ZodString>;
    data: z.ZodUnknown;
  }, z.core.$strip>;
}, z.core.$strip>;
/**
 * Hints to use for model selection.
 *
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to calling LLM
 * provider APIs directly.
 */
declare const ModelHintSchema: z.ZodObject<{
  name: z.ZodOptional<z.ZodString>;
}, z.core.$strip>;
/**
 * The server's preferences for model selection, requested of the client during sampling.
 *
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to calling LLM
 * provider APIs directly.
 */
declare const ModelPreferencesSchema: z.ZodObject<{
  hints: z.ZodOptional<z.ZodArray<z.ZodObject<{
    name: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>>>;
  costPriority: z.ZodOptional<z.ZodNumber>;
  speedPriority: z.ZodOptional<z.ZodNumber>;
  intelligencePriority: z.ZodOptional<z.ZodNumber>;
}, z.core.$strip>;
/**
 * Controls tool usage behavior in sampling requests.
 *
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to calling LLM
 * provider APIs directly.
 */
declare const ToolChoiceSchema: z.ZodObject<{
  mode: z.ZodOptional<z.ZodEnum<{
    required: "required";
    auto: "auto";
    none: "none";
  }>>;
}, z.core.$strip>;
/**
 * The result of a tool execution, provided by the user (server).
 * Represents the outcome of invoking a tool requested via `ToolUseContent`.
 *
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to calling LLM
 * provider APIs directly.
 */
declare const ToolResultContentSchema: z.ZodObject<{
  type: z.ZodLiteral<"tool_result">;
  toolUseId: z.ZodString;
  content: z.ZodArray<z.ZodUnion<readonly [z.ZodObject<{
    type: z.ZodLiteral<"text">;
    text: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"image">;
    data: z.ZodString;
    mimeType: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"audio">;
    data: z.ZodString;
    mimeType: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    uri: z.ZodString;
    description: z.ZodOptional<z.ZodString>;
    mimeType: z.ZodOptional<z.ZodString>;
    size: z.ZodOptional<z.ZodNumber>;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
    icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
      src: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
      theme: z.ZodOptional<z.ZodEnum<{
        light: "light";
        dark: "dark";
      }>>;
    }, z.core.$strip>>>;
    name: z.ZodString;
    title: z.ZodOptional<z.ZodString>;
    type: z.ZodLiteral<"resource_link">;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"resource">;
    resource: z.ZodUnion<readonly [z.ZodObject<{
      uri: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      text: z.ZodString;
    }, z.core.$strip>, z.ZodObject<{
      uri: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      blob: z.ZodString;
    }, z.core.$strip>]>;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>]>>;
  structuredContent: z.ZodOptional<z.ZodUnknown>;
  isError: z.ZodOptional<z.ZodBoolean>;
  _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
}, z.core.$strip>;
/**
 * Basic content types for sampling responses (without tool use).
 * Used for backwards-compatible {@linkcode CreateMessageResult} when tools are not used.
 *
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to calling LLM
 * provider APIs directly.
 */
declare const SamplingContentSchema: z.ZodDiscriminatedUnion<[z.ZodObject<{
  type: z.ZodLiteral<"text">;
  text: z.ZodString;
  annotations: z.ZodOptional<z.ZodObject<{
    audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
      user: "user";
      assistant: "assistant";
    }>>>;
    priority: z.ZodOptional<z.ZodNumber>;
    lastModified: z.ZodOptional<z.ZodISODateTime>;
  }, z.core.$strip>>;
  _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
}, z.core.$strip>, z.ZodObject<{
  type: z.ZodLiteral<"image">;
  data: z.ZodString;
  mimeType: z.ZodString;
  annotations: z.ZodOptional<z.ZodObject<{
    audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
      user: "user";
      assistant: "assistant";
    }>>>;
    priority: z.ZodOptional<z.ZodNumber>;
    lastModified: z.ZodOptional<z.ZodISODateTime>;
  }, z.core.$strip>>;
  _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
}, z.core.$strip>, z.ZodObject<{
  type: z.ZodLiteral<"audio">;
  data: z.ZodString;
  mimeType: z.ZodString;
  annotations: z.ZodOptional<z.ZodObject<{
    audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
      user: "user";
      assistant: "assistant";
    }>>>;
    priority: z.ZodOptional<z.ZodNumber>;
    lastModified: z.ZodOptional<z.ZodISODateTime>;
  }, z.core.$strip>>;
  _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
}, z.core.$strip>], "type">;
/**
 * Content block types allowed in sampling messages.
 * This includes text, image, audio, tool use requests, and tool results.
 *
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to calling LLM
 * provider APIs directly.
 */
declare const SamplingMessageContentBlockSchema: z.ZodDiscriminatedUnion<[z.ZodObject<{
  type: z.ZodLiteral<"text">;
  text: z.ZodString;
  annotations: z.ZodOptional<z.ZodObject<{
    audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
      user: "user";
      assistant: "assistant";
    }>>>;
    priority: z.ZodOptional<z.ZodNumber>;
    lastModified: z.ZodOptional<z.ZodISODateTime>;
  }, z.core.$strip>>;
  _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
}, z.core.$strip>, z.ZodObject<{
  type: z.ZodLiteral<"image">;
  data: z.ZodString;
  mimeType: z.ZodString;
  annotations: z.ZodOptional<z.ZodObject<{
    audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
      user: "user";
      assistant: "assistant";
    }>>>;
    priority: z.ZodOptional<z.ZodNumber>;
    lastModified: z.ZodOptional<z.ZodISODateTime>;
  }, z.core.$strip>>;
  _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
}, z.core.$strip>, z.ZodObject<{
  type: z.ZodLiteral<"audio">;
  data: z.ZodString;
  mimeType: z.ZodString;
  annotations: z.ZodOptional<z.ZodObject<{
    audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
      user: "user";
      assistant: "assistant";
    }>>>;
    priority: z.ZodOptional<z.ZodNumber>;
    lastModified: z.ZodOptional<z.ZodISODateTime>;
  }, z.core.$strip>>;
  _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
}, z.core.$strip>, z.ZodObject<{
  type: z.ZodLiteral<"tool_use">;
  name: z.ZodString;
  id: z.ZodString;
  input: z.ZodRecord<z.ZodString, z.ZodUnknown>;
  _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
}, z.core.$strip>, z.ZodObject<{
  type: z.ZodLiteral<"tool_result">;
  toolUseId: z.ZodString;
  content: z.ZodArray<z.ZodUnion<readonly [z.ZodObject<{
    type: z.ZodLiteral<"text">;
    text: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"image">;
    data: z.ZodString;
    mimeType: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"audio">;
    data: z.ZodString;
    mimeType: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    uri: z.ZodString;
    description: z.ZodOptional<z.ZodString>;
    mimeType: z.ZodOptional<z.ZodString>;
    size: z.ZodOptional<z.ZodNumber>;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
    icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
      src: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
      theme: z.ZodOptional<z.ZodEnum<{
        light: "light";
        dark: "dark";
      }>>;
    }, z.core.$strip>>>;
    name: z.ZodString;
    title: z.ZodOptional<z.ZodString>;
    type: z.ZodLiteral<"resource_link">;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"resource">;
    resource: z.ZodUnion<readonly [z.ZodObject<{
      uri: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      text: z.ZodString;
    }, z.core.$strip>, z.ZodObject<{
      uri: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      blob: z.ZodString;
    }, z.core.$strip>]>;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>]>>;
  structuredContent: z.ZodOptional<z.ZodUnknown>;
  isError: z.ZodOptional<z.ZodBoolean>;
  _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
}, z.core.$strip>], "type">;
/**
 * Describes a message issued to or received from an LLM API.
 *
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to calling LLM
 * provider APIs directly.
 */
declare const SamplingMessageSchema: z.ZodObject<{
  role: z.ZodEnum<{
    user: "user";
    assistant: "assistant";
  }>;
  content: z.ZodUnion<readonly [z.ZodDiscriminatedUnion<[z.ZodObject<{
    type: z.ZodLiteral<"text">;
    text: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"image">;
    data: z.ZodString;
    mimeType: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"audio">;
    data: z.ZodString;
    mimeType: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"tool_use">;
    name: z.ZodString;
    id: z.ZodString;
    input: z.ZodRecord<z.ZodString, z.ZodUnknown>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"tool_result">;
    toolUseId: z.ZodString;
    content: z.ZodArray<z.ZodUnion<readonly [z.ZodObject<{
      type: z.ZodLiteral<"text">;
      text: z.ZodString;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"image">;
      data: z.ZodString;
      mimeType: z.ZodString;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"audio">;
      data: z.ZodString;
      mimeType: z.ZodString;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      uri: z.ZodString;
      description: z.ZodOptional<z.ZodString>;
      mimeType: z.ZodOptional<z.ZodString>;
      size: z.ZodOptional<z.ZodNumber>;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
      type: z.ZodLiteral<"resource_link">;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"resource">;
      resource: z.ZodUnion<readonly [z.ZodObject<{
        uri: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        text: z.ZodString;
      }, z.core.$strip>, z.ZodObject<{
        uri: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        blob: z.ZodString;
      }, z.core.$strip>]>;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>]>>;
    structuredContent: z.ZodOptional<z.ZodUnknown>;
    isError: z.ZodOptional<z.ZodBoolean>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>], "type">, z.ZodArray<z.ZodDiscriminatedUnion<[z.ZodObject<{
    type: z.ZodLiteral<"text">;
    text: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"image">;
    data: z.ZodString;
    mimeType: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"audio">;
    data: z.ZodString;
    mimeType: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"tool_use">;
    name: z.ZodString;
    id: z.ZodString;
    input: z.ZodRecord<z.ZodString, z.ZodUnknown>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"tool_result">;
    toolUseId: z.ZodString;
    content: z.ZodArray<z.ZodUnion<readonly [z.ZodObject<{
      type: z.ZodLiteral<"text">;
      text: z.ZodString;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"image">;
      data: z.ZodString;
      mimeType: z.ZodString;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"audio">;
      data: z.ZodString;
      mimeType: z.ZodString;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      uri: z.ZodString;
      description: z.ZodOptional<z.ZodString>;
      mimeType: z.ZodOptional<z.ZodString>;
      size: z.ZodOptional<z.ZodNumber>;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
      type: z.ZodLiteral<"resource_link">;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"resource">;
      resource: z.ZodUnion<readonly [z.ZodObject<{
        uri: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        text: z.ZodString;
      }, z.core.$strip>, z.ZodObject<{
        uri: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        blob: z.ZodString;
      }, z.core.$strip>]>;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>]>>;
    structuredContent: z.ZodOptional<z.ZodUnknown>;
    isError: z.ZodOptional<z.ZodBoolean>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>], "type">>]>;
  _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
}, z.core.$strip>;
/**
 * Parameters for a `sampling/createMessage` request.
 *
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to calling LLM
 * provider APIs directly.
 */
declare const CreateMessageRequestParamsSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    /**
     * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
     */
    progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
    /**
     * If specified, this request is related to the provided task.
     */
    "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
      taskId: z.ZodString;
    }, z.core.$strip>>;
  }, z.core.$loose>>;
  task: z.ZodOptional<z.ZodObject<{
    ttl: z.ZodOptional<z.ZodNumber>;
  }, z.core.$strip>>;
  messages: z.ZodArray<z.ZodObject<{
    role: z.ZodEnum<{
      user: "user";
      assistant: "assistant";
    }>;
    content: z.ZodUnion<readonly [z.ZodDiscriminatedUnion<[z.ZodObject<{
      type: z.ZodLiteral<"text">;
      text: z.ZodString;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"image">;
      data: z.ZodString;
      mimeType: z.ZodString;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"audio">;
      data: z.ZodString;
      mimeType: z.ZodString;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"tool_use">;
      name: z.ZodString;
      id: z.ZodString;
      input: z.ZodRecord<z.ZodString, z.ZodUnknown>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"tool_result">;
      toolUseId: z.ZodString;
      content: z.ZodArray<z.ZodUnion<readonly [z.ZodObject<{
        type: z.ZodLiteral<"text">;
        text: z.ZodString;
        annotations: z.ZodOptional<z.ZodObject<{
          audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
            user: "user";
            assistant: "assistant";
          }>>>;
          priority: z.ZodOptional<z.ZodNumber>;
          lastModified: z.ZodOptional<z.ZodISODateTime>;
        }, z.core.$strip>>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      }, z.core.$strip>, z.ZodObject<{
        type: z.ZodLiteral<"image">;
        data: z.ZodString;
        mimeType: z.ZodString;
        annotations: z.ZodOptional<z.ZodObject<{
          audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
            user: "user";
            assistant: "assistant";
          }>>>;
          priority: z.ZodOptional<z.ZodNumber>;
          lastModified: z.ZodOptional<z.ZodISODateTime>;
        }, z.core.$strip>>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      }, z.core.$strip>, z.ZodObject<{
        type: z.ZodLiteral<"audio">;
        data: z.ZodString;
        mimeType: z.ZodString;
        annotations: z.ZodOptional<z.ZodObject<{
          audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
            user: "user";
            assistant: "assistant";
          }>>>;
          priority: z.ZodOptional<z.ZodNumber>;
          lastModified: z.ZodOptional<z.ZodISODateTime>;
        }, z.core.$strip>>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      }, z.core.$strip>, z.ZodObject<{
        uri: z.ZodString;
        description: z.ZodOptional<z.ZodString>;
        mimeType: z.ZodOptional<z.ZodString>;
        size: z.ZodOptional<z.ZodNumber>;
        annotations: z.ZodOptional<z.ZodObject<{
          audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
            user: "user";
            assistant: "assistant";
          }>>>;
          priority: z.ZodOptional<z.ZodNumber>;
          lastModified: z.ZodOptional<z.ZodISODateTime>;
        }, z.core.$strip>>;
        _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
        icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
          src: z.ZodString;
          mimeType: z.ZodOptional<z.ZodString>;
          sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
          theme: z.ZodOptional<z.ZodEnum<{
            light: "light";
            dark: "dark";
          }>>;
        }, z.core.$strip>>>;
        name: z.ZodString;
        title: z.ZodOptional<z.ZodString>;
        type: z.ZodLiteral<"resource_link">;
      }, z.core.$strip>, z.ZodObject<{
        type: z.ZodLiteral<"resource">;
        resource: z.ZodUnion<readonly [z.ZodObject<{
          uri: z.ZodString;
          mimeType: z.ZodOptional<z.ZodString>;
          _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
          text: z.ZodString;
        }, z.core.$strip>, z.ZodObject<{
          uri: z.ZodString;
          mimeType: z.ZodOptional<z.ZodString>;
          _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
          blob: z.ZodString;
        }, z.core.$strip>]>;
        annotations: z.ZodOptional<z.ZodObject<{
          audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
            user: "user";
            assistant: "assistant";
          }>>>;
          priority: z.ZodOptional<z.ZodNumber>;
          lastModified: z.ZodOptional<z.ZodISODateTime>;
        }, z.core.$strip>>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      }, z.core.$strip>]>>;
      structuredContent: z.ZodOptional<z.ZodUnknown>;
      isError: z.ZodOptional<z.ZodBoolean>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>], "type">, z.ZodArray<z.ZodDiscriminatedUnion<[z.ZodObject<{
      type: z.ZodLiteral<"text">;
      text: z.ZodString;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"image">;
      data: z.ZodString;
      mimeType: z.ZodString;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"audio">;
      data: z.ZodString;
      mimeType: z.ZodString;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"tool_use">;
      name: z.ZodString;
      id: z.ZodString;
      input: z.ZodRecord<z.ZodString, z.ZodUnknown>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"tool_result">;
      toolUseId: z.ZodString;
      content: z.ZodArray<z.ZodUnion<readonly [z.ZodObject<{
        type: z.ZodLiteral<"text">;
        text: z.ZodString;
        annotations: z.ZodOptional<z.ZodObject<{
          audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
            user: "user";
            assistant: "assistant";
          }>>>;
          priority: z.ZodOptional<z.ZodNumber>;
          lastModified: z.ZodOptional<z.ZodISODateTime>;
        }, z.core.$strip>>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      }, z.core.$strip>, z.ZodObject<{
        type: z.ZodLiteral<"image">;
        data: z.ZodString;
        mimeType: z.ZodString;
        annotations: z.ZodOptional<z.ZodObject<{
          audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
            user: "user";
            assistant: "assistant";
          }>>>;
          priority: z.ZodOptional<z.ZodNumber>;
          lastModified: z.ZodOptional<z.ZodISODateTime>;
        }, z.core.$strip>>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      }, z.core.$strip>, z.ZodObject<{
        type: z.ZodLiteral<"audio">;
        data: z.ZodString;
        mimeType: z.ZodString;
        annotations: z.ZodOptional<z.ZodObject<{
          audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
            user: "user";
            assistant: "assistant";
          }>>>;
          priority: z.ZodOptional<z.ZodNumber>;
          lastModified: z.ZodOptional<z.ZodISODateTime>;
        }, z.core.$strip>>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      }, z.core.$strip>, z.ZodObject<{
        uri: z.ZodString;
        description: z.ZodOptional<z.ZodString>;
        mimeType: z.ZodOptional<z.ZodString>;
        size: z.ZodOptional<z.ZodNumber>;
        annotations: z.ZodOptional<z.ZodObject<{
          audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
            user: "user";
            assistant: "assistant";
          }>>>;
          priority: z.ZodOptional<z.ZodNumber>;
          lastModified: z.ZodOptional<z.ZodISODateTime>;
        }, z.core.$strip>>;
        _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
        icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
          src: z.ZodString;
          mimeType: z.ZodOptional<z.ZodString>;
          sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
          theme: z.ZodOptional<z.ZodEnum<{
            light: "light";
            dark: "dark";
          }>>;
        }, z.core.$strip>>>;
        name: z.ZodString;
        title: z.ZodOptional<z.ZodString>;
        type: z.ZodLiteral<"resource_link">;
      }, z.core.$strip>, z.ZodObject<{
        type: z.ZodLiteral<"resource">;
        resource: z.ZodUnion<readonly [z.ZodObject<{
          uri: z.ZodString;
          mimeType: z.ZodOptional<z.ZodString>;
          _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
          text: z.ZodString;
        }, z.core.$strip>, z.ZodObject<{
          uri: z.ZodString;
          mimeType: z.ZodOptional<z.ZodString>;
          _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
          blob: z.ZodString;
        }, z.core.$strip>]>;
        annotations: z.ZodOptional<z.ZodObject<{
          audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
            user: "user";
            assistant: "assistant";
          }>>>;
          priority: z.ZodOptional<z.ZodNumber>;
          lastModified: z.ZodOptional<z.ZodISODateTime>;
        }, z.core.$strip>>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      }, z.core.$strip>]>>;
      structuredContent: z.ZodOptional<z.ZodUnknown>;
      isError: z.ZodOptional<z.ZodBoolean>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>], "type">>]>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>>;
  modelPreferences: z.ZodOptional<z.ZodObject<{
    hints: z.ZodOptional<z.ZodArray<z.ZodObject<{
      name: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
    costPriority: z.ZodOptional<z.ZodNumber>;
    speedPriority: z.ZodOptional<z.ZodNumber>;
    intelligencePriority: z.ZodOptional<z.ZodNumber>;
  }, z.core.$strip>>;
  systemPrompt: z.ZodOptional<z.ZodString>;
  includeContext: z.ZodOptional<z.ZodEnum<{
    none: "none";
    thisServer: "thisServer";
    allServers: "allServers";
  }>>;
  temperature: z.ZodOptional<z.ZodNumber>;
  maxTokens: z.ZodNumber;
  stopSequences: z.ZodOptional<z.ZodArray<z.ZodString>>;
  metadata: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
  tools: z.ZodOptional<z.ZodArray<z.ZodObject<{
    description: z.ZodOptional<z.ZodString>;
    inputSchema: z.ZodObject<{
      type: z.ZodLiteral<"object">;
      properties: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodType<JSONValue, JSONValue, z.core.$ZodTypeInternals<JSONValue, JSONValue>>>>;
      required: z.ZodOptional<z.ZodArray<z.ZodString>>;
    }, z.core.$catchall<z.ZodUnknown>>;
    outputSchema: z.ZodOptional<z.ZodObject<{
      $schema: z.ZodOptional<z.ZodString>;
    }, z.core.$loose>>;
    annotations: z.ZodOptional<z.ZodObject<{
      title: z.ZodOptional<z.ZodString>;
      readOnlyHint: z.ZodOptional<z.ZodBoolean>;
      destructiveHint: z.ZodOptional<z.ZodBoolean>;
      idempotentHint: z.ZodOptional<z.ZodBoolean>;
      openWorldHint: z.ZodOptional<z.ZodBoolean>;
    }, z.core.$strip>>;
    execution: z.ZodOptional<z.ZodObject<{
      taskSupport: z.ZodOptional<z.ZodEnum<{
        optional: "optional";
        required: "required";
        forbidden: "forbidden";
      }>>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
      src: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
      theme: z.ZodOptional<z.ZodEnum<{
        light: "light";
        dark: "dark";
      }>>;
    }, z.core.$strip>>>;
    name: z.ZodString;
    title: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>>>;
  toolChoice: z.ZodOptional<z.ZodObject<{
    mode: z.ZodOptional<z.ZodEnum<{
      required: "required";
      auto: "auto";
      none: "none";
    }>>;
  }, z.core.$strip>>;
}, z.core.$strip>;
/**
 * A request from the server to sample an LLM via the client. The client has full discretion over which model to select. The client should also inform the user before beginning sampling, to allow them to inspect the request (human in the loop) and decide whether to approve it.
 *
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to calling LLM
 * provider APIs directly.
 */
declare const CreateMessageRequestSchema: z.ZodObject<{
  method: z.ZodLiteral<"sampling/createMessage">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    task: z.ZodOptional<z.ZodObject<{
      ttl: z.ZodOptional<z.ZodNumber>;
    }, z.core.$strip>>;
    messages: z.ZodArray<z.ZodObject<{
      role: z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>;
      content: z.ZodUnion<readonly [z.ZodDiscriminatedUnion<[z.ZodObject<{
        type: z.ZodLiteral<"text">;
        text: z.ZodString;
        annotations: z.ZodOptional<z.ZodObject<{
          audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
            user: "user";
            assistant: "assistant";
          }>>>;
          priority: z.ZodOptional<z.ZodNumber>;
          lastModified: z.ZodOptional<z.ZodISODateTime>;
        }, z.core.$strip>>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      }, z.core.$strip>, z.ZodObject<{
        type: z.ZodLiteral<"image">;
        data: z.ZodString;
        mimeType: z.ZodString;
        annotations: z.ZodOptional<z.ZodObject<{
          audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
            user: "user";
            assistant: "assistant";
          }>>>;
          priority: z.ZodOptional<z.ZodNumber>;
          lastModified: z.ZodOptional<z.ZodISODateTime>;
        }, z.core.$strip>>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      }, z.core.$strip>, z.ZodObject<{
        type: z.ZodLiteral<"audio">;
        data: z.ZodString;
        mimeType: z.ZodString;
        annotations: z.ZodOptional<z.ZodObject<{
          audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
            user: "user";
            assistant: "assistant";
          }>>>;
          priority: z.ZodOptional<z.ZodNumber>;
          lastModified: z.ZodOptional<z.ZodISODateTime>;
        }, z.core.$strip>>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      }, z.core.$strip>, z.ZodObject<{
        type: z.ZodLiteral<"tool_use">;
        name: z.ZodString;
        id: z.ZodString;
        input: z.ZodRecord<z.ZodString, z.ZodUnknown>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      }, z.core.$strip>, z.ZodObject<{
        type: z.ZodLiteral<"tool_result">;
        toolUseId: z.ZodString;
        content: z.ZodArray<z.ZodUnion<readonly [z.ZodObject<{
          type: z.ZodLiteral<"text">;
          text: z.ZodString;
          annotations: z.ZodOptional<z.ZodObject<{
            audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
              user: "user";
              assistant: "assistant";
            }>>>;
            priority: z.ZodOptional<z.ZodNumber>;
            lastModified: z.ZodOptional<z.ZodISODateTime>;
          }, z.core.$strip>>;
          _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        }, z.core.$strip>, z.ZodObject<{
          type: z.ZodLiteral<"image">;
          data: z.ZodString;
          mimeType: z.ZodString;
          annotations: z.ZodOptional<z.ZodObject<{
            audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
              user: "user";
              assistant: "assistant";
            }>>>;
            priority: z.ZodOptional<z.ZodNumber>;
            lastModified: z.ZodOptional<z.ZodISODateTime>;
          }, z.core.$strip>>;
          _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        }, z.core.$strip>, z.ZodObject<{
          type: z.ZodLiteral<"audio">;
          data: z.ZodString;
          mimeType: z.ZodString;
          annotations: z.ZodOptional<z.ZodObject<{
            audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
              user: "user";
              assistant: "assistant";
            }>>>;
            priority: z.ZodOptional<z.ZodNumber>;
            lastModified: z.ZodOptional<z.ZodISODateTime>;
          }, z.core.$strip>>;
          _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        }, z.core.$strip>, z.ZodObject<{
          uri: z.ZodString;
          description: z.ZodOptional<z.ZodString>;
          mimeType: z.ZodOptional<z.ZodString>;
          size: z.ZodOptional<z.ZodNumber>;
          annotations: z.ZodOptional<z.ZodObject<{
            audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
              user: "user";
              assistant: "assistant";
            }>>>;
            priority: z.ZodOptional<z.ZodNumber>;
            lastModified: z.ZodOptional<z.ZodISODateTime>;
          }, z.core.$strip>>;
          _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
          icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
            src: z.ZodString;
            mimeType: z.ZodOptional<z.ZodString>;
            sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
            theme: z.ZodOptional<z.ZodEnum<{
              light: "light";
              dark: "dark";
            }>>;
          }, z.core.$strip>>>;
          name: z.ZodString;
          title: z.ZodOptional<z.ZodString>;
          type: z.ZodLiteral<"resource_link">;
        }, z.core.$strip>, z.ZodObject<{
          type: z.ZodLiteral<"resource">;
          resource: z.ZodUnion<readonly [z.ZodObject<{
            uri: z.ZodString;
            mimeType: z.ZodOptional<z.ZodString>;
            _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
            text: z.ZodString;
          }, z.core.$strip>, z.ZodObject<{
            uri: z.ZodString;
            mimeType: z.ZodOptional<z.ZodString>;
            _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
            blob: z.ZodString;
          }, z.core.$strip>]>;
          annotations: z.ZodOptional<z.ZodObject<{
            audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
              user: "user";
              assistant: "assistant";
            }>>>;
            priority: z.ZodOptional<z.ZodNumber>;
            lastModified: z.ZodOptional<z.ZodISODateTime>;
          }, z.core.$strip>>;
          _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        }, z.core.$strip>]>>;
        structuredContent: z.ZodOptional<z.ZodUnknown>;
        isError: z.ZodOptional<z.ZodBoolean>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      }, z.core.$strip>], "type">, z.ZodArray<z.ZodDiscriminatedUnion<[z.ZodObject<{
        type: z.ZodLiteral<"text">;
        text: z.ZodString;
        annotations: z.ZodOptional<z.ZodObject<{
          audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
            user: "user";
            assistant: "assistant";
          }>>>;
          priority: z.ZodOptional<z.ZodNumber>;
          lastModified: z.ZodOptional<z.ZodISODateTime>;
        }, z.core.$strip>>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      }, z.core.$strip>, z.ZodObject<{
        type: z.ZodLiteral<"image">;
        data: z.ZodString;
        mimeType: z.ZodString;
        annotations: z.ZodOptional<z.ZodObject<{
          audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
            user: "user";
            assistant: "assistant";
          }>>>;
          priority: z.ZodOptional<z.ZodNumber>;
          lastModified: z.ZodOptional<z.ZodISODateTime>;
        }, z.core.$strip>>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      }, z.core.$strip>, z.ZodObject<{
        type: z.ZodLiteral<"audio">;
        data: z.ZodString;
        mimeType: z.ZodString;
        annotations: z.ZodOptional<z.ZodObject<{
          audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
            user: "user";
            assistant: "assistant";
          }>>>;
          priority: z.ZodOptional<z.ZodNumber>;
          lastModified: z.ZodOptional<z.ZodISODateTime>;
        }, z.core.$strip>>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      }, z.core.$strip>, z.ZodObject<{
        type: z.ZodLiteral<"tool_use">;
        name: z.ZodString;
        id: z.ZodString;
        input: z.ZodRecord<z.ZodString, z.ZodUnknown>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      }, z.core.$strip>, z.ZodObject<{
        type: z.ZodLiteral<"tool_result">;
        toolUseId: z.ZodString;
        content: z.ZodArray<z.ZodUnion<readonly [z.ZodObject<{
          type: z.ZodLiteral<"text">;
          text: z.ZodString;
          annotations: z.ZodOptional<z.ZodObject<{
            audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
              user: "user";
              assistant: "assistant";
            }>>>;
            priority: z.ZodOptional<z.ZodNumber>;
            lastModified: z.ZodOptional<z.ZodISODateTime>;
          }, z.core.$strip>>;
          _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        }, z.core.$strip>, z.ZodObject<{
          type: z.ZodLiteral<"image">;
          data: z.ZodString;
          mimeType: z.ZodString;
          annotations: z.ZodOptional<z.ZodObject<{
            audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
              user: "user";
              assistant: "assistant";
            }>>>;
            priority: z.ZodOptional<z.ZodNumber>;
            lastModified: z.ZodOptional<z.ZodISODateTime>;
          }, z.core.$strip>>;
          _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        }, z.core.$strip>, z.ZodObject<{
          type: z.ZodLiteral<"audio">;
          data: z.ZodString;
          mimeType: z.ZodString;
          annotations: z.ZodOptional<z.ZodObject<{
            audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
              user: "user";
              assistant: "assistant";
            }>>>;
            priority: z.ZodOptional<z.ZodNumber>;
            lastModified: z.ZodOptional<z.ZodISODateTime>;
          }, z.core.$strip>>;
          _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        }, z.core.$strip>, z.ZodObject<{
          uri: z.ZodString;
          description: z.ZodOptional<z.ZodString>;
          mimeType: z.ZodOptional<z.ZodString>;
          size: z.ZodOptional<z.ZodNumber>;
          annotations: z.ZodOptional<z.ZodObject<{
            audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
              user: "user";
              assistant: "assistant";
            }>>>;
            priority: z.ZodOptional<z.ZodNumber>;
            lastModified: z.ZodOptional<z.ZodISODateTime>;
          }, z.core.$strip>>;
          _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
          icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
            src: z.ZodString;
            mimeType: z.ZodOptional<z.ZodString>;
            sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
            theme: z.ZodOptional<z.ZodEnum<{
              light: "light";
              dark: "dark";
            }>>;
          }, z.core.$strip>>>;
          name: z.ZodString;
          title: z.ZodOptional<z.ZodString>;
          type: z.ZodLiteral<"resource_link">;
        }, z.core.$strip>, z.ZodObject<{
          type: z.ZodLiteral<"resource">;
          resource: z.ZodUnion<readonly [z.ZodObject<{
            uri: z.ZodString;
            mimeType: z.ZodOptional<z.ZodString>;
            _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
            text: z.ZodString;
          }, z.core.$strip>, z.ZodObject<{
            uri: z.ZodString;
            mimeType: z.ZodOptional<z.ZodString>;
            _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
            blob: z.ZodString;
          }, z.core.$strip>]>;
          annotations: z.ZodOptional<z.ZodObject<{
            audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
              user: "user";
              assistant: "assistant";
            }>>>;
            priority: z.ZodOptional<z.ZodNumber>;
            lastModified: z.ZodOptional<z.ZodISODateTime>;
          }, z.core.$strip>>;
          _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        }, z.core.$strip>]>>;
        structuredContent: z.ZodOptional<z.ZodUnknown>;
        isError: z.ZodOptional<z.ZodBoolean>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      }, z.core.$strip>], "type">>]>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>>;
    modelPreferences: z.ZodOptional<z.ZodObject<{
      hints: z.ZodOptional<z.ZodArray<z.ZodObject<{
        name: z.ZodOptional<z.ZodString>;
      }, z.core.$strip>>>;
      costPriority: z.ZodOptional<z.ZodNumber>;
      speedPriority: z.ZodOptional<z.ZodNumber>;
      intelligencePriority: z.ZodOptional<z.ZodNumber>;
    }, z.core.$strip>>;
    systemPrompt: z.ZodOptional<z.ZodString>;
    includeContext: z.ZodOptional<z.ZodEnum<{
      none: "none";
      thisServer: "thisServer";
      allServers: "allServers";
    }>>;
    temperature: z.ZodOptional<z.ZodNumber>;
    maxTokens: z.ZodNumber;
    stopSequences: z.ZodOptional<z.ZodArray<z.ZodString>>;
    metadata: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
    tools: z.ZodOptional<z.ZodArray<z.ZodObject<{
      description: z.ZodOptional<z.ZodString>;
      inputSchema: z.ZodObject<{
        type: z.ZodLiteral<"object">;
        properties: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodType<JSONValue, JSONValue, z.core.$ZodTypeInternals<JSONValue, JSONValue>>>>;
        required: z.ZodOptional<z.ZodArray<z.ZodString>>;
      }, z.core.$catchall<z.ZodUnknown>>;
      outputSchema: z.ZodOptional<z.ZodObject<{
        $schema: z.ZodOptional<z.ZodString>;
      }, z.core.$loose>>;
      annotations: z.ZodOptional<z.ZodObject<{
        title: z.ZodOptional<z.ZodString>;
        readOnlyHint: z.ZodOptional<z.ZodBoolean>;
        destructiveHint: z.ZodOptional<z.ZodBoolean>;
        idempotentHint: z.ZodOptional<z.ZodBoolean>;
        openWorldHint: z.ZodOptional<z.ZodBoolean>;
      }, z.core.$strip>>;
      execution: z.ZodOptional<z.ZodObject<{
        taskSupport: z.ZodOptional<z.ZodEnum<{
          optional: "optional";
          required: "required";
          forbidden: "forbidden";
        }>>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
    toolChoice: z.ZodOptional<z.ZodObject<{
      mode: z.ZodOptional<z.ZodEnum<{
        required: "required";
        auto: "auto";
        none: "none";
      }>>;
    }, z.core.$strip>>;
  }, z.core.$strip>;
}, z.core.$strip>;
/**
 * The client's response to a `sampling/create_message` request from the server.
 * This is the backwards-compatible version that returns single content (no arrays).
 * Used when the request does not include tools.
 *
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to calling LLM
 * provider APIs directly.
 */
declare const CreateMessageResultSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  model: z.ZodString;
  stopReason: z.ZodOptional<z.ZodUnion<[z.ZodEnum<{
    maxTokens: "maxTokens";
    endTurn: "endTurn";
    stopSequence: "stopSequence";
  }>, z.ZodString]>>;
  role: z.ZodEnum<{
    user: "user";
    assistant: "assistant";
  }>;
  content: z.ZodDiscriminatedUnion<[z.ZodObject<{
    type: z.ZodLiteral<"text">;
    text: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"image">;
    data: z.ZodString;
    mimeType: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"audio">;
    data: z.ZodString;
    mimeType: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>], "type">;
}, z.core.$loose>;
/**
 * The client's response to a `sampling/create_message` request when tools were provided.
 * This version supports array content for tool use flows.
 *
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to calling LLM
 * provider APIs directly.
 */
declare const CreateMessageResultWithToolsSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  model: z.ZodString;
  stopReason: z.ZodOptional<z.ZodUnion<[z.ZodEnum<{
    maxTokens: "maxTokens";
    endTurn: "endTurn";
    stopSequence: "stopSequence";
    toolUse: "toolUse";
  }>, z.ZodString]>>;
  role: z.ZodEnum<{
    user: "user";
    assistant: "assistant";
  }>;
  content: z.ZodUnion<readonly [z.ZodDiscriminatedUnion<[z.ZodObject<{
    type: z.ZodLiteral<"text">;
    text: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"image">;
    data: z.ZodString;
    mimeType: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"audio">;
    data: z.ZodString;
    mimeType: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"tool_use">;
    name: z.ZodString;
    id: z.ZodString;
    input: z.ZodRecord<z.ZodString, z.ZodUnknown>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"tool_result">;
    toolUseId: z.ZodString;
    content: z.ZodArray<z.ZodUnion<readonly [z.ZodObject<{
      type: z.ZodLiteral<"text">;
      text: z.ZodString;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"image">;
      data: z.ZodString;
      mimeType: z.ZodString;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"audio">;
      data: z.ZodString;
      mimeType: z.ZodString;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      uri: z.ZodString;
      description: z.ZodOptional<z.ZodString>;
      mimeType: z.ZodOptional<z.ZodString>;
      size: z.ZodOptional<z.ZodNumber>;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
      type: z.ZodLiteral<"resource_link">;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"resource">;
      resource: z.ZodUnion<readonly [z.ZodObject<{
        uri: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        text: z.ZodString;
      }, z.core.$strip>, z.ZodObject<{
        uri: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        blob: z.ZodString;
      }, z.core.$strip>]>;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>]>>;
    structuredContent: z.ZodOptional<z.ZodUnknown>;
    isError: z.ZodOptional<z.ZodBoolean>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>], "type">, z.ZodArray<z.ZodDiscriminatedUnion<[z.ZodObject<{
    type: z.ZodLiteral<"text">;
    text: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"image">;
    data: z.ZodString;
    mimeType: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"audio">;
    data: z.ZodString;
    mimeType: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"tool_use">;
    name: z.ZodString;
    id: z.ZodString;
    input: z.ZodRecord<z.ZodString, z.ZodUnknown>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"tool_result">;
    toolUseId: z.ZodString;
    content: z.ZodArray<z.ZodUnion<readonly [z.ZodObject<{
      type: z.ZodLiteral<"text">;
      text: z.ZodString;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"image">;
      data: z.ZodString;
      mimeType: z.ZodString;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"audio">;
      data: z.ZodString;
      mimeType: z.ZodString;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      uri: z.ZodString;
      description: z.ZodOptional<z.ZodString>;
      mimeType: z.ZodOptional<z.ZodString>;
      size: z.ZodOptional<z.ZodNumber>;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
      type: z.ZodLiteral<"resource_link">;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"resource">;
      resource: z.ZodUnion<readonly [z.ZodObject<{
        uri: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        text: z.ZodString;
      }, z.core.$strip>, z.ZodObject<{
        uri: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        blob: z.ZodString;
      }, z.core.$strip>]>;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>]>>;
    structuredContent: z.ZodOptional<z.ZodUnknown>;
    isError: z.ZodOptional<z.ZodBoolean>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>], "type">>]>;
}, z.core.$loose>;
/**
 * Primitive schema definition for boolean fields.
 */
declare const BooleanSchemaSchema: z.ZodObject<{
  type: z.ZodLiteral<"boolean">;
  title: z.ZodOptional<z.ZodString>;
  description: z.ZodOptional<z.ZodString>;
  default: z.ZodOptional<z.ZodBoolean>;
}, z.core.$strip>;
/**
 * Primitive schema definition for string fields.
 */
declare const StringSchemaSchema: z.ZodObject<{
  type: z.ZodLiteral<"string">;
  title: z.ZodOptional<z.ZodString>;
  description: z.ZodOptional<z.ZodString>;
  minLength: z.ZodOptional<z.ZodNumber>;
  maxLength: z.ZodOptional<z.ZodNumber>;
  format: z.ZodOptional<z.ZodEnum<{
    date: "date";
    uri: "uri";
    email: "email";
    "date-time": "date-time";
  }>>;
  default: z.ZodOptional<z.ZodString>;
}, z.core.$strip>;
/**
 * Primitive schema definition for number fields.
 */
declare const NumberSchemaSchema: z.ZodObject<{
  type: z.ZodEnum<{
    number: "number";
    integer: "integer";
  }>;
  title: z.ZodOptional<z.ZodString>;
  description: z.ZodOptional<z.ZodString>;
  minimum: z.ZodOptional<z.ZodNumber>;
  maximum: z.ZodOptional<z.ZodNumber>;
  default: z.ZodOptional<z.ZodNumber>;
}, z.core.$strip>;
/**
 * Schema for single-selection enumeration without display titles for options.
 */
declare const UntitledSingleSelectEnumSchemaSchema: z.ZodObject<{
  type: z.ZodLiteral<"string">;
  title: z.ZodOptional<z.ZodString>;
  description: z.ZodOptional<z.ZodString>;
  enum: z.ZodArray<z.ZodString>;
  default: z.ZodOptional<z.ZodString>;
}, z.core.$strip>;
/**
 * Schema for single-selection enumeration with display titles for each option.
 */
declare const TitledSingleSelectEnumSchemaSchema: z.ZodObject<{
  type: z.ZodLiteral<"string">;
  title: z.ZodOptional<z.ZodString>;
  description: z.ZodOptional<z.ZodString>;
  oneOf: z.ZodArray<z.ZodObject<{
    const: z.ZodString;
    title: z.ZodString;
  }, z.core.$strip>>;
  default: z.ZodOptional<z.ZodString>;
}, z.core.$strip>;
/**
 * Use {@linkcode TitledSingleSelectEnumSchema} instead.
 * This interface will be removed in a future version.
 */
declare const LegacyTitledEnumSchemaSchema: z.ZodObject<{
  type: z.ZodLiteral<"string">;
  title: z.ZodOptional<z.ZodString>;
  description: z.ZodOptional<z.ZodString>;
  enum: z.ZodArray<z.ZodString>;
  enumNames: z.ZodOptional<z.ZodArray<z.ZodString>>;
  default: z.ZodOptional<z.ZodString>;
}, z.core.$strip>;
declare const SingleSelectEnumSchemaSchema: z.ZodUnion<readonly [z.ZodObject<{
  type: z.ZodLiteral<"string">;
  title: z.ZodOptional<z.ZodString>;
  description: z.ZodOptional<z.ZodString>;
  enum: z.ZodArray<z.ZodString>;
  default: z.ZodOptional<z.ZodString>;
}, z.core.$strip>, z.ZodObject<{
  type: z.ZodLiteral<"string">;
  title: z.ZodOptional<z.ZodString>;
  description: z.ZodOptional<z.ZodString>;
  oneOf: z.ZodArray<z.ZodObject<{
    const: z.ZodString;
    title: z.ZodString;
  }, z.core.$strip>>;
  default: z.ZodOptional<z.ZodString>;
}, z.core.$strip>]>;
/**
 * Schema for multiple-selection enumeration without display titles for options.
 */
declare const UntitledMultiSelectEnumSchemaSchema: z.ZodObject<{
  type: z.ZodLiteral<"array">;
  title: z.ZodOptional<z.ZodString>;
  description: z.ZodOptional<z.ZodString>;
  minItems: z.ZodOptional<z.ZodNumber>;
  maxItems: z.ZodOptional<z.ZodNumber>;
  items: z.ZodObject<{
    type: z.ZodLiteral<"string">;
    enum: z.ZodArray<z.ZodString>;
  }, z.core.$strip>;
  default: z.ZodOptional<z.ZodArray<z.ZodString>>;
}, z.core.$strip>;
/**
 * Schema for multiple-selection enumeration with display titles for each option.
 */
declare const TitledMultiSelectEnumSchemaSchema: z.ZodObject<{
  type: z.ZodLiteral<"array">;
  title: z.ZodOptional<z.ZodString>;
  description: z.ZodOptional<z.ZodString>;
  minItems: z.ZodOptional<z.ZodNumber>;
  maxItems: z.ZodOptional<z.ZodNumber>;
  items: z.ZodObject<{
    anyOf: z.ZodArray<z.ZodObject<{
      const: z.ZodString;
      title: z.ZodString;
    }, z.core.$strip>>;
  }, z.core.$strip>;
  default: z.ZodOptional<z.ZodArray<z.ZodString>>;
}, z.core.$strip>;
/**
 * Combined schema for multiple-selection enumeration
 */
declare const MultiSelectEnumSchemaSchema: z.ZodUnion<readonly [z.ZodObject<{
  type: z.ZodLiteral<"array">;
  title: z.ZodOptional<z.ZodString>;
  description: z.ZodOptional<z.ZodString>;
  minItems: z.ZodOptional<z.ZodNumber>;
  maxItems: z.ZodOptional<z.ZodNumber>;
  items: z.ZodObject<{
    type: z.ZodLiteral<"string">;
    enum: z.ZodArray<z.ZodString>;
  }, z.core.$strip>;
  default: z.ZodOptional<z.ZodArray<z.ZodString>>;
}, z.core.$strip>, z.ZodObject<{
  type: z.ZodLiteral<"array">;
  title: z.ZodOptional<z.ZodString>;
  description: z.ZodOptional<z.ZodString>;
  minItems: z.ZodOptional<z.ZodNumber>;
  maxItems: z.ZodOptional<z.ZodNumber>;
  items: z.ZodObject<{
    anyOf: z.ZodArray<z.ZodObject<{
      const: z.ZodString;
      title: z.ZodString;
    }, z.core.$strip>>;
  }, z.core.$strip>;
  default: z.ZodOptional<z.ZodArray<z.ZodString>>;
}, z.core.$strip>]>;
/**
 * Primitive schema definition for enum fields.
 */
declare const EnumSchemaSchema: z.ZodUnion<readonly [z.ZodObject<{
  type: z.ZodLiteral<"string">;
  title: z.ZodOptional<z.ZodString>;
  description: z.ZodOptional<z.ZodString>;
  enum: z.ZodArray<z.ZodString>;
  enumNames: z.ZodOptional<z.ZodArray<z.ZodString>>;
  default: z.ZodOptional<z.ZodString>;
}, z.core.$strip>, z.ZodUnion<readonly [z.ZodObject<{
  type: z.ZodLiteral<"string">;
  title: z.ZodOptional<z.ZodString>;
  description: z.ZodOptional<z.ZodString>;
  enum: z.ZodArray<z.ZodString>;
  default: z.ZodOptional<z.ZodString>;
}, z.core.$strip>, z.ZodObject<{
  type: z.ZodLiteral<"string">;
  title: z.ZodOptional<z.ZodString>;
  description: z.ZodOptional<z.ZodString>;
  oneOf: z.ZodArray<z.ZodObject<{
    const: z.ZodString;
    title: z.ZodString;
  }, z.core.$strip>>;
  default: z.ZodOptional<z.ZodString>;
}, z.core.$strip>]>, z.ZodUnion<readonly [z.ZodObject<{
  type: z.ZodLiteral<"array">;
  title: z.ZodOptional<z.ZodString>;
  description: z.ZodOptional<z.ZodString>;
  minItems: z.ZodOptional<z.ZodNumber>;
  maxItems: z.ZodOptional<z.ZodNumber>;
  items: z.ZodObject<{
    type: z.ZodLiteral<"string">;
    enum: z.ZodArray<z.ZodString>;
  }, z.core.$strip>;
  default: z.ZodOptional<z.ZodArray<z.ZodString>>;
}, z.core.$strip>, z.ZodObject<{
  type: z.ZodLiteral<"array">;
  title: z.ZodOptional<z.ZodString>;
  description: z.ZodOptional<z.ZodString>;
  minItems: z.ZodOptional<z.ZodNumber>;
  maxItems: z.ZodOptional<z.ZodNumber>;
  items: z.ZodObject<{
    anyOf: z.ZodArray<z.ZodObject<{
      const: z.ZodString;
      title: z.ZodString;
    }, z.core.$strip>>;
  }, z.core.$strip>;
  default: z.ZodOptional<z.ZodArray<z.ZodString>>;
}, z.core.$strip>]>]>;
/**
 * Union of all primitive schema definitions.
 */
declare const PrimitiveSchemaDefinitionSchema: z.ZodUnion<readonly [z.ZodUnion<readonly [z.ZodObject<{
  type: z.ZodLiteral<"string">;
  title: z.ZodOptional<z.ZodString>;
  description: z.ZodOptional<z.ZodString>;
  enum: z.ZodArray<z.ZodString>;
  enumNames: z.ZodOptional<z.ZodArray<z.ZodString>>;
  default: z.ZodOptional<z.ZodString>;
}, z.core.$strip>, z.ZodUnion<readonly [z.ZodObject<{
  type: z.ZodLiteral<"string">;
  title: z.ZodOptional<z.ZodString>;
  description: z.ZodOptional<z.ZodString>;
  enum: z.ZodArray<z.ZodString>;
  default: z.ZodOptional<z.ZodString>;
}, z.core.$strip>, z.ZodObject<{
  type: z.ZodLiteral<"string">;
  title: z.ZodOptional<z.ZodString>;
  description: z.ZodOptional<z.ZodString>;
  oneOf: z.ZodArray<z.ZodObject<{
    const: z.ZodString;
    title: z.ZodString;
  }, z.core.$strip>>;
  default: z.ZodOptional<z.ZodString>;
}, z.core.$strip>]>, z.ZodUnion<readonly [z.ZodObject<{
  type: z.ZodLiteral<"array">;
  title: z.ZodOptional<z.ZodString>;
  description: z.ZodOptional<z.ZodString>;
  minItems: z.ZodOptional<z.ZodNumber>;
  maxItems: z.ZodOptional<z.ZodNumber>;
  items: z.ZodObject<{
    type: z.ZodLiteral<"string">;
    enum: z.ZodArray<z.ZodString>;
  }, z.core.$strip>;
  default: z.ZodOptional<z.ZodArray<z.ZodString>>;
}, z.core.$strip>, z.ZodObject<{
  type: z.ZodLiteral<"array">;
  title: z.ZodOptional<z.ZodString>;
  description: z.ZodOptional<z.ZodString>;
  minItems: z.ZodOptional<z.ZodNumber>;
  maxItems: z.ZodOptional<z.ZodNumber>;
  items: z.ZodObject<{
    anyOf: z.ZodArray<z.ZodObject<{
      const: z.ZodString;
      title: z.ZodString;
    }, z.core.$strip>>;
  }, z.core.$strip>;
  default: z.ZodOptional<z.ZodArray<z.ZodString>>;
}, z.core.$strip>]>]>, z.ZodObject<{
  type: z.ZodLiteral<"boolean">;
  title: z.ZodOptional<z.ZodString>;
  description: z.ZodOptional<z.ZodString>;
  default: z.ZodOptional<z.ZodBoolean>;
}, z.core.$strip>, z.ZodObject<{
  type: z.ZodLiteral<"string">;
  title: z.ZodOptional<z.ZodString>;
  description: z.ZodOptional<z.ZodString>;
  minLength: z.ZodOptional<z.ZodNumber>;
  maxLength: z.ZodOptional<z.ZodNumber>;
  format: z.ZodOptional<z.ZodEnum<{
    date: "date";
    uri: "uri";
    email: "email";
    "date-time": "date-time";
  }>>;
  default: z.ZodOptional<z.ZodString>;
}, z.core.$strip>, z.ZodObject<{
  type: z.ZodEnum<{
    number: "number";
    integer: "integer";
  }>;
  title: z.ZodOptional<z.ZodString>;
  description: z.ZodOptional<z.ZodString>;
  minimum: z.ZodOptional<z.ZodNumber>;
  maximum: z.ZodOptional<z.ZodNumber>;
  default: z.ZodOptional<z.ZodNumber>;
}, z.core.$strip>]>;
/**
 * Parameters for an `elicitation/create` request for form-based elicitation.
 */
declare const ElicitRequestFormParamsSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    /**
     * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
     */
    progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
    /**
     * If specified, this request is related to the provided task.
     */
    "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
      taskId: z.ZodString;
    }, z.core.$strip>>;
  }, z.core.$loose>>;
  task: z.ZodOptional<z.ZodObject<{
    ttl: z.ZodOptional<z.ZodNumber>;
  }, z.core.$strip>>;
  mode: z.ZodOptional<z.ZodLiteral<"form">>;
  message: z.ZodString;
  requestedSchema: z.ZodObject<{
    type: z.ZodLiteral<"object">;
    properties: z.ZodRecord<z.ZodString, z.ZodUnion<readonly [z.ZodUnion<readonly [z.ZodObject<{
      type: z.ZodLiteral<"string">;
      title: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      enum: z.ZodArray<z.ZodString>;
      enumNames: z.ZodOptional<z.ZodArray<z.ZodString>>;
      default: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>, z.ZodUnion<readonly [z.ZodObject<{
      type: z.ZodLiteral<"string">;
      title: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      enum: z.ZodArray<z.ZodString>;
      default: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"string">;
      title: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      oneOf: z.ZodArray<z.ZodObject<{
        const: z.ZodString;
        title: z.ZodString;
      }, z.core.$strip>>;
      default: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>]>, z.ZodUnion<readonly [z.ZodObject<{
      type: z.ZodLiteral<"array">;
      title: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      minItems: z.ZodOptional<z.ZodNumber>;
      maxItems: z.ZodOptional<z.ZodNumber>;
      items: z.ZodObject<{
        type: z.ZodLiteral<"string">;
        enum: z.ZodArray<z.ZodString>;
      }, z.core.$strip>;
      default: z.ZodOptional<z.ZodArray<z.ZodString>>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"array">;
      title: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      minItems: z.ZodOptional<z.ZodNumber>;
      maxItems: z.ZodOptional<z.ZodNumber>;
      items: z.ZodObject<{
        anyOf: z.ZodArray<z.ZodObject<{
          const: z.ZodString;
          title: z.ZodString;
        }, z.core.$strip>>;
      }, z.core.$strip>;
      default: z.ZodOptional<z.ZodArray<z.ZodString>>;
    }, z.core.$strip>]>]>, z.ZodObject<{
      type: z.ZodLiteral<"boolean">;
      title: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      default: z.ZodOptional<z.ZodBoolean>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"string">;
      title: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      minLength: z.ZodOptional<z.ZodNumber>;
      maxLength: z.ZodOptional<z.ZodNumber>;
      format: z.ZodOptional<z.ZodEnum<{
        date: "date";
        uri: "uri";
        email: "email";
        "date-time": "date-time";
      }>>;
      default: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodEnum<{
        number: "number";
        integer: "integer";
      }>;
      title: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      minimum: z.ZodOptional<z.ZodNumber>;
      maximum: z.ZodOptional<z.ZodNumber>;
      default: z.ZodOptional<z.ZodNumber>;
    }, z.core.$strip>]>>;
    required: z.ZodOptional<z.ZodArray<z.ZodString>>;
  }, z.core.$catchall<z.ZodUnknown>>;
}, z.core.$strip>;
/**
 * Parameters for an {@linkcode ElicitRequest | elicitation/create} request for URL-based elicitation.
 */
declare const ElicitRequestURLParamsSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    /**
     * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
     */
    progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
    /**
     * If specified, this request is related to the provided task.
     */
    "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
      taskId: z.ZodString;
    }, z.core.$strip>>;
  }, z.core.$loose>>;
  task: z.ZodOptional<z.ZodObject<{
    ttl: z.ZodOptional<z.ZodNumber>;
  }, z.core.$strip>>;
  mode: z.ZodLiteral<"url">;
  message: z.ZodString;
  elicitationId: z.ZodString;
  url: z.ZodString;
}, z.core.$strip>;
/**
 * The parameters for a request to elicit additional information from the user via the client.
 */
declare const ElicitRequestParamsSchema: z.ZodUnion<readonly [z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    /**
     * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
     */
    progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
    /**
     * If specified, this request is related to the provided task.
     */
    "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
      taskId: z.ZodString;
    }, z.core.$strip>>;
  }, z.core.$loose>>;
  task: z.ZodOptional<z.ZodObject<{
    ttl: z.ZodOptional<z.ZodNumber>;
  }, z.core.$strip>>;
  mode: z.ZodOptional<z.ZodLiteral<"form">>;
  message: z.ZodString;
  requestedSchema: z.ZodObject<{
    type: z.ZodLiteral<"object">;
    properties: z.ZodRecord<z.ZodString, z.ZodUnion<readonly [z.ZodUnion<readonly [z.ZodObject<{
      type: z.ZodLiteral<"string">;
      title: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      enum: z.ZodArray<z.ZodString>;
      enumNames: z.ZodOptional<z.ZodArray<z.ZodString>>;
      default: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>, z.ZodUnion<readonly [z.ZodObject<{
      type: z.ZodLiteral<"string">;
      title: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      enum: z.ZodArray<z.ZodString>;
      default: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"string">;
      title: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      oneOf: z.ZodArray<z.ZodObject<{
        const: z.ZodString;
        title: z.ZodString;
      }, z.core.$strip>>;
      default: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>]>, z.ZodUnion<readonly [z.ZodObject<{
      type: z.ZodLiteral<"array">;
      title: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      minItems: z.ZodOptional<z.ZodNumber>;
      maxItems: z.ZodOptional<z.ZodNumber>;
      items: z.ZodObject<{
        type: z.ZodLiteral<"string">;
        enum: z.ZodArray<z.ZodString>;
      }, z.core.$strip>;
      default: z.ZodOptional<z.ZodArray<z.ZodString>>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"array">;
      title: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      minItems: z.ZodOptional<z.ZodNumber>;
      maxItems: z.ZodOptional<z.ZodNumber>;
      items: z.ZodObject<{
        anyOf: z.ZodArray<z.ZodObject<{
          const: z.ZodString;
          title: z.ZodString;
        }, z.core.$strip>>;
      }, z.core.$strip>;
      default: z.ZodOptional<z.ZodArray<z.ZodString>>;
    }, z.core.$strip>]>]>, z.ZodObject<{
      type: z.ZodLiteral<"boolean">;
      title: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      default: z.ZodOptional<z.ZodBoolean>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"string">;
      title: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      minLength: z.ZodOptional<z.ZodNumber>;
      maxLength: z.ZodOptional<z.ZodNumber>;
      format: z.ZodOptional<z.ZodEnum<{
        date: "date";
        uri: "uri";
        email: "email";
        "date-time": "date-time";
      }>>;
      default: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodEnum<{
        number: "number";
        integer: "integer";
      }>;
      title: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      minimum: z.ZodOptional<z.ZodNumber>;
      maximum: z.ZodOptional<z.ZodNumber>;
      default: z.ZodOptional<z.ZodNumber>;
    }, z.core.$strip>]>>;
    required: z.ZodOptional<z.ZodArray<z.ZodString>>;
  }, z.core.$catchall<z.ZodUnknown>>;
}, z.core.$strip>, z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    /**
     * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
     */
    progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
    /**
     * If specified, this request is related to the provided task.
     */
    "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
      taskId: z.ZodString;
    }, z.core.$strip>>;
  }, z.core.$loose>>;
  task: z.ZodOptional<z.ZodObject<{
    ttl: z.ZodOptional<z.ZodNumber>;
  }, z.core.$strip>>;
  mode: z.ZodLiteral<"url">;
  message: z.ZodString;
  elicitationId: z.ZodString;
  url: z.ZodString;
}, z.core.$strip>]>;
/**
 * A request from the server to elicit user input via the client.
 * The client should present the message and form fields to the user (form mode)
 * or navigate to a URL (URL mode).
 */
declare const ElicitRequestSchema: z.ZodObject<{
  method: z.ZodLiteral<"elicitation/create">;
  params: z.ZodUnion<readonly [z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    task: z.ZodOptional<z.ZodObject<{
      ttl: z.ZodOptional<z.ZodNumber>;
    }, z.core.$strip>>;
    mode: z.ZodOptional<z.ZodLiteral<"form">>;
    message: z.ZodString;
    requestedSchema: z.ZodObject<{
      type: z.ZodLiteral<"object">;
      properties: z.ZodRecord<z.ZodString, z.ZodUnion<readonly [z.ZodUnion<readonly [z.ZodObject<{
        type: z.ZodLiteral<"string">;
        title: z.ZodOptional<z.ZodString>;
        description: z.ZodOptional<z.ZodString>;
        enum: z.ZodArray<z.ZodString>;
        enumNames: z.ZodOptional<z.ZodArray<z.ZodString>>;
        default: z.ZodOptional<z.ZodString>;
      }, z.core.$strip>, z.ZodUnion<readonly [z.ZodObject<{
        type: z.ZodLiteral<"string">;
        title: z.ZodOptional<z.ZodString>;
        description: z.ZodOptional<z.ZodString>;
        enum: z.ZodArray<z.ZodString>;
        default: z.ZodOptional<z.ZodString>;
      }, z.core.$strip>, z.ZodObject<{
        type: z.ZodLiteral<"string">;
        title: z.ZodOptional<z.ZodString>;
        description: z.ZodOptional<z.ZodString>;
        oneOf: z.ZodArray<z.ZodObject<{
          const: z.ZodString;
          title: z.ZodString;
        }, z.core.$strip>>;
        default: z.ZodOptional<z.ZodString>;
      }, z.core.$strip>]>, z.ZodUnion<readonly [z.ZodObject<{
        type: z.ZodLiteral<"array">;
        title: z.ZodOptional<z.ZodString>;
        description: z.ZodOptional<z.ZodString>;
        minItems: z.ZodOptional<z.ZodNumber>;
        maxItems: z.ZodOptional<z.ZodNumber>;
        items: z.ZodObject<{
          type: z.ZodLiteral<"string">;
          enum: z.ZodArray<z.ZodString>;
        }, z.core.$strip>;
        default: z.ZodOptional<z.ZodArray<z.ZodString>>;
      }, z.core.$strip>, z.ZodObject<{
        type: z.ZodLiteral<"array">;
        title: z.ZodOptional<z.ZodString>;
        description: z.ZodOptional<z.ZodString>;
        minItems: z.ZodOptional<z.ZodNumber>;
        maxItems: z.ZodOptional<z.ZodNumber>;
        items: z.ZodObject<{
          anyOf: z.ZodArray<z.ZodObject<{
            const: z.ZodString;
            title: z.ZodString;
          }, z.core.$strip>>;
        }, z.core.$strip>;
        default: z.ZodOptional<z.ZodArray<z.ZodString>>;
      }, z.core.$strip>]>]>, z.ZodObject<{
        type: z.ZodLiteral<"boolean">;
        title: z.ZodOptional<z.ZodString>;
        description: z.ZodOptional<z.ZodString>;
        default: z.ZodOptional<z.ZodBoolean>;
      }, z.core.$strip>, z.ZodObject<{
        type: z.ZodLiteral<"string">;
        title: z.ZodOptional<z.ZodString>;
        description: z.ZodOptional<z.ZodString>;
        minLength: z.ZodOptional<z.ZodNumber>;
        maxLength: z.ZodOptional<z.ZodNumber>;
        format: z.ZodOptional<z.ZodEnum<{
          date: "date";
          uri: "uri";
          email: "email";
          "date-time": "date-time";
        }>>;
        default: z.ZodOptional<z.ZodString>;
      }, z.core.$strip>, z.ZodObject<{
        type: z.ZodEnum<{
          number: "number";
          integer: "integer";
        }>;
        title: z.ZodOptional<z.ZodString>;
        description: z.ZodOptional<z.ZodString>;
        minimum: z.ZodOptional<z.ZodNumber>;
        maximum: z.ZodOptional<z.ZodNumber>;
        default: z.ZodOptional<z.ZodNumber>;
      }, z.core.$strip>]>>;
      required: z.ZodOptional<z.ZodArray<z.ZodString>>;
    }, z.core.$catchall<z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    task: z.ZodOptional<z.ZodObject<{
      ttl: z.ZodOptional<z.ZodNumber>;
    }, z.core.$strip>>;
    mode: z.ZodLiteral<"url">;
    message: z.ZodString;
    elicitationId: z.ZodString;
    url: z.ZodString;
  }, z.core.$strip>]>;
}, z.core.$strip>;
/**
 * Parameters for a {@linkcode ElicitationCompleteNotification | notifications/elicitation/complete} notification.
 *
 * @deprecated Removed from the spec by #2891 (2026-07-28). The client learns the outcome
 * of an out-of-band interaction by retrying the original request; no server-initiated
 * completion signal exists in the 2026-07-28 revision. Kept here for the 2025-era flow
 * only. The 2026-07-28 wire codec excludes this notification.
 * @category notifications/elicitation/complete
 */
declare const ElicitationCompleteNotificationParamsSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    /**
     * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
     */
    progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
    /**
     * If specified, this request is related to the provided task.
     */
    "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
      taskId: z.ZodString;
    }, z.core.$strip>>;
  }, z.core.$loose>>;
  elicitationId: z.ZodString;
}, z.core.$strip>;
/**
 * A notification from the server to the client, informing it of a completion of an out-of-band elicitation request.
 *
 * @deprecated Removed from the spec by #2891 (2026-07-28). The client learns the outcome
 * of an out-of-band interaction by retrying the original request; no server-initiated
 * completion signal exists in the 2026-07-28 revision. Kept here for the 2025-era flow
 * only. The 2026-07-28 wire codec excludes this notification.
 * @category notifications/elicitation/complete
 */
declare const ElicitationCompleteNotificationSchema: z.ZodObject<{
  method: z.ZodLiteral<"notifications/elicitation/complete">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    elicitationId: z.ZodString;
  }, z.core.$strip>;
}, z.core.$strip>;
/**
 * The client's response to an {@linkcode ElicitRequest | elicitation/create} request from the server.
 */
declare const ElicitResultSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  action: z.ZodEnum<{
    cancel: "cancel";
    accept: "accept";
    decline: "decline";
  }>;
  content: z.ZodPipe<z.ZodTransform<{} | undefined, unknown>, z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnion<readonly [z.ZodString, z.ZodNumber, z.ZodBoolean, z.ZodArray<z.ZodString>]>>>>;
}, z.core.$loose>;
/**
 * A reference to a resource or resource template definition.
 */
declare const ResourceTemplateReferenceSchema: z.ZodObject<{
  type: z.ZodLiteral<"ref/resource">;
  uri: z.ZodString;
}, z.core.$strip>;
/**
 * Identifies a prompt.
 */
declare const PromptReferenceSchema: z.ZodObject<{
  type: z.ZodLiteral<"ref/prompt">;
  name: z.ZodString;
}, z.core.$strip>;
/**
 * Parameters for a {@linkcode CompleteRequest | completion/complete} request.
 */
declare const CompleteRequestParamsSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    /**
     * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
     */
    progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
    /**
     * If specified, this request is related to the provided task.
     */
    "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
      taskId: z.ZodString;
    }, z.core.$strip>>;
  }, z.core.$loose>>;
  ref: z.ZodUnion<readonly [z.ZodObject<{
    type: z.ZodLiteral<"ref/prompt">;
    name: z.ZodString;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"ref/resource">;
    uri: z.ZodString;
  }, z.core.$strip>]>;
  argument: z.ZodObject<{
    name: z.ZodString;
    value: z.ZodString;
  }, z.core.$strip>;
  context: z.ZodOptional<z.ZodObject<{
    arguments: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodString>>;
  }, z.core.$strip>>;
}, z.core.$strip>;
/**
 * A request from the client to the server, to ask for completion options.
 */
declare const CompleteRequestSchema: z.ZodObject<{
  method: z.ZodLiteral<"completion/complete">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    ref: z.ZodUnion<readonly [z.ZodObject<{
      type: z.ZodLiteral<"ref/prompt">;
      name: z.ZodString;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"ref/resource">;
      uri: z.ZodString;
    }, z.core.$strip>]>;
    argument: z.ZodObject<{
      name: z.ZodString;
      value: z.ZodString;
    }, z.core.$strip>;
    context: z.ZodOptional<z.ZodObject<{
      arguments: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodString>>;
    }, z.core.$strip>>;
  }, z.core.$strip>;
}, z.core.$strip>;
/**
 * The server's response to a {@linkcode CompleteRequest | completion/complete} request
 */
declare const CompleteResultSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  completion: z.ZodObject<{
    /**
     * An array of completion values. Must not exceed 100 items.
     */
    values: z.ZodArray<z.ZodString>;
    /**
     * The total number of completion options available. This can exceed the number of values actually sent in the response.
     */
    total: z.ZodOptional<z.ZodNumber>;
    /**
     * Indicates whether there are additional completion options beyond those provided in the current response, even if the exact total is unknown.
     */
    hasMore: z.ZodOptional<z.ZodBoolean>;
  }, z.core.$loose>;
}, z.core.$loose>;
/**
 * Represents a root directory or file that the server can operate on.
 *
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to passing paths via
 * tool parameters, resource URIs, or configuration.
 */
declare const RootSchema: z.ZodObject<{
  uri: z.ZodString;
  name: z.ZodOptional<z.ZodString>;
  _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
}, z.core.$strip>;
/**
 * Sent from the server to request a list of root URIs from the client.
 *
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to passing paths via
 * tool parameters, resource URIs, or configuration.
 */
declare const ListRootsRequestSchema: z.ZodObject<{
  method: z.ZodLiteral<"roots/list">;
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
  }, z.core.$strip>>;
}, z.core.$strip>;
/**
 * The client's response to a `roots/list` request from the server.
 *
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to passing paths via
 * tool parameters, resource URIs, or configuration.
 */
declare const ListRootsResultSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  roots: z.ZodArray<z.ZodObject<{
    uri: z.ZodString;
    name: z.ZodOptional<z.ZodString>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>>;
}, z.core.$loose>;
/**
 * A notification from the client to the server, informing it that the list of roots has changed.
 *
 * @deprecated Deprecated as of protocol version 2026-07-28 (SEP-2577); remains
 * in the specification for at least twelve months. Migrate to passing paths via
 * tool parameters, resource URIs, or configuration.
 */
declare const RootsListChangedNotificationSchema: z.ZodObject<{
  method: z.ZodLiteral<"notifications/roots/list_changed">;
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
  }, z.core.$strip>>;
}, z.core.$strip>;
/**
 * Task creation parameters, used to ask that the server create a task to represent a request.
 *
 * @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only.
 */
declare const TaskCreationParamsSchema: z.ZodObject<{
  /**
   * Requested duration in milliseconds to retain task from creation.
   */
  ttl: z.ZodOptional<z.ZodNumber>;
  /**
   * Time in milliseconds to wait between task status requests.
   */
  pollInterval: z.ZodOptional<z.ZodNumber>;
}, z.core.$loose>;
/**
 * The status of a task.
 *
 * @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only.
 */
declare const TaskStatusSchema: z.ZodEnum<{
  working: "working";
  input_required: "input_required";
  completed: "completed";
  failed: "failed";
  cancelled: "cancelled";
}>;
/**
 * A pollable state object associated with a request.
 *
 * @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only.
 */
declare const TaskSchema: z.ZodObject<{
  taskId: z.ZodString;
  status: z.ZodEnum<{
    working: "working";
    input_required: "input_required";
    completed: "completed";
    failed: "failed";
    cancelled: "cancelled";
  }>;
  ttl: z.ZodUnion<readonly [z.ZodNumber, z.ZodNull]>;
  createdAt: z.ZodString;
  lastUpdatedAt: z.ZodString;
  pollInterval: z.ZodOptional<z.ZodNumber>;
  statusMessage: z.ZodOptional<z.ZodString>;
}, z.core.$strip>;
/**
 * Result returned when a task is created, containing the task data wrapped in a `task` field.
 *
 * @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only.
 */
declare const CreateTaskResultSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  task: z.ZodObject<{
    taskId: z.ZodString;
    status: z.ZodEnum<{
      working: "working";
      input_required: "input_required";
      completed: "completed";
      failed: "failed";
      cancelled: "cancelled";
    }>;
    ttl: z.ZodUnion<readonly [z.ZodNumber, z.ZodNull]>;
    createdAt: z.ZodString;
    lastUpdatedAt: z.ZodString;
    pollInterval: z.ZodOptional<z.ZodNumber>;
    statusMessage: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>;
}, z.core.$loose>;
/**
 * Parameters for task status notification.
 *
 * @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only.
 */
declare const TaskStatusNotificationParamsSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    /**
     * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
     */
    progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
    /**
     * If specified, this request is related to the provided task.
     */
    "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
      taskId: z.ZodString;
    }, z.core.$strip>>;
  }, z.core.$loose>>;
  taskId: z.ZodString;
  status: z.ZodEnum<{
    working: "working";
    input_required: "input_required";
    completed: "completed";
    failed: "failed";
    cancelled: "cancelled";
  }>;
  ttl: z.ZodUnion<readonly [z.ZodNumber, z.ZodNull]>;
  createdAt: z.ZodString;
  lastUpdatedAt: z.ZodString;
  pollInterval: z.ZodOptional<z.ZodNumber>;
  statusMessage: z.ZodOptional<z.ZodString>;
}, z.core.$strip>;
/**
 * A notification sent when a task's status changes.
 *
 * @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only.
 */
declare const TaskStatusNotificationSchema: z.ZodObject<{
  method: z.ZodLiteral<"notifications/tasks/status">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    taskId: z.ZodString;
    status: z.ZodEnum<{
      working: "working";
      input_required: "input_required";
      completed: "completed";
      failed: "failed";
      cancelled: "cancelled";
    }>;
    ttl: z.ZodUnion<readonly [z.ZodNumber, z.ZodNull]>;
    createdAt: z.ZodString;
    lastUpdatedAt: z.ZodString;
    pollInterval: z.ZodOptional<z.ZodNumber>;
    statusMessage: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>;
}, z.core.$strip>;
/**
 * A request to get the state of a specific task.
 *
 * @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only.
 */
declare const GetTaskRequestSchema: z.ZodObject<{
  method: z.ZodLiteral<"tasks/get">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    taskId: z.ZodString;
  }, z.core.$strip>;
}, z.core.$strip>;
/**
 * The response to a {@linkcode GetTaskRequest | tasks/get} request.
 *
 * @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only.
 */
declare const GetTaskResultSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  taskId: z.ZodString;
  status: z.ZodEnum<{
    working: "working";
    input_required: "input_required";
    completed: "completed";
    failed: "failed";
    cancelled: "cancelled";
  }>;
  ttl: z.ZodUnion<readonly [z.ZodNumber, z.ZodNull]>;
  createdAt: z.ZodString;
  lastUpdatedAt: z.ZodString;
  pollInterval: z.ZodOptional<z.ZodNumber>;
  statusMessage: z.ZodOptional<z.ZodString>;
}, z.core.$strip>;
/**
 * A request to get the result of a specific task.
 *
 * @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only.
 */
declare const GetTaskPayloadRequestSchema: z.ZodObject<{
  method: z.ZodLiteral<"tasks/result">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    taskId: z.ZodString;
  }, z.core.$strip>;
}, z.core.$strip>;
/**
 * The response to a `tasks/result` request.
 * The structure matches the result type of the original request.
 * For example, a {@linkcode CallToolRequest | tools/call} task would return the `CallToolResult` structure.
 *
 *
 * @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only.
 */
declare const GetTaskPayloadResultSchema: z.ZodObject<{
  /**
   * See [MCP specification](https://modelcontextprotocol.io/specification/2026-07-28/basic/index#general-fields)
   * for notes on `_meta` usage.
   */
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
}, z.core.$loose>;
/**
 * A request to list tasks.
 *
 * @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only.
 */
declare const ListTasksRequestSchema: z.ZodObject<{
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    cursor: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>>;
  method: z.ZodLiteral<"tasks/list">;
}, z.core.$strip>;
/**
 * The response to a {@linkcode ListTasksRequest | tasks/list} request.
 *
 * @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only.
 */
declare const ListTasksResultSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  nextCursor: z.ZodOptional<z.ZodString>;
  tasks: z.ZodArray<z.ZodObject<{
    taskId: z.ZodString;
    status: z.ZodEnum<{
      working: "working";
      input_required: "input_required";
      completed: "completed";
      failed: "failed";
      cancelled: "cancelled";
    }>;
    ttl: z.ZodUnion<readonly [z.ZodNumber, z.ZodNull]>;
    createdAt: z.ZodString;
    lastUpdatedAt: z.ZodString;
    pollInterval: z.ZodOptional<z.ZodNumber>;
    statusMessage: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>>;
}, z.core.$loose>;
/**
 * A request to cancel a specific task.
 *
 * @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only.
 */
declare const CancelTaskRequestSchema: z.ZodObject<{
  method: z.ZodLiteral<"tasks/cancel">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    taskId: z.ZodString;
  }, z.core.$strip>;
}, z.core.$strip>;
/**
 * The response to a {@linkcode CancelTaskRequest | tasks/cancel} request.
 *
 * @deprecated 2025-11-25 wire vocabulary with no SDK runtime; kept importable for interoperability only.
 */
declare const CancelTaskResultSchema: z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  taskId: z.ZodString;
  status: z.ZodEnum<{
    working: "working";
    input_required: "input_required";
    completed: "completed";
    failed: "failed";
    cancelled: "cancelled";
  }>;
  ttl: z.ZodUnion<readonly [z.ZodNumber, z.ZodNull]>;
  createdAt: z.ZodString;
  lastUpdatedAt: z.ZodString;
  pollInterval: z.ZodOptional<z.ZodNumber>;
  statusMessage: z.ZodOptional<z.ZodString>;
}, z.core.$strip>;
declare const ClientRequestSchema: z.ZodUnion<readonly [z.ZodObject<{
  method: z.ZodLiteral<"ping">;
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
  }, z.core.$strip>>;
}, z.core.$strip>, z.ZodObject<{
  method: z.ZodLiteral<"initialize">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    protocolVersion: z.ZodString;
    capabilities: z.ZodObject<{
      experimental: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>>;
      sampling: z.ZodOptional<z.ZodObject<{
        context: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
        tools: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
      }, z.core.$strip>>;
      elicitation: z.ZodOptional<z.ZodPipe<z.ZodTransform<unknown, unknown>, z.ZodIntersection<z.ZodObject<{
        form: z.ZodOptional<z.ZodIntersection<z.ZodObject<{
          applyDefaults: z.ZodOptional<z.ZodBoolean>;
        }, z.core.$strip>, z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>>;
        url: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
      }, z.core.$strip>, z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>>>>;
      roots: z.ZodOptional<z.ZodObject<{
        listChanged: z.ZodOptional<z.ZodBoolean>;
      }, z.core.$strip>>;
      tasks: z.ZodOptional<z.ZodObject<{
        /**
         * Present if the client supports listing tasks.
         */
        list: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
        /**
         * Present if the client supports cancelling tasks.
         */
        cancel: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
        /**
         * Capabilities for task creation on specific request types.
         */
        requests: z.ZodOptional<z.ZodObject<{
          /**
           * Task support for sampling requests.
           */
          sampling: z.ZodOptional<z.ZodObject<{
            createMessage: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
          }, z.core.$loose>>;
          /**
           * Task support for elicitation requests.
           */
          elicitation: z.ZodOptional<z.ZodObject<{
            create: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
          }, z.core.$loose>>;
        }, z.core.$loose>>;
      }, z.core.$loose>>;
      extensions: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>>;
    }, z.core.$strip>;
    clientInfo: z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>;
  }, z.core.$strip>;
}, z.core.$strip>, z.ZodObject<{
  method: z.ZodLiteral<"server/discover">;
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
  }, z.core.$strip>>;
}, z.core.$strip>, z.ZodObject<{
  method: z.ZodLiteral<"completion/complete">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    ref: z.ZodUnion<readonly [z.ZodObject<{
      type: z.ZodLiteral<"ref/prompt">;
      name: z.ZodString;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"ref/resource">;
      uri: z.ZodString;
    }, z.core.$strip>]>;
    argument: z.ZodObject<{
      name: z.ZodString;
      value: z.ZodString;
    }, z.core.$strip>;
    context: z.ZodOptional<z.ZodObject<{
      arguments: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodString>>;
    }, z.core.$strip>>;
  }, z.core.$strip>;
}, z.core.$strip>, z.ZodObject<{
  method: z.ZodLiteral<"logging/setLevel">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    level: z.ZodEnum<{
      error: "error";
      debug: "debug";
      info: "info";
      notice: "notice";
      warning: "warning";
      critical: "critical";
      alert: "alert";
      emergency: "emergency";
    }>;
  }, z.core.$strip>;
}, z.core.$strip>, z.ZodObject<{
  method: z.ZodLiteral<"prompts/get">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    name: z.ZodString;
    arguments: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodString>>;
  }, z.core.$strip>;
}, z.core.$strip>, z.ZodObject<{
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    cursor: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>>;
  method: z.ZodLiteral<"prompts/list">;
}, z.core.$strip>, z.ZodObject<{
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    cursor: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>>;
  method: z.ZodLiteral<"resources/list">;
}, z.core.$strip>, z.ZodObject<{
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    cursor: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>>;
  method: z.ZodLiteral<"resources/templates/list">;
}, z.core.$strip>, z.ZodObject<{
  method: z.ZodLiteral<"resources/read">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    uri: z.ZodString;
  }, z.core.$strip>;
}, z.core.$strip>, z.ZodObject<{
  method: z.ZodLiteral<"resources/subscribe">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    uri: z.ZodString;
  }, z.core.$strip>;
}, z.core.$strip>, z.ZodObject<{
  method: z.ZodLiteral<"resources/unsubscribe">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    uri: z.ZodString;
  }, z.core.$strip>;
}, z.core.$strip>, z.ZodObject<{
  method: z.ZodLiteral<"subscriptions/listen">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    notifications: z.ZodObject<{
      toolsListChanged: z.ZodOptional<z.ZodBoolean>;
      promptsListChanged: z.ZodOptional<z.ZodBoolean>;
      resourcesListChanged: z.ZodOptional<z.ZodBoolean>;
      resourceSubscriptions: z.ZodOptional<z.ZodArray<z.ZodString>>;
    }, z.core.$strip>;
  }, z.core.$strip>;
}, z.core.$strip>, z.ZodObject<{
  method: z.ZodLiteral<"tools/call">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    task: z.ZodOptional<z.ZodObject<{
      ttl: z.ZodOptional<z.ZodNumber>;
    }, z.core.$strip>>;
    name: z.ZodString;
    arguments: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>;
}, z.core.$strip>, z.ZodObject<{
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    cursor: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>>;
  method: z.ZodLiteral<"tools/list">;
}, z.core.$strip>]>;
declare const ClientNotificationSchema: z.ZodUnion<readonly [z.ZodObject<{
  method: z.ZodLiteral<"notifications/cancelled">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    requestId: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
    reason: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>;
}, z.core.$strip>, z.ZodObject<{
  method: z.ZodLiteral<"notifications/progress">;
  params: z.ZodObject<{
    progressToken: z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>;
    progress: z.ZodNumber;
    total: z.ZodOptional<z.ZodNumber>;
    message: z.ZodOptional<z.ZodString>;
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
  }, z.core.$strip>;
}, z.core.$strip>, z.ZodObject<{
  method: z.ZodLiteral<"notifications/initialized">;
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
  }, z.core.$strip>>;
}, z.core.$strip>, z.ZodObject<{
  method: z.ZodLiteral<"notifications/roots/list_changed">;
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
  }, z.core.$strip>>;
}, z.core.$strip>]>;
declare const ClientResultSchema: z.ZodUnion<readonly [z.ZodObject<{
  /**
   * See [MCP specification](https://modelcontextprotocol.io/specification/2026-07-28/basic/index#general-fields)
   * for notes on `_meta` usage.
   */
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
}, z.core.$strict>, z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  model: z.ZodString;
  stopReason: z.ZodOptional<z.ZodUnion<[z.ZodEnum<{
    maxTokens: "maxTokens";
    endTurn: "endTurn";
    stopSequence: "stopSequence";
  }>, z.ZodString]>>;
  role: z.ZodEnum<{
    user: "user";
    assistant: "assistant";
  }>;
  content: z.ZodDiscriminatedUnion<[z.ZodObject<{
    type: z.ZodLiteral<"text">;
    text: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"image">;
    data: z.ZodString;
    mimeType: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"audio">;
    data: z.ZodString;
    mimeType: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>], "type">;
}, z.core.$loose>, z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  model: z.ZodString;
  stopReason: z.ZodOptional<z.ZodUnion<[z.ZodEnum<{
    maxTokens: "maxTokens";
    endTurn: "endTurn";
    stopSequence: "stopSequence";
    toolUse: "toolUse";
  }>, z.ZodString]>>;
  role: z.ZodEnum<{
    user: "user";
    assistant: "assistant";
  }>;
  content: z.ZodUnion<readonly [z.ZodDiscriminatedUnion<[z.ZodObject<{
    type: z.ZodLiteral<"text">;
    text: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"image">;
    data: z.ZodString;
    mimeType: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"audio">;
    data: z.ZodString;
    mimeType: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"tool_use">;
    name: z.ZodString;
    id: z.ZodString;
    input: z.ZodRecord<z.ZodString, z.ZodUnknown>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"tool_result">;
    toolUseId: z.ZodString;
    content: z.ZodArray<z.ZodUnion<readonly [z.ZodObject<{
      type: z.ZodLiteral<"text">;
      text: z.ZodString;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"image">;
      data: z.ZodString;
      mimeType: z.ZodString;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"audio">;
      data: z.ZodString;
      mimeType: z.ZodString;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      uri: z.ZodString;
      description: z.ZodOptional<z.ZodString>;
      mimeType: z.ZodOptional<z.ZodString>;
      size: z.ZodOptional<z.ZodNumber>;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
      type: z.ZodLiteral<"resource_link">;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"resource">;
      resource: z.ZodUnion<readonly [z.ZodObject<{
        uri: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        text: z.ZodString;
      }, z.core.$strip>, z.ZodObject<{
        uri: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        blob: z.ZodString;
      }, z.core.$strip>]>;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>]>>;
    structuredContent: z.ZodOptional<z.ZodUnknown>;
    isError: z.ZodOptional<z.ZodBoolean>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>], "type">, z.ZodArray<z.ZodDiscriminatedUnion<[z.ZodObject<{
    type: z.ZodLiteral<"text">;
    text: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"image">;
    data: z.ZodString;
    mimeType: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"audio">;
    data: z.ZodString;
    mimeType: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"tool_use">;
    name: z.ZodString;
    id: z.ZodString;
    input: z.ZodRecord<z.ZodString, z.ZodUnknown>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"tool_result">;
    toolUseId: z.ZodString;
    content: z.ZodArray<z.ZodUnion<readonly [z.ZodObject<{
      type: z.ZodLiteral<"text">;
      text: z.ZodString;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"image">;
      data: z.ZodString;
      mimeType: z.ZodString;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"audio">;
      data: z.ZodString;
      mimeType: z.ZodString;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      uri: z.ZodString;
      description: z.ZodOptional<z.ZodString>;
      mimeType: z.ZodOptional<z.ZodString>;
      size: z.ZodOptional<z.ZodNumber>;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
      type: z.ZodLiteral<"resource_link">;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"resource">;
      resource: z.ZodUnion<readonly [z.ZodObject<{
        uri: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        text: z.ZodString;
      }, z.core.$strip>, z.ZodObject<{
        uri: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        blob: z.ZodString;
      }, z.core.$strip>]>;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>]>>;
    structuredContent: z.ZodOptional<z.ZodUnknown>;
    isError: z.ZodOptional<z.ZodBoolean>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>], "type">>]>;
}, z.core.$loose>, z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  action: z.ZodEnum<{
    cancel: "cancel";
    accept: "accept";
    decline: "decline";
  }>;
  content: z.ZodPipe<z.ZodTransform<{} | undefined, unknown>, z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnion<readonly [z.ZodString, z.ZodNumber, z.ZodBoolean, z.ZodArray<z.ZodString>]>>>>;
}, z.core.$loose>, z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  roots: z.ZodArray<z.ZodObject<{
    uri: z.ZodString;
    name: z.ZodOptional<z.ZodString>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>>;
}, z.core.$loose>]>;
declare const ServerRequestSchema: z.ZodUnion<readonly [z.ZodObject<{
  method: z.ZodLiteral<"ping">;
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
  }, z.core.$strip>>;
}, z.core.$strip>, z.ZodObject<{
  method: z.ZodLiteral<"sampling/createMessage">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    task: z.ZodOptional<z.ZodObject<{
      ttl: z.ZodOptional<z.ZodNumber>;
    }, z.core.$strip>>;
    messages: z.ZodArray<z.ZodObject<{
      role: z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>;
      content: z.ZodUnion<readonly [z.ZodDiscriminatedUnion<[z.ZodObject<{
        type: z.ZodLiteral<"text">;
        text: z.ZodString;
        annotations: z.ZodOptional<z.ZodObject<{
          audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
            user: "user";
            assistant: "assistant";
          }>>>;
          priority: z.ZodOptional<z.ZodNumber>;
          lastModified: z.ZodOptional<z.ZodISODateTime>;
        }, z.core.$strip>>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      }, z.core.$strip>, z.ZodObject<{
        type: z.ZodLiteral<"image">;
        data: z.ZodString;
        mimeType: z.ZodString;
        annotations: z.ZodOptional<z.ZodObject<{
          audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
            user: "user";
            assistant: "assistant";
          }>>>;
          priority: z.ZodOptional<z.ZodNumber>;
          lastModified: z.ZodOptional<z.ZodISODateTime>;
        }, z.core.$strip>>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      }, z.core.$strip>, z.ZodObject<{
        type: z.ZodLiteral<"audio">;
        data: z.ZodString;
        mimeType: z.ZodString;
        annotations: z.ZodOptional<z.ZodObject<{
          audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
            user: "user";
            assistant: "assistant";
          }>>>;
          priority: z.ZodOptional<z.ZodNumber>;
          lastModified: z.ZodOptional<z.ZodISODateTime>;
        }, z.core.$strip>>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      }, z.core.$strip>, z.ZodObject<{
        type: z.ZodLiteral<"tool_use">;
        name: z.ZodString;
        id: z.ZodString;
        input: z.ZodRecord<z.ZodString, z.ZodUnknown>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      }, z.core.$strip>, z.ZodObject<{
        type: z.ZodLiteral<"tool_result">;
        toolUseId: z.ZodString;
        content: z.ZodArray<z.ZodUnion<readonly [z.ZodObject<{
          type: z.ZodLiteral<"text">;
          text: z.ZodString;
          annotations: z.ZodOptional<z.ZodObject<{
            audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
              user: "user";
              assistant: "assistant";
            }>>>;
            priority: z.ZodOptional<z.ZodNumber>;
            lastModified: z.ZodOptional<z.ZodISODateTime>;
          }, z.core.$strip>>;
          _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        }, z.core.$strip>, z.ZodObject<{
          type: z.ZodLiteral<"image">;
          data: z.ZodString;
          mimeType: z.ZodString;
          annotations: z.ZodOptional<z.ZodObject<{
            audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
              user: "user";
              assistant: "assistant";
            }>>>;
            priority: z.ZodOptional<z.ZodNumber>;
            lastModified: z.ZodOptional<z.ZodISODateTime>;
          }, z.core.$strip>>;
          _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        }, z.core.$strip>, z.ZodObject<{
          type: z.ZodLiteral<"audio">;
          data: z.ZodString;
          mimeType: z.ZodString;
          annotations: z.ZodOptional<z.ZodObject<{
            audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
              user: "user";
              assistant: "assistant";
            }>>>;
            priority: z.ZodOptional<z.ZodNumber>;
            lastModified: z.ZodOptional<z.ZodISODateTime>;
          }, z.core.$strip>>;
          _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        }, z.core.$strip>, z.ZodObject<{
          uri: z.ZodString;
          description: z.ZodOptional<z.ZodString>;
          mimeType: z.ZodOptional<z.ZodString>;
          size: z.ZodOptional<z.ZodNumber>;
          annotations: z.ZodOptional<z.ZodObject<{
            audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
              user: "user";
              assistant: "assistant";
            }>>>;
            priority: z.ZodOptional<z.ZodNumber>;
            lastModified: z.ZodOptional<z.ZodISODateTime>;
          }, z.core.$strip>>;
          _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
          icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
            src: z.ZodString;
            mimeType: z.ZodOptional<z.ZodString>;
            sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
            theme: z.ZodOptional<z.ZodEnum<{
              light: "light";
              dark: "dark";
            }>>;
          }, z.core.$strip>>>;
          name: z.ZodString;
          title: z.ZodOptional<z.ZodString>;
          type: z.ZodLiteral<"resource_link">;
        }, z.core.$strip>, z.ZodObject<{
          type: z.ZodLiteral<"resource">;
          resource: z.ZodUnion<readonly [z.ZodObject<{
            uri: z.ZodString;
            mimeType: z.ZodOptional<z.ZodString>;
            _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
            text: z.ZodString;
          }, z.core.$strip>, z.ZodObject<{
            uri: z.ZodString;
            mimeType: z.ZodOptional<z.ZodString>;
            _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
            blob: z.ZodString;
          }, z.core.$strip>]>;
          annotations: z.ZodOptional<z.ZodObject<{
            audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
              user: "user";
              assistant: "assistant";
            }>>>;
            priority: z.ZodOptional<z.ZodNumber>;
            lastModified: z.ZodOptional<z.ZodISODateTime>;
          }, z.core.$strip>>;
          _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        }, z.core.$strip>]>>;
        structuredContent: z.ZodOptional<z.ZodUnknown>;
        isError: z.ZodOptional<z.ZodBoolean>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      }, z.core.$strip>], "type">, z.ZodArray<z.ZodDiscriminatedUnion<[z.ZodObject<{
        type: z.ZodLiteral<"text">;
        text: z.ZodString;
        annotations: z.ZodOptional<z.ZodObject<{
          audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
            user: "user";
            assistant: "assistant";
          }>>>;
          priority: z.ZodOptional<z.ZodNumber>;
          lastModified: z.ZodOptional<z.ZodISODateTime>;
        }, z.core.$strip>>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      }, z.core.$strip>, z.ZodObject<{
        type: z.ZodLiteral<"image">;
        data: z.ZodString;
        mimeType: z.ZodString;
        annotations: z.ZodOptional<z.ZodObject<{
          audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
            user: "user";
            assistant: "assistant";
          }>>>;
          priority: z.ZodOptional<z.ZodNumber>;
          lastModified: z.ZodOptional<z.ZodISODateTime>;
        }, z.core.$strip>>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      }, z.core.$strip>, z.ZodObject<{
        type: z.ZodLiteral<"audio">;
        data: z.ZodString;
        mimeType: z.ZodString;
        annotations: z.ZodOptional<z.ZodObject<{
          audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
            user: "user";
            assistant: "assistant";
          }>>>;
          priority: z.ZodOptional<z.ZodNumber>;
          lastModified: z.ZodOptional<z.ZodISODateTime>;
        }, z.core.$strip>>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      }, z.core.$strip>, z.ZodObject<{
        type: z.ZodLiteral<"tool_use">;
        name: z.ZodString;
        id: z.ZodString;
        input: z.ZodRecord<z.ZodString, z.ZodUnknown>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      }, z.core.$strip>, z.ZodObject<{
        type: z.ZodLiteral<"tool_result">;
        toolUseId: z.ZodString;
        content: z.ZodArray<z.ZodUnion<readonly [z.ZodObject<{
          type: z.ZodLiteral<"text">;
          text: z.ZodString;
          annotations: z.ZodOptional<z.ZodObject<{
            audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
              user: "user";
              assistant: "assistant";
            }>>>;
            priority: z.ZodOptional<z.ZodNumber>;
            lastModified: z.ZodOptional<z.ZodISODateTime>;
          }, z.core.$strip>>;
          _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        }, z.core.$strip>, z.ZodObject<{
          type: z.ZodLiteral<"image">;
          data: z.ZodString;
          mimeType: z.ZodString;
          annotations: z.ZodOptional<z.ZodObject<{
            audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
              user: "user";
              assistant: "assistant";
            }>>>;
            priority: z.ZodOptional<z.ZodNumber>;
            lastModified: z.ZodOptional<z.ZodISODateTime>;
          }, z.core.$strip>>;
          _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        }, z.core.$strip>, z.ZodObject<{
          type: z.ZodLiteral<"audio">;
          data: z.ZodString;
          mimeType: z.ZodString;
          annotations: z.ZodOptional<z.ZodObject<{
            audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
              user: "user";
              assistant: "assistant";
            }>>>;
            priority: z.ZodOptional<z.ZodNumber>;
            lastModified: z.ZodOptional<z.ZodISODateTime>;
          }, z.core.$strip>>;
          _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        }, z.core.$strip>, z.ZodObject<{
          uri: z.ZodString;
          description: z.ZodOptional<z.ZodString>;
          mimeType: z.ZodOptional<z.ZodString>;
          size: z.ZodOptional<z.ZodNumber>;
          annotations: z.ZodOptional<z.ZodObject<{
            audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
              user: "user";
              assistant: "assistant";
            }>>>;
            priority: z.ZodOptional<z.ZodNumber>;
            lastModified: z.ZodOptional<z.ZodISODateTime>;
          }, z.core.$strip>>;
          _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
          icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
            src: z.ZodString;
            mimeType: z.ZodOptional<z.ZodString>;
            sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
            theme: z.ZodOptional<z.ZodEnum<{
              light: "light";
              dark: "dark";
            }>>;
          }, z.core.$strip>>>;
          name: z.ZodString;
          title: z.ZodOptional<z.ZodString>;
          type: z.ZodLiteral<"resource_link">;
        }, z.core.$strip>, z.ZodObject<{
          type: z.ZodLiteral<"resource">;
          resource: z.ZodUnion<readonly [z.ZodObject<{
            uri: z.ZodString;
            mimeType: z.ZodOptional<z.ZodString>;
            _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
            text: z.ZodString;
          }, z.core.$strip>, z.ZodObject<{
            uri: z.ZodString;
            mimeType: z.ZodOptional<z.ZodString>;
            _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
            blob: z.ZodString;
          }, z.core.$strip>]>;
          annotations: z.ZodOptional<z.ZodObject<{
            audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
              user: "user";
              assistant: "assistant";
            }>>>;
            priority: z.ZodOptional<z.ZodNumber>;
            lastModified: z.ZodOptional<z.ZodISODateTime>;
          }, z.core.$strip>>;
          _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        }, z.core.$strip>]>>;
        structuredContent: z.ZodOptional<z.ZodUnknown>;
        isError: z.ZodOptional<z.ZodBoolean>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      }, z.core.$strip>], "type">>]>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>>;
    modelPreferences: z.ZodOptional<z.ZodObject<{
      hints: z.ZodOptional<z.ZodArray<z.ZodObject<{
        name: z.ZodOptional<z.ZodString>;
      }, z.core.$strip>>>;
      costPriority: z.ZodOptional<z.ZodNumber>;
      speedPriority: z.ZodOptional<z.ZodNumber>;
      intelligencePriority: z.ZodOptional<z.ZodNumber>;
    }, z.core.$strip>>;
    systemPrompt: z.ZodOptional<z.ZodString>;
    includeContext: z.ZodOptional<z.ZodEnum<{
      none: "none";
      thisServer: "thisServer";
      allServers: "allServers";
    }>>;
    temperature: z.ZodOptional<z.ZodNumber>;
    maxTokens: z.ZodNumber;
    stopSequences: z.ZodOptional<z.ZodArray<z.ZodString>>;
    metadata: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
    tools: z.ZodOptional<z.ZodArray<z.ZodObject<{
      description: z.ZodOptional<z.ZodString>;
      inputSchema: z.ZodObject<{
        type: z.ZodLiteral<"object">;
        properties: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodType<JSONValue, JSONValue, z.core.$ZodTypeInternals<JSONValue, JSONValue>>>>;
        required: z.ZodOptional<z.ZodArray<z.ZodString>>;
      }, z.core.$catchall<z.ZodUnknown>>;
      outputSchema: z.ZodOptional<z.ZodObject<{
        $schema: z.ZodOptional<z.ZodString>;
      }, z.core.$loose>>;
      annotations: z.ZodOptional<z.ZodObject<{
        title: z.ZodOptional<z.ZodString>;
        readOnlyHint: z.ZodOptional<z.ZodBoolean>;
        destructiveHint: z.ZodOptional<z.ZodBoolean>;
        idempotentHint: z.ZodOptional<z.ZodBoolean>;
        openWorldHint: z.ZodOptional<z.ZodBoolean>;
      }, z.core.$strip>>;
      execution: z.ZodOptional<z.ZodObject<{
        taskSupport: z.ZodOptional<z.ZodEnum<{
          optional: "optional";
          required: "required";
          forbidden: "forbidden";
        }>>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
    toolChoice: z.ZodOptional<z.ZodObject<{
      mode: z.ZodOptional<z.ZodEnum<{
        required: "required";
        auto: "auto";
        none: "none";
      }>>;
    }, z.core.$strip>>;
  }, z.core.$strip>;
}, z.core.$strip>, z.ZodObject<{
  method: z.ZodLiteral<"elicitation/create">;
  params: z.ZodUnion<readonly [z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    task: z.ZodOptional<z.ZodObject<{
      ttl: z.ZodOptional<z.ZodNumber>;
    }, z.core.$strip>>;
    mode: z.ZodOptional<z.ZodLiteral<"form">>;
    message: z.ZodString;
    requestedSchema: z.ZodObject<{
      type: z.ZodLiteral<"object">;
      properties: z.ZodRecord<z.ZodString, z.ZodUnion<readonly [z.ZodUnion<readonly [z.ZodObject<{
        type: z.ZodLiteral<"string">;
        title: z.ZodOptional<z.ZodString>;
        description: z.ZodOptional<z.ZodString>;
        enum: z.ZodArray<z.ZodString>;
        enumNames: z.ZodOptional<z.ZodArray<z.ZodString>>;
        default: z.ZodOptional<z.ZodString>;
      }, z.core.$strip>, z.ZodUnion<readonly [z.ZodObject<{
        type: z.ZodLiteral<"string">;
        title: z.ZodOptional<z.ZodString>;
        description: z.ZodOptional<z.ZodString>;
        enum: z.ZodArray<z.ZodString>;
        default: z.ZodOptional<z.ZodString>;
      }, z.core.$strip>, z.ZodObject<{
        type: z.ZodLiteral<"string">;
        title: z.ZodOptional<z.ZodString>;
        description: z.ZodOptional<z.ZodString>;
        oneOf: z.ZodArray<z.ZodObject<{
          const: z.ZodString;
          title: z.ZodString;
        }, z.core.$strip>>;
        default: z.ZodOptional<z.ZodString>;
      }, z.core.$strip>]>, z.ZodUnion<readonly [z.ZodObject<{
        type: z.ZodLiteral<"array">;
        title: z.ZodOptional<z.ZodString>;
        description: z.ZodOptional<z.ZodString>;
        minItems: z.ZodOptional<z.ZodNumber>;
        maxItems: z.ZodOptional<z.ZodNumber>;
        items: z.ZodObject<{
          type: z.ZodLiteral<"string">;
          enum: z.ZodArray<z.ZodString>;
        }, z.core.$strip>;
        default: z.ZodOptional<z.ZodArray<z.ZodString>>;
      }, z.core.$strip>, z.ZodObject<{
        type: z.ZodLiteral<"array">;
        title: z.ZodOptional<z.ZodString>;
        description: z.ZodOptional<z.ZodString>;
        minItems: z.ZodOptional<z.ZodNumber>;
        maxItems: z.ZodOptional<z.ZodNumber>;
        items: z.ZodObject<{
          anyOf: z.ZodArray<z.ZodObject<{
            const: z.ZodString;
            title: z.ZodString;
          }, z.core.$strip>>;
        }, z.core.$strip>;
        default: z.ZodOptional<z.ZodArray<z.ZodString>>;
      }, z.core.$strip>]>]>, z.ZodObject<{
        type: z.ZodLiteral<"boolean">;
        title: z.ZodOptional<z.ZodString>;
        description: z.ZodOptional<z.ZodString>;
        default: z.ZodOptional<z.ZodBoolean>;
      }, z.core.$strip>, z.ZodObject<{
        type: z.ZodLiteral<"string">;
        title: z.ZodOptional<z.ZodString>;
        description: z.ZodOptional<z.ZodString>;
        minLength: z.ZodOptional<z.ZodNumber>;
        maxLength: z.ZodOptional<z.ZodNumber>;
        format: z.ZodOptional<z.ZodEnum<{
          date: "date";
          uri: "uri";
          email: "email";
          "date-time": "date-time";
        }>>;
        default: z.ZodOptional<z.ZodString>;
      }, z.core.$strip>, z.ZodObject<{
        type: z.ZodEnum<{
          number: "number";
          integer: "integer";
        }>;
        title: z.ZodOptional<z.ZodString>;
        description: z.ZodOptional<z.ZodString>;
        minimum: z.ZodOptional<z.ZodNumber>;
        maximum: z.ZodOptional<z.ZodNumber>;
        default: z.ZodOptional<z.ZodNumber>;
      }, z.core.$strip>]>>;
      required: z.ZodOptional<z.ZodArray<z.ZodString>>;
    }, z.core.$catchall<z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    task: z.ZodOptional<z.ZodObject<{
      ttl: z.ZodOptional<z.ZodNumber>;
    }, z.core.$strip>>;
    mode: z.ZodLiteral<"url">;
    message: z.ZodString;
    elicitationId: z.ZodString;
    url: z.ZodString;
  }, z.core.$strip>]>;
}, z.core.$strip>, z.ZodObject<{
  method: z.ZodLiteral<"roots/list">;
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
  }, z.core.$strip>>;
}, z.core.$strip>]>;
declare const ServerNotificationSchema: z.ZodUnion<readonly [z.ZodObject<{
  method: z.ZodLiteral<"notifications/cancelled">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    requestId: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
    reason: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>;
}, z.core.$strip>, z.ZodObject<{
  method: z.ZodLiteral<"notifications/progress">;
  params: z.ZodObject<{
    progressToken: z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>;
    progress: z.ZodNumber;
    total: z.ZodOptional<z.ZodNumber>;
    message: z.ZodOptional<z.ZodString>;
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
  }, z.core.$strip>;
}, z.core.$strip>, z.ZodObject<{
  method: z.ZodLiteral<"notifications/message">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    level: z.ZodEnum<{
      error: "error";
      debug: "debug";
      info: "info";
      notice: "notice";
      warning: "warning";
      critical: "critical";
      alert: "alert";
      emergency: "emergency";
    }>;
    logger: z.ZodOptional<z.ZodString>;
    data: z.ZodUnknown;
  }, z.core.$strip>;
}, z.core.$strip>, z.ZodObject<{
  method: z.ZodLiteral<"notifications/resources/updated">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    uri: z.ZodString;
  }, z.core.$strip>;
}, z.core.$strip>, z.ZodObject<{
  method: z.ZodLiteral<"notifications/resources/list_changed">;
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
  }, z.core.$strip>>;
}, z.core.$strip>, z.ZodObject<{
  method: z.ZodLiteral<"notifications/tools/list_changed">;
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
  }, z.core.$strip>>;
}, z.core.$strip>, z.ZodObject<{
  method: z.ZodLiteral<"notifications/prompts/list_changed">;
  params: z.ZodOptional<z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
  }, z.core.$strip>>;
}, z.core.$strip>, z.ZodObject<{
  method: z.ZodLiteral<"notifications/subscriptions/acknowledged">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    notifications: z.ZodObject<{
      toolsListChanged: z.ZodOptional<z.ZodBoolean>;
      promptsListChanged: z.ZodOptional<z.ZodBoolean>;
      resourcesListChanged: z.ZodOptional<z.ZodBoolean>;
      resourceSubscriptions: z.ZodOptional<z.ZodArray<z.ZodString>>;
    }, z.core.$strip>;
  }, z.core.$strip>;
}, z.core.$strip>, z.ZodObject<{
  method: z.ZodLiteral<"notifications/elicitation/complete">;
  params: z.ZodObject<{
    _meta: z.ZodOptional<z.ZodObject<{
      /**
       * If specified, the caller is requesting out-of-band progress notifications for this request (as represented by notifications/progress). The value of this parameter is an opaque token that will be attached to any subsequent notifications. The receiver is not obligated to provide these notifications.
       */
      progressToken: z.ZodOptional<z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>>;
      /**
       * If specified, this request is related to the provided task.
       */
      "io.modelcontextprotocol/related-task": z.ZodOptional<z.ZodObject<{
        taskId: z.ZodString;
      }, z.core.$strip>>;
    }, z.core.$loose>>;
    elicitationId: z.ZodString;
  }, z.core.$strip>;
}, z.core.$strip>]>;
declare const ServerResultSchema: z.ZodUnion<readonly [z.ZodObject<{
  /**
   * See [MCP specification](https://modelcontextprotocol.io/specification/2026-07-28/basic/index#general-fields)
   * for notes on `_meta` usage.
   */
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
}, z.core.$strict>, z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  protocolVersion: z.ZodString;
  capabilities: z.ZodObject<{
    experimental: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>>;
    logging: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
    completions: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
    prompts: z.ZodOptional<z.ZodObject<{
      listChanged: z.ZodOptional<z.ZodBoolean>;
    }, z.core.$strip>>;
    resources: z.ZodOptional<z.ZodObject<{
      subscribe: z.ZodOptional<z.ZodBoolean>;
      listChanged: z.ZodOptional<z.ZodBoolean>;
    }, z.core.$strip>>;
    tools: z.ZodOptional<z.ZodObject<{
      listChanged: z.ZodOptional<z.ZodBoolean>;
    }, z.core.$strip>>;
    tasks: z.ZodOptional<z.ZodObject<{
      /**
       * Present if the server supports listing tasks.
       */
      list: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
      /**
       * Present if the server supports cancelling tasks.
       */
      cancel: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
      /**
       * Capabilities for task creation on specific request types.
       */
      requests: z.ZodOptional<z.ZodObject<{
        /**
         * Task support for tool requests.
         */
        tools: z.ZodOptional<z.ZodObject<{
          call: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
        }, z.core.$loose>>;
      }, z.core.$loose>>;
    }, z.core.$loose>>;
    extensions: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>>;
  }, z.core.$strip>;
  serverInfo: z.ZodObject<{
    version: z.ZodString;
    websiteUrl: z.ZodOptional<z.ZodString>;
    description: z.ZodOptional<z.ZodString>;
    icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
      src: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
      theme: z.ZodOptional<z.ZodEnum<{
        light: "light";
        dark: "dark";
      }>>;
    }, z.core.$strip>>>;
    name: z.ZodString;
    title: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>;
  instructions: z.ZodOptional<z.ZodString>;
}, z.core.$loose>, z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  supportedVersions: z.ZodArray<z.ZodString>;
  capabilities: z.ZodObject<{
    experimental: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>>;
    logging: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
    completions: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
    prompts: z.ZodOptional<z.ZodObject<{
      listChanged: z.ZodOptional<z.ZodBoolean>;
    }, z.core.$strip>>;
    resources: z.ZodOptional<z.ZodObject<{
      subscribe: z.ZodOptional<z.ZodBoolean>;
      listChanged: z.ZodOptional<z.ZodBoolean>;
    }, z.core.$strip>>;
    tools: z.ZodOptional<z.ZodObject<{
      listChanged: z.ZodOptional<z.ZodBoolean>;
    }, z.core.$strip>>;
    tasks: z.ZodOptional<z.ZodObject<{
      /**
       * Present if the server supports listing tasks.
       */
      list: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
      /**
       * Present if the server supports cancelling tasks.
       */
      cancel: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
      /**
       * Capabilities for task creation on specific request types.
       */
      requests: z.ZodOptional<z.ZodObject<{
        /**
         * Task support for tool requests.
         */
        tools: z.ZodOptional<z.ZodObject<{
          call: z.ZodOptional<z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>;
        }, z.core.$loose>>;
      }, z.core.$loose>>;
    }, z.core.$loose>>;
    extensions: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodType<JSONObject, JSONObject, z.core.$ZodTypeInternals<JSONObject, JSONObject>>>>;
  }, z.core.$strip>;
  instructions: z.ZodOptional<z.ZodString>;
}, z.core.$loose>, z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  completion: z.ZodObject<{
    /**
     * An array of completion values. Must not exceed 100 items.
     */
    values: z.ZodArray<z.ZodString>;
    /**
     * The total number of completion options available. This can exceed the number of values actually sent in the response.
     */
    total: z.ZodOptional<z.ZodNumber>;
    /**
     * Indicates whether there are additional completion options beyond those provided in the current response, even if the exact total is unknown.
     */
    hasMore: z.ZodOptional<z.ZodBoolean>;
  }, z.core.$loose>;
}, z.core.$loose>, z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  description: z.ZodOptional<z.ZodString>;
  messages: z.ZodArray<z.ZodObject<{
    role: z.ZodEnum<{
      user: "user";
      assistant: "assistant";
    }>;
    content: z.ZodUnion<readonly [z.ZodObject<{
      type: z.ZodLiteral<"text">;
      text: z.ZodString;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"image">;
      data: z.ZodString;
      mimeType: z.ZodString;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"audio">;
      data: z.ZodString;
      mimeType: z.ZodString;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>, z.ZodObject<{
      uri: z.ZodString;
      description: z.ZodOptional<z.ZodString>;
      mimeType: z.ZodOptional<z.ZodString>;
      size: z.ZodOptional<z.ZodNumber>;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
      type: z.ZodLiteral<"resource_link">;
    }, z.core.$strip>, z.ZodObject<{
      type: z.ZodLiteral<"resource">;
      resource: z.ZodUnion<readonly [z.ZodObject<{
        uri: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        text: z.ZodString;
      }, z.core.$strip>, z.ZodObject<{
        uri: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
        blob: z.ZodString;
      }, z.core.$strip>]>;
      annotations: z.ZodOptional<z.ZodObject<{
        audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
          user: "user";
          assistant: "assistant";
        }>>>;
        priority: z.ZodOptional<z.ZodNumber>;
        lastModified: z.ZodOptional<z.ZodISODateTime>;
      }, z.core.$strip>>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    }, z.core.$strip>]>;
  }, z.core.$strip>>;
}, z.core.$loose>, z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  nextCursor: z.ZodOptional<z.ZodString>;
  prompts: z.ZodArray<z.ZodObject<{
    description: z.ZodOptional<z.ZodString>;
    arguments: z.ZodOptional<z.ZodArray<z.ZodObject<{
      name: z.ZodString;
      description: z.ZodOptional<z.ZodString>;
      required: z.ZodOptional<z.ZodBoolean>;
    }, z.core.$strip>>>;
    _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
    icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
      src: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
      theme: z.ZodOptional<z.ZodEnum<{
        light: "light";
        dark: "dark";
      }>>;
    }, z.core.$strip>>>;
    name: z.ZodString;
    title: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>>;
}, z.core.$loose>, z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  nextCursor: z.ZodOptional<z.ZodString>;
  resources: z.ZodArray<z.ZodObject<{
    uri: z.ZodString;
    description: z.ZodOptional<z.ZodString>;
    mimeType: z.ZodOptional<z.ZodString>;
    size: z.ZodOptional<z.ZodNumber>;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
    icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
      src: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
      theme: z.ZodOptional<z.ZodEnum<{
        light: "light";
        dark: "dark";
      }>>;
    }, z.core.$strip>>>;
    name: z.ZodString;
    title: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>>;
}, z.core.$loose>, z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  nextCursor: z.ZodOptional<z.ZodString>;
  resourceTemplates: z.ZodArray<z.ZodObject<{
    uriTemplate: z.ZodString;
    description: z.ZodOptional<z.ZodString>;
    mimeType: z.ZodOptional<z.ZodString>;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
    icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
      src: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
      theme: z.ZodOptional<z.ZodEnum<{
        light: "light";
        dark: "dark";
      }>>;
    }, z.core.$strip>>>;
    name: z.ZodString;
    title: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>>;
}, z.core.$loose>, z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  contents: z.ZodArray<z.ZodUnion<readonly [z.ZodObject<{
    uri: z.ZodString;
    mimeType: z.ZodOptional<z.ZodString>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    text: z.ZodString;
  }, z.core.$strip>, z.ZodObject<{
    uri: z.ZodString;
    mimeType: z.ZodOptional<z.ZodString>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    blob: z.ZodString;
  }, z.core.$strip>]>>;
}, z.core.$loose>, z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  content: z.ZodDefault<z.ZodArray<z.ZodUnion<readonly [z.ZodObject<{
    type: z.ZodLiteral<"text">;
    text: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"image">;
    data: z.ZodString;
    mimeType: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"audio">;
    data: z.ZodString;
    mimeType: z.ZodString;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>, z.ZodObject<{
    uri: z.ZodString;
    description: z.ZodOptional<z.ZodString>;
    mimeType: z.ZodOptional<z.ZodString>;
    size: z.ZodOptional<z.ZodNumber>;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodObject<{}, z.core.$loose>>;
    icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
      src: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
      theme: z.ZodOptional<z.ZodEnum<{
        light: "light";
        dark: "dark";
      }>>;
    }, z.core.$strip>>>;
    name: z.ZodString;
    title: z.ZodOptional<z.ZodString>;
    type: z.ZodLiteral<"resource_link">;
  }, z.core.$strip>, z.ZodObject<{
    type: z.ZodLiteral<"resource">;
    resource: z.ZodUnion<readonly [z.ZodObject<{
      uri: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      text: z.ZodString;
    }, z.core.$strip>, z.ZodObject<{
      uri: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
      blob: z.ZodString;
    }, z.core.$strip>]>;
    annotations: z.ZodOptional<z.ZodObject<{
      audience: z.ZodOptional<z.ZodArray<z.ZodEnum<{
        user: "user";
        assistant: "assistant";
      }>>>;
      priority: z.ZodOptional<z.ZodNumber>;
      lastModified: z.ZodOptional<z.ZodISODateTime>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
  }, z.core.$strip>]>>>;
  structuredContent: z.ZodOptional<z.ZodUnknown>;
  isError: z.ZodOptional<z.ZodBoolean>;
}, z.core.$loose>, z.ZodObject<{
  _meta: z.ZodOptional<z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
  }, z.core.$loose>>;
  nextCursor: z.ZodOptional<z.ZodString>;
  tools: z.ZodArray<z.ZodObject<{
    description: z.ZodOptional<z.ZodString>;
    inputSchema: z.ZodObject<{
      type: z.ZodLiteral<"object">;
      properties: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodType<JSONValue, JSONValue, z.core.$ZodTypeInternals<JSONValue, JSONValue>>>>;
      required: z.ZodOptional<z.ZodArray<z.ZodString>>;
    }, z.core.$catchall<z.ZodUnknown>>;
    outputSchema: z.ZodOptional<z.ZodObject<{
      $schema: z.ZodOptional<z.ZodString>;
    }, z.core.$loose>>;
    annotations: z.ZodOptional<z.ZodObject<{
      title: z.ZodOptional<z.ZodString>;
      readOnlyHint: z.ZodOptional<z.ZodBoolean>;
      destructiveHint: z.ZodOptional<z.ZodBoolean>;
      idempotentHint: z.ZodOptional<z.ZodBoolean>;
      openWorldHint: z.ZodOptional<z.ZodBoolean>;
    }, z.core.$strip>>;
    execution: z.ZodOptional<z.ZodObject<{
      taskSupport: z.ZodOptional<z.ZodEnum<{
        optional: "optional";
        required: "required";
        forbidden: "forbidden";
      }>>;
    }, z.core.$strip>>;
    _meta: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodUnknown>>;
    icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
      src: z.ZodString;
      mimeType: z.ZodOptional<z.ZodString>;
      sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
      theme: z.ZodOptional<z.ZodEnum<{
        light: "light";
        dark: "dark";
      }>>;
    }, z.core.$strip>>>;
    name: z.ZodString;
    title: z.ZodOptional<z.ZodString>;
  }, z.core.$strip>>;
}, z.core.$loose>, z.ZodObject<{
  _meta: z.ZodObject<{
    readonly "io.modelcontextprotocol/serverInfo": z.ZodCatch<z.ZodOptional<z.ZodObject<{
      version: z.ZodString;
      websiteUrl: z.ZodOptional<z.ZodString>;
      description: z.ZodOptional<z.ZodString>;
      icons: z.ZodOptional<z.ZodArray<z.ZodObject<{
        src: z.ZodString;
        mimeType: z.ZodOptional<z.ZodString>;
        sizes: z.ZodOptional<z.ZodArray<z.ZodString>>;
        theme: z.ZodOptional<z.ZodEnum<{
          light: "light";
          dark: "dark";
        }>>;
      }, z.core.$strip>>>;
      name: z.ZodString;
      title: z.ZodOptional<z.ZodString>;
    }, z.core.$strip>>>;
    "io.modelcontextprotocol/subscriptionId": z.ZodUnion<readonly [z.ZodString, z.ZodNumber]>;
  }, z.core.$loose>;
}, z.core.$loose>]>;
//#endregion
//#region src/auth.d.ts
/**
 * Reusable URL validation that disallows `javascript:` scheme
 */
declare const SafeUrlSchema: z.ZodURL;
/**
 * RFC 9728 OAuth Protected Resource Metadata
 */
declare const OAuthProtectedResourceMetadataSchema: z.ZodObject<{
  resource: z.ZodString;
  authorization_servers: z.ZodOptional<z.ZodArray<z.ZodURL>>;
  jwks_uri: z.ZodOptional<z.ZodString>;
  scopes_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  bearer_methods_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  resource_signing_alg_values_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  resource_name: z.ZodOptional<z.ZodString>;
  resource_documentation: z.ZodOptional<z.ZodString>;
  resource_policy_uri: z.ZodOptional<z.ZodString>;
  resource_tos_uri: z.ZodOptional<z.ZodString>;
  tls_client_certificate_bound_access_tokens: z.ZodOptional<z.ZodBoolean>;
  authorization_details_types_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  dpop_signing_alg_values_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  dpop_bound_access_tokens_required: z.ZodOptional<z.ZodBoolean>;
}, z.core.$loose>;
/**
 * RFC 8414 OAuth 2.0 Authorization Server Metadata
 */
declare const OAuthMetadataSchema: z.ZodObject<{
  issuer: z.ZodString;
  authorization_endpoint: z.ZodURL;
  token_endpoint: z.ZodURL;
  registration_endpoint: z.ZodOptional<z.ZodURL>;
  scopes_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  response_types_supported: z.ZodArray<z.ZodString>;
  response_modes_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  grant_types_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  token_endpoint_auth_methods_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  token_endpoint_auth_signing_alg_values_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  service_documentation: z.ZodOptional<z.ZodURL>;
  revocation_endpoint: z.ZodOptional<z.ZodURL>;
  revocation_endpoint_auth_methods_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  revocation_endpoint_auth_signing_alg_values_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  introspection_endpoint: z.ZodOptional<z.ZodString>;
  introspection_endpoint_auth_methods_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  introspection_endpoint_auth_signing_alg_values_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  code_challenge_methods_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  client_id_metadata_document_supported: z.ZodOptional<z.ZodBoolean>;
  authorization_response_iss_parameter_supported: z.ZodCatch<z.ZodOptional<z.ZodBoolean>>;
}, z.core.$loose>;
/**
 * OpenID Connect Discovery 1.0 Provider Metadata
 *
 * @see https://openid.net/specs/openid-connect-discovery-1_0.html#ProviderMetadata
 */
declare const OpenIdProviderMetadataSchema: z.ZodObject<{
  issuer: z.ZodString;
  authorization_endpoint: z.ZodURL;
  token_endpoint: z.ZodURL;
  userinfo_endpoint: z.ZodOptional<z.ZodURL>;
  jwks_uri: z.ZodURL;
  registration_endpoint: z.ZodOptional<z.ZodURL>;
  scopes_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  response_types_supported: z.ZodArray<z.ZodString>;
  response_modes_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  grant_types_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  acr_values_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  subject_types_supported: z.ZodArray<z.ZodString>;
  id_token_signing_alg_values_supported: z.ZodArray<z.ZodString>;
  id_token_encryption_alg_values_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  id_token_encryption_enc_values_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  userinfo_signing_alg_values_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  userinfo_encryption_alg_values_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  userinfo_encryption_enc_values_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  request_object_signing_alg_values_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  request_object_encryption_alg_values_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  request_object_encryption_enc_values_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  token_endpoint_auth_methods_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  token_endpoint_auth_signing_alg_values_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  display_values_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  claim_types_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  claims_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  service_documentation: z.ZodOptional<z.ZodString>;
  claims_locales_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  ui_locales_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  claims_parameter_supported: z.ZodOptional<z.ZodBoolean>;
  request_parameter_supported: z.ZodOptional<z.ZodBoolean>;
  request_uri_parameter_supported: z.ZodOptional<z.ZodBoolean>;
  require_request_uri_registration: z.ZodOptional<z.ZodBoolean>;
  op_policy_uri: z.ZodOptional<z.ZodURL>;
  op_tos_uri: z.ZodOptional<z.ZodURL>;
  client_id_metadata_document_supported: z.ZodOptional<z.ZodBoolean>;
  authorization_response_iss_parameter_supported: z.ZodCatch<z.ZodOptional<z.ZodBoolean>>;
}, z.core.$loose>;
/**
 * OpenID Connect Discovery metadata that may include OAuth 2.0 fields
 * This schema represents the real-world scenario where OIDC providers
 * return a mix of OpenID Connect and OAuth 2.0 metadata fields
 */
declare const OpenIdProviderDiscoveryMetadataSchema: z.ZodObject<{
  code_challenge_methods_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  issuer: z.ZodString;
  authorization_endpoint: z.ZodURL;
  token_endpoint: z.ZodURL;
  userinfo_endpoint: z.ZodOptional<z.ZodURL>;
  jwks_uri: z.ZodURL;
  registration_endpoint: z.ZodOptional<z.ZodURL>;
  scopes_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  response_types_supported: z.ZodArray<z.ZodString>;
  response_modes_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  grant_types_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  acr_values_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  subject_types_supported: z.ZodArray<z.ZodString>;
  id_token_signing_alg_values_supported: z.ZodArray<z.ZodString>;
  id_token_encryption_alg_values_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  id_token_encryption_enc_values_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  userinfo_signing_alg_values_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  userinfo_encryption_alg_values_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  userinfo_encryption_enc_values_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  request_object_signing_alg_values_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  request_object_encryption_alg_values_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  request_object_encryption_enc_values_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  token_endpoint_auth_methods_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  token_endpoint_auth_signing_alg_values_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  display_values_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  claim_types_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  claims_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  service_documentation: z.ZodOptional<z.ZodString>;
  claims_locales_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  ui_locales_supported: z.ZodOptional<z.ZodArray<z.ZodString>>;
  claims_parameter_supported: z.ZodOptional<z.ZodBoolean>;
  request_parameter_supported: z.ZodOptional<z.ZodBoolean>;
  request_uri_parameter_supported: z.ZodOptional<z.ZodBoolean>;
  require_request_uri_registration: z.ZodOptional<z.ZodBoolean>;
  op_policy_uri: z.ZodOptional<z.ZodURL>;
  op_tos_uri: z.ZodOptional<z.ZodURL>;
  client_id_metadata_document_supported: z.ZodOptional<z.ZodBoolean>;
  authorization_response_iss_parameter_supported: z.ZodCatch<z.ZodOptional<z.ZodBoolean>>;
}, z.core.$strip>;
/**
 * OAuth 2.1 token response
 */
declare const OAuthTokensSchema: z.ZodObject<{
  access_token: z.ZodString;
  id_token: z.ZodOptional<z.ZodString>;
  token_type: z.ZodString;
  expires_in: z.ZodOptional<z.ZodCoercedNumber<unknown>>;
  scope: z.ZodOptional<z.ZodString>;
  refresh_token: z.ZodOptional<z.ZodString>;
}, z.core.$strip>;
/**
 * RFC 8693 §2.2.1 Token Exchange response for ID-JAG tokens.
 *
 * `token_type` is intentionally optional: per RFC 8693 §2.2.1 it is informational when
 * the issued token is not an access token, and per RFC 6749 §5.1 it is case-insensitive,
 * so strict checking rejects conformant IdPs.
 */
declare const IdJagTokenExchangeResponseSchema: z.ZodObject<{
  issued_token_type: z.ZodLiteral<"urn:ietf:params:oauth:token-type:id-jag">;
  access_token: z.ZodString;
  token_type: z.ZodOptional<z.ZodString>;
  expires_in: z.ZodOptional<z.ZodNumber>;
  scope: z.ZodOptional<z.ZodString>;
}, z.core.$strip>;
type IdJagTokenExchangeResponse = z.infer<typeof IdJagTokenExchangeResponseSchema>;
/**
 * OAuth 2.1 error response
 */
declare const OAuthErrorResponseSchema: z.ZodObject<{
  error: z.ZodString;
  error_description: z.ZodOptional<z.ZodString>;
  error_uri: z.ZodOptional<z.ZodString>;
}, z.core.$strip>;
/**
 * Optional version of {@linkcode SafeUrlSchema} that allows empty string for backward compatibility on `tos_uri` and `logo_uri`
 */
declare const OptionalSafeUrlSchema: z.ZodUnion<[z.ZodOptional<z.ZodURL>, z.ZodPipe<z.ZodLiteral<"">, z.ZodTransform<undefined, "">>]>;
/**
 * RFC 7591 OAuth 2.0 Dynamic Client Registration metadata
 */
declare const OAuthClientMetadataSchema: z.ZodObject<{
  redirect_uris: z.ZodArray<z.ZodURL>;
  token_endpoint_auth_method: z.ZodOptional<z.ZodString>;
  grant_types: z.ZodOptional<z.ZodArray<z.ZodString>>;
  response_types: z.ZodOptional<z.ZodArray<z.ZodString>>;
  application_type: z.ZodOptional<z.ZodString>;
  client_name: z.ZodOptional<z.ZodString>;
  client_uri: z.ZodOptional<z.ZodURL>;
  logo_uri: z.ZodUnion<[z.ZodOptional<z.ZodURL>, z.ZodPipe<z.ZodLiteral<"">, z.ZodTransform<undefined, "">>]>;
  scope: z.ZodOptional<z.ZodString>;
  contacts: z.ZodOptional<z.ZodArray<z.ZodString>>;
  tos_uri: z.ZodUnion<[z.ZodOptional<z.ZodURL>, z.ZodPipe<z.ZodLiteral<"">, z.ZodTransform<undefined, "">>]>;
  policy_uri: z.ZodOptional<z.ZodString>;
  jwks_uri: z.ZodOptional<z.ZodURL>;
  jwks: z.ZodOptional<z.ZodAny>;
  software_id: z.ZodOptional<z.ZodString>;
  software_version: z.ZodOptional<z.ZodString>;
  software_statement: z.ZodOptional<z.ZodString>;
}, z.core.$strip>;
/**
 * RFC 7591 OAuth 2.0 Dynamic Client Registration client information
 */
declare const OAuthClientInformationSchema: z.ZodObject<{
  client_id: z.ZodString;
  client_secret: z.ZodOptional<z.ZodString>;
  client_id_issued_at: z.ZodOptional<z.ZodNumber>;
  client_secret_expires_at: z.ZodOptional<z.ZodNumber>;
}, z.core.$strip>;
/**
 * RFC 7591 OAuth 2.0 Dynamic Client Registration full response (client information plus metadata)
 */
declare const OAuthClientInformationFullSchema: z.ZodObject<{
  redirect_uris: z.ZodArray<z.ZodURL>;
  token_endpoint_auth_method: z.ZodOptional<z.ZodString>;
  grant_types: z.ZodOptional<z.ZodArray<z.ZodString>>;
  response_types: z.ZodOptional<z.ZodArray<z.ZodString>>;
  application_type: z.ZodOptional<z.ZodString>;
  client_name: z.ZodOptional<z.ZodString>;
  client_uri: z.ZodOptional<z.ZodURL>;
  logo_uri: z.ZodUnion<[z.ZodOptional<z.ZodURL>, z.ZodPipe<z.ZodLiteral<"">, z.ZodTransform<undefined, "">>]>;
  scope: z.ZodOptional<z.ZodString>;
  contacts: z.ZodOptional<z.ZodArray<z.ZodString>>;
  tos_uri: z.ZodUnion<[z.ZodOptional<z.ZodURL>, z.ZodPipe<z.ZodLiteral<"">, z.ZodTransform<undefined, "">>]>;
  policy_uri: z.ZodOptional<z.ZodString>;
  jwks_uri: z.ZodOptional<z.ZodURL>;
  jwks: z.ZodOptional<z.ZodAny>;
  software_id: z.ZodOptional<z.ZodString>;
  software_version: z.ZodOptional<z.ZodString>;
  software_statement: z.ZodOptional<z.ZodString>;
  client_id: z.ZodString;
  client_secret: z.ZodOptional<z.ZodString>;
  client_id_issued_at: z.ZodOptional<z.ZodNumber>;
  client_secret_expires_at: z.ZodOptional<z.ZodNumber>;
}, z.core.$strip>;
/**
 * RFC 7591 OAuth 2.0 Dynamic Client Registration error response
 */
declare const OAuthClientRegistrationErrorSchema: z.ZodObject<{
  error: z.ZodString;
  error_description: z.ZodOptional<z.ZodString>;
}, z.core.$strip>;
/**
 * RFC 7009 OAuth 2.0 Token Revocation request
 */
declare const OAuthTokenRevocationRequestSchema: z.ZodObject<{
  token: z.ZodString;
  token_type_hint: z.ZodOptional<z.ZodString>;
}, z.core.$strip>;
type OAuthMetadata = z.infer<typeof OAuthMetadataSchema>;
type OpenIdProviderMetadata = z.infer<typeof OpenIdProviderMetadataSchema>;
type OpenIdProviderDiscoveryMetadata = z.infer<typeof OpenIdProviderDiscoveryMetadataSchema>;
type OAuthTokens = z.infer<typeof OAuthTokensSchema>;
type OAuthErrorResponse = z.infer<typeof OAuthErrorResponseSchema>;
type OAuthClientMetadata = z.infer<typeof OAuthClientMetadataSchema>;
type OAuthClientInformation = z.infer<typeof OAuthClientInformationSchema>;
type OAuthClientInformationFull = z.infer<typeof OAuthClientInformationFullSchema>;
type OAuthClientInformationMixed = OAuthClientInformation | OAuthClientInformationFull;
/**
 * {@linkcode OAuthTokens} as persisted by an `OAuthClientProvider`. Adds an
 * SDK-stamped authorization-server `issuer` identifier so stored tokens are
 * bound to the AS that issued them. The `issuer` field is **not** part of the
 * RFC 6749 wire response and is intentionally absent from the wire-response
 * schema; the client SDK writes it before calling `saveTokens`.
 */
type StoredOAuthTokens = OAuthTokens & {
  issuer?: string;
};
/**
 * {@linkcode OAuthClientInformationMixed} as persisted by an
 * `OAuthClientProvider`. Adds an SDK-stamped authorization-server `issuer`
 * identifier so stored client credentials are bound to the AS that issued them.
 * The `issuer` field is **not** part of the RFC 7591 wire response and is
 * intentionally absent from the wire-response schema; the client SDK writes it
 * before calling `saveClientInformation`.
 */
type StoredOAuthClientInformation = OAuthClientInformationMixed & {
  issuer?: string;
};
type OAuthClientRegistrationError = z.infer<typeof OAuthClientRegistrationErrorSchema>;
type OAuthTokenRevocationRequest = z.infer<typeof OAuthTokenRevocationRequestSchema>;
type OAuthProtectedResourceMetadata = z.infer<typeof OAuthProtectedResourceMetadataSchema>;
type AuthorizationServerMetadata = OAuthMetadata | OpenIdProviderDiscoveryMetadata;
//#endregion
export { ContentBlockSchema as $, ServerTasksCapabilitySchema as $n, ListTasksResultSchema as $t, StoredOAuthTokens as A, RequestMetaSchema as An, ToolUseContentSchema as Ar, InitializeRequestSchema as At, CancelTaskRequestSchema as B, ResourceUpdatedNotificationSchema as Bn, JSONRPCResultResponseSchema as Bt, OpenIdProviderDiscoveryMetadata as C, PromptReferenceSchema as Cn, TitledSingleSelectEnumSchemaSchema as Cr, GetTaskRequestSchema as Ct, OptionalSafeUrlSchema as D, ReadResourceResultSchema as Dn, ToolListChangedNotificationSchema as Dr, ImageContentSchema as Dt, OpenIdProviderMetadataSchema as E, ReadResourceRequestSchema as En, ToolExecutionSchema as Er, IconsSchema as Et, BlobResourceContentsSchema as F, ResourceRequestParamsSchema as Fn, JSONArray as Fr, JSONRPCErrorResponseSchema as Ft, ClientNotificationSchema as G, RootsListChangedNotificationSchema as Gn, ListPromptsResultSchema as Gt, CancelledNotificationParamsSchema as H, ResultSchema as Hn, LegacyTitledEnumSchemaSchema as Ht, BooleanSchemaSchema as I, ResourceSchema as In, JSONObject as Ir, JSONRPCMessageSchema as It, ClientTasksCapabilitySchema as J, SamplingMessageSchema as Jn, ListResourcesRequestSchema as Jt, ClientRequestSchema as K, SamplingContentSchema as Kn, ListResourceTemplatesRequestSchema as Kt, CallToolRequestParamsSchema as L, ResourceTemplateReferenceSchema as Ln, JSONValue as Lr, JSONRPCNotificationSchema as Lt, AudioContentSchema as M, ResourceContentsSchema as Mn, UnsubscribeRequestSchema as Mr, InitializedNotificationSchema as Mt, BaseMetadataSchema as N, ResourceLinkSchema as Nn, UntitledMultiSelectEnumSchemaSchema as Nr, JSONArraySchema as Nt, SafeUrlSchema as O, RelatedTaskMetadataSchema as On, ToolResultContentSchema as Or, ImplementationSchema as Ot, BaseRequestParamsSchema as P, ResourceListChangedNotificationSchema as Pn, UntitledSingleSelectEnumSchemaSchema as Pr, JSONObjectSchema as Pt, CompleteResultSchema as Q, ServerResultSchema as Qn, ListTasksRequestSchema as Qt, CallToolRequestSchema as R, ResourceTemplateSchema as Rn, JSONRPCRequestSchema as Rt, OAuthTokensSchema as S, PromptMessageSchema as Sn, TitledMultiSelectEnumSchemaSchema as Sr, GetTaskPayloadResultSchema as St, OpenIdProviderMetadata as T, ReadResourceRequestParamsSchema as Tn, ToolChoiceSchema as Tr, IconSchema as Tt, CancelledNotificationSchema as U, RoleSchema as Un, ListChangedOptionsBaseSchema as Ut, CancelTaskResultSchema as V, ResultMetaObjectSchema as Vn, JSONValueSchema as Vt, ClientCapabilitiesSchema as W, RootSchema as Wn, ListPromptsRequestSchema as Wt, CompleteRequestParamsSchema as X, ServerNotificationSchema as Xn, ListRootsRequestSchema as Xt, CompatibilityCallToolResultSchema as Y, ServerCapabilitiesSchema as Yn, ListResourcesResultSchema as Yt, CompleteRequestSchema as Z, ServerRequestSchema as Zn, ListRootsResultSchema as Zt, OAuthProtectedResourceMetadata as _, ProgressNotificationSchema as _n, TaskStatusNotificationParamsSchema as _r, EnumSchemaSchema as _t, OAuthClientInformationFull as a, ModelHintSchema as an, SubscribeRequestSchema as ar, CursorSchema as at, OAuthTokenRevocationRequestSchema as b, PromptArgumentSchema as bn, TextContentSchema as br, GetPromptResultSchema as bt, OAuthClientInformationSchema as c, NotificationSchema as cn, SubscriptionsAcknowledgedNotificationSchema as cr, ElicitRequestFormParamsSchema as ct, OAuthClientRegistrationError as d, PaginatedRequestParamsSchema as dn, SubscriptionsListenResultMetaSchema as dr, ElicitRequestURLParamsSchema as dt, ListToolsRequestSchema as en, SetLevelRequestParamsSchema as er, CreateMessageRequestParamsSchema as et, OAuthClientRegistrationErrorSchema as f, PaginatedRequestSchema as fn, SubscriptionsListenResultSchema as fr, ElicitResultSchema as ft, OAuthMetadataSchema as g, ProgressNotificationParamsSchema as gn, TaskSchema as gr, EmptyResultSchema as gt, OAuthMetadata as h, PrimitiveSchemaDefinitionSchema as hn, TaskMetadataSchema as hr, EmbeddedResourceSchema as ht, OAuthClientInformation as i, LoggingMessageNotificationSchema as in, SubscribeRequestParamsSchema as ir, CreateTaskResultSchema as it, AnnotationsSchema as j, RequestSchema as jn, UnsubscribeRequestParamsSchema as jr, InitializeResultSchema as jt, StoredOAuthClientInformation as k, RequestIdSchema as kn, ToolSchema as kr, InitializeRequestParamsSchema as kt, OAuthClientMetadata as l, NotificationsParamsSchema as ln, SubscriptionsListenRequestParamsSchema as lr, ElicitRequestParamsSchema as lt, OAuthErrorResponseSchema as m, PingRequestSchema as mn, TaskCreationParamsSchema as mr, ElicitationCompleteNotificationSchema as mt, IdJagTokenExchangeResponse as n, LoggingLevelSchema as nn, SingleSelectEnumSchemaSchema as nr, CreateMessageResultSchema as nt, OAuthClientInformationFullSchema as o, ModelPreferencesSchema as on, SubscriptionFilterSchema as or, DiscoverRequestSchema as ot, OAuthErrorResponse as p, PaginatedResultSchema as pn, TaskAugmentedRequestParamsSchema as pr, ElicitationCompleteNotificationParamsSchema as pt, ClientResultSchema as q, SamplingMessageContentBlockSchema as qn, ListResourceTemplatesResultSchema as qt, IdJagTokenExchangeResponseSchema as r, LoggingMessageNotificationParamsSchema as rn, StringSchemaSchema as rr, CreateMessageResultWithToolsSchema as rt, OAuthClientInformationMixed as s, MultiSelectEnumSchemaSchema as sn, SubscriptionsAcknowledgedNotificationParamsSchema as sr, DiscoverResultSchema as st, AuthorizationServerMetadata as t, ListToolsResultSchema as tn, SetLevelRequestSchema as tr, CreateMessageRequestSchema as tt, OAuthClientMetadataSchema as u, NumberSchemaSchema as un, SubscriptionsListenRequestSchema as ur, ElicitRequestSchema as ut, OAuthProtectedResourceMetadataSchema as v, ProgressSchema as vn, TaskStatusNotificationSchema as vr, GetPromptRequestParamsSchema as vt, OpenIdProviderDiscoveryMetadataSchema as w, PromptSchema as wn, ToolAnnotationsSchema as wr, GetTaskResultSchema as wt, OAuthTokens as x, PromptListChangedNotificationSchema as xn, TextResourceContentsSchema as xr, GetTaskPayloadRequestSchema as xt, OAuthTokenRevocationRequest as y, ProgressTokenSchema as yn, TaskStatusSchema as yr, GetPromptRequestSchema as yt, CallToolResultSchema as z, ResourceUpdatedNotificationParamsSchema as zn, JSONRPCResponseSchema as zt };
//# sourceMappingURL=auth-BWdKR39I.d.mts.map