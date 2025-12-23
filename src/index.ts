import NativeReactNativeNitroTor, {
	TorConfig,
	HiddenServiceParams,
	StartTorParams as NativeStartTorParams,
	StartTorResponse as NativeStartTorResponse,
	HiddenServiceResponse,
	HttpGetParams,
	HttpPostParams,
	HttpPutParams,
	HttpDeleteParams,
	HttpResponse,
} from "./NativeReactNativeNitroTor";

export type KeySpec = {
	onion?: string;
	seed_hex?: string;
	pub_hex?: string;
	generate?: boolean;
};

export type StartTorParams = Omit<NativeStartTorParams, "keys_json"> & {
	/**
	 * List of key configurations to host the same service.
	 * generate:false => use provided seed/pub (static bootstrap key)
	 * generate:true  => let Tor generate a fresh key.
	 */
	keys?: KeySpec[];
};

export type StartTorResponse = NativeStartTorResponse & {
	/** Parsed list of onion addresses, if multiple were created. */
	onion_addresses?: string[];
};

interface RnTorSpec {
	initTorService(config: TorConfig): Promise<boolean>;
	createHiddenService(
		params: HiddenServiceParams
	): Promise<HiddenServiceResponse>;
	startTorIfNotRunning(params: StartTorParams): Promise<StartTorResponse>;
	getServiceStatus(): Promise<number>;
	deleteHiddenService(onionAddress: string): Promise<boolean>;
	shutdownService(): Promise<boolean>;
	httpGet(params: HttpGetParams): Promise<HttpResponse>;
	httpPost(params: HttpPostParams): Promise<HttpResponse>;
	httpPut(params: HttpPutParams): Promise<HttpResponse>;
	httpDelete(params: HttpDeleteParams): Promise<HttpResponse>;
}

const RnTorImpl: RnTorSpec = {
	...NativeReactNativeNitroTor,

	async startTorIfNotRunning(params: StartTorParams): Promise<StartTorResponse> {
		const { keys, ...rest } = params as any;

		const nativeParams: NativeStartTorParams = {
			...(rest as NativeStartTorParams),
			// Pass keys to native side as JSON string; empty string means no keys.
			keys_json: keys && keys.length > 0 ? JSON.stringify(keys) : "",
		};

		const nativeResp: NativeStartTorResponse = await (NativeReactNativeNitroTor as any).startTorIfNotRunning(
			nativeParams,
		);

		let onion_addresses: string[] | undefined;
		if (nativeResp.onion_addresses_json) {
			try {
				const parsed = JSON.parse(nativeResp.onion_addresses_json);
				if (Array.isArray(parsed)) {
					onion_addresses = parsed.filter((x) => typeof x === "string");
				}
			} catch {
				// ignore parse errors and leave onion_addresses undefined
			}
		}

		return {
			...nativeResp,
			onion_addresses,
		};
	},
};

export const RnTor = RnTorImpl;
export default RnTorImpl;

