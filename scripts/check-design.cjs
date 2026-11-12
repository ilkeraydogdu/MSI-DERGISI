/* Run against an isolated test database. Optional QA operator credentials unlock editor tests. */
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || "playwright");
const fs = require("node:fs");
const assert = require("node:assert/strict");
const base = process.env.MSI_TEST_URL || "http://127.0.0.1:8091";
const output = process.env.MSI_TEST_OUTPUT || "artifacts/design-review";
(async () => {
  fs.mkdirSync(output, { recursive: true });
  const browser = await chromium.launch({
    headless: true,
    ...(process.env.PLAYWRIGHT_EXECUTABLE_PATH
      ? { executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH }
      : {}),
    args: ["--no-sandbox"],
  });
  const context = await browser.newContext({
    viewport: { width: 1440, height: 1000 },
    locale: "tr-TR",
  });
  const page = await context.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  const report = { routes: [], interactions: [], errors };
  const check = (name, condition = true) => {
    assert(condition, name);
    report.interactions.push(name);
  };
  try {
    const contents = await (
      await context.request.get(base + "/api/v1/contents")
    ).json();
    const items = Array.isArray(contents)
      ? contents
      : contents.contents || contents.items || contents.data;
    assert(items?.length, "Content API should return articles");
    const article = items[0];
    const routes = [
      "/",
      "/?mode=news#haber",
      "/arama",
      "/arama?q=KAAN",
      "/arama?q=bulunmayacakicerikxyz",
      "/kategori/haberler",
      "/kategori/soylesiler",
      "/icerik/" + article.slug,
      "/e-dergi",
      "/abonelik-ve-satis",
      "/hesabim",
      "/kurumsal/hakkimizda",
      "/kurumsal/kunye",
      "/kurumsal/yolculugumuz",
      "/kurumsal/cozum-ortaklarimiz",
      "/kurumsal/kvkk",
      "/kurumsal/iletisim",
      "/olmayan-sayfa",
    ];
    for (const width of process.env.MSI_SKIP_ROUTES ? [] : [1440, 390, 320]) {
      await page.setViewportSize({ width, height: 900 });
      for (const route of routes) {
        const response = await page.goto(base + route, {
          waitUntil: "networkidle",
        });
        await page.locator("img").evaluateAll(async (images) => {
          await Promise.all(
            images
              .filter((i) => i.getClientRects().length)
              .map((i) => {
                i.loading = "eager";
                return i.decode().catch(() => {});
              }),
          );
        });
        const metrics = await page.evaluate(() => ({
          overflow: document.documentElement.scrollWidth > innerWidth + 1,
          broken: [...document.images]
            .filter(
              (i) =>
                i.getClientRects().length && (!i.complete || !i.naturalWidth),
            )
            .map((i) => i.getAttribute("src")),
          h1: [...document.querySelectorAll("h1")].filter(
            (e) => e.getClientRects().length,
          ).length,
          main: document.querySelectorAll("main").length,
        }));
        report.routes.push({
          route,
          width,
          status: response.status(),
          ...metrics,
        });
        assert(!metrics.overflow, `Overflow ${route} ${width}`);
        assert.equal(metrics.h1, 1, `One visible h1 ${route}`);
        assert.equal(metrics.main, 1, `One main ${route}`);
        assert(
          !metrics.broken.length,
          `Broken images ${route}: ${metrics.broken}`,
        );
        if (
          width !== 320 &&
          [
            "/",
            "/?mode=news#haber",
            "/e-dergi",
            "/arama",
            "/icerik/" + article.slug,
          ].includes(route)
        ) {
          const name =
            route === "/"
              ? "magazine"
              : route.includes("mode=news")
                ? "news"
                : route.startsWith("/icerik")
                  ? "article"
                  : route.slice(1);
          await page.screenshot({
            path: `${output}/${name}-${width}.png`,
            fullPage: true,
          });
        }
      }
    }
    await page.setViewportSize({ width: 1440, height: 1000 });
    await page.goto(base + "/", { waitUntil: "networkidle" });
    await page.click("#btnSwitchHaber");
    check(
      "News portal switches without reload",
      await page.locator("#view-news").isVisible(),
    );
    await page.goBack();
    check(
      "Browser back restores magazine portal",
      await page.locator("#view-magazine").isVisible(),
    );
    await page.click("#btnToggleThemeMode");
    check(
      "Soft dark theme applied",
      (await page.locator("html").getAttribute("data-theme")) === "dark",
    );
    await page.reload();
    check(
      "Theme preference persists",
      (await page.locator("html").getAttribute("data-theme")) === "dark",
    );
    await page.screenshot({
      path: `${output}/magazine-dark.png`,
      fullPage: true,
    });
    await page.click('[data-open-dialog="notifications-dialog"]');
    check(
      "Notification dialog opens",
      await page.locator("#notifications-dialog").isVisible(),
    );
    await page.click("[data-read-notifications]");
    await page.keyboard.press("Escape");
    check(
      "Notification Escape closes and restores focus",
      await page
        .locator('[data-open-dialog="notifications-dialog"]')
        .evaluate((e) => e === document.activeElement),
    );
    await page.goto(base + "/e-dergi");
    const first = page.locator("[data-open-issue]").first();
    const issue = await first.getAttribute("data-open-issue");
    await first.click();
    check(
      "Reader uses selected issue",
      (await page.locator("#issue-dialog-meta").textContent()).includes(issue),
    );
    if (!(await page.locator("[data-reader-next]").isDisabled())) {
      const before = await page.locator("#reader-position").textContent();
      await page.click("[data-reader-next]");
      check(
        "Reader next changes actual spread",
        before !== (await page.locator("#reader-position").textContent()),
      );
    }
    await page.keyboard.press("Tab");
    check(
      "Dialog keeps keyboard focus",
      await page.evaluate(() =>
        document
          .querySelector("#issue-dialog")
          .contains(document.activeElement),
      ),
    );
    await page.keyboard.press("Escape");
    await page.goto(base + "/abonelik-ve-satis");
    await page.locator("[data-select-plan]").first().click();
    await page.locator("#offer-dialog").waitFor({ state: "visible" });
    check(
      "Subscription selection has real details",
      await page.locator("#offer-dialog").isVisible(),
    );
    check(
      "Contact action has selected plan",
      (await page.locator("#offer-contact").getAttribute("href")).startsWith(
        "mailto:",
      ),
    );
    await page.keyboard.press("Escape");
    await page.goto(base + "/hesabim?email=murat.yilmaz@msb.gov.tr");
    check(
      "Query string cannot impersonate a reader",
      await page.locator(".account-welcome").isVisible(),
    );
    await page.click('[href="#modal-login"]');
    await page.locator("#login-form [name=email]").fill("invalid@example.test");
    await page
      .locator("#login-form [name=password]")
      .fill("invalid-passphrase");
    await page.locator("#login-form button").click();
    await page.waitForSelector("#login-form .error");
    check(
      "Login error is inline and retryable",
      !(await page.locator("#login-form button").isDisabled()),
    );
    await page.keyboard.press("Escape");
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto(base + "/");
    await page.click(".mobile-toggle");
    check(
      "Mobile menu opens",
      await page.locator("#site-navigation").isVisible(),
    );
    await page.keyboard.press("Escape");
    check(
      "Mobile menu closes",
      !(await page.locator("#site-navigation").isVisible()),
    );
    check(
      "Unauthenticated design mutations blocked",
      (
        await context.request.post(base + "/api/v1/admin/design", { data: {} })
      ).status() === 401,
    );
    if (process.env.MSI_QA_USER && process.env.MSI_QA_PASSWORD) {
      const login = await context.request.post(base + "/api/v1/auth/login", {
        data: {
          email: process.env.MSI_QA_USER,
          password: process.env.MSI_QA_PASSWORD,
        },
      });
      assert.equal(login.status(), 200, "QA operator login");
      const design = await (
        await context.request.get(base + "/api/v1/admin/design")
      ).json();
      const original = design.config;
      const changed = structuredClone(original);
      changed.pages.push({
        slug: "qa-dynamic-page",
        title: "QA dinamik sayfa",
        description: "Test sayfası",
        mode: "magazine",
        sections: [
          {
            kind: "text",
            title: "QA bölüm",
            description: "Merkezi motor testi.",
            enabled: true,
            layout: "grid",
            source: "all",
            limit: 6,
            offset: 0,
            eyebrow: "",
            link: "",
            link_label: "",
            category: "",
          },
        ],
      });
      try {
        const invalid = structuredClone(changed);
        invalid.theme.light.text = invalid.theme.light.surface;
        check(
          "Invalid contrast rejected",
          (
            await context.request.post(base + "/api/v1/admin/design", {
              data: invalid,
            })
          ).status() === 422,
        );
        check(
          "Valid design saved",
          (
            await context.request.post(base + "/api/v1/admin/design", {
              data: changed,
            })
          ).status() === 200,
        );
        await page.goto(base + "/sayfa/qa-dynamic-page");
        check(
          "New page renders from saved configuration",
          (await page.locator("h1").textContent()) === "QA dinamik sayfa",
        );
        check(
          "Custom page is in sitemap",
          (
            await (await context.request.get(base + "/sitemap.xml")).text()
          ).includes("/sayfa/qa-dynamic-page"),
        );
        await page.setViewportSize({ width: 1440, height: 1000 });
        await page.goto(base + "/admin/tasarim", { waitUntil: "networkidle" });
        await page.waitForFunction(() =>
          document
            .querySelector("#design-preview")
            .contentDocument?.querySelector("h1"),
        );
        check(
          "Design editor has no load errors",
          !(await page.locator("#editor-feedback").textContent()),
        );
        await page.click('[data-editor-tab="theme"]');
        await page.locator("#theme-radius").fill("8");
        await page.waitForTimeout(1000);
        check(
          "Unsaved preview renders tokens",
          await page
            .locator("#design-preview")
            .evaluate((f) =>
              f.contentDocument
                .querySelector("#design-tokens")
                .textContent.includes("--radius:8px"),
            ),
        );
        await page.click('[data-preview-width="390px"]');
        await page.waitForFunction(() => Math.round(document.querySelector("#design-preview").getBoundingClientRect().width) === 390);
        check(
          "Mobile preview width changes",
          await page
            .locator("#design-preview")
            .evaluate(
              (e) => Math.round(e.getBoundingClientRect().width) === 390,
            ),
        );
        await page.click("#preview-theme");
        check(
          "Dark preview works",
          await page
            .locator("#design-preview")
            .evaluate(
              (e) => e.contentDocument.documentElement.dataset.theme === "dark",
            ),
        );
        await page.click('[data-preview-width="100%"]');
        await page.screenshot({ path: `${output}/editor.png`, fullPage: true });
        await page.click("#discard-draft");
        await page.goto(base + "/admin", { waitUntil: "networkidle" });
        for (let i = 1; i <= 8; i++) {
          await page.locator(`[data-tab="tab-${i}"]`).click();
          check(
            `Admin tab ${i} opens`,
            await page.locator(`#tab-${i}`).isVisible(),
          );
        }
        await page.screenshot({ path: `${output}/admin.png`, fullPage: true });
      } finally {
        const response = await context.request.post(
          base + "/api/v1/admin/design",
          { data: original },
        );
        assert.equal(response.status(), 200, "Restore QA config");
      }
      await context.request.post(base + "/api/v1/auth/logout");
      await page.goto(base + "/hesabim");
      check(
        "Logout clears account access",
        await page.locator(".account-welcome").isVisible(),
      );
    }
    assert.equal(errors.length, 0, "Browser runtime errors");
    report.status = "passed";
  } catch (error) {
    report.status = "failed";
    report.failure = error.message;
    console.error(error.stack);
    process.exitCode = 1;
  } finally {
    fs.writeFileSync(
      output + "/browser-report.json",
      JSON.stringify(report, null, 2),
    );
    console.log(
      JSON.stringify({
        status: report.status,
        routes: report.routes.length,
        interactions: report.interactions.length,
        errors,
        failure: report.failure,
      }),
    );
    await browser.close();
  }
})();
