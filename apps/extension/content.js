// Find password inputs on the page and offer to fill from 2Password.
(function () {
  const fields = document.querySelectorAll('input[type="password"]');
  if (!fields.length) return;

  function userFieldFor(field) {
    return field.form?.querySelector(
      'input[type="text"],input[type="email"],input[name*="user"],input[name*="login"]'
    );
  }

  fields.forEach((field) => {
    const badge = document.createElement("span");
    badge.textContent = "🔑";
    badge.title = "2Password で入力";
    badge.style.cssText = "cursor:pointer;margin-left:6px;user-select:none;";
    badge.addEventListener("click", () => {
      chrome.runtime.sendMessage(
        { type: "findLogins", url: location.hostname },
        (res) => {
          const items = res?.items ?? [];
          if (items.length === 1) {
            fill(items[0].uuid);
          } else {
            // Show a minimal chooser (multiple/no matches → open popup flow).
            const choice = window.prompt(
              "2Password 一致候補:\n" +
                items.map((i, n) => `${n + 1}: ${i.title} (${i.username})`).join("\n") +
                "\n番号を入力:",
              "1"
            );
            const idx = parseInt(choice ?? "", 10) - 1;
            if (items[idx]) fill(items[idx].uuid);
          }
        }
      );
    });
    field.insertAdjacentElement("afterend", badge);
  });

  function fill(uuid) {
    chrome.runtime.sendMessage(
      { type: "getPassword", uuid, url: location.hostname },
      (res) => {
        if (!res || res.error) return;
        const field = document.querySelector('input[type="password"]');
        const user = field && userFieldFor(field);
        if (user && res.username) user.value = res.username;
        if (field && res.password) field.value = res.password;
      }
    );
  }
})();
