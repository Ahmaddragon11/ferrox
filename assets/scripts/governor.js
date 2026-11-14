// SPDX-License-Identifier: MIT OR Apache-2.0
// Ferrox Compute Governor — injected at document_start to reduce & optimize website CPU/GPU work.
(function () {
  "use strict";
  if (window.__ferroxGovernorActive) return;
  window.__ferroxGovernorActive = true;

  const cfg = window.__FERROX_POLICY__ || {
    mode: "balanced",
    throttleHiddenTimers: true,
    minHiddenIntervalMs: 1000,
    maxBackgroundFps: 1,
    deferOffscreenMedia: true,
    reduceMotion: false,
    blockAutoplay: true,
    disableWebgl: false,
    coalesceMutations: true
  };

  const stats = (window.__ferroxStats = {
    timersThrottled: 0,
    rafThrottled: 0,
    mediaDeferred: 0,
    mutationsCoalesced: 0
  });

  if (cfg.mode === "full") {
    return;
  }

  // 1. Timer Coalescing for hidden/background tabs or Eco mode
  const origSetTimeout = window.setTimeout.bind(window);
  const origSetInterval = window.setInterval.bind(window);

  window.setTimeout = function (fn, delay, ...args) {
    let d = Number(delay) || 0;
    if (document.hidden && cfg.throttleHiddenTimers && d < cfg.minHiddenIntervalMs) {
      d = cfg.minHiddenIntervalMs;
      stats.timersThrottled++;
    } else if (cfg.mode === "eco" && d > 0 && d < 50) {
      d = 50;
      stats.timersThrottled++;
    }
    return origSetTimeout(fn, d, ...args);
  };

  window.setInterval = function (fn, delay, ...args) {
    let d = Number(delay) || 0;
    if (document.hidden && cfg.throttleHiddenTimers && d < cfg.minHiddenIntervalMs) {
      d = cfg.minHiddenIntervalMs;
      stats.timersThrottled++;
    } else if (cfg.mode === "eco" && d < 100) {
      d = 100;
      stats.timersThrottled++;
    }
    return origSetInterval(fn, d, ...args);
  };

  // 2. requestAnimationFrame throttling when hidden or in Eco mode
  const origRaf = window.requestAnimationFrame.bind(window);
  let lastRafTime = 0;
  window.requestAnimationFrame = function (cb) {
    if (document.hidden && cfg.throttleHiddenTimers) {
      stats.rafThrottled++;
      return origSetTimeout(() => cb(performance.now()), 1000 / Math.max(1, cfg.maxBackgroundFps));
    }
    if (cfg.mode === "eco") {
      const now = performance.now();
      const minDelta = 33.3; // Cap at ~30fps in Eco mode to save GPU/battery
      if (now - lastRafTime < minDelta) {
        stats.rafThrottled++;
        return origSetTimeout(() => {
          lastRafTime = performance.now();
          cb(lastRafTime);
        }, minDelta - (now - lastRafTime));
      }
      lastRafTime = now;
    }
    return origRaf(cb);
  };

  // 3. Optional WebGL blocker in strict Eco mode
  if (cfg.disableWebgl) {
    const origGetContext = HTMLCanvasElement.prototype.getContext;
    HTMLCanvasElement.prototype.getContext = function (type, ...rest) {
      if (type === "webgl" || type === "webgl2" || type === "experimental-webgl") {
        return null;
      }
      return origGetContext.call(this, type, ...rest);
    };
  }

  // 4. Reduced motion & autoplay prevention via early stylesheet + DOM observer
  const applyEarlyStyles = () => {
    if (!document.head && !document.documentElement) return;
    const style = document.createElement("style");
    style.id = "__ferrox_governor_style";
    let css = "";
    if (cfg.reduceMotion || cfg.mode === "eco") {
      css += "*, *::before, *::after { animation-duration: 0.001ms !important; transition-duration: 0.001ms !important; scroll-behavior: auto !important; }\n";
    }
    (document.head || document.documentElement).appendChild(style);
    style.textContent = css;
  };

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", applyEarlyStyles, { once: true });
  } else {
    applyEarlyStyles();
  }

  // 5. Viewport-first deferral for heavy offscreen iframes, images, and autoplay media
  if (cfg.deferOffscreenMedia && typeof IntersectionObserver !== "undefined") {
    const io = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (!entry.isIntersecting) continue;
          const el = entry.target;
          if (el.dataset && el.dataset.ferroxSrc) {
            el.src = el.dataset.ferroxSrc;
            delete el.dataset.ferroxSrc;
          }
          io.unobserve(el);
        }
      },
      { rootMargin: "250px" }
    );

    const processNode = (node) => {
      if (!node || node.nodeType !== 1) return;
      const tag = node.tagName;
      if (tag === "IMG" && !node.loading) {
        node.loading = "lazy";
      } else if (tag === "VIDEO" && cfg.blockAutoplay) {
        if (node.autoplay) {
          node.autoplay = false;
          node.preload = "none";
          stats.mediaDeferred++;
        }
      } else if (tag === "IFRAME" && node.src && !node.loading) {
        node.loading = "lazy";
        stats.mediaDeferred++;
      }
    };

    const mo = new MutationObserver((mutations) => {
      if (cfg.coalesceMutations && mutations.length > 50) {
        stats.mutationsCoalesced += mutations.length;
      }
      for (const m of mutations) {
        for (const n of m.addedNodes) {
          processNode(n);
        }
      }
    });

    const startObserver = () => {
      if (document.documentElement) {
        mo.observe(document.documentElement, { childList: true, subtree: true });
      }
    };
    if (document.documentElement) startObserver();
    else document.addEventListener("DOMContentLoaded", startObserver, { once: true });
  }
})();
