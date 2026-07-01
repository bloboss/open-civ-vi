// WebGL2 pointy-top hex renderer for a WorldSnapshot.
//
// Draws terrain hexes via instanced rendering, dims fog, tints owned tiles,
// overlays city/unit markers, and supports drag-pan / wheel-zoom / click-pick.
// Consumes only the typed api.ts data, so it is a drop-in reference renderer.
//
// DRAFT (loop-authored, pointy-top + default palette). Visual look/feel is
// meant to be refined in a browser — palette, marker style, and camera tuning
// are the obvious knobs.

import type { CityRow, Unit, WorldSnapshot } from "../gen/protocol/protocol.js";
import { linkProgram, makeBuffer } from "./gl.js";
import { axialToPixel, hexTriangleMesh, pixelToAxial } from "./hexgeom.js";

type RGBA = [number, number, number, number];

const TERRAIN_PALETTE: Record<string, [number, number, number]> = {
  Grassland: [0.36, 0.55, 0.28],
  Plains: [0.66, 0.6, 0.32],
  Desert: [0.83, 0.74, 0.46],
  Tundra: [0.58, 0.6, 0.52],
  Snow: [0.88, 0.9, 0.93],
  Coast: [0.3, 0.55, 0.7],
  Ocean: [0.16, 0.34, 0.55],
  Mountain: [0.4, 0.38, 0.36],
};

/** Default terrain palette (refine visually). RGB in 0..1. */
function terrainColor(terrain: string): [number, number, number] {
  return TERRAIN_PALETTE[terrain] ?? [0.5, 0.5, 0.5];
}

const TERRAIN_VS = `#version 300 es
in vec2 a_pos;
in vec2 a_offset;
in vec4 a_color;
uniform vec2 u_cam;
uniform float u_zoom;
uniform vec2 u_res;
out vec4 v_color;
void main() {
  vec2 world = a_offset + a_pos;
  vec2 px = (world - u_cam) * u_zoom;
  vec2 clip = px / (u_res * 0.5);
  gl_Position = vec4(clip.x, -clip.y, 0.0, 1.0);
  v_color = a_color;
}`;

const TERRAIN_FS = `#version 300 es
precision mediump float;
in vec4 v_color;
out vec4 frag;
void main() { frag = v_color; }`;

const MARKER_VS = `#version 300 es
in vec2 a_offset;
in vec4 a_color;
uniform vec2 u_cam;
uniform float u_zoom;
uniform vec2 u_res;
uniform float u_size;
out vec4 v_color;
void main() {
  vec2 px = (a_offset - u_cam) * u_zoom;
  vec2 clip = px / (u_res * 0.5);
  gl_Position = vec4(clip.x, -clip.y, 0.0, 1.0);
  gl_PointSize = max(5.0, u_zoom * u_size);
  v_color = a_color;
}`;

const MARKER_FS = `#version 300 es
precision mediump float;
in vec4 v_color;
out vec4 frag;
void main() {
  vec2 c = gl_PointCoord - 0.5;
  if (dot(c, c) > 0.25) discard;
  frag = v_color;
}`;

interface Uniforms {
  cam: WebGLUniformLocation | null;
  zoom: WebGLUniformLocation | null;
  res: WebGLUniformLocation | null;
  size?: WebGLUniformLocation | null;
}

export class HexMap {
  private readonly gl: WebGL2RenderingContext;
  private readonly canvas: HTMLCanvasElement;

  private readonly terrainProg: WebGLProgram;
  private readonly markerProg: WebGLProgram;
  private readonly terrainU: Uniforms;
  private readonly markerU: Uniforms;

  private readonly meshBuf: WebGLBuffer;
  private readonly terrainVao: WebGLVertexArrayObject;
  private readonly offsetBuf: WebGLBuffer;
  private readonly colorBuf: WebGLBuffer;
  private terrainCount = 0;

  private readonly markerVao: WebGLVertexArrayObject;
  private readonly markerBuf: WebGLBuffer; // interleaved [x,y,r,g,b,a]
  private markerCount = 0;

  // Dedicated single-point buffer for the selection highlight (kept separate
  // so it never clobbers the marker buffer).
  private readonly selVao: WebGLVertexArrayObject;
  private readonly selBuf: WebGLBuffer;

  // Camera: world center + pixels-per-world-unit.
  private camX = 0;
  private camY = 0;
  private zoom = 22;

  private selection: { q: number; r: number } | null = null;
  /** Fired when the user clicks a tile. */
  onPick?: (q: number, r: number) => void;

  constructor(canvas: HTMLCanvasElement) {
    const gl = canvas.getContext("webgl2");
    if (!gl) throw new Error("WebGL2 not available");
    this.gl = gl;
    this.canvas = canvas;

    this.terrainProg = linkProgram(gl, TERRAIN_VS, TERRAIN_FS);
    this.markerProg = linkProgram(gl, MARKER_VS, MARKER_FS);
    this.terrainU = {
      cam: gl.getUniformLocation(this.terrainProg, "u_cam"),
      zoom: gl.getUniformLocation(this.terrainProg, "u_zoom"),
      res: gl.getUniformLocation(this.terrainProg, "u_res"),
    };
    this.markerU = {
      cam: gl.getUniformLocation(this.markerProg, "u_cam"),
      zoom: gl.getUniformLocation(this.markerProg, "u_zoom"),
      res: gl.getUniformLocation(this.markerProg, "u_res"),
      size: gl.getUniformLocation(this.markerProg, "u_size"),
    };

    this.meshBuf = makeBuffer(gl, hexTriangleMesh());
    this.offsetBuf = gl.createBuffer()!;
    this.colorBuf = gl.createBuffer()!;
    this.terrainVao = gl.createVertexArray()!;
    this.setupTerrainVao();

    this.markerBuf = gl.createBuffer()!;
    this.markerVao = gl.createVertexArray()!;
    this.setupPointVao(this.markerVao, this.markerBuf);

    this.selBuf = gl.createBuffer()!;
    this.selVao = gl.createVertexArray()!;
    this.setupPointVao(this.selVao, this.selBuf);

    this.attachInput();
    this.resize();
  }

  private setupTerrainVao(): void {
    const gl = this.gl;
    gl.bindVertexArray(this.terrainVao);
    // a_pos (location 0) from the static hex mesh, per-vertex.
    const aPos = gl.getAttribLocation(this.terrainProg, "a_pos");
    gl.bindBuffer(gl.ARRAY_BUFFER, this.meshBuf);
    gl.enableVertexAttribArray(aPos);
    gl.vertexAttribPointer(aPos, 2, gl.FLOAT, false, 0, 0);
    // a_offset, per-instance.
    const aOff = gl.getAttribLocation(this.terrainProg, "a_offset");
    gl.bindBuffer(gl.ARRAY_BUFFER, this.offsetBuf);
    gl.enableVertexAttribArray(aOff);
    gl.vertexAttribPointer(aOff, 2, gl.FLOAT, false, 0, 0);
    gl.vertexAttribDivisor(aOff, 1);
    // a_color, per-instance.
    const aCol = gl.getAttribLocation(this.terrainProg, "a_color");
    gl.bindBuffer(gl.ARRAY_BUFFER, this.colorBuf);
    gl.enableVertexAttribArray(aCol);
    gl.vertexAttribPointer(aCol, 4, gl.FLOAT, false, 0, 0);
    gl.vertexAttribDivisor(aCol, 1);
    gl.bindVertexArray(null);
  }

  /** Configure a VAO for the marker program: interleaved [x,y, r,g,b,a]. */
  private setupPointVao(vao: WebGLVertexArrayObject, buf: WebGLBuffer): void {
    const gl = this.gl;
    gl.bindVertexArray(vao);
    const stride = 6 * 4;
    const aOff = gl.getAttribLocation(this.markerProg, "a_offset");
    gl.bindBuffer(gl.ARRAY_BUFFER, buf);
    gl.enableVertexAttribArray(aOff);
    gl.vertexAttribPointer(aOff, 2, gl.FLOAT, false, stride, 0);
    const aCol = gl.getAttribLocation(this.markerProg, "a_color");
    gl.enableVertexAttribArray(aCol);
    gl.vertexAttribPointer(aCol, 4, gl.FLOAT, false, stride, 2 * 4);
    gl.bindVertexArray(null);
  }

  /** Rebuild terrain instance buffers from a snapshot; recenters the camera. */
  setSnapshot(snap: WorldSnapshot): void {
    const gl = this.gl;
    const n = snap.tiles.length;
    const offsets = new Float32Array(n * 2);
    const colors = new Float32Array(n * 4);
    let sumX = 0;
    let sumY = 0;
    for (let i = 0; i < n; i++) {
      const t = snap.tiles[i]!;
      const [x, y] = axialToPixel(t.q, t.r);
      offsets[i * 2] = x;
      offsets[i * 2 + 1] = y;
      sumX += x;
      sumY += y;
      let [r, g, b] = terrainColor(t.terrain);
      if (t.fog) {
        // Desaturate + darken fogged tiles.
        const m = (r + g + b) / 3;
        r = (r * 0.4 + m * 0.6) * 0.55;
        g = (g * 0.4 + m * 0.6) * 0.55;
        b = (b * 0.4 + m * 0.6) * 0.55;
      } else if (t.owner) {
        // Subtle warm tint on owned territory (refine per-civ later).
        r = r * 0.85 + 0.15 * 0.9;
        g = g * 0.85 + 0.15 * 0.75;
        b = b * 0.85 + 0.15 * 0.3;
      }
      colors[i * 4] = r;
      colors[i * 4 + 1] = g;
      colors[i * 4 + 2] = b;
      colors[i * 4 + 3] = 1;
    }
    gl.bindBuffer(gl.ARRAY_BUFFER, this.offsetBuf);
    gl.bufferData(gl.ARRAY_BUFFER, offsets, gl.DYNAMIC_DRAW);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.colorBuf);
    gl.bufferData(gl.ARRAY_BUFFER, colors, gl.DYNAMIC_DRAW);
    this.terrainCount = n;
    if (n > 0) {
      this.camX = sumX / n;
      this.camY = sumY / n;
    }
  }

  /** Rebuild the city/unit marker buffer. Cities = white, units = own-color. */
  setEntities(cities: CityRow[], units: Unit[]): void {
    const gl = this.gl;
    const data: number[] = [];
    for (const c of cities) {
      const [x, y] = axialToPixel(c.position.q, c.position.r);
      const col: RGBA = c.capital ? [1, 0.92, 0.5, 1] : [0.95, 0.95, 0.95, 1];
      data.push(x, y, ...col);
    }
    for (const u of units) {
      const [x, y] = axialToPixel(u.position.q, u.position.r);
      data.push(x, y, 0.85, 0.3, 0.3, 1);
    }
    gl.bindBuffer(gl.ARRAY_BUFFER, this.markerBuf);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(data), gl.DYNAMIC_DRAW);
    this.markerCount = cities.length + units.length;
  }

  setSelection(q: number, r: number): void {
    this.selection = { q, r };
  }

  resize(): void {
    const dpr = globalThis.devicePixelRatio || 1;
    const w = Math.floor(this.canvas.clientWidth * dpr);
    const h = Math.floor(this.canvas.clientHeight * dpr);
    if (this.canvas.width !== w || this.canvas.height !== h) {
      this.canvas.width = w;
      this.canvas.height = h;
    }
    this.gl.viewport(0, 0, this.canvas.width, this.canvas.height);
  }

  draw(): void {
    const gl = this.gl;
    const res: [number, number] = [this.canvas.width, this.canvas.height];
    gl.clearColor(0.05, 0.05, 0.07, 1);
    gl.clear(gl.COLOR_BUFFER_BIT);

    // Terrain.
    gl.useProgram(this.terrainProg);
    gl.uniform2f(this.terrainU.cam, this.camX, this.camY);
    gl.uniform1f(this.terrainU.zoom, this.zoom);
    gl.uniform2f(this.terrainU.res, res[0], res[1]);
    gl.bindVertexArray(this.terrainVao);
    if (this.terrainCount > 0) {
      gl.drawArraysInstanced(gl.TRIANGLES, 0, 18, this.terrainCount);
    }
    gl.bindVertexArray(null);

    // Markers (cities/units) + selection dot.
    gl.useProgram(this.markerProg);
    gl.uniform2f(this.markerU.cam, this.camX, this.camY);
    gl.uniform1f(this.markerU.zoom, this.zoom);
    gl.uniform2f(this.markerU.res, res[0], res[1]);
    gl.bindVertexArray(this.markerVao);
    if (this.markerCount > 0) {
      gl.uniform1f(this.markerU.size ?? null, 0.5);
      gl.drawArrays(gl.POINTS, 0, this.markerCount);
    }
    gl.bindVertexArray(null);

    if (this.selection) {
      this.drawSelection(res);
    }
  }

  private drawSelection(res: [number, number]): void {
    const gl = this.gl;
    const [x, y] = axialToPixel(this.selection!.q, this.selection!.r);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.selBuf);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([x, y, 1, 1, 0.2, 1]), gl.DYNAMIC_DRAW);
    gl.useProgram(this.markerProg);
    gl.uniform2f(this.markerU.cam, this.camX, this.camY);
    gl.uniform1f(this.markerU.zoom, this.zoom);
    gl.uniform2f(this.markerU.res, res[0], res[1]);
    gl.uniform1f(this.markerU.size ?? null, 0.9);
    gl.bindVertexArray(this.selVao);
    gl.drawArrays(gl.POINTS, 0, 1);
    gl.bindVertexArray(null);
  }

  // ── input ──────────────────────────────────────────────────────────────────
  private attachInput(): void {
    const c = this.canvas;
    let dragging = false;
    let lastX = 0;
    let lastY = 0;
    let moved = false;

    c.addEventListener("pointerdown", (e) => {
      dragging = true;
      moved = false;
      lastX = e.clientX;
      lastY = e.clientY;
      try {
        c.setPointerCapture(e.pointerId);
      } catch {
        /* synthetic / inactive pointer — capture is optional */
      }
    });
    c.addEventListener("pointermove", (e) => {
      if (!dragging) return;
      const dx = e.clientX - lastX;
      const dy = e.clientY - lastY;
      if (Math.abs(dx) + Math.abs(dy) > 2) moved = true;
      const dpr = globalThis.devicePixelRatio || 1;
      this.camX -= (dx * dpr) / this.zoom;
      this.camY -= (dy * dpr) / this.zoom;
      lastX = e.clientX;
      lastY = e.clientY;
      this.draw();
    });
    c.addEventListener("pointerup", (e) => {
      dragging = false;
      try {
        c.releasePointerCapture(e.pointerId);
      } catch {
        /* pointer was never captured */
      }
      if (!moved) this.pick(e.clientX, e.clientY);
    });
    c.addEventListener(
      "wheel",
      (e) => {
        e.preventDefault();
        const factor = Math.exp(-e.deltaY * 0.0015);
        this.zoom = Math.min(120, Math.max(4, this.zoom * factor));
        this.draw();
      },
      { passive: false },
    );
    globalThis.addEventListener("resize", () => {
      this.resize();
      this.draw();
    });
  }

  private pick(clientX: number, clientY: number): void {
    const rect = this.canvas.getBoundingClientRect();
    const dpr = globalThis.devicePixelRatio || 1;
    const px = (clientX - rect.left) * dpr - this.canvas.width / 2;
    const py = (clientY - rect.top) * dpr - this.canvas.height / 2;
    const worldX = px / this.zoom + this.camX;
    const worldY = py / this.zoom + this.camY;
    const [q, r] = pixelToAxial(worldX, worldY);
    this.setSelection(q, r);
    this.draw();
    this.onPick?.(q, r);
  }
}
