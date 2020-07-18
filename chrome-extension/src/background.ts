// // /// <reference path="../node_modules/@types/chrome/index.d.ts"/>

// const IS_DEV = true; // change with IS_DEV in env.ts

// type API = {
//     transform: (
//         requestId: RequestId,
//         quality: Quality,
//         backgroundColor: Color,
//         pageRange?: PageRange) =>
//         AsyncIterable<Image | ImagesMetadata>,
//     fallback: (requestId?: RequestId) => void
// };

// type RequestId = string;

// interface PageRange {
//     starting_index: number,
//     count: number
// }

// enum Quality {
//     Extreme = "Extreme",
//     High = "High",
//     Normal = "Normal",
//     Low = "Low",
//     ExtraLow = "ExtraLow",
// }

// interface Color {
//     r: number,
//     g: number,
//     b: number,
// };

// interface Image {
//     isMetadata: false,
//     url: string,
//     offset: number
// }

// interface ImagesMetadata {
//     isMetadata: true,
//     originalTitle: string,
//     pageCount: number,
// }

// type ErrorLike = Error | string;

// class TransformError extends Error {
//     name: string;
//     message: string;
//     type: TransformErrorTypeDetail;
//     source?: ErrorLike;

//     constructor(type: TransformErrorType, source?: ErrorLike) {
//         super();

//         const details: TransformErrorTypeDetail = TRANSFORM_ERROR_TYPE_DETAILS[type];

//         this.name = "TransformError";
//         this.message = details.title;
//         this.type = details;
//         this.source = source;
//     }
// }

// enum TransformErrorType {
//     RequestNotFound = "RequestNotFound",
//     ServerError = "ServerError",
//     PdfDownloadError = "PdfDownloadError",
//     LocalPdfDownloadError = "LocalPdfDownloadError"
// }

// interface TransformErrorTypeDetail {
//     title: string,
//     children: string[]
// }

// const TRANSFORM_ERROR_TYPE_DETAILS = {
//     [TransformErrorType.RequestNotFound]: {
//         title: "Original request not found",
//         children: ["Try re-opening the PDF."]
//     },
//     [TransformErrorType.ServerError]: {
//         title: "Server error",
//         children: ["A server error occurred while transforming the PDF."]
//     },
//     [TransformErrorType.PdfDownloadError]: {
//         title: "Network error",
//         children: [
//             "Network error downloading the PDF to transform.",
//             "You may not be connected to the internet, or the website may be down."
//         ]
//     },
//     [TransformErrorType.LocalPdfDownloadError]: {
//         title: "Error opening PDF",
//         children: [
//             "An error occurred while attempting to open the PDF on your computer.",
//             "You most likely haven't allowed this extension to access files on your computer.",
//             "Open the extensions page, open the details for this extension, and make sure 'Allow access to file URLs' is toggled on.",
//             "This error can also occur if the file was just deleted."
//         ]
//     }
// };

// (window as any).api = (function (): API {
//     const REDIRECT_RESOURCE_URL = chrome.runtime.getURL('index.html');

//     if (!IS_DEV) {
//         console.log = (...args: any[]) => { };
//     }

//     let ENDPOINT: string;
//     if (IS_DEV) {
//         ENDPOINT = 'https://localhost:8000/purpleifypdf/transform';
//     } else {
//         ENDPOINT = 'https://danielzfranklin.org/purpleifypdf/transform';
//     }

//     enum ProgressTitle {
//         DOWN_ORIGINAL = "Downloading",
//         UP = "Sending to be transformed",
//         DOWN_TRANS = "Downloading transformed",
//     }

//     let __uidCache: string | undefined = undefined;
//     function uid(): Promise<string> {
//         return new Promise(resolve => {
//             if (__uidCache !== undefined) {
//                 resolve(__uidCache);
//             } else {
//                 chrome.storage.sync.get('uid', (resp: { uid?: string }) => {
//                     if (chrome.runtime.lastError) {
//                         console.warn('Chrome runtime error retreiving uid, so regenerating: ' + chrome.runtime.lastError);
//                     }
//                     if ('uid' in resp) {
//                         __uidCache = resp.uid;
//                     } else {
//                         let uid = Array.from(window.crypto.getRandomValues(new Uint8Array(16))).map(n => n.toString(36)).join('');
//                         chrome.storage.sync.set({ uid });
//                         __uidCache = uid;
//                     }
//                     resolve(__uidCache)
//                 });
//             }
//         })
//     }

//     class RotatingCache<K, V> {
//         // UB if null is used as a key

//         private keys: Array<any>;
//         private index: number;
//         private map: Map<any, any>;
//         private persistPrefix?: string;

//         constructor(private maxSize: number, persistName?: string) {
//             this.keys = Array(maxSize).fill(null);
//             this.index = 0;
//             this.map = new Map();

//             if (persistName) {
//                 const prefix = persistName + '-';
//                 chrome.storage.local.get(null, items => {
//                     if (chrome.runtime.lastError) {
//                         throw new Error('Failed to restore persisted rotating cache items: ' +
//                             chrome.runtime.lastError.message);
//                     }

//                     for (let k in items) {
//                         if (k.startsWith(prefix)) {
//                             let { key, value } = items[k];
//                             this.set(key, value);
//                         }
//                     }
//                 });
//                 this.persistPrefix = prefix;
//             }
//         }

//         get(key: K): V | undefined {
//             let value = this.map.get(key);
//             return value;
//         }

//         has(key: K): boolean {
//             return this.map.has(key);
//         }

//         set(key: K, value: V): void {
//             let i = this.index % this.maxSize;

//             this.map.delete(this.keys[i]);
//             this.keys[i] = key;
//             this.map.set(key, value);

//             if (this.persistPrefix) {
//                 chrome.storage.local.set({ [this.persistPrefix + key]: { key, value } });
//             }

//             this.index++;
//         }
//     }

//     interface RequestData {
//         requestHeaders: chrome.webRequest.HttpHeader[],
//         method: string,
//         url: string
//     }

//     const temporarilyDisabledCache = new RotatingCache<number, boolean>(100, 'temporarily_disabled_cache');
//     const requestDataCache = new RotatingCache<RequestId, RequestData>(100, 'request_data_cache');

//     interface TransformMetadata {
//         clientUid: string,
//         pageRange?: PageRange,
//         quality: Quality,
//         backgroundColor: Color,
//         source: string,
//     }

//     /// Intended to be called by frontend
//     async function* transform(
//         requestId: RequestId,
//         quality: Quality,
//         backgroundColor: Color,
//         pageRange?: PageRange):
//         AsyncIterable<Image | ImagesMetadata> {

//         const requestData = requestDataCache.get(requestId);
//         if (requestData === undefined) {
//             throw new TransformError(TransformErrorType.RequestNotFound);
//         }

//         const meta: TransformMetadata = {
//             clientUid: await uid(),
//             source: requestData.url,
//             pageRange: pageRange,
//             quality,
//             backgroundColor
//         }

//         let original: Blob | Response | null = null;
//         if (isFileUrl(requestData.url)) {
//             try {
//                 original = await getFileUrl(requestData);
//             } catch (err) {
//                 throw new TransformError(TransformErrorType.LocalPdfDownloadError, err);
//             }
//         } else {
//             try {
//                 original = await fetch(requestData.url, {
//                     method: requestData.method,
//                     headers: requestData.requestHeaders.map(h => [h.name, h.value]),
//                 });
//             } catch (err) {
//                 throw new TransformError(TransformErrorType.PdfDownloadError, err);
//             }
//         }

//         try {
//             const transformed = await upload(meta, original);
//             yield* getImages(transformed);
//         } catch (err) {
//             throw new TransformError(TransformErrorType.ServerError, err);
//         }
//     }

//     /// Intended to be called by frontend
//     async function fallback(requestId?: RequestId): Promise<URL | undefined> {
//         return new Promise((resolve, reject) => {
//             chrome.tabs.getCurrent(tab => {
//                 if (chrome.runtime.lastError) {
//                     reject(Error('Chrome runtime error: ' + chrome.runtime.lastError.message));
//                 }
//                 if (typeof tab?.id !== 'number') {
//                     reject(Error('The current tab has no id'));
//                 }

//                 temporarilyDisabledCache.set(tab.id, true);

//                 if (requestId) {
//                     let requestData = requestDataCache.get(requestId);
//                     if (requestData) {
//                         try {
//                             let url = new URL(requestData.url);
//                             resolve(url);
//                         } catch (err) {
//                             console.warn('fallback: Not providing fallback to redirect to because requestData.url is not a valid URL');
//                             resolve(undefined);
//                         }
//                     } else {
//                         console.warn('fallback: Not providing fallback to redirect to because requestData not found');
//                     }
//                 } else {
//                     console.warn('fallback: Not providing fallback to redirect to because no redirectId specified in function call');
//                 }

//                 resolve(undefined);
//             });
//         });
//     }

//     const HEADER_PREFIX_STRING: string = "PPDF";
//     const HEADER_PREFIX: Uint8Array = Uint8Array.from(HEADER_PREFIX_STRING, s => s.charCodeAt(0));

//     const HEADER_OFFSET_BYTES: number = 4;

//     const HEADER_POSTFIX_BYTES: number = 3;
//     const HEADER_IMG_POSTFIX_STRING: string = "IMG";
//     const HEADER_IMG_POSTFIX: Uint8Array = Uint8Array.from(HEADER_IMG_POSTFIX_STRING, s => s.charCodeAt(0));
//     const HEADER_META_POSTFIX_STRING: string = "MET";
//     const HEADER_META_POSTFIX: Uint8Array = Uint8Array.from(HEADER_META_POSTFIX_STRING, s => s.charCodeAt(0));

//     const HEADER_SIZE: number = HEADER_PREFIX.length + HEADER_OFFSET_BYTES + HEADER_POSTFIX_BYTES;

//     const INITIAL_IMAGE_BUFFER_SIZE: number = 500000; // 0.5 MB in bytes

//     async function* getImages(stream: ReadableStream<Uint8Array>): AsyncIterable<Image | ImagesMetadata> {
//         const reader = stream.getReader();

//         let buf = new Uint8Array(INITIAL_IMAGE_BUFFER_SIZE);
//         let usedBytes = 0;

//         let headerStartI = 0;
//         let bodyEndI: number | null = null;
//         let bodyIsMeta = false;
//         let imageIndex: number = 0;

//         while (true) {
//             const { done: isDone, value } = await reader.read();

//             if (isDone) {
//                 return;
//             }

//             const unusedCapacity = buf.length - usedBytes;
//             if (unusedCapacity < value.length) {
//                 // re-allocate
//                 let necessary = value.length - unusedCapacity;
//                 let extra = value.length
//                 let newBuf = new Uint8Array(buf.length + necessary + extra);
//                 newBuf.set(buf);
//                 buf = newBuf;
//             }
//             buf.set(value, usedBytes);
//             usedBytes += value.length;

//             if (bodyEndI !== null && bodyEndI <= usedBytes) {
//                 let bodyStartI = headerStartI + HEADER_SIZE;
//                 let body = buf.slice(bodyStartI, bodyEndI);

//                 if (bodyIsMeta) {
//                     let chars = [];
//                     for (let n of body) {
//                         chars.push(String.fromCharCode(n));
//                     }
//                     let json = JSON.parse(chars.join(''))
//                     yield {
//                         isMetadata: true,
//                         ...json
//                     } as ImagesMetadata;

//                     bodyIsMeta = false;
//                 } else {
//                     yield {
//                         isMetadata: false,
//                         url: URL.createObjectURL(new Blob([body], { type: 'image/png' })),
//                         offset: imageIndex
//                     };
//                 }

//                 // TODO: re-start at start of buffer?
//                 headerStartI = bodyEndI;
//                 bodyEndI = null;

//                 imageIndex++;
//             }

//             if (bodyEndI == null && headerStartI + HEADER_SIZE <= usedBytes) {
//                 let start = headerStartI;
//                 let stop = headerStartI + HEADER_PREFIX.length;
//                 let prefix = buf.slice(start, stop);

//                 start = stop;
//                 stop = start + HEADER_OFFSET_BYTES;
//                 let offsetBytes = new DataView(buf.buffer, start, HEADER_OFFSET_BYTES);
//                 let offset = offsetBytes.getInt32(0, false);

//                 start = stop;
//                 stop = start + HEADER_POSTFIX_BYTES;
//                 let postfix = buf.slice(start, stop);

//                 let isValidPrefix = areArraysEqual(prefix, HEADER_PREFIX);
//                 let isMetaPostfix = areArraysEqual(postfix, HEADER_META_POSTFIX);
//                 let isImgPostfix = areArraysEqual(postfix, HEADER_IMG_POSTFIX);
//                 let isValidPostfix = isMetaPostfix || isImgPostfix;

//                 if (!isValidPrefix || !isValidPostfix) {
//                     let start = headerStartI;
//                     let stop = start + HEADER_SIZE;
//                     console.warn(buf);
//                     throw new Error("Invalid header: " + buf.slice(start, stop));
//                 }

//                 bodyIsMeta = isMetaPostfix;
//                 bodyEndI = headerStartI + offset;
//             }
//         }
//     }

//     function areArraysEqual(a: ArrayLike<any>, b: ArrayLike<any>): boolean {
//         let length = a.length;
//         if (length !== b.length) {
//             return false;
//         }

//         for (let i = 0; i < length; i++) {
//             if (a[i] !== b[i]) {
//                 return false;
//             }
//         }

//         return true;
//     }

//     function isFileUrl(url: string): boolean {
//         try {
//             let parsed = new URL(url);
//             return parsed.protocol === 'file:';
//         } catch (e) {
//             console.warn("isFileUrl returning false: A invalid URL is not a file URL");
//             return false;
//         }
//     }

//     function getFileUrl(requestData: RequestData): Promise<Blob> {
//         return new Promise((resolve, reject) => {
//             const req = new XMLHttpRequest();
//             req.open(requestData.method, requestData.url);
//             req.responseType = 'blob';

//             req.onload = () => {
//                 const responseData = req.response;
//                 const blob = responseData.slice(0, responseData.size, 'application/pdf');
//                 resolve(blob);
//             }
//             req.onerror = () => reject();

//             req.send(null);
//         });
//     }

//     async function upload(meta: TransformMetadata, data: Blob | Response): Promise<ReadableStream<Uint8Array>> {
//         if (!(data instanceof Blob)) {
//             // TODO: possibly use chrome.sockets.tcp api to stream upload instead of waiting
//             data = await data.blob();
//         }

//         const meta_param = encodeURIComponent(JSON.stringify(meta));

//         let req = await fetch(`${ENDPOINT}?meta=${meta_param}`, {
//             method: 'POST',
//             body: data
//         });

//         return req.body;
//     }

//     const TOP_LEVEL_RESOURCE_TYPES: Array<chrome.webRequest.ResourceType> = ['main_frame', 'sub_frame'];
//     const REQUEST_FILTER: chrome.webRequest.RequestFilter = {
//         'urls': ['*://*/*'],
//         'types': TOP_LEVEL_RESOURCE_TYPES
//     }

//     chrome.webRequest.onBeforeRequest.addListener((req: chrome.webRequest.WebRequestFullDetails) => {
//         console.log('Preparing to redirect from local pdf: ', req);

//         requestDataCache.set(req.requestId, {
//             requestHeaders: [],
//             method: 'GET',
//             url: req.url
//         });

//         return createRedirect(req);
//     }, {
//         ...REQUEST_FILTER,
//         urls: ['file://*.pdf']
//     }, ['blocking']);

//     chrome.webRequest.onBeforeSendHeaders.addListener((req: chrome.webRequest.WebRequestHeadersDetails) => {
//         let { requestId, method, url, requestHeaders } = req;
//         if (!requestDataCache.has(requestId)) {
//             console.log('Storing request data for potential PDF: ', req);
//             requestDataCache.set(requestId, {
//                 method,
//                 url,
//                 requestHeaders
//             });
//         }
//     }, REQUEST_FILTER, ['requestHeaders', 'extraHeaders']);

//     chrome.webRequest.onHeadersReceived.addListener((req: chrome.webRequest.WebResponseHeadersDetails) => {
//         if (req.method !== 'OPTIONS' && isPdf(req)) {
//             console.log('Preparing to redirect from remote pdf: ', req);
//             return createRedirect(req);
//         }
//     }, REQUEST_FILTER, ['blocking', 'responseHeaders']);

//     function createRedirect(req: chrome.webRequest.WebResponseHeadersDetails |
//         chrome.webRequest.WebRequestHeadersDetails): chrome.webRequest.BlockingResponse {

//         if (req.tabId !== -1 && temporarilyDisabledCache.has(req.tabId)) {
//             console.warn("Skipping redirect because temporarily disabled for tab");
//             return {};
//         }

//         return {
//             "redirectUrl": REDIRECT_RESOURCE_URL + '?request_id=' + req.requestId
//         }
//     }

//     function isPdf(req: HasHeaders & chrome.webRequest.ResourceRequest): Boolean {
//         let header = getHeader(req, 'content-type');
//         if ((header === undefined || header.includes('application/octet-stream')) && req.url.endsWith('.pdf')) {
//             return true;
//         } else if (header !== undefined && header.includes('application/pdf')) {
//             return true;
//         } else {
//             return false;
//         }
//     }

//     type HasHeaders = (
//         { requestHeaders?: Array<chrome.webRequest.HttpHeader>, responseHeaders?: never } |
//         { responseHeaders?: Array<chrome.webRequest.HttpHeader>, requestHeaders?: never }
//     );

//     function getHeader(req: HasHeaders, name: string): string | undefined {
//         let headers: Array<chrome.webRequest.HttpHeader>;
//         if (req.requestHeaders !== undefined) {
//             headers = req.requestHeaders;
//         } else if (req.responseHeaders !== undefined) {
//             headers = req.responseHeaders;
//         } else {
//             return undefined;
//         }
//         let normalized_name = name.toUpperCase();


//         let header = headers.find(i => i.name.toUpperCase() === normalized_name);
//         if (header) {
//             if (header.value) {
//                 return header.value;
//             } else {
//                 return undefined;
//             }
//         }
//     }

//     return {
//         transform,
//         fallback,
//     };
// })();
console.log(12);
export { };