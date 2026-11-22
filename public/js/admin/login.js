(function () {
  "use strict";

  const byId = (id) => document.getElementById(id);
  const T = window.MSIAdminTheme;
  function t(k, fbTr, fbEn) {
    if (window.MSIAdmin && typeof window.MSIAdmin.t === "function") {
      return window.MSIAdmin.t(k, fbTr, fbEn);
    }
    const lang = (document.body?.dataset?.adminLang || document.documentElement?.lang || "tr").toLowerCase();
    return lang.startsWith("en") && fbEn !== undefined ? fbEn : fbTr;
  }

  function extractMessage(data, fallback) {
    if (!data) return fallback;
    const err = (v) => (typeof v === "string" ? v : v && typeof v.message === "string" ? v.message : "");
    return err(data.message) || err(data.error) || (data.data && (err(data.data.message) || err(data.data.error))) || fallback;
  }

  function renderSvgIcon(name, idAttr, extraCls) {
    const svg = window.MSIAdmin?.svgIcon
      ? window.MSIAdmin.svgIcon(name, extraCls || "")
      : T?.svgIcon
        ? T.svgIcon(name, extraCls || "")
        : "";
    if (!svg) return "";
    return idAttr ? svg.replace("<svg ", `<svg id="${idAttr}" `) : svg;
  }

  function setButtonLoading(btn, isLoading, defaultIcon) {
    if (!btn) return;
    const icon = btn.querySelector(".adm-icon");
    btn.classList.toggle("loading", isLoading);
    btn.disabled = isLoading;
    if (icon) {
      const keepArrowCls = icon.classList.contains("cta-arrow") ? "cta-arrow" : "";
      const next = renderSvgIcon(isLoading ? "spinner" : defaultIcon, "", keepArrowCls);
      if (next) icon.outerHTML = next;
    }
  }

  const prefersReducedMotion = () => Boolean(window.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches);

  function shakeCard() {
    const card = byId("login-card");
    if (!card || !window.gsap || prefersReducedMotion()) return;
    window.gsap.killTweensOf(card, "x");
    window.gsap.fromTo(
      card,
      { x: 0 },
      {
        keyframes: [
          { x: -6, duration: 0.06 },
          { x: 6, duration: 0.06 },
          { x: -4, duration: 0.06 },
          { x: 4, duration: 0.06 },
          { x: 0, duration: 0.08 },
        ],
        ease: "power2.out",
        clearProps: "x",
      }
    );
  }

  function animateLoginEntry() {
    if (prefersReducedMotion() || !window.gsap) return;
    const card = byId("login-card");
    if (card) {
      window.gsap.killTweensOf(card);
      window.gsap.fromTo(
        card,
        { y: 14, opacity: 0.25 },
        {
          y: 0,
          opacity: 1,
          duration: 0.5,
          ease: "power3.out",
          clearProps: "transform,opacity",
        }
      );
    }

    const formItems = document.querySelectorAll(
      ".login-brand > *, #login-form .form-group, #login-form .login-submit"
    );
    if (formItems.length) {
      window.gsap.killTweensOf(formItems);
      window.gsap.fromTo(
        formItems,
        { y: 8, opacity: 0.25 },
        {
          y: 0,
          opacity: 1,
          duration: 0.36,
          stagger: 0.035,
          delay: 0.08,
          ease: "power3.out",
          clearProps: "transform,opacity",
        }
      );
    }
  }

  function initLogin() {
    const loginForm = byId("login-form");
    if (!loginForm) return;

    T?.initAutoTheme ? T.initAutoTheme() : T?.initTheme?.();
    T?.bindGlow?.();
    animateLoginEntry();

    const totpForm = byId("totp-login-form");
    const forgotForm = byId("forgot-password-form");
    const errorEl = byId("login-error");
    const successEl = byId("login-success");
    const submitBtn = byId("login-submit-btn");
    const totpBtn = byId("totp-submit-btn");
    const forgotBtn = byId("forgot-submit-btn");
    const rawAdminPath = ((document.body && document.body.dataset && document.body.dataset.adminPath) || "/podmin").trim();
    const adminRedirectUrl = "/" + rawAdminPath.replace(/^\/+/, "");
    let pendingTempToken = "";
    let activeView = "login";
    const fieldIds = ["login-email", "login-password", "totp-code-input", "forgot-email"];

    function clearInvalid() {
      fieldIds.forEach((id) => byId(id)?.removeAttribute("aria-invalid"));
    }

    fieldIds.forEach((id) => {
      byId(id)?.addEventListener("input", () => {
        byId(id)?.removeAttribute("aria-invalid");
      });
    });

    function clearMessages() {
      if (errorEl) { errorEl.textContent = ""; errorEl.classList.remove("visible"); }
      if (successEl) { successEl.textContent = ""; successEl.classList.remove("visible"); }
    }

    function showBanner(el, msg) {
      clearMessages();
      if (el) {
        el.textContent = msg;
        el.classList.add("visible");
        if (el === errorEl) shakeCard();
      }
    }

    function flagInvalid(inputEl) {
      if (!inputEl) return;
      inputEl.setAttribute("aria-invalid", "true");
      inputEl.focus();
    }

    function switchView(target) {
      activeView = target;
      clearMessages();
      clearInvalid();
      const forms = { login: loginForm, totp: totpForm, forgot: forgotForm };
      Object.entries(forms).forEach(([key, form]) => {
        if (form) form.classList.toggle("hidden", key !== target);
      });
      const activeForm = forms[target];
      if (activeForm && window.gsap && !prefersReducedMotion()) {
        window.gsap.killTweensOf(Object.values(forms).filter(Boolean));
        window.gsap.fromTo(
          activeForm,
          { opacity: 0.65, y: 10 },
          { opacity: 1, y: 0, duration: 0.28, ease: "power3.out", clearProps: "transform,opacity" }
        );
      }
      if (target === "totp") {
        const codeEl = byId("totp-code-input");
        if (codeEl) { codeEl.value = ""; codeEl.focus(); }
      } else if (target === "forgot") {
        const emailEl = byId("login-email");
        const fEmail = byId("forgot-email");
        if (fEmail && emailEl && emailEl.value.trim()) fEmail.value = emailEl.value.trim();
        fEmail?.focus();
      } else {
        const emailEl = byId("login-email");
        const pwEl = byId("login-password");
        if (emailEl && emailEl.value.trim() && pwEl) pwEl.focus();
        else emailEl?.focus();
      }
    }

    async function postJson(url, body, btn, defaultIcon) {
      setButtonLoading(btn, true, defaultIcon);
      try {
        const csrf = T ? T.getCookieValue("csrf_token") : "";
        const res = await fetch(url, {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            "X-CSRF-Token": csrf,
          },
          credentials: "same-origin",
          body: JSON.stringify(body),
        });
        let data = {};
        try { data = await res.json(); } catch (_) {}
        return { ok: res.ok, status: res.status, data };
      } catch (_) {
        showBanner(errorEl, t("admin.login.error_network", "Bağlantı hatası. Lütfen tekrar deneyin.", "Connection error. Please try again."));
        return null;
      } finally {
        setButtonLoading(btn, false, defaultIcon);
      }
    }

    const togglePwBtn = byId("btn-toggle-pw");
    togglePwBtn?.addEventListener("click", () => {
      const pwEl = byId("login-password");
      const ic = byId("pw-toggle-icon");
      if (!pwEl) return;
      const reveal = pwEl.type === "password";
      pwEl.type = reveal ? "text" : "password";
      togglePwBtn.setAttribute("aria-pressed", String(reveal));
      if (ic) {
        const next = renderSvgIcon(reveal ? "eye-slash" : "eye", "pw-toggle-icon");
        if (next) ic.outerHTML = next;
      }
    });

    byId("btn-show-forgot")?.addEventListener("click", () => switchView("forgot"));
    byId("btn-back-from-totp")?.addEventListener("click", () => { pendingTempToken = ""; switchView("login"); });
    byId("btn-back-from-forgot")?.addEventListener("click", () => switchView("login"));

    document.addEventListener("keydown", (ev) => {
      if (ev.key !== "Escape") return;
      if (activeView === "totp") {
        ev.preventDefault();
        pendingTempToken = "";
        switchView("login");
      } else if (activeView === "forgot") {
        ev.preventDefault();
        switchView("login");
      }
    });

    loginForm.addEventListener("submit", async (e) => {
      e.preventDefault();
      clearMessages();
      clearInvalid();
      const emailEl = byId("login-email");
      const pwEl = byId("login-password");
      const email = (emailEl?.value || "").trim();
      const password = pwEl?.value || "";
      if (!email || !password) {
        if (!email) flagInvalid(emailEl);
        else flagInvalid(pwEl);
        showBanner(errorEl, t("admin.login.error_credentials_required", "Lütfen e-posta ve parolanızı girin.", "Please enter your email and password."));
        return;
      }
      const res = await postJson("/api/v1/auth/login", { email, password }, submitBtn, "arrow-right");
      if (!res) return;
      const root = res.data || {};
      const inner = root.data || root;
      const needsTotp = Boolean(
        root.requires_totp || root.requires_2fa || root.status === "totp_required" ||
        inner.requires_totp || inner.requires_2fa || inner.status === "totp_required"
      );
      if (res.ok && needsTotp) {
        pendingTempToken = root.temp_token || inner.temp_token || "";
        switchView("totp");
      } else if (res.ok) {
        window.location.href = adminRedirectUrl;
      } else {
        showBanner(errorEl, extractMessage(root, t("admin.login.error_invalid", "Giriş başarısız. Bilgilerinizi kontrol edin.", "Sign in failed. Check your credentials.")));
      }
    });

    totpForm?.addEventListener("submit", async (e) => {
      e.preventDefault();
      clearMessages();
      clearInvalid();
      const codeEl = byId("totp-code-input");
      const code = (codeEl?.value || "").replace(/\s+/g, "");
      if (!pendingTempToken) {
        switchView("login");
        showBanner(errorEl, t("admin.login.error_session_expired", "Oturum doğrulama süresi doldu. Lütfen tekrar giriş yapın.", "Verification session expired. Please sign in again."));
        return;
      }
      if (!code) {
        flagInvalid(codeEl);
        showBanner(errorEl, t("admin.login.error_totp_required", "Geçerli 6 haneli doğrulama kodunu girin.", "Enter a valid 6-digit verification code."));
        return;
      }
      const res = await postJson("/api/v1/auth/totp/login", { temp_token: pendingTempToken, code }, totpBtn, "arrow-right");
      if (!res) return;
      if (res.ok) window.location.href = adminRedirectUrl;
      else showBanner(errorEl, extractMessage(res.data, t("admin.login.error_totp", "Doğrulama kodu geçersiz veya süresi dolmuş.", "Verification code is invalid or expired.")));
    });

    forgotForm?.addEventListener("submit", async (e) => {
      e.preventDefault();
      clearMessages();
      clearInvalid();
      const fEmailEl = byId("forgot-email");
      const email = (fEmailEl?.value || "").trim();
      if (!email) {
        flagInvalid(fEmailEl);
        showBanner(errorEl, t("admin.login.error_email_required", "Lütfen e-posta adresinizi girin.", "Please enter your email address."));
        return;
      }
      const res = await postJson("/api/v1/auth/forgot-password", { email }, forgotBtn, "arrow-right");
      if (!res) return;
      if (res.ok) showBanner(successEl, t("admin.login.forgot_success", "Eşleşen hesap varsa sıfırlama bağlantısı gönderildi.", "If an account matches, a reset link has been sent."));
      else showBanner(errorEl, extractMessage(res.data, t("admin.login.forgot_failure", "Sıfırlama talebi iletilemedi.", "Password reset request could not be sent.")));
    });
  }

  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", initLogin);
  else initLogin();
})();
