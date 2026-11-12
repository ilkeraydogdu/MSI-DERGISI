(() => {
  try {
    const saved = localStorage.getItem("msi_theme");
    const preference = document.documentElement.dataset.themePreference;
    const theme =
      saved ||
      (preference === "system"
        ? matchMedia("(prefers-color-scheme: dark)").matches
          ? "dark"
          : "light"
        : preference) ||
      "light";
    document.documentElement.dataset.theme =
      theme === "dark" ? "dark" : "light";
  } catch (_) {
    /* The server palette also works without storage. */
  }
})();
