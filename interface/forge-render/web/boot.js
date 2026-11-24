// The page's start (was index.html's inline module; the CSP allows no inline script).
import init, { start, bench_offscreen, start_dwg, open_dwg, dwg_offscreen } from "./forge_render.js";
const q = new URLSearchParams(location.search);
const number = (k) => Number(q.get(k) ?? 0);
const adapter = await navigator.gpu?.requestAdapter({ powerPreference: "high-performance" });
const vendor = adapter?.info?.vendor ?? "none";
console.log(`forge-render adapter ${vendor} ${adapter?.info?.architecture ?? ""}`);
const show = (rgba, w, h) => {
  const canvas = document.body.appendChild(document.createElement("canvas"));
  canvas.width = w;
  canvas.height = h;
  canvas.getContext("2d").putImageData(new ImageData(new Uint8ClampedArray(rgba), w, h), 0, 0);
  console.log("forge-render done");
};
if (q.has("need") && vendor !== q.get("need")) {
  console.log(`forge-render refused: needs ${q.get("need")}`);
} else if (q.has("dwg")) {
  await init();
  // the app's Save and Open: a download, the file picker
  window.forge_save = (name, bytes) => {
    const a = document.createElement("a");
    a.href = URL.createObjectURL(new Blob([bytes], { type: "application/acad" }));
    a.download = name;
    a.click();
    setTimeout(() => URL.revokeObjectURL(a.href), 1000);
  };
  const input = document.getElementById("file");
  const pick = document.getElementById("pick");
  window.forge_pick = () => input.click();
  let started = false;
  const take = (name, bytes) => {
    pick.style.display = "none";
    if (started) {
      open_dwg(name, bytes);
    } else {
      started = true;
      start_dwg(name, bytes);
    }
  };
  const read = async (f) => take(f.name, new Uint8Array(await f.arrayBuffer()));
  input.onchange = () => input.files[0] && read(input.files[0]);
  pick.querySelector("button").onclick = () => input.click();
  addEventListener("dragover", (e) => e.preventDefault());
  addEventListener("drop", (e) => {
    e.preventDefault();
    const f = e.dataTransfer.files[0];
    if (f) read(f);
  });
  const url = q.get("dwg");
  if (url) {
    const bytes = new Uint8Array(await (await fetch(url)).arrayBuffer());
    const name = url.split("/").pop();
    if (q.has("offscreen")) {
      const [w, h] = q.get("offscreen").split("x").map(Number);
      const clicks = new Float32Array((q.get("clicks") ?? "").split(",").filter((s) => s).map(Number));
      show(await dwg_offscreen(name, bytes, w, h, number("space"), clicks, q.get("typed") ?? ""), w, h);
    } else {
      take(name, bytes);
    }
  } else {
    pick.style.display = "flex";
  }
} else if (!["part", "stress", "bench", "moving"].some((k) => q.has(k))) {
  await import("./app.js");
} else {
  await init();
  const stress = number("stress");
  const names = stress > 0
    ? (await (await fetch("parts/list.txt")).text()).trim().split("\n")
    : [q.get("part") ?? "crank"];
  const files = await Promise.all(names.map(async (n) =>
    new Uint8Array(await (await fetch(`parts/${n}.mesh`)).arrayBuffer())));
  if (q.has("offscreen")) {
    const [w, h] = q.get("offscreen").split("x").map(Number);
    show(await bench_offscreen(files, stress, number("moving"), number("bench"), w, h), w, h);
  } else {
    start(files, stress, number("moving"), number("bench"));
  }
}
