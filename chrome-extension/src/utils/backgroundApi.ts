import { IS_DEV, EXTENSION_ID } from './env';

export async function getApi(): Promise<any> {
    return new Promise((resolve, reject) => {
        chrome.runtime.getBackgroundPage((win: any) => {
            if (chrome.runtime.lastError) {
                reject(new Error(chrome.runtime.lastError?.message || "Undefined Chrome runtime error"));
            } else if (win?.api === undefined) {
                console.warn('rejecting', win);
                reject(new Error("window.api is unset"));
            }
            resolve(win.api);
        });
    });
}
