/* ============================================================
 * hex-webgl-hud.js — WebGL2 renderer for the main HUD hex map.
 *
 * Mirrors the architecture of hex-webgl-demos/demo-overlays.js
 * (one big fragment shader, packed attrs, instanced rendering)
 * but bound to the sparse JSON tile model used by the wireframe
 * (WORLD + TILES Map from world-snapshot.json).
 *
 * Three passes:
 *   1. Terrain  — instanced hex fans. Fragment shader handles
 *                 terrain color, fog, OOB stripes, undiscovered
 *                 paper, yield dots, selection ring, hex outline.
 *   2. Edges    — instanced quads along hex edges. One instance
 *                 per (tile × edge × kind). Solid / dashed style
 *                 chosen by kind.
 *   3. Markers  — instanced discs over tiles with city / unit /
 *                 wonder / resource glyphs.
 *
 * Exposes window.HexHud.create(canvas, board) returning
 *   { render(cam), pick(sx, sy, cam), HEX_R }.
 *
 * The caller (the wireframe) supplies cam + reads WORLD/TILES
 * via the global accessors WORLD, tileAt(q, r).
 * ============================================================ */
(function () {
  "use strict";

  // ── geometry ───────────────────────────────────────────────
  const HEX_R = 46;
  const SQRT3 = Math.sqrt(3);

  // ── terrain palette — muted paper tones, distinct per biome ─
  const PAPER_PALETTE = [
    [0.71, 0.79, 0.88], // 0  Ocean
    [0.85, 0.89, 0.93], // 1  Coast
    [0.94, 0.91, 0.78], // 2  Plains
    [0.85, 0.91, 0.74], // 3  Grass
    [0.88, 0.81, 0.66], // 4  Hills
    [0.74, 0.83, 0.64], // 5  Forest
    [0.96, 0.91, 0.74], // 6  Desert
    [0.92, 0.93, 0.91], // 7  Tundra
    [0.78, 0.77, 0.74], // 8  Mountain
    [0.92, 0.88, 0.74], // 9  Floodpl
    [0.66, 0.79, 0.62], // 10 Jungle
    [0.80, 0.83, 0.70], // 11 Marsh
    [0.84, 0.88, 0.94], // 12 River
    [0.96, 0.96, 0.94], // 13 Snow
    [0.85, 0.91, 0.91], // 14 Reef
    [0.95, 0.92, 0.86], // 15 Bare / default
  ];

  // accepted terrain strings → palette index
  const TERRAIN_INDEX = {
    Ocean: 0, Coast: 1, Plains: 2, Grass: 3, Hills: 4, Forest: 5,
    Desert: 6, Tundra: 7, Mtn: 8, Mountain: 8,
    Floodpl: 9, Floodplain: 9, Jungle: 10, Marsh: 11,
    River: 12, Snow: 13, Reef: 14, Bare: 15,
  };

  function terrainToIdx(name) {
    if (name == null) return 15;
    return TERRAIN_INDEX[name] != null ? TERRAIN_INDEX[name] : 15;
  }

  // ── edge directions ─────────────────────────────────────────
  // Order matches the wireframe's EDGE_KEYS and EDGE_VERT_PAIRS.
  // Vertices: 0=top, 1=upper-right, 2=lower-right, 3=bottom,
  //           4=lower-left, 5=upper-left
  const EDGE_DIR_LIST = ["N", "NE", "SE", "S", "SW", "NW"];
  const EDGE_VERT_PAIRS = {
    NE: [0, 1], SE: [1, 2], S: [2, 3],
    SW: [3, 4], NW: [4, 5], N:  [5, 0],
  };
  const EDGE_KIND_INDEX = { river: 0, cliff: 1, coast: 2, wonder: 3, border: 4 };
  // ink-blue, ink, lighter blue, warm brown, ink-blue (border)
  const EDGE_COLORS = [
    [0.17, 0.29, 0.42],
    [0.10, 0.10, 0.10],
    [0.35, 0.49, 0.61],
    [0.54, 0.29, 0.17],
    [0.17, 0.29, 0.42],
  ];
  // bit 0 = solid, bit 1 = dashed
  // bit 0 = solid, bit 1 = dashed
  const EDGE_DASHED = [0, 1, 0, 1, 1];
  // half-width in unit-hex coordinates (R≈46px, so 0.06 → ~5.5 px stroke)
  const EDGE_WIDTH  = [0.07, 0.06, 0.05, 0.06, 0.045];

  // ── marker kinds ────────────────────────────────────────────
  const MARK_CITY = 0, MARK_UNIT = 1, MARK_WONDER = 2, MARK_RESOURCE = 3;

  // unit kind → glyph id (matches demo-overlays.js)
  function unitKindId(k) {
    switch (k) {
      case "Warrior":  return 0;
      case "Archer":   return 1;
      case "Horseman": return 2;
      case "Settler":  return 3;
      case "Scout":    return 4;
      case "Worker":   return 5;
      case "Spearman": return 6;
      case "Crossbow": return 7;
      default:         return 8;
    }
  }

  // ── shader sources ──────────────────────────────────────────
  const TERRAIN_VS = `#version 300 es
in vec2 aLocal;        // unit hex vertex (centered, R=1)
in vec4 aInstance;     // (q, r, packed, sel)
in vec3 aYields;       // (f, p, g)
uniform vec2  uCam;
uniform float uZoom;
uniform vec2  uView;
uniform float uHexR;
flat out int  vPacked;
flat out int  vSel;
flat out ivec3 vYields;
out vec2 vLocal;
const float SQRT3 = 1.7320508;
void main() {
  float R = uHexR * uZoom;
  float hw = SQRT3 * R, hh = 1.5 * R;
  float q = aInstance.x, r = aInstance.y;
  float p = float(int(r) & 1);
  vec2 center = vec2((q + p * 0.5) * hw, r * hh);
  float camP = float(int(uCam.y) & 1);
  vec2 camPx = vec2((uCam.x + camP * 0.5) * hw, uCam.y * hh);
  vec2 screen = center + uView * 0.5 - camPx + aLocal * R;
  gl_Position = vec4(screen.x/uView.x*2.0 - 1.0, 1.0 - screen.y/uView.y*2.0, 0.0, 1.0);
  vLocal = aLocal;
  vPacked = int(aInstance.z);
  vSel = int(aInstance.w);
  vYields = ivec3(int(aYields.x), int(aYields.y), int(aYields.z));
}`;

  const TERRAIN_FS = `#version 300 es
precision highp float;
flat in int   vPacked;
flat in int   vSel;
flat in ivec3 vYields;
in vec2 vLocal;
uniform vec3 uPalette[16];
uniform vec3 uPaper;
uniform vec3 uInk;
uniform vec3 uAccent;
uniform float uZoom;
out vec4 fragColor;

float disc(vec2 p, vec2 c, float r) {
  return 1.0 - smoothstep(r - 0.012, r + 0.012, length(p - c));
}

// Distance from local position to the hex outline.
// Pointy-top unit hex (R=1, top vertex at y=-1).
float hexEdgeDist(vec2 p) {
  vec2 ap = vec2(abs(p.x), abs(p.y));
  return 0.866025 - max(ap.x, ap.x * 0.5 + ap.y * 0.866025);
}

void main() {
  int  terrain   = vPacked & 0xF;
  bool oob       = ((vPacked >> 4)  & 1) == 1;
  bool unloaded  = ((vPacked >> 5)  & 1) == 1;
  bool fogTile   = ((vPacked >> 6)  & 1) == 1;
  bool owned     = ((vPacked >> 7)  & 1) == 1;
  bool cityHere  = ((vPacked >> 8)  & 1) == 1;
  int  flood     = (vPacked >> 9)  & 0x3;
  int  status    = (vPacked >> 11) & 0x3;  // 0 none / 1 center / 2 working / 3 idle
  bool fringe    = ((vPacked >> 13) & 1) == 1;

  vec3 c;
  if (oob) {
    // OOB: warm paper with diagonal stripes
    float s = step(0.5, fract((vLocal.x + vLocal.y) * 6.0));
    c = mix(vec3(0.89, 0.87, 0.83), vec3(0.83, 0.81, 0.76), s);
  } else if (unloaded) {
    // Undiscovered: flat paper, slightly darker
    c = vec3(0.92, 0.91, 0.86);
  } else {
    c = uPalette[terrain];
  }

  // owned tint — light wash in ink-blue when this tile belongs
  // to the player's empire (subtle, doesn't dominate the terrain)
  if (owned && !oob && !unloaded) {
    c = mix(c, uAccent, 0.06);
  }

  // city tile status (set only by the city screen):
  //   1 = city center — saturated wash
  //   2 = working     — warm parchment wash
  //   3 = idle        — slight desaturation, dimmer
  if (status == 1) {
    c = mix(c, vec3(0.85, 0.94, 0.78), 0.40);
  } else if (status == 2) {
    c = mix(c, vec3(0.94, 0.90, 0.74), 0.30);
  } else if (status == 3) {
    c = mix(c, vec3(0.85, 0.84, 0.80), 0.18);
  }

  // flood stripes overlay (level 2 & 3) — pale blue diagonal bands
  if (flood > 0 && !oob && !unloaded && !fogTile) {
    float w = (flood == 3) ? 4.0 : 3.0;
    float band = step(0.55, fract(vLocal.y * w + 0.5));
    c = mix(c, vec3(0.78, 0.84, 0.92), band * (flood == 3 ? 0.45 : 0.30));
  }

  // yield dots along bottom — only when zoomed in enough
  if (!oob && !unloaded && !fogTile && !cityHere && uZoom > 0.65) {
    int total = vYields.x + vYields.y + vYields.z;
    if (total > 0 && total <= 6) {
      float dy = 0.46;
      float startX = -float(total - 1) * 0.10;
      for (int i = 0; i < 6; i++) {
        if (i >= vYields.x) break;
        c = mix(c, vec3(0.24, 0.42, 0.20), disc(vLocal, vec2(startX + float(i) * 0.20, dy), 0.072));
      }
      for (int i = 0; i < 6; i++) {
        if (i >= vYields.y) break;
        c = mix(c, vec3(0.36, 0.36, 0.36), disc(vLocal, vec2(startX + float(vYields.x + i) * 0.20, dy), 0.072));
      }
      for (int i = 0; i < 6; i++) {
        if (i >= vYields.z) break;
        c = mix(c, vec3(0.82, 0.66, 0.20), disc(vLocal, vec2(startX + float(vYields.x + vYields.y + i) * 0.20, dy), 0.072));
      }
    }
  }

  // city-status glyph (★/●/○) baked into shader so it survives any zoom
  // — drawn as a small geometric mark in the upper area of the hex.
  if (status > 0 && !cityHere) {
    vec2 g = vLocal - vec2(0.0, -0.30);
    if (status == 2) {
      // working: filled small disc
      c = mix(c, uInk, disc(vec2(0.0,0.0), g, 0.12));
    } else if (status == 3) {
      // idle: hollow ring
      float d = length(g);
      float ring = smoothstep(0.13, 0.115, d) - smoothstep(0.10, 0.085, d);
      c = mix(c, uInk, clamp(ring, 0.0, 1.0));
    }
    // center status falls on the city tile itself — handled separately
  }

  // fog overlay (visible-but-stale tiles) — pale gray wash + dot pattern
  if (fogTile && !oob && !unloaded) {
    float dotG = step(0.55, fract(vLocal.x * 8.0)) * step(0.55, fract(vLocal.y * 8.0));
    c = mix(c, vec3(0.88, 0.86, 0.81), 0.55);
    c = mix(c, vec3(0.74, 0.72, 0.66), dotG * 0.30);
  }

  // fringe — tiles outside the city view radius; rendered as
  // dimmed paper context so we can see the world beyond the city.
  if (fringe) {
    c = mix(c, vec3(0.88, 0.86, 0.81), 0.70);
  }

  // hex outline — thin ink line along the edge of every tile
  float ed = hexEdgeDist(vLocal);
  float ow = 0.018 / max(uZoom, 0.45);
  float edge = 1.0 - smoothstep(ow * 0.6, ow * 1.4, ed);
  vec3 edgeC = (oob || unloaded || fringe) ? vec3(0.65, 0.62, 0.55) : uInk;
  c = mix(c, edgeC, edge * (oob || fringe ? 0.55 : 0.85));

  // selection ring — accent blue ring inset from the edge
  if (vSel == 1) {
    float band = smoothstep(0.0, 0.08, ed) - smoothstep(0.08, 0.16, ed);
    c = mix(c, uAccent, band * 0.95);
  }

  fragColor = vec4(c, 1.0);
}`;

  // ── edge pass — quads along a single hex edge ───────────────
  const EDGE_VS = `#version 300 es
in vec2 aLocal;        // (t, n) along/perpendicular to edge, t in [0,1], n in [-1,1]
in vec4 aInstance;     // (q, r, v1Idx, v2Idx)
in vec2 aKind;         // (kindIdx, widthScale)
uniform vec2  uCam;
uniform float uZoom;
uniform vec2  uView;
uniform float uHexR;
flat out int vKind;
out vec2 vUV;
const float SQRT3 = 1.7320508;
const float PI = 3.14159265;
vec2 hexVert(int i) {
  // pointy-top: 0=top going clockwise
  float a = radians(-90.0 + 60.0 * float(i));
  return vec2(cos(a), sin(a));
}
void main() {
  float R = uHexR * uZoom;
  float hw = SQRT3 * R, hh = 1.5 * R;
  float q = aInstance.x, r = aInstance.y;
  float p = float(int(r) & 1);
  vec2 center = vec2((q + p * 0.5) * hw, r * hh);
  float camP = float(int(uCam.y) & 1);
  vec2 camPx = vec2((uCam.x + camP * 0.5) * hw, uCam.y * hh);
  vec2 v1 = hexVert(int(aInstance.z));
  vec2 v2 = hexVert(int(aInstance.w));
  vec2 mid  = (v1 + v2) * 0.5;
  vec2 along = v2 - v1;
  vec2 perp  = vec2(-along.y, along.x);    // outward-ish
  // place along edge: t in [0,1] → pos along v1→v2
  // n in [-1,1] → perpendicular offset, scaled by widthScale * 1/zoom feel
  float halfW = aKind.y;
  vec2 local  = mix(v1, v2, aLocal.x) + perp * aLocal.y * halfW;
  vec2 world  = center + local * R;
  vec2 screen = world + uView * 0.5 - camPx;
  gl_Position = vec4(screen.x/uView.x*2.0 - 1.0, 1.0 - screen.y/uView.y*2.0, 0.0, 1.0);
  vKind = int(aKind.x);
  vUV = vec2(aLocal.x, aLocal.y);
}`;

  const EDGE_FS = `#version 300 es
precision highp float;
flat in int vKind;
in vec2 vUV;
uniform vec3 uEdgeColors[5];
out vec4 fragColor;
void main() {
  vec3 c = uEdgeColors[vKind];
  float a = 1.0 - smoothstep(0.70, 1.0, abs(vUV.y));
  // dashed pattern for cliff (1), wonder (3), border (4)
  if (vKind == 1 || vKind == 3 || vKind == 4) {
    float dash = step(0.5, fract(vUV.x * 6.0));
    a *= dash;
  }
  if (a < 0.05) discard;
  // border is slightly transparent
  float alpha = (vKind == 4) ? a * 0.7 : a;
  fragColor = vec4(c, alpha);
}`;

  // ── marker pass — disc with feature glyph ───────────────────
  const MARKER_VS = `#version 300 es
in vec2 aLocal;     // unit disc, radius 1
in vec4 aInstance;  // (q, r, markerKind, packed)
uniform vec2  uCam;
uniform float uZoom;
uniform vec2  uView;
uniform float uHexR;
flat out int vKind;
flat out int vPacked;
out vec2 vLocal;
const float SQRT3 = 1.7320508;
void main() {
  float R = uHexR * uZoom;
  float hw = SQRT3 * R, hh = 1.5 * R;
  float q = aInstance.x, r = aInstance.y;
  float p = float(int(r) & 1);
  vec2 center = vec2((q + p * 0.5) * hw, r * hh);
  float camP = float(int(uCam.y) & 1);
  vec2 camPx = vec2((uCam.x + camP * 0.5) * hw, uCam.y * hh);
  int kind = int(aInstance.z);
  // marker size & offset
  float sz = 0.42;
  vec2  off = vec2(0.0, 0.0);
  if (kind == 0) { sz = 0.46; off = vec2(0.0, -0.05); } // city
  if (kind == 1) { sz = 0.32; off = vec2( 0.30, 0.00); } // unit (top-right corner)
  if (kind == 2) { sz = 0.34; off = vec2(-0.30, -0.20); } // wonder (upper-left)
  if (kind == 3) { sz = 0.20; off = vec2( 0.28, 0.22); } // resource (bottom-right)
  vec2 local  = off + aLocal * sz;
  vec2 world  = center + local * R;
  vec2 screen = world + uView * 0.5 - camPx;
  gl_Position = vec4(screen.x/uView.x*2.0 - 1.0, 1.0 - screen.y/uView.y*2.0, 0.0, 1.0);
  vLocal = aLocal;
  vKind  = kind;
  vPacked = int(aInstance.w);
}`;

  const MARKER_FS = `#version 300 es
precision highp float;
flat in int vKind;
flat in int vPacked;
in vec2 vLocal;
uniform vec3 uInk;
uniform vec3 uPaper;
uniform vec3 uAccent;
uniform vec3 uWarn;
uniform vec3 uGood;
out vec4 fragColor;

void main() {
  float d = length(vLocal);
  if (d > 1.05) discard;

  // background of the marker (disc) + outline
  vec3 c;
  vec3 outline = uInk;
  vec3 body    = uPaper;
  if (vKind == 0)      body = mix(uPaper, uAccent, 0.20);  // city — pale accent
  else if (vKind == 1) body = mix(uPaper, uGood,   0.18);  // unit
  else if (vKind == 2) body = mix(uPaper, uWarn,   0.22);  // wonder
  else if (vKind == 3) body = mix(uPaper, uAccent, 0.35);  // resource

  if (d > 0.95)      c = outline;
  else if (d > 0.78) c = outline;       // ring
  else if (d > 0.70) c = body;
  else               c = body;

  // glyph (ink) drawn inside ring
  vec3 ink = uInk;
  bool draw = false;
  vec2 g = vLocal;

  if (vKind == 0) {
    // city — house silhouette (filled triangle roof + small body square)
    bool body2 = abs(g.x) < 0.35 && g.y > -0.05 && g.y < 0.35;
    float roofT = abs(g.x) + (g.y + 0.30) * 0.55;
    bool roof  = roofT < 0.42 && g.y > -0.50 && g.y < -0.05;
    // pop dot in upper-right corner (always 1 dot, just a hint)
    if (body2 || roof) draw = true;
    // little square window
    if (abs(g.x) < 0.07 && abs(g.y - 0.12) < 0.07) draw = false;
  } else if (vKind == 1) {
    // unit — kind glyph
    int k = vPacked & 0xF;
    if (k == 0) {
      // Warrior — triangle ^
      float t = abs(g.x) + max(0.0, (g.y + 0.10) * 1.5);
      if (t < 0.40 && g.y < 0.10 && g.y > -0.40) draw = true;
    } else if (k == 1) {
      // Archer — chevron V
      float arm = abs(abs(g.x) - g.y * 0.9);
      if (arm < 0.12 && g.y > -0.05 && g.y < 0.40 && abs(g.x) < 0.40) draw = true;
    } else if (k == 2) {
      // Horseman — diamond
      if (abs(g.x) + abs(g.y) < 0.42) draw = true;
    } else if (k == 3) {
      // Settler — small house
      bool b1 = abs(g.x) < 0.26 && g.y > -0.05 && g.y < 0.28;
      float rt = abs(g.x) + (g.y + 0.32) * 0.5;
      bool r1 = rt < 0.30 && g.y > -0.36 && g.y < -0.05;
      draw = b1 || r1;
    } else if (k == 4) {
      // Scout — dot
      if (length(g) < 0.30) draw = true;
    } else if (k == 5) {
      // Worker — square ring
      if (abs(g.x) < 0.38 && abs(g.y) < 0.38) draw = true;
      if (abs(g.x) < 0.24 && abs(g.y) < 0.24) draw = false;
    } else if (k == 6) {
      // Spearman — vertical bar
      if (abs(g.x) < 0.12 && abs(g.y) < 0.42) draw = true;
    } else if (k == 7) {
      // Crossbow — X
      float xa = abs(abs(g.x) - abs(g.y));
      if (xa < 0.12 && abs(g.x) < 0.40) draw = true;
    } else {
      if (length(g) < 0.35) draw = true;
    }
  } else if (vKind == 2) {
    // wonder — 5-point starlike triangle
    float t = abs(g.x) + max(0.0, (g.y + 0.10) * 1.3);
    if (t < 0.50 && g.y < 0.10 && g.y > -0.45) draw = true;
    // inverted triangle for star effect
    float t2 = abs(g.x) + max(0.0, (-g.y + 0.10) * 1.3);
    if (t2 < 0.40 && -g.y < 0.10 && -g.y > -0.45) draw = true;
  } else if (vKind == 3) {
    // resource — solid filled diamond
    if (abs(g.x) + abs(g.y) < 0.55) draw = true;
  }

  if (draw) c = ink;
  fragColor = vec4(c, 1.0);
}`;

  // ── helpers ─────────────────────────────────────────────────
  function compile(gl, type, src, label) {
    const sh = gl.createShader(type);
    gl.shaderSource(sh, src);
    gl.compileShader(sh);
    if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) {
      console.error(label + " compile error:\n" + gl.getShaderInfoLog(sh));
      console.error(src);
      throw new Error(label + " compile failed");
    }
    return sh;
  }
  function makeProgram(gl, vsSrc, fsSrc, label) {
    const p = gl.createProgram();
    gl.attachShader(p, compile(gl, gl.VERTEX_SHADER,   vsSrc, label + ".vs"));
    gl.attachShader(p, compile(gl, gl.FRAGMENT_SHADER, fsSrc, label + ".fs"));
    gl.linkProgram(p);
    if (!gl.getProgramParameter(p, gl.LINK_STATUS)) {
      throw new Error(label + " link error:\n" + gl.getProgramInfoLog(p));
    }
    return p;
  }
  function hexFanMesh() {
    // center + 6 corners + closing vertex (TRIANGLE_FAN, 8 verts)
    const m = new Float32Array(16);
    m[0] = 0; m[1] = 0;
    for (let i = 0; i < 7; i++) {
      const a = (Math.PI / 180) * (-90 + 60 * (i % 6));
      m[(i + 1) * 2]     = Math.cos(a);
      m[(i + 1) * 2 + 1] = Math.sin(a);
    }
    return m;
  }
  function edgeQuadMesh() {
    // tri-strip: (t=0, n=-1) (t=1, n=-1) (t=0, n=1) (t=1, n=1)
    return new Float32Array([0,-1, 1,-1, 0,1, 1,1]);
  }
  function discMesh() {
    const m = new Float32Array(2 * 14);
    m[0] = 0; m[1] = 0;
    for (let i = 0; i < 13; i++) {
      const a = (Math.PI*2) * i / 12;
      m[(i+1)*2]   = Math.cos(a);
      m[(i+1)*2+1] = Math.sin(a);
    }
    return m;
  }
  function resizeForDPR(canvas, board) {
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const cssW = board.clientWidth;
    const cssH = board.clientHeight;
    const w = Math.max(1, Math.round(cssW * dpr));
    const h = Math.max(1, Math.round(cssH * dpr));
    if (canvas.width !== w || canvas.height !== h) {
      canvas.width = w; canvas.height = h;
      canvas.style.width = cssW + "px";
      canvas.style.height = cssH + "px";
    }
    return { w, h, dpr, cssW, cssH };
  }

  // ── packed-attr helpers ─────────────────────────────────────
  function packTerrain(tile, extra) {
    if (tile.oob) return (15 & 0xF) | (1 << 4);
    if (tile.unloaded) return (15 & 0xF) | (1 << 5);
    let p = terrainToIdx(tile.terrain) & 0xF;
    if (tile.fog)    p |= 1 << 6;
    if (tile.owner)  p |= 1 << 7;
    if (tile.city)   p |= 1 << 8;
    const flood = Math.max(0, Math.min(3, tile.flood | 0));
    p |= (flood & 0x3) << 9;
    if (extra) {
      // status: 1=center, 2=working, 3=idle (used by the city screen)
      const st = extra.status | 0;
      p |= (st & 0x3) << 11;
      if (extra.fringe)   p |= 1 << 13;
      if (extra.boundary) p |= 1 << 14;
    }
    return p;
  }

  // ── axial → cube distance (for city-focused mode) ───────────
  function offsetToCube(q, r) {
    const x = q - ((r - (r & 1)) >> 1);
    const z = r;
    return { x, y: -x - z, z };
  }
  function cubeDist(q1, r1, q2, r2) {
    const a = offsetToCube(q1, r1), b = offsetToCube(q2, r2);
    return Math.max(Math.abs(a.x - b.x), Math.abs(a.y - b.y), Math.abs(a.z - b.z));
  }
  // pointy-top axial neighbor offsets matching EDGE_DIR_LIST
  //   index 0..5 → ["N", "NE", "SE", "S", "SW", "NW"]
  // Note: the wireframe's edge naming is the *compass heading of the
  // edge*, not the neighbor. For a pointy-top hex with top vertex up,
  // those edges face NW, NE, E, SE, SW, W respectively. Offsets below
  // are for odd-r horizontal offset coords.
  const NEIGHBOR_OFFSETS_EVEN = [
    [-1, -1],  // "N"  edge → NW neighbor
    [ 0, -1],  // "NE" edge → NE neighbor
    [ 1,  0],  // "SE" edge → E neighbor
    [ 0,  1],  // "S"  edge → SE neighbor
    [-1,  1],  // "SW" edge → SW neighbor
    [-1,  0],  // "NW" edge → W neighbor
  ];
  const NEIGHBOR_OFFSETS_ODD = [
    [ 0, -1],  // "N"  edge → NW neighbor
    [ 1, -1],  // "NE" edge → NE neighbor
    [ 1,  0],  // "SE" edge → E neighbor
    [ 1,  1],  // "S"  edge → SE neighbor
    [ 0,  1],  // "SW" edge → SW neighbor
    [-1,  0],  // "NW" edge → W neighbor
  ];
  function neighborAxial(q, r, dir) {
    const off = (r & 1) ? NEIGHBOR_OFFSETS_ODD[dir] : NEIGHBOR_OFFSETS_EVEN[dir];
    return [q + off[0], r + off[1]];
  }

  // ── init ────────────────────────────────────────────────────
  function create(canvas, board) {
    const gl = canvas.getContext("webgl2", {
      antialias: true,
      premultipliedAlpha: false,
      preserveDrawingBuffer: true,
    });
    if (!gl) {
      console.warn("WebGL2 unavailable; HUD hex map will be blank.");
      return { render: () => {}, pick: () => null, HEX_R };
    }

    // paper bg matches --paper #f5f2ea
    gl.clearColor(0.961, 0.949, 0.918, 1);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

    // ── shared color uniforms (from wireframe palette) ───────
    const INK    = [0.10, 0.10, 0.10];
    const PAPER  = [0.961, 0.949, 0.918];
    const ACCENT = [0.169, 0.290, 0.420]; // #2b4a6b
    const WARN   = [0.541, 0.290, 0.169]; // #8a4a2b
    const GOOD   = [0.239, 0.353, 0.169]; // #3d5a2b

    // ── terrain program ──────────────────────────────────────
    const tProg = makeProgram(gl, TERRAIN_VS, TERRAIN_FS, "hud.terrain");
    gl.useProgram(tProg);
    const tu = {
      cam:  gl.getUniformLocation(tProg, "uCam"),
      zoom: gl.getUniformLocation(tProg, "uZoom"),
      view: gl.getUniformLocation(tProg, "uView"),
      hexR: gl.getUniformLocation(tProg, "uHexR"),
      pal:  gl.getUniformLocation(tProg, "uPalette"),
      paper:  gl.getUniformLocation(tProg, "uPaper"),
      ink:    gl.getUniformLocation(tProg, "uInk"),
      accent: gl.getUniformLocation(tProg, "uAccent"),
    };
    const palFlat = new Float32Array(16 * 3);
    for (let i = 0; i < 16; i++) {
      palFlat[i*3]   = PAPER_PALETTE[i][0];
      palFlat[i*3+1] = PAPER_PALETTE[i][1];
      palFlat[i*3+2] = PAPER_PALETTE[i][2];
    }
    gl.uniform3fv(tu.pal, palFlat);
    gl.uniform3fv(tu.paper,  PAPER);
    gl.uniform3fv(tu.ink,    INK);
    gl.uniform3fv(tu.accent, ACCENT);

    const tVao = gl.createVertexArray();
    gl.bindVertexArray(tVao);
    const hexBuf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, hexBuf);
    gl.bufferData(gl.ARRAY_BUFFER, hexFanMesh(), gl.STATIC_DRAW);
    const tLocalLoc = gl.getAttribLocation(tProg, "aLocal");
    gl.enableVertexAttribArray(tLocalLoc);
    gl.vertexAttribPointer(tLocalLoc, 2, gl.FLOAT, false, 0, 0);

    const MAX_T = 4096;
    const T_STRIDE = 7; // (q, r, packed, sel, f, p, g)
    const tInst = new Float32Array(MAX_T * T_STRIDE);
    const tInstBuf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, tInstBuf);
    gl.bufferData(gl.ARRAY_BUFFER, tInst.byteLength, gl.DYNAMIC_DRAW);
    const tInstLoc = gl.getAttribLocation(tProg, "aInstance");
    gl.enableVertexAttribArray(tInstLoc);
    gl.vertexAttribPointer(tInstLoc, 4, gl.FLOAT, false, T_STRIDE * 4, 0);
    gl.vertexAttribDivisor(tInstLoc, 1);
    const tYieldsLoc = gl.getAttribLocation(tProg, "aYields");
    if (tYieldsLoc >= 0) {
      gl.enableVertexAttribArray(tYieldsLoc);
      gl.vertexAttribPointer(tYieldsLoc, 3, gl.FLOAT, false, T_STRIDE * 4, 16);
      gl.vertexAttribDivisor(tYieldsLoc, 1);
    }
    gl.bindVertexArray(null);

    // ── edge program ─────────────────────────────────────────
    const eProg = makeProgram(gl, EDGE_VS, EDGE_FS, "hud.edge");
    gl.useProgram(eProg);
    const eu = {
      cam:    gl.getUniformLocation(eProg, "uCam"),
      zoom:   gl.getUniformLocation(eProg, "uZoom"),
      view:   gl.getUniformLocation(eProg, "uView"),
      hexR:   gl.getUniformLocation(eProg, "uHexR"),
      colors: gl.getUniformLocation(eProg, "uEdgeColors"),
    };
    const edgeColorsFlat = new Float32Array(5 * 3);
    for (let i = 0; i < 5; i++) {
      edgeColorsFlat[i*3]   = EDGE_COLORS[i][0];
      edgeColorsFlat[i*3+1] = EDGE_COLORS[i][1];
      edgeColorsFlat[i*3+2] = EDGE_COLORS[i][2];
    }
    gl.uniform3fv(eu.colors, edgeColorsFlat);

    const eVao = gl.createVertexArray();
    gl.bindVertexArray(eVao);
    const eQuadBuf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, eQuadBuf);
    gl.bufferData(gl.ARRAY_BUFFER, edgeQuadMesh(), gl.STATIC_DRAW);
    const eLocalLoc = gl.getAttribLocation(eProg, "aLocal");
    gl.enableVertexAttribArray(eLocalLoc);
    gl.vertexAttribPointer(eLocalLoc, 2, gl.FLOAT, false, 0, 0);

    const MAX_E = 8192;
    const E_STRIDE = 6; // (q, r, v1, v2, kind, width)
    const eInst = new Float32Array(MAX_E * E_STRIDE);
    const eInstBuf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, eInstBuf);
    gl.bufferData(gl.ARRAY_BUFFER, eInst.byteLength, gl.DYNAMIC_DRAW);
    const eInstLoc = gl.getAttribLocation(eProg, "aInstance");
    gl.enableVertexAttribArray(eInstLoc);
    gl.vertexAttribPointer(eInstLoc, 4, gl.FLOAT, false, E_STRIDE * 4, 0);
    gl.vertexAttribDivisor(eInstLoc, 1);
    const eKindLoc = gl.getAttribLocation(eProg, "aKind");
    gl.enableVertexAttribArray(eKindLoc);
    gl.vertexAttribPointer(eKindLoc, 2, gl.FLOAT, false, E_STRIDE * 4, 16);
    gl.vertexAttribDivisor(eKindLoc, 1);
    gl.bindVertexArray(null);

    // ── marker program ───────────────────────────────────────
    const mProg = makeProgram(gl, MARKER_VS, MARKER_FS, "hud.marker");
    gl.useProgram(mProg);
    const mu = {
      cam:    gl.getUniformLocation(mProg, "uCam"),
      zoom:   gl.getUniformLocation(mProg, "uZoom"),
      view:   gl.getUniformLocation(mProg, "uView"),
      hexR:   gl.getUniformLocation(mProg, "uHexR"),
      ink:    gl.getUniformLocation(mProg, "uInk"),
      paper:  gl.getUniformLocation(mProg, "uPaper"),
      accent: gl.getUniformLocation(mProg, "uAccent"),
      warn:   gl.getUniformLocation(mProg, "uWarn"),
      good:   gl.getUniformLocation(mProg, "uGood"),
    };
    gl.uniform3fv(mu.ink,    INK);
    gl.uniform3fv(mu.paper,  PAPER);
    gl.uniform3fv(mu.accent, ACCENT);
    gl.uniform3fv(mu.warn,   WARN);
    gl.uniform3fv(mu.good,   GOOD);

    const mVao = gl.createVertexArray();
    gl.bindVertexArray(mVao);
    const mDiscBuf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, mDiscBuf);
    gl.bufferData(gl.ARRAY_BUFFER, discMesh(), gl.STATIC_DRAW);
    const mLocalLoc = gl.getAttribLocation(mProg, "aLocal");
    gl.enableVertexAttribArray(mLocalLoc);
    gl.vertexAttribPointer(mLocalLoc, 2, gl.FLOAT, false, 0, 0);

    const MAX_M = 2048;
    const M_STRIDE = 4; // (q, r, kind, packed)
    const mInst = new Float32Array(MAX_M * M_STRIDE);
    const mInstBuf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, mInstBuf);
    gl.bufferData(gl.ARRAY_BUFFER, mInst.byteLength, gl.DYNAMIC_DRAW);
    const mInstLoc = gl.getAttribLocation(mProg, "aInstance");
    gl.enableVertexAttribArray(mInstLoc);
    gl.vertexAttribPointer(mInstLoc, 4, gl.FLOAT, false, M_STRIDE * 4, 0);
    gl.vertexAttribDivisor(mInstLoc, 1);
    gl.bindVertexArray(null);

    // ── render ───────────────────────────────────────────────
    // opts (optional, used by the city-focused view):
    //   centerHex:        { q, r }   — restrict to a cube-distance ring
    //   viewRadius:       int        — render dist <= viewRadius + 1
    //   ownershipRadius:  int        — emit boundary edges at dist === r
    //   tileStatus:       Map<"wq,r" → "center"|"working"|"idle">
    function render(cam, opts) {
      const W = window.WORLD;
      const tileAtFn = window.tileAt;
      const dim = resizeForDPR(canvas, board);
      gl.viewport(0, 0, dim.w, dim.h);
      gl.clear(gl.COLOR_BUFFER_BIT);
      if (!W || !tileAtFn) return;

      opts = opts || {};
      const center = opts.centerHex || null;
      const viewR  = opts.viewRadius != null ? opts.viewRadius : -1;
      const ownR   = opts.ownershipRadius != null ? opts.ownershipRadius : -1;
      const status = opts.tileStatus || null;

      const R = HEX_R * cam.zoom;
      const hw = SQRT3 * R, hh = 1.5 * R;
      let q0, q1, r0, r1;
      if (center && viewR >= 0) {
        // tight iteration window around the city
        const pad = viewR + 2;
        q0 = center.q - pad; q1 = center.q + pad;
        r0 = center.r - pad; r1 = center.r + pad;
      } else {
        const tx = Math.ceil(dim.cssW / hw) + 4;
        const ty = Math.ceil(dim.cssH / hh) + 4;
        q0 = Math.floor(cam.x - tx/2);
        q1 = Math.ceil(cam.x + tx/2);
        r0 = Math.floor(cam.y - ty/2);
        r1 = Math.ceil(cam.y + ty/2);
      }

      let ti = 0, ei = 0, mi = 0;
      const selQ = cam.sel ? cam.sel.q : -999;
      const selR = cam.sel ? cam.sel.r : -999;

      for (let r = r0; r <= r1; r++) {
        for (let q = q0; q <= q1; q++) {
          if (ti >= MAX_T) break;

          // distance gate for city-focused mode
          let dist = -1, fringe = false;
          if (center && viewR >= 0) {
            dist = cubeDist(q, r, center.q, center.r);
            if (dist > viewR + 1) continue;
            fringe = dist > viewR;
          }

          const t = tileAtFn(q, r);
          const wq = t.wq != null ? t.wq : ((q % W.width) + W.width) % W.width;

          // per-tile status from city overlay (if any)
          let stCode = 0;
          if (status) {
            const s = status.get(wq + "," + r);
            if      (s === "center")  stCode = 1;
            else if (s === "working") stCode = 2;
            else if (s === "idle")    stCode = 3;
          }

          const extra = { status: stCode, fringe: fringe };
          const packed = packTerrain(t, extra);
          const sel = (!t.oob && !t.unloaded && wq === selQ && r === selR) ? 1 : 0;

          const o = ti * T_STRIDE;
          tInst[o]   = q;
          tInst[o+1] = r;
          tInst[o+2] = packed;
          tInst[o+3] = sel;
          const ys = (fringe || t.fog) ? null : (t.yields || null);
          tInst[o+4] = ys ? (ys.f | 0) : 0;
          tInst[o+5] = ys ? (ys.p | 0) : 0;
          tInst[o+6] = ys ? (ys.g | 0) : 0;
          ti++;

          if (t.oob || t.unloaded) continue;

          // edges — only render edges for tiles strictly inside the view
          if (t.edges && !fringe) {
            for (let dirIdx = 0; dirIdx < EDGE_DIR_LIST.length; dirIdx++) {
              const ek = EDGE_DIR_LIST[dirIdx];
              const kinds = t.edges[ek];
              if (!kinds || kinds.length === 0) continue;
              const pair = EDGE_VERT_PAIRS[ek];
              for (const kind of kinds) {
                const kidx = EDGE_KIND_INDEX[kind];
                if (kidx == null) continue;
                if (ei >= MAX_E) break;
                const oo = ei * E_STRIDE;
                eInst[oo]   = q;
                eInst[oo+1] = r;
                eInst[oo+2] = pair[0];
                eInst[oo+3] = pair[1];
                eInst[oo+4] = kidx;
                eInst[oo+5] = EDGE_WIDTH[kidx] / Math.max(0.6, cam.zoom * 0.85);
                ei++;
              }
            }
          }

          // ownership-boundary edges (city screen only) — emit a
          // dashed border on each side that crosses ownR → ownR+1.
          if (center && ownR >= 0 && !fringe && dist <= ownR) {
            for (let dirIdx = 0; dirIdx < 6; dirIdx++) {
              const [nq, nr] = neighborAxial(q, r, dirIdx);
              const nd = cubeDist(nq, nr, center.q, center.r);
              if (nd <= ownR) continue;
              if (ei >= MAX_E) break;
              const ek = EDGE_DIR_LIST[dirIdx];
              const pair = EDGE_VERT_PAIRS[ek];
              const kidx = EDGE_KIND_INDEX.border;
              const oo = ei * E_STRIDE;
              eInst[oo]   = q;
              eInst[oo+1] = r;
              eInst[oo+2] = pair[0];
              eInst[oo+3] = pair[1];
              eInst[oo+4] = kidx;
              // thicker than a normal civ border
              eInst[oo+5] = (EDGE_WIDTH[kidx] * 1.8) / Math.max(0.6, cam.zoom * 0.85);
              ei++;
            }
          }

          // markers — order matters (city behind unit etc.)
          if (t.city && mi < MAX_M) {
            const oo = mi * M_STRIDE;
            mInst[oo]   = q;
            mInst[oo+1] = r;
            mInst[oo+2] = MARK_CITY;
            mInst[oo+3] = (t.city.capital ? 1 : 0) | ((t.city.pop || 0) << 1);
            mi++;
          }
          if (t.wonder && mi < MAX_M) {
            const oo = mi * M_STRIDE;
            mInst[oo]   = q;
            mInst[oo+1] = r;
            mInst[oo+2] = MARK_WONDER;
            mInst[oo+3] = 0;
            mi++;
          }
          if (t.resource && !t.city && mi < MAX_M) {
            const oo = mi * M_STRIDE;
            mInst[oo]   = q;
            mInst[oo+1] = r;
            mInst[oo+2] = MARK_RESOURCE;
            mInst[oo+3] = 0;
            mi++;
          }
          if (t.unit && mi < MAX_M) {
            const k = unitKindId(t.unit.kind);
            const oo = mi * M_STRIDE;
            mInst[oo]   = q;
            mInst[oo+1] = r;
            mInst[oo+2] = MARK_UNIT;
            mInst[oo+3] = k & 0xF;
            mi++;
          }
        }
      }

      // terrain pass
      gl.useProgram(tProg);
      gl.uniform2f(tu.cam, cam.x, cam.y);
      gl.uniform1f(tu.zoom, cam.zoom);
      gl.uniform2f(tu.view, dim.cssW, dim.cssH);
      gl.uniform1f(tu.hexR, HEX_R);
      gl.bindBuffer(gl.ARRAY_BUFFER, tInstBuf);
      gl.bufferSubData(gl.ARRAY_BUFFER, 0, tInst, 0, ti * T_STRIDE);
      gl.bindVertexArray(tVao);
      gl.drawArraysInstanced(gl.TRIANGLE_FAN, 0, 8, ti);

      // edge pass
      if (ei > 0) {
        gl.useProgram(eProg);
        gl.uniform2f(eu.cam, cam.x, cam.y);
        gl.uniform1f(eu.zoom, cam.zoom);
        gl.uniform2f(eu.view, dim.cssW, dim.cssH);
        gl.uniform1f(eu.hexR, HEX_R);
        gl.bindBuffer(gl.ARRAY_BUFFER, eInstBuf);
        gl.bufferSubData(gl.ARRAY_BUFFER, 0, eInst, 0, ei * E_STRIDE);
        gl.bindVertexArray(eVao);
        gl.drawArraysInstanced(gl.TRIANGLE_STRIP, 0, 4, ei);
      }

      // marker pass
      if (mi > 0) {
        gl.useProgram(mProg);
        gl.uniform2f(mu.cam, cam.x, cam.y);
        gl.uniform1f(mu.zoom, cam.zoom);
        gl.uniform2f(mu.view, dim.cssW, dim.cssH);
        gl.uniform1f(mu.hexR, HEX_R);
        gl.bindBuffer(gl.ARRAY_BUFFER, mInstBuf);
        gl.bufferSubData(gl.ARRAY_BUFFER, 0, mInst, 0, mi * M_STRIDE);
        gl.bindVertexArray(mVao);
        gl.drawArraysInstanced(gl.TRIANGLE_FAN, 0, 14, mi);
      }
      gl.bindVertexArray(null);
    }

    // ── pick: screen pixel → wrapped (q, r) ─────────────────
    // Inverse of vertex-shader placement. Returns null if OOB.
    function pick(sx, sy, cam) {
      const W = window.WORLD;
      if (!W) return null;
      const rect = board.getBoundingClientRect();
      const cssW = rect.width, cssH = rect.height;
      const R  = HEX_R * cam.zoom;
      const hw = SQRT3 * R, hh = 1.5 * R;
      // pixel offset from camera center
      const px = sx - rect.left - cssW * 0.5;
      const py = sy - rect.top  - cssH * 0.5;
      // world-pixel coords (cam-centered)
      const camP = (cam.y | 0) & 1;
      const camPx = (cam.x + camP * 0.5) * hw;
      const camPy = cam.y * hh;
      const wx = px + camPx;
      const wy = py + camPy;
      // approximate row first
      const rGuess = wy / hh;
      // try the 2 nearest rows; pick closest hex by Euclidean distance
      let best = null;
      let bestD = Infinity;
      for (let dr = -1; dr <= 1; dr++) {
        const r = Math.round(rGuess) + dr;
        if (!W.wrapY && (r < 0 || r >= W.height)) continue;
        const p = (r & 1) ? 0.5 : 0;
        const qf = wx / hw - p;
        for (let dq = -1; dq <= 1; dq++) {
          const q = Math.round(qf) + dq;
          // center pixel of (q, r)
          const cxh = (q + p) * hw;
          const cyh = r * hh;
          const d = Math.hypot(wx - cxh, wy - cyh);
          if (d < bestD) {
            bestD = d;
            const wq = ((q % W.width) + W.width) % W.width;
            best = { q: wq, r };
          }
        }
      }
      return best;
    }

    return { render, pick, HEX_R };
  }

  window.HexHud = { create, HEX_R };
})();
