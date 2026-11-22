(function () {
  "use strict";

  const STAGE_DURATION_MS = 7200;
  const TRANSITION_SEC = 1.15;
  const SWIPE_THRESHOLD_PX = 48;

  const VERT_SRC = [
    "attribute vec2 a_pos;",
    "varying vec2 v_uv;",
    "void main() {",
    "  v_uv = a_pos * 0.5 + 0.5;",
    "  gl_Position = vec4(a_pos, 0.0, 1.0);",
    "}",
  ].join("\n");

  const FRAG_SRC = [
    "#ifdef GL_FRAGMENT_PRECISION_HIGH",
    "precision highp float;",
    "#else",
    "precision mediump float;",
    "#endif",
    "varying vec2 v_uv;",
    "uniform sampler2D u_tex0;",
    "uniform sampler2D u_tex1;",
    "uniform vec2 u_res;",
    "uniform vec2 u_img0;",
    "uniform vec2 u_img1;",
    "uniform float u_progress;",
    "uniform float u_dir;",
    "uniform float u_zoom0;",
    "uniform float u_zoom1;",
    "uniform vec2 u_pan;",
    "uniform vec2 u_focus0;",
    "uniform vec2 u_focus1;",
    "uniform vec2 u_mouse;",
    "uniform float u_hover;",
    "",
    "vec2 coverUV(vec2 uv, vec2 res, vec2 img, vec2 focus, float zoom, vec2 pan) {",
    "  float canvasAsp = res.x / max(res.y, 1.0);",
    "  float imgAsp = img.x / max(img.y, 1.0);",
    "  vec2 scale = canvasAsp > imgAsp",
    "    ? vec2(1.0, imgAsp / canvasAsp)",
    "    : vec2(canvasAsp / imgAsp, 1.0);",
    "  vec2 halfExt = 0.5 * (scale / max(zoom, 1.0));",
    "  vec2 safeFocus = clamp(focus + pan, halfExt + vec2(0.002), vec2(0.998) - halfExt);",
    "  vec2 mapped = (uv - 0.5) * (scale / max(zoom, 1.0)) + safeFocus;",
    "  return clamp(mapped, vec2(0.002), vec2(0.998));",
    "}",
    "",
    "void main() {",
    "  float aspect = u_res.x / max(u_res.y, 1.0);",
    "  vec2 mVec = (v_uv - u_mouse) * vec2(aspect, 1.0);",
    "  float mDist = length(mVec);",
    "  float lens = smoothstep(0.24, 0.0, mDist) * u_hover * 0.016;",
    "  vec2 uv = v_uv - mVec * lens;",
    "",
    "  float p = clamp(u_progress, 0.0, 1.0);",
    "  float env = sin(p * 3.14159265);",
    "  float wave = (sin(uv.y * 8.4 + p * 3.5) * 0.042 + cos(uv.y * 14.2 - 0.8) * 0.014) * env;",
    "  float axis = mix(1.0 - uv.x, uv.x, step(0.0, u_dir));",
    "  float edge = p * 1.24 - 0.12 + wave;",
    "  float wipe = 1.0 - smoothstep(edge - 0.095, edge + 0.095, axis);",
    "",
    "  float crest = wipe * (1.0 - wipe) * 4.0;",
    "  vec2 displace = vec2(crest * 0.018 * u_dir, sin(uv.y * 12.0) * crest * 0.006);",
    "  vec2 caShift = vec2(0.0038 * crest * u_dir, 0.0);",
    "",
    "  vec2 uv0 = coverUV(uv + displace, u_res, u_img0, u_focus0, u_zoom0, u_pan);",
    "  vec2 uv1 = coverUV(uv - displace * 0.65, u_res, u_img1, u_focus1, u_zoom1, u_pan);",
    "",
    "  vec3 col0 = vec3(",
    "    texture2D(u_tex0, clamp(uv0 + caShift, 0.002, 0.998)).r,",
    "    texture2D(u_tex0, uv0).g,",
    "    texture2D(u_tex0, clamp(uv0 - caShift, 0.002, 0.998)).b",
    "  );",
    "  vec3 col1 = vec3(",
    "    texture2D(u_tex1, clamp(uv1 + caShift, 0.002, 0.998)).r,",
    "    texture2D(u_tex1, uv1).g,",
    "    texture2D(u_tex1, clamp(uv1 - caShift, 0.002, 0.998)).b",
    "  );",
    "",
    "  vec3 mixed = mix(col0, col1, wipe);",
    "  mixed += vec3(0.05, 0.07, 0.11) * crest * 0.45;",
    "  float vig = smoothstep(1.38, 0.34, length((v_uv - 0.5) * vec2(1.12, 1.05)));",
    "  mixed *= mix(0.76, 1.0, vig);",
    "  gl_FragColor = vec4(mixed, 1.0);",
    "}",
  ].join("\n");

  function byId(id) {
    return document.getElementById(id);
  }

  function prefersReducedMotion() {
    return Boolean(window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches);
  }

  function isFinePointer() {
    return Boolean(window.matchMedia && window.matchMedia("(hover: hover) and (pointer: fine)").matches);
  }

  function compileShader(gl, type, source) {
    const sh = gl.createShader(type);
    if (!sh) return null;
    gl.shaderSource(sh, source);
    gl.compileShader(sh);
    if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) {
      gl.deleteShader(sh);
      return null;
    }
    return sh;
  }

  function createProgram(gl) {
    const vs = compileShader(gl, gl.VERTEX_SHADER, VERT_SRC);
    const fs = compileShader(gl, gl.FRAGMENT_SHADER, FRAG_SRC);
    if (!vs || !fs) return null;
    const prog = gl.createProgram();
    if (!prog) return null;
    gl.attachShader(prog, vs);
    gl.attachShader(prog, fs);
    gl.linkProgram(prog);
    gl.deleteShader(vs);
    gl.deleteShader(fs);
    if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
      gl.deleteProgram(prog);
      return null;
    }
    return prog;
  }

  function init() {
    const stage = byId("login-stage");
    const dataEl = byId("stage-slides-data");
    if (!stage || !dataEl || stage.dataset.stageInit === "1") return;
    stage.dataset.stageInit = "1";

    let slides = [];
    try {
      slides = JSON.parse(dataEl.getAttribute("data-slides") || dataEl.textContent || "[]");
    } catch (_) {
      slides = [];
    }
    if (!slides.length) return;

    const canvas = byId("stage-canvas");
    const fallbackImg = byId("stage-fallback-img");
    const veil = byId("stage-veil");
    const capIndex = byId("cap-index");
    const capDomain = byId("cap-domain");
    const capTitle = byId("cap-title");
    const railItems = Array.from(stage.querySelectorAll(".rail-item"));
    const prevBtn = byId("stage-prev");
    const nextBtn = byId("stage-next");
    const toggleBtn = byId("stage-toggle");
    const cursorEl = byId("stage-cursor");
    const dockEl = document.querySelector(".login-dock");
    const cardEl = byId("login-card");

    const reduced = prefersReducedMotion();
    const gsap = window.gsap;
    const finePtr = isFinePointer();

    let activeIndex = 0;
    let fromIndex = 0;
    let isTransitioning = false;
    let userPaused = false;
    let holdDockHover = false;
    let holdCardFocus = false;
    let docHidden = Boolean(document.hidden);
    let contextLost = false;
    let progressVal = 0;
    let dirVal = 1;
    let autoElapsed = 0;
    let lastTick = performance.now();
    let zoomStart0 = performance.now();
    let zoomStart1 = performance.now();
    let transitionTween = null;
    let captionTween = null;

    // Smoothly damped pointer state
    const ptr = {
      tx: 0.5,
      ty: 0.5,
      x: 0.5,
      y: 0.5,
      vx: 0,
      vy: 0,
      hoverTarget: 0,
      hover: 0,
    };

    // WebGL setup
    let gl = null;
    let prog = null;
    let locs = null;
    const textures = new Map();
    const loadedImgs = new Map();
    const pendingLoads = new Map();
    let width = 1;
    let height = 1;

    function pickImageSrc(slide) {
      const dpr = Math.min(window.devicePixelRatio || 1, 1.5);
      return window.innerWidth * dpr > 1400 ? slide.src : slide.srcSm;
    }

    function createPlaceholderTexture(glCtx) {
      const tex = glCtx.createTexture();
      glCtx.bindTexture(glCtx.TEXTURE_2D, tex);
      glCtx.texParameteri(glCtx.TEXTURE_2D, glCtx.TEXTURE_WRAP_S, glCtx.CLAMP_TO_EDGE);
      glCtx.texParameteri(glCtx.TEXTURE_2D, glCtx.TEXTURE_WRAP_T, glCtx.CLAMP_TO_EDGE);
      glCtx.texParameteri(glCtx.TEXTURE_2D, glCtx.TEXTURE_MIN_FILTER, glCtx.LINEAR);
      glCtx.texParameteri(glCtx.TEXTURE_2D, glCtx.TEXTURE_MAG_FILTER, glCtx.LINEAR);
      const px = new Uint8Array([10, 14, 22, 255]);
      glCtx.texImage2D(glCtx.TEXTURE_2D, 0, glCtx.RGBA, 1, 1, 0, glCtx.RGBA, glCtx.UNSIGNED_BYTE, px);
      return tex;
    }

    let fallbackTex = null;

    function uploadTexture(idx, img) {
      if (!gl || contextLost) return null;
      const existing = textures.get(idx);
      if (existing) return existing;
      const tex = gl.createTexture();
      if (!tex) return null;
      gl.bindTexture(gl.TEXTURE_2D, tex);
      gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, 1);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, img);
      textures.set(idx, tex);
      return tex;
    }

    function ensureSlideImage(idx) {
      const safeIdx = ((idx % slides.length) + slides.length) % slides.length;
      if (loadedImgs.has(safeIdx)) {
        return Promise.resolve(loadedImgs.get(safeIdx));
      }
      if (pendingLoads.has(safeIdx)) {
        return pendingLoads.get(safeIdx);
      }
      const s = slides[safeIdx];
      const p = new Promise((resolve) => {
        const img = new Image();
        img.decoding = "async";
        img.onload = function () {
          loadedImgs.set(safeIdx, img);
          uploadTexture(safeIdx, img);
          if (safeIdx === activeIndex && gl && !contextLost) {
            markCanvasReady();
          }
          resolve(img);
        };
        img.onerror = function () {
          resolve(null);
        };
        img.src = pickImageSrc(s);
      });
      pendingLoads.set(safeIdx, p);
      return p;
    }

    function prefetchNeighbors(idx) {
      const nextIdx = (idx + 1) % slides.length;
      const prevIdx = (idx - 1 + slides.length) % slides.length;
      ensureSlideImage(nextIdx);
      ensureSlideImage(prevIdx);
    }

    function initWebGL() {
      if (!canvas) return false;
      const opts = {
        alpha: false,
        antialias: true,
        depth: false,
        stencil: false,
        powerPreference: "high-performance",
        preserveDrawingBuffer: false,
      };
      gl = canvas.getContext("webgl2", opts) || canvas.getContext("webgl", opts);
      if (!gl) return false;
      prog = createProgram(gl);
      if (!prog) {
        gl = null;
        return false;
      }

      const buf = gl.createBuffer();
      gl.bindBuffer(gl.ARRAY_BUFFER, buf);
      gl.bufferData(
        gl.ARRAY_BUFFER,
        new Float32Array([-1, -1, 1, -1, -1, 1, -1, 1, 1, -1, 1, 1]),
        gl.STATIC_DRAW
      );
      const posAttr = gl.getAttribLocation(prog, "a_pos");
      gl.enableVertexAttribArray(posAttr);
      gl.vertexAttribPointer(posAttr, 2, gl.FLOAT, false, 0, 0);
      gl.useProgram(prog);

      locs = {
        tex0: gl.getUniformLocation(prog, "u_tex0"),
        tex1: gl.getUniformLocation(prog, "u_tex1"),
        res: gl.getUniformLocation(prog, "u_res"),
        img0: gl.getUniformLocation(prog, "u_img0"),
        img1: gl.getUniformLocation(prog, "u_img1"),
        progress: gl.getUniformLocation(prog, "u_progress"),
        dir: gl.getUniformLocation(prog, "u_dir"),
        zoom0: gl.getUniformLocation(prog, "u_zoom0"),
        zoom1: gl.getUniformLocation(prog, "u_zoom1"),
        pan: gl.getUniformLocation(prog, "u_pan"),
        focus0: gl.getUniformLocation(prog, "u_focus0"),
        focus1: gl.getUniformLocation(prog, "u_focus1"),
        mouse: gl.getUniformLocation(prog, "u_mouse"),
        hover: gl.getUniformLocation(prog, "u_hover"),
      };
      fallbackTex = createPlaceholderTexture(gl);
      gl.uniform1i(locs.tex0, 0);
      gl.uniform1i(locs.tex1, 1);

      canvas.addEventListener("webglcontextlost", function (e) {
        e.preventDefault();
        contextLost = true;
        canvas.classList.remove("is-ready");
        stage.classList.remove("is-ready");
        syncFallbackImg(activeIndex);
      });
      canvas.addEventListener("webglcontextrestored", function () {
        contextLost = false;
        textures.clear();
        prog = null;
        if (initWebGL()) {
          loadedImgs.forEach(function (img, idx) {
            uploadTexture(idx, img);
          });
          resizeCanvas();
          renderFrame(performance.now());
          markCanvasReady();
        }
      });

      return true;
    }

    function markCanvasReady() {
      if (!canvas || contextLost || !gl) return;
      if (!textures.has(activeIndex)) return;
      renderFrame(performance.now());
      canvas.classList.add("is-ready");
      stage.classList.add("is-ready");
    }

    function resizeCanvas() {
      if (!canvas) return;
      const rect = stage.getBoundingClientRect();
      const dpr = Math.min(window.devicePixelRatio || 1, 1.5);
      width = Math.max(1, Math.round(rect.width * dpr));
      height = Math.max(1, Math.round(rect.height * dpr));
      if (canvas.width !== width || canvas.height !== height) {
        canvas.width = width;
        canvas.height = height;
        if (gl && !contextLost) {
          gl.viewport(0, 0, width, height);
        }
      }
    }

    function renderFrame(now) {
      if (!gl || contextLost || !prog || !locs) return;
      const s0 = slides[fromIndex] || slides[0];
      const s1 = slides[activeIndex] || s0;

      const t0 = textures.get(fromIndex) || fallbackTex;
      const t1 = textures.get(activeIndex) || t0 || fallbackTex;

      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, t0);
      gl.activeTexture(gl.TEXTURE1);
      gl.bindTexture(gl.TEXTURE_2D, t1);

      const elapsed0 = reduced ? 0 : Math.min(1.0, (now - zoomStart0) / (STAGE_DURATION_MS + 1800));
      const elapsed1 = reduced ? 0 : Math.min(1.0, (now - zoomStart1) / (STAGE_DURATION_MS + 1800));
      const z0 = 1.0 + elapsed0 * 0.06;
      const z1 = 1.0 + elapsed1 * 0.06;

      gl.uniform2f(locs.res, width, height);
      gl.uniform2f(locs.img0, s0.w || 2400, s0.h || 1600);
      gl.uniform2f(locs.img1, s1.w || 2400, s1.h || 1600);
      gl.uniform1f(locs.progress, progressVal);
      gl.uniform1f(locs.dir, dirVal);
      gl.uniform1f(locs.zoom0, z0);
      gl.uniform1f(locs.zoom1, z1);
      gl.uniform2f(locs.pan, reduced ? 0 : (ptr.x - 0.5) * 0.024, reduced ? 0 : (0.5 - ptr.y) * 0.018);
      gl.uniform2f(locs.focus0, s0.fx ?? 0.5, 1.0 - (s0.fy ?? 0.5));
      gl.uniform2f(locs.focus1, s1.fx ?? 0.5, 1.0 - (s1.fy ?? 0.5));
      gl.uniform2f(locs.mouse, ptr.x, 1.0 - ptr.y);
      gl.uniform1f(locs.hover, reduced ? 0.0 : ptr.hover);

      gl.drawArrays(gl.TRIANGLES, 0, 6);
    }

    function syncFallbackImg(idx) {
      if (!fallbackImg) return;
      const s = slides[idx];
      if (!s) return;
      fallbackImg.src = s.src;
      fallbackImg.srcset = s.srcSm + " 1280w, " + s.src + " 2400w";
      fallbackImg.alt = s.alt || s.title;
    }

    function setCaptionTitle(text) {
      if (!capTitle) return;
      if (captionTween) {
        captionTween.kill();
        captionTween = null;
      }
      capTitle.setAttribute("aria-label", text);
      capTitle.textContent = "";

      if (reduced || !gsap) {
        capTitle.textContent = text;
        return;
      }

      const chars = [];
      const words = text.split(" ");
      words.forEach(function (word, wi) {
        const wordWrap = document.createElement("span");
        wordWrap.className = "cap-word";
        wordWrap.setAttribute("aria-hidden", "true");
        for (let ci = 0; ci < word.length; ci += 1) {
          const ch = document.createElement("span");
          ch.className = "cap-char";
          ch.textContent = word[ci];
          wordWrap.appendChild(ch);
          chars.push(ch);
        }
        capTitle.appendChild(wordWrap);
        if (wi < words.length - 1) {
          capTitle.appendChild(document.createTextNode(" "));
        }
      });

      captionTween = gsap.fromTo(
        chars,
        { yPercent: 95, opacity: 0 },
        {
          yPercent: 0,
          opacity: 1,
          duration: 0.52,
          stagger: 0.016,
          ease: "power3.out",
          clearProps: "transform",
        }
      );
    }

    function updateRailUI(activeIdx, progressRatio) {
      railItems.forEach(function (btn, i) {
        const active = i === activeIdx;
        btn.classList.toggle("is-active", active);
        btn.setAttribute("aria-selected", active ? "true" : "false");
        const fill = btn.querySelector(".rail-fill");
        if (fill) {
          if (i < activeIdx) {
            fill.style.transform = "scaleX(1)";
          } else if (active) {
            fill.style.transform = "scaleX(" + Math.max(0, Math.min(1, progressRatio)).toFixed(4) + ")";
          } else {
            fill.style.transform = "scaleX(0)";
          }
        }
      });
    }

    function updateCaptionMeta(idx) {
      const s = slides[idx];
      if (!s) return;
      stage.setAttribute("data-active-index", String(idx));
      if (capIndex) {
        capIndex.textContent = String(idx + 1).padStart(2, "0");
      }
      if (capDomain) {
        capDomain.textContent = s.domain;
      }
      setCaptionTitle(s.title);
      syncFallbackImg(idx);
    }

    function isAutoPaused() {
      return userPaused || holdDockHover || holdCardFocus || docHidden || isTransitioning;
    }

    function goToSlide(nextIdx, explicitDir) {
      const target = ((nextIdx % slides.length) + slides.length) % slides.length;
      if (target === activeIndex) {
        autoElapsed = 0;
        return;
      }

      dirVal = explicitDir || (target > activeIndex ? 1 : -1);
      fromIndex = activeIndex;
      activeIndex = target;
      autoElapsed = 0;
      zoomStart1 = performance.now();

      ensureSlideImage(activeIndex).then(function () {
        if (gl && !contextLost) {
          markCanvasReady();
        }
      });
      prefetchNeighbors(activeIndex);
      updateCaptionMeta(activeIndex);
      updateRailUI(activeIndex, 0);

      if (transitionTween) {
        transitionTween.kill();
        transitionTween = null;
      }

      if (reduced || !gl || contextLost) {
        progressVal = 1;
        fromIndex = activeIndex;
        zoomStart0 = zoomStart1;
        isTransitioning = false;
        renderFrame(performance.now());
        return;
      }

      isTransitioning = true;
      const state = { t: 0 };
      progressVal = 0;

      if (gsap) {
        transitionTween = gsap.to(state, {
          t: 1,
          duration: TRANSITION_SEC,
          ease: "power3.inOut",
          onUpdate: function () {
            progressVal = state.t;
            renderFrame(performance.now());
          },
          onComplete: function () {
            progressVal = 1;
            fromIndex = activeIndex;
            zoomStart0 = zoomStart1;
            isTransitioning = false;
            transitionTween = null;
            renderFrame(performance.now());
          },
        });
      } else {
        const t0 = performance.now();
        function stepFallback(now) {
          const raw = Math.min(1, (now - t0) / (TRANSITION_SEC * 1000));
          progressVal = raw < 0.5 ? 4 * raw * raw * raw : 1 - Math.pow(-2 * raw + 2, 3) / 2;
          renderFrame(now);
          if (raw < 1) {
            requestAnimationFrame(stepFallback);
          } else {
            fromIndex = activeIndex;
            zoomStart0 = zoomStart1;
            isTransitioning = false;
          }
        }
        requestAnimationFrame(stepFallback);
      }
    }

    function setUserPaused(paused) {
      userPaused = Boolean(paused);
      stage.classList.toggle("is-paused", userPaused);
      if (toggleBtn) {
        toggleBtn.setAttribute("aria-pressed", userPaused ? "true" : "false");
      }
    }

    // Main loop for Ken Burns breathing, cursor damping, and autoplay progress
    function tick(now) {
      const dt = Math.min(100, Math.max(0, now - lastTick));
      lastTick = now;

      if (!docHidden) {
        if (!reduced) {
          ptr.x += (ptr.tx - ptr.x) * 0.11;
          ptr.y += (ptr.ty - ptr.y) * 0.11;
          ptr.hover += (ptr.hoverTarget - ptr.hover) * 0.12;
        }

        if (!isAutoPaused()) {
          autoElapsed += dt;
          if (autoElapsed >= STAGE_DURATION_MS) {
            goToSlide(activeIndex + 1, 1);
          } else {
            updateRailUI(activeIndex, autoElapsed / STAGE_DURATION_MS);
          }
        }

        if (gl && !contextLost && !isTransitioning && !reduced) {
          renderFrame(now);
        }
      }

      requestAnimationFrame(tick);
    }

    // Bind controls
    railItems.forEach(function (btn) {
      btn.addEventListener("click", function () {
        const idx = Number(btn.getAttribute("data-stage-index"));
        if (!Number.isNaN(idx)) {
          goToSlide(idx);
        }
      });
    });

    if (prevBtn) {
      prevBtn.addEventListener("click", function () {
        goToSlide(activeIndex - 1, -1);
      });
    }
    if (nextBtn) {
      nextBtn.addEventListener("click", function () {
        goToSlide(activeIndex + 1, 1);
      });
    }
    if (toggleBtn) {
      toggleBtn.addEventListener("click", function () {
        setUserPaused(!userPaused);
      });
    }

    // Pause autoplay while interacting with login dock/card
    if (dockEl) {
      dockEl.addEventListener("mouseenter", function () {
        holdDockHover = true;
      });
      dockEl.addEventListener("mouseleave", function () {
        holdDockHover = false;
      });
    }
    if (cardEl) {
      cardEl.addEventListener("focusin", function () {
        holdCardFocus = true;
      });
      cardEl.addEventListener("focusout", function (e) {
        if (!e.relatedTarget || !cardEl.contains(e.relatedTarget)) {
          holdCardFocus = false;
        }
      });
    }

    // Keyboard navigation when not typing
    document.addEventListener("keydown", function (e) {
      const tag = document.activeElement && document.activeElement.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;
      if (e.key === "ArrowRight") {
        goToSlide(activeIndex + 1, 1);
      } else if (e.key === "ArrowLeft") {
        goToSlide(activeIndex - 1, -1);
      }
    });

    // Pointer parallax, custom optical cursor & drag/swipe
    let dragStart = null;
    stage.addEventListener("pointerdown", function (e) {
      if (e.button !== 0 && e.pointerType === "mouse") return;
      if (e.target && e.target.closest(".stage-controls, .stage-top, .login-dock")) return;
      dragStart = { x: e.clientX, y: e.clientY };
    });

    stage.addEventListener("pointermove", function (e) {
      const rect = stage.getBoundingClientRect();
      if (!rect.width || !rect.height) return;
      const nx = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
      const ny = Math.max(0, Math.min(1, (e.clientY - rect.top) / rect.height));
      ptr.tx = nx;
      ptr.ty = ny;

      const overInteractive = Boolean(
        e.target && e.target.closest(".stage-controls, .stage-top, .login-dock")
      );
      ptr.hoverTarget = finePtr && !overInteractive ? 1 : 0;

      if (cursorEl && finePtr && !reduced) {
        cursorEl.style.transform = "translate3d(" + e.clientX + "px, " + e.clientY + "px, 0) translate(-50%, -50%)";
        cursorEl.classList.toggle("is-visible", !overInteractive);
      }
    });

    function finishDrag(e) {
      if (!dragStart) return;
      const dx = e.clientX - dragStart.x;
      const dy = e.clientY - dragStart.y;
      dragStart = null;
      if (Math.abs(dx) > SWIPE_THRESHOLD_PX && Math.abs(dx) > Math.abs(dy) * 1.2) {
        goToSlide(dx < 0 ? activeIndex + 1 : activeIndex - 1, dx < 0 ? 1 : -1);
      }
    }

    stage.addEventListener("pointerup", finishDrag);
    stage.addEventListener("pointercancel", function () {
      dragStart = null;
    });
    stage.addEventListener("pointerleave", function () {
      ptr.tx = 0.5;
      ptr.ty = 0.5;
      ptr.hoverTarget = 0;
      if (cursorEl) cursorEl.classList.remove("is-visible");
    });

    document.addEventListener("visibilitychange", function () {
      docHidden = Boolean(document.hidden);
      if (!docHidden) {
        lastTick = performance.now();
      }
    });

    if (typeof ResizeObserver !== "undefined") {
      const ro = new ResizeObserver(function () {
        resizeCanvas();
        if (gl && !contextLost) renderFrame(performance.now());
      });
      ro.observe(stage);
    } else {
      window.addEventListener("resize", function () {
        resizeCanvas();
        if (gl && !contextLost) renderFrame(performance.now());
      });
    }

    // Boot WebGL + intro sequence
    const hasGL = initWebGL();
    resizeCanvas();
    updateRailUI(0, 0);
    setCaptionTitle(slides[0].title);

    ensureSlideImage(0).then(function () {
      if (hasGL) markCanvasReady();
      prefetchNeighbors(0);
    });

    // Intro GSAP veil & caption entrance (skipped on reduced-motion)
    if (!reduced && gsap) {
      if (veil) {
        gsap.fromTo(
          veil,
          { opacity: 0.92 },
          {
            opacity: 0,
            duration: 0.85,
            ease: "power2.out",
            onComplete: function () {
              veil.classList.add("is-done");
            },
          }
        );
      }
      if (capDomain || capIndex) {
        gsap.from(".stage-caption .cap-meta", {
          y: 12,
          opacity: 0,
          duration: 0.55,
          delay: 0.12,
          ease: "power3.out",
          clearProps: "transform",
        });
      }
    } else if (veil) {
      veil.classList.add("is-done");
    }

    requestAnimationFrame(tick);
  }

  window.MSILoginStage = { init: init };
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
