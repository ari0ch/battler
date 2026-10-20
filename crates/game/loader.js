// Trunk "initializer": hooks into loading the game's WebAssembly so the page
// can show a progress bar instead of a black screen while ~8 MB downloads.
// See https://trunkrs.dev (data-initializer on the rust link in index.html).
//
// `current` and `total` normally count the same bytes. If a server ever
// reports a compressed Content-Length while counting decompressed bytes, the
// ratio would overshoot; we remember the ratio seen when loading finished
// (localStorage, a per-browser convenience) and divide by it next time.

const RATIO_KEY = "battler.wasmRatio";

function savedRatio() {
  try {
    const r = parseFloat(localStorage.getItem(RATIO_KEY));
    return r >= 1 && r < 10 ? r : 1;
  } catch {
    return 1;
  }
}

export default function initializer() {
  const loader = document.getElementById("loader");
  const bar = document.getElementById("loader-bar");
  const label = document.getElementById("loader-label");
  let last = { current: 0, total: 0 };

  const show = (fraction, text) => {
    if (bar) bar.style.width = `${Math.round(Math.min(1, fraction) * 100)}%`;
    if (label) label.textContent = text;
  };

  return {
    onStart: () => show(0, "Loading..."),
    onProgress: ({ current, total }) => {
      last = { current, total };
      const mb = (current / 1e6).toFixed(1);
      const fraction = total ? current / (total * savedRatio()) : 0;
      show(Math.min(fraction, 0.97), `Loading... ${mb} MB`);
    },
    onComplete: () => {
      if (last.total) {
        try {
          localStorage.setItem(RATIO_KEY, String(last.current / last.total));
        } catch {}
      }
      show(1, "Starting...");
    },
    onSuccess: () => {
      // The game is running; its first frame follows within a moment.
      if (loader) {
        loader.classList.add("done");
        setTimeout(() => loader.remove(), 600);
      }
    },
    onFailure: (error) => {
      show(0, "The game failed to start. Try reloading, or a browser with WebGL2.");
      console.error(error);
    },
  };
}
