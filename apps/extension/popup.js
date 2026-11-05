chrome.tabs.query({ active: true, currentWindow: true }, ([tab]) => {
  const host = new URL(tab.url).hostname;
  chrome.runtime.sendMessage({ type: "findLogins", url: host }, (res) => {
    const status = document.getElementById("status");
    const list = document.getElementById("items");
    if (res?.error) { status.textContent = res.error; return; }
    if (!res?.items?.length) { status.textContent = "一致するログインはありません"; return; }
    status.textContent = `${res.items.length} 件見つかりました（フィールド横の🔑で入力）`;
    for (const it of res.items) {
      const li = document.createElement("li");
      li.textContent = `${it.title} — ${it.username}`;
      list.appendChild(li);
    }
  });
});
