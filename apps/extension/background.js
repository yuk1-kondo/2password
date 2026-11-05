// Native-messaging bridge to the desktop app (spec §8.3 — secrets never
// live in the extension process; every request goes to the host).
const HOST = "dev.twopassword.native";

function sendNative(message) {
  return new Promise((resolve, reject) => {
    chrome.runtime.sendNativeMessage(HOST, message, (res) => {
      if (chrome.runtime.lastError) reject(chrome.runtime.lastError);
      else if (res && res.error) reject(new Error(res.error));
      else resolve(res);
    });
  });
}

chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
  if (msg.type === "findLogins") {
    sendNative({ type: "findLogins", url: msg.url })
      .then((res) => sendResponse(res))
      .catch((e) => sendResponse({ error: String(e) }));
    return true; // async
  }
  if (msg.type === "getPassword") {
    sendNative({ type: "getPassword", uuid: msg.uuid, url: msg.url })
      .then((res) => sendResponse(res))
      .catch((e) => sendResponse({ error: String(e) }));
    return true;
  }
});
