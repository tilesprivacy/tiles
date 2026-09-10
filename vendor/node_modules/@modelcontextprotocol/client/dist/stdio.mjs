import { At as SdkErrorCode, kt as SdkError, s as ReadBuffer, u as serializeMessage } from "./src-D_zzAWoS.mjs";
import process from "node:process";
import { PassThrough } from "node:stream";
import spawn from "cross-spawn";

//#region src/client/stdio.ts
/**
* Environment variables to inherit by default, if an environment is not explicitly given.
*/
const DEFAULT_INHERITED_ENV_VARS = process.platform === "win32" ? [
	"APPDATA",
	"HOMEDRIVE",
	"HOMEPATH",
	"LOCALAPPDATA",
	"PATH",
	"PROCESSOR_ARCHITECTURE",
	"SYSTEMDRIVE",
	"SYSTEMROOT",
	"TEMP",
	"USERNAME",
	"USERPROFILE",
	"PROGRAMFILES"
] : [
	"HOME",
	"LOGNAME",
	"PATH",
	"SHELL",
	"TERM",
	"USER"
];
/**
* Returns a default environment object including only environment variables deemed safe to inherit.
*/
function getDefaultEnvironment() {
	const env = {};
	for (const key of DEFAULT_INHERITED_ENV_VARS) {
		const value = process.env[key];
		if (value === void 0) continue;
		if (value.startsWith("()")) continue;
		env[key] = value;
	}
	return env;
}
/**
* Client transport for stdio: this will connect to a server by spawning a process and communicating with it over stdin/stdout.
*
* This transport is only available in Node.js environments.
*/
var StdioClientTransport = class {
	_process;
	_readBuffer;
	_serverParams;
	_stderrStream = null;
	onclose;
	onerror;
	onmessage;
	constructor(server) {
		this._serverParams = server;
		this._readBuffer = new ReadBuffer({ maxBufferSize: server.maxBufferSize });
		if (server.stderr === "pipe" || server.stderr === "overlapped") this._stderrStream = new PassThrough();
	}
	/**
	* Starts the server process and prepares to communicate with it.
	*/
	async start() {
		if (this._process) throw new Error("StdioClientTransport already started! If using Client class, note that connect() calls start() automatically.");
		return new Promise((resolve, reject) => {
			this._process = spawn(this._serverParams.command, this._serverParams.args ?? [], {
				env: {
					...getDefaultEnvironment(),
					...this._serverParams.env
				},
				stdio: [
					"pipe",
					"pipe",
					this._serverParams.stderr ?? "inherit"
				],
				shell: false,
				windowsHide: process.platform === "win32",
				cwd: this._serverParams.cwd
			});
			this._process.on("error", (error) => {
				reject(error);
				this.onerror?.(error);
			});
			this._process.on("spawn", () => {
				resolve();
			});
			this._process.on("close", (_code) => {
				this._process = void 0;
				this.onclose?.();
			});
			this._process.stdin?.on("error", (error) => {
				this.onerror?.(error);
			});
			this._process.stdout?.on("data", (chunk) => {
				try {
					this._readBuffer.append(chunk);
					this.processReadBuffer();
				} catch (error) {
					this.onerror?.(error);
					this.close().catch(() => {});
				}
			});
			this._process.stdout?.on("error", (error) => {
				this.onerror?.(error);
			});
			if (this._stderrStream && this._process.stderr) this._process.stderr.pipe(this._stderrStream);
		});
	}
	/**
	* The `stderr` stream of the child process, if {@linkcode StdioServerParameters.stderr} was set to `"pipe"` or `"overlapped"`.
	*
	* If `stderr` piping was requested, a `PassThrough` stream is returned _immediately_, allowing callers to
	* attach listeners before the `start` method is invoked. This prevents loss of any early
	* error output emitted by the child process.
	*/
	get stderr() {
		if (this._stderrStream) return this._stderrStream;
		return this._process?.stderr ?? null;
	}
	/**
	* The child process pid spawned by this transport.
	*
	* This is only available after the transport has been started.
	*/
	get pid() {
		return this._process?.pid ?? null;
	}
	processReadBuffer() {
		while (true) try {
			const message = this._readBuffer.readMessage();
			if (message === null) break;
			this.onmessage?.(message);
		} catch (error) {
			this.onerror?.(error);
		}
	}
	/**
	* Reap a disposable probe sibling (see the version-negotiation sibling
	* flow): signal-first teardown awaiting process `exit` — never the `close`
	* event, so a helper process holding the child's stdio pipes can never
	* block disposal. Not part of the public transport lifecycle.
	*
	* @internal
	*/
	async _dispose() {
		const proc = this._process;
		this._process = void 0;
		if (proc && proc.exitCode === null && proc.signalCode === null) {
			const exited = new Promise((resolve) => proc.once("exit", () => resolve()));
			try {
				proc.stdin?.end();
			} catch {}
			try {
				proc.kill("SIGTERM");
			} catch {}
			await Promise.race([exited, new Promise((resolve) => setTimeout(resolve, 1e3).unref())]);
			if (proc.exitCode === null && proc.signalCode === null) try {
				proc.kill("SIGKILL");
			} catch {}
			await exited;
		}
		try {
			proc?.stdout?.destroy();
		} catch {}
		try {
			proc?.stdin?.destroy();
		} catch {}
		try {
			proc?.stderr?.destroy();
		} catch {}
		this._readBuffer.clear();
	}
	async close() {
		if (this._process) {
			const processToClose = this._process;
			this._process = void 0;
			const closePromise = new Promise((resolve) => {
				processToClose.once("close", () => {
					resolve();
				});
			});
			try {
				processToClose.stdin?.end();
			} catch {}
			await Promise.race([closePromise, new Promise((resolve) => setTimeout(resolve, 2e3).unref())]);
			if (processToClose.exitCode === null) {
				try {
					processToClose.kill("SIGTERM");
				} catch {}
				await Promise.race([closePromise, new Promise((resolve) => setTimeout(resolve, 2e3).unref())]);
			}
			if (processToClose.exitCode === null) try {
				processToClose.kill("SIGKILL");
			} catch {}
		}
		this._readBuffer.clear();
	}
	send(message) {
		return new Promise((resolve) => {
			if (!this._process?.stdin) throw new SdkError(SdkErrorCode.NotConnected, "Not connected");
			const json = serializeMessage(message);
			if (this._process.stdin.write(json)) resolve();
			else this._process.stdin.once("drain", resolve);
		});
	}
};

//#endregion
export { DEFAULT_INHERITED_ENV_VARS, StdioClientTransport, getDefaultEnvironment };
//# sourceMappingURL=stdio.mjs.map