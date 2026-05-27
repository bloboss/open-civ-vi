//! WebGL2 hex-map renderer.
//!
//! Rust port of `open4x-server/static/vi/hex-webgl-hud.js` — three
//! instanced passes (terrain / edges / markers) sharing the same hex
//! geometry and camera. GLSL is reproduced verbatim from the JS source
//! so visual parity is direct; the Rust glue mirrors the same buffer
//! layouts.
//!
//! The renderer is **stateful**: program objects, buffers, VAOs, and
//! instance scratch buffers live on the `WebglRenderer` struct.
//! [`WebglRenderer::render`] rebuilds the instance buffers from the
//! current [`WorldSnapshot`] and the caller-supplied camera every
//! frame. [`WebglRenderer::pick`] inverts the vertex-shader placement
//! to map a screen pixel back to a wrapped `(q, r)` tile coordinate.
//!
//! Wire-format limitation: [`open4x_protocol::v1::web::world::TileView`]
//! does not yet carry per-edge metadata (rivers, cliffs, coasts,
//! wonders, civ borders) — the edge pass is therefore dormant and
//! lights up automatically once the protocol grows those fields. The
//! terrain + marker passes already render every tile.

use std::collections::HashMap;

use wasm_bindgen::JsCast;
use web_sys::{
    HtmlCanvasElement, WebGl2RenderingContext as GL, WebGlBuffer, WebGlProgram, WebGlShader,
    WebGlUniformLocation, WebGlVertexArrayObject,
};

use open4x_protocol::v1::web::world::{TileView, WorldSnapshot};

pub const HEX_R: f32 = 46.0;
const SQRT3: f32 = 1.732_050_8;

/// Caller-supplied camera state.
#[derive(Copy, Clone, Debug)]
pub struct Camera {
    /// Axial-q at viewport center (hex units, fractional OK).
    pub x: f32,
    /// Axial-r at viewport center (hex units, fractional OK).
    pub y: f32,
    /// 1.0 = native; larger = zoomed in.
    pub zoom: f32,
    /// Selected tile in wrapped (q, r), if any.
    pub sel: Option<(i32, i32)>,
}

impl Default for Camera {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0, zoom: 1.0, sel: None }
    }
}

/// Terrain palette — muted paper tones, indexed 0..15.
/// Copied verbatim from `hex-webgl-hud.js` lines 33-50.
const PAPER_PALETTE: [[f32; 3]; 16] = [
    [0.71, 0.79, 0.88], // 0  Ocean
    [0.85, 0.89, 0.93], // 1  Coast
    [0.94, 0.91, 0.78], // 2  Plains
    [0.85, 0.91, 0.74], // 3  Grass
    [0.88, 0.81, 0.66], // 4  Hills
    [0.74, 0.83, 0.64], // 5  Forest
    [0.96, 0.91, 0.74], // 6  Desert
    [0.92, 0.93, 0.91], // 7  Tundra
    [0.78, 0.77, 0.74], // 8  Mountain
    [0.92, 0.88, 0.74], // 9  Floodplain
    [0.66, 0.79, 0.62], // 10 Jungle
    [0.80, 0.83, 0.70], // 11 Marsh
    [0.84, 0.88, 0.94], // 12 River
    [0.96, 0.96, 0.94], // 13 Snow
    [0.85, 0.91, 0.91], // 14 Reef
    [0.95, 0.92, 0.86], // 15 Bare / default
];

const INK:    [f32; 3] = [0.10,  0.10,  0.10];
const PAPER:  [f32; 3] = [0.961, 0.949, 0.918];
const ACCENT: [f32; 3] = [0.169, 0.290, 0.420];
const WARN:   [f32; 3] = [0.541, 0.290, 0.169];
const GOOD:   [f32; 3] = [0.239, 0.353, 0.169];

fn terrain_to_idx(name: &str) -> u32 {
    match name {
        "Ocean" => 0, "Coast" => 1, "Plains" => 2, "Grass" => 3,
        "Hills" => 4, "Forest" => 5, "Desert" => 6, "Tundra" => 7,
        "Mtn" | "Mountain" => 8, "Floodpl" | "Floodplain" => 9,
        "Jungle" => 10, "Marsh" => 11, "River" => 12, "Snow" => 13,
        "Reef" => 14, _ => 15,
    }
}

fn unit_kind_id(k: &str) -> u32 {
    match k {
        "Warrior" => 0, "Archer" => 1, "Horseman" => 2, "Settler" => 3,
        "Scout" => 4, "Worker" => 5, "Spearman" => 6, "Crossbow" => 7,
        _ => 8,
    }
}

// ── shader sources (verbatim from hex-webgl-hud.js) ──────────────────────────

const TERRAIN_VS: &str = r#"#version 300 es
in vec2 aLocal;
in vec4 aInstance;
in vec3 aYields;
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
}"#;

const TERRAIN_FS: &str = r#"#version 300 es
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
float hexEdgeDist(vec2 p) {
  vec2 ap = vec2(abs(p.x), abs(p.y));
  return 0.866025 - max(ap.x, ap.x * 0.5 + ap.y * 0.866025);
}
void main() {
  int  terrain  = vPacked & 0xF;
  bool oob      = ((vPacked >> 4) & 1) == 1;
  bool unloaded = ((vPacked >> 5) & 1) == 1;
  bool fogTile  = ((vPacked >> 6) & 1) == 1;
  bool owned    = ((vPacked >> 7) & 1) == 1;
  bool cityHere = ((vPacked >> 8) & 1) == 1;
  int  flood    = (vPacked >> 9) & 0x3;
  vec3 c;
  if (oob) {
    float s = step(0.5, fract((vLocal.x + vLocal.y) * 6.0));
    c = mix(vec3(0.89,0.87,0.83), vec3(0.83,0.81,0.76), s);
  } else if (unloaded) {
    c = vec3(0.92, 0.91, 0.86);
  } else {
    c = uPalette[terrain];
  }
  if (owned && !oob && !unloaded) c = mix(c, uAccent, 0.06);
  if (flood > 0 && !oob && !unloaded && !fogTile) {
    float w = (flood == 3) ? 4.0 : 3.0;
    float band = step(0.55, fract(vLocal.y * w + 0.5));
    c = mix(c, vec3(0.78,0.84,0.92), band * (flood == 3 ? 0.45 : 0.30));
  }
  if (!oob && !unloaded && !fogTile && !cityHere && uZoom > 0.65) {
    int total = vYields.x + vYields.y + vYields.z;
    if (total > 0 && total <= 6) {
      float dy = 0.46;
      float startX = -float(total - 1) * 0.10;
      for (int i = 0; i < 6; i++) {
        if (i >= vYields.x) break;
        c = mix(c, vec3(0.24,0.42,0.20), disc(vLocal, vec2(startX + float(i) * 0.20, dy), 0.072));
      }
      for (int i = 0; i < 6; i++) {
        if (i >= vYields.y) break;
        c = mix(c, vec3(0.36,0.36,0.36), disc(vLocal, vec2(startX + float(vYields.x + i) * 0.20, dy), 0.072));
      }
      for (int i = 0; i < 6; i++) {
        if (i >= vYields.z) break;
        c = mix(c, vec3(0.82,0.66,0.20), disc(vLocal, vec2(startX + float(vYields.x + vYields.y + i) * 0.20, dy), 0.072));
      }
    }
  }
  if (fogTile && !oob && !unloaded) {
    float dotG = step(0.55, fract(vLocal.x * 8.0)) * step(0.55, fract(vLocal.y * 8.0));
    c = mix(c, vec3(0.88,0.86,0.81), 0.55);
    c = mix(c, vec3(0.74,0.72,0.66), dotG * 0.30);
  }
  float ed = hexEdgeDist(vLocal);
  float ow = 0.018 / max(uZoom, 0.45);
  float edge = 1.0 - smoothstep(ow * 0.6, ow * 1.4, ed);
  vec3 edgeC = (oob || unloaded) ? vec3(0.65,0.62,0.55) : uInk;
  c = mix(c, edgeC, edge * (oob ? 0.55 : 0.85));
  if (vSel == 1) {
    float band = smoothstep(0.0, 0.08, ed) - smoothstep(0.08, 0.16, ed);
    c = mix(c, uAccent, band * 0.95);
  }
  fragColor = vec4(c, 1.0);
}"#;

const MARKER_VS: &str = r#"#version 300 es
in vec2 aLocal;
in vec4 aInstance;
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
  float sz = 0.42;
  vec2  off = vec2(0.0, 0.0);
  if (kind == 0) { sz = 0.46; off = vec2(0.0, -0.05); }
  if (kind == 1) { sz = 0.32; off = vec2( 0.30, 0.00); }
  if (kind == 2) { sz = 0.34; off = vec2(-0.30, -0.20); }
  if (kind == 3) { sz = 0.20; off = vec2( 0.28, 0.22); }
  vec2 local  = off + aLocal * sz;
  vec2 world  = center + local * R;
  vec2 screen = world + uView * 0.5 - camPx;
  gl_Position = vec4(screen.x/uView.x*2.0 - 1.0, 1.0 - screen.y/uView.y*2.0, 0.0, 1.0);
  vLocal = aLocal;
  vKind  = kind;
  vPacked = int(aInstance.w);
}"#;

const MARKER_FS: &str = r#"#version 300 es
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
  vec3 c;
  vec3 outline = uInk;
  vec3 body    = uPaper;
  if (vKind == 0)      body = mix(uPaper, uAccent, 0.20);
  else if (vKind == 1) body = mix(uPaper, uGood,   0.18);
  else if (vKind == 2) body = mix(uPaper, uWarn,   0.22);
  else if (vKind == 3) body = mix(uPaper, uAccent, 0.35);
  if (d > 0.95)      c = outline;
  else if (d > 0.78) c = outline;
  else if (d > 0.70) c = body;
  else               c = body;
  vec3 ink = uInk;
  bool draw = false;
  vec2 g = vLocal;
  if (vKind == 0) {
    bool body2 = abs(g.x) < 0.35 && g.y > -0.05 && g.y < 0.35;
    float roofT = abs(g.x) + (g.y + 0.30) * 0.55;
    bool roof  = roofT < 0.42 && g.y > -0.50 && g.y < -0.05;
    if (body2 || roof) draw = true;
    if (abs(g.x) < 0.07 && abs(g.y - 0.12) < 0.07) draw = false;
  } else if (vKind == 1) {
    int k = vPacked & 0xF;
    if (k == 0) {
      float t = abs(g.x) + max(0.0, (g.y + 0.10) * 1.5);
      if (t < 0.40 && g.y < 0.10 && g.y > -0.40) draw = true;
    } else if (k == 1) {
      float arm = abs(abs(g.x) - g.y * 0.9);
      if (arm < 0.12 && g.y > -0.05 && g.y < 0.40 && abs(g.x) < 0.40) draw = true;
    } else if (k == 2) {
      if (abs(g.x) + abs(g.y) < 0.42) draw = true;
    } else if (k == 3) {
      bool b1 = abs(g.x) < 0.26 && g.y > -0.05 && g.y < 0.28;
      float rt = abs(g.x) + (g.y + 0.32) * 0.5;
      bool r1 = rt < 0.30 && g.y > -0.36 && g.y < -0.05;
      draw = b1 || r1;
    } else if (k == 4) {
      if (length(g) < 0.30) draw = true;
    } else if (k == 5) {
      if (abs(g.x) < 0.38 && abs(g.y) < 0.38) draw = true;
      if (abs(g.x) < 0.24 && abs(g.y) < 0.24) draw = false;
    } else if (k == 6) {
      if (abs(g.x) < 0.12 && abs(g.y) < 0.42) draw = true;
    } else if (k == 7) {
      float xa = abs(abs(g.x) - abs(g.y));
      if (xa < 0.12 && abs(g.x) < 0.40) draw = true;
    } else {
      if (length(g) < 0.35) draw = true;
    }
  } else if (vKind == 3) {
    if (abs(g.x) + abs(g.y) < 0.55) draw = true;
  }
  if (draw) c = ink;
  fragColor = vec4(c, 1.0);
}"#;

// ── compile/link helpers ─────────────────────────────────────────────────────

fn compile_shader(gl: &GL, kind: u32, src: &str, label: &str) -> Result<WebGlShader, String> {
    let sh = gl.create_shader(kind).ok_or_else(|| format!("createShader({label}) returned null"))?;
    gl.shader_source(&sh, src);
    gl.compile_shader(&sh);
    if gl.get_shader_parameter(&sh, GL::COMPILE_STATUS).as_bool().unwrap_or(false) {
        Ok(sh)
    } else {
        Err(gl.get_shader_info_log(&sh).unwrap_or_else(|| format!("{label}: unknown compile error")))
    }
}

fn link_program(gl: &GL, vs: &str, fs: &str, label: &str) -> Result<WebGlProgram, String> {
    let vshader = compile_shader(gl, GL::VERTEX_SHADER,   vs, &format!("{label}.vs"))?;
    let fshader = compile_shader(gl, GL::FRAGMENT_SHADER, fs, &format!("{label}.fs"))?;
    let prog = gl.create_program().ok_or_else(|| format!("createProgram({label}) returned null"))?;
    gl.attach_shader(&prog, &vshader);
    gl.attach_shader(&prog, &fshader);
    gl.link_program(&prog);
    if gl.get_program_parameter(&prog, GL::LINK_STATUS).as_bool().unwrap_or(false) {
        Ok(prog)
    } else {
        Err(gl.get_program_info_log(&prog).unwrap_or_else(|| format!("{label}: unknown link error")))
    }
}

fn upload_static_buf(gl: &GL, data: &[f32]) -> Result<WebGlBuffer, String> {
    let buf = gl.create_buffer().ok_or("createBuffer returned null")?;
    gl.bind_buffer(GL::ARRAY_BUFFER, Some(&buf));
    // SAFETY: bound for the duration of bufferData; the Float32Array view
    // is dropped before any wasm allocation can move the underlying memory.
    unsafe {
        let view = js_sys::Float32Array::view(data);
        gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::STATIC_DRAW);
    }
    Ok(buf)
}

fn upload_dynamic_buf(gl: &GL, byte_len: i32) -> Result<WebGlBuffer, String> {
    let buf = gl.create_buffer().ok_or("createBuffer returned null")?;
    gl.bind_buffer(GL::ARRAY_BUFFER, Some(&buf));
    gl.buffer_data_with_i32(GL::ARRAY_BUFFER, byte_len, GL::DYNAMIC_DRAW);
    Ok(buf)
}

fn hex_fan_mesh() -> [f32; 16] {
    // center + 6 corners + closing vertex (TRIANGLE_FAN, 8 verts)
    let mut m = [0.0f32; 16];
    for i in 0..7 {
        let a = (-90.0_f32 + 60.0 * ((i % 6) as f32)).to_radians();
        m[(i + 1) * 2]     = a.cos();
        m[(i + 1) * 2 + 1] = a.sin();
    }
    m
}

fn disc_mesh() -> [f32; 28] {
    let mut m = [0.0f32; 28];
    for i in 0..13 {
        let a = std::f32::consts::TAU * (i as f32) / 12.0;
        m[(i + 1) * 2]     = a.cos();
        m[(i + 1) * 2 + 1] = a.sin();
    }
    m
}

// ── pass containers ─────────────────────────────────────────────────────────

struct TerrainPass {
    program: WebGlProgram,
    vao: WebGlVertexArrayObject,
    inst_buf: WebGlBuffer,
    u_cam: WebGlUniformLocation,
    u_zoom: WebGlUniformLocation,
    u_view: WebGlUniformLocation,
    u_hex_r: WebGlUniformLocation,
}

const T_STRIDE: usize = 7;  // (q, r, packed, sel, f, p, g)
const MAX_T: usize    = 4096;

struct MarkerPass {
    program: WebGlProgram,
    vao: WebGlVertexArrayObject,
    inst_buf: WebGlBuffer,
    u_cam: WebGlUniformLocation,
    u_zoom: WebGlUniformLocation,
    u_view: WebGlUniformLocation,
    u_hex_r: WebGlUniformLocation,
}

const M_STRIDE: usize = 4;  // (q, r, kind, packed)
const MAX_M: usize    = 2048;

// ── public renderer ─────────────────────────────────────────────────────────

pub struct WebglRenderer {
    pub canvas: HtmlCanvasElement,
    gl: GL,
    terrain: TerrainPass,
    marker: MarkerPass,
    /// Instance scratch buffers (avoid re-allocating per frame).
    t_inst: Vec<f32>,
    m_inst: Vec<f32>,
}

impl WebglRenderer {
    /// Build a renderer against the given canvas. Returns `None` if
    /// WebGL2 is unavailable.
    pub fn create(canvas: HtmlCanvasElement) -> Result<Self, String> {
        let gl = canvas
            .get_context("webgl2")
            .map_err(|e| format!("getContext(webgl2) threw: {e:?}"))?
            .ok_or("WebGL2 unavailable")?
            .dyn_into::<GL>()
            .map_err(|_| "context is not WebGl2RenderingContext".to_string())?;

        gl.clear_color(PAPER[0], PAPER[1], PAPER[2], 1.0);
        gl.enable(GL::BLEND);
        gl.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);

        let terrain = Self::build_terrain_pass(&gl)?;
        let marker  = Self::build_marker_pass(&gl)?;

        Ok(Self {
            canvas, gl, terrain, marker,
            t_inst: vec![0.0; MAX_T * T_STRIDE],
            m_inst: vec![0.0; MAX_M * M_STRIDE],
        })
    }

    fn build_terrain_pass(gl: &GL) -> Result<TerrainPass, String> {
        let program = link_program(gl, TERRAIN_VS, TERRAIN_FS, "hud.terrain")?;
        gl.use_program(Some(&program));

        // Static uniforms.
        let u_palette = gl.get_uniform_location(&program, "uPalette");
        let mut pal = [0.0f32; 48];
        for (i, c) in PAPER_PALETTE.iter().enumerate() {
            pal[i * 3]     = c[0];
            pal[i * 3 + 1] = c[1];
            pal[i * 3 + 2] = c[2];
        }
        gl.uniform3fv_with_f32_array(u_palette.as_ref(), &pal);
        gl.uniform3fv_with_f32_array(gl.get_uniform_location(&program, "uPaper").as_ref(),  &PAPER);
        gl.uniform3fv_with_f32_array(gl.get_uniform_location(&program, "uInk").as_ref(),    &INK);
        gl.uniform3fv_with_f32_array(gl.get_uniform_location(&program, "uAccent").as_ref(), &ACCENT);

        let vao = gl.create_vertex_array().ok_or("createVertexArray returned null")?;
        gl.bind_vertex_array(Some(&vao));

        let hex = hex_fan_mesh();
        upload_static_buf(gl, &hex)?;
        let local_loc = gl.get_attrib_location(&program, "aLocal") as u32;
        gl.enable_vertex_attrib_array(local_loc);
        gl.vertex_attrib_pointer_with_i32(local_loc, 2, GL::FLOAT, false, 0, 0);

        let inst_buf = upload_dynamic_buf(gl, (MAX_T * T_STRIDE * 4) as i32)?;
        let inst_loc = gl.get_attrib_location(&program, "aInstance") as u32;
        gl.enable_vertex_attrib_array(inst_loc);
        gl.vertex_attrib_pointer_with_i32(inst_loc, 4, GL::FLOAT, false, (T_STRIDE * 4) as i32, 0);
        gl.vertex_attrib_divisor(inst_loc, 1);
        let yields_loc = gl.get_attrib_location(&program, "aYields");
        if yields_loc >= 0 {
            let yields_loc = yields_loc as u32;
            gl.enable_vertex_attrib_array(yields_loc);
            gl.vertex_attrib_pointer_with_i32(yields_loc, 3, GL::FLOAT, false, (T_STRIDE * 4) as i32, 16);
            gl.vertex_attrib_divisor(yields_loc, 1);
        }
        gl.bind_vertex_array(None);

        Ok(TerrainPass {
            u_cam:   gl.get_uniform_location(&program, "uCam").ok_or("missing uCam")?,
            u_zoom:  gl.get_uniform_location(&program, "uZoom").ok_or("missing uZoom")?,
            u_view:  gl.get_uniform_location(&program, "uView").ok_or("missing uView")?,
            u_hex_r: gl.get_uniform_location(&program, "uHexR").ok_or("missing uHexR")?,
            program, vao, inst_buf,
        })
    }

    fn build_marker_pass(gl: &GL) -> Result<MarkerPass, String> {
        let program = link_program(gl, MARKER_VS, MARKER_FS, "hud.marker")?;
        gl.use_program(Some(&program));

        gl.uniform3fv_with_f32_array(gl.get_uniform_location(&program, "uInk").as_ref(),    &INK);
        gl.uniform3fv_with_f32_array(gl.get_uniform_location(&program, "uPaper").as_ref(),  &PAPER);
        gl.uniform3fv_with_f32_array(gl.get_uniform_location(&program, "uAccent").as_ref(), &ACCENT);
        gl.uniform3fv_with_f32_array(gl.get_uniform_location(&program, "uWarn").as_ref(),   &WARN);
        gl.uniform3fv_with_f32_array(gl.get_uniform_location(&program, "uGood").as_ref(),   &GOOD);

        let vao = gl.create_vertex_array().ok_or("createVertexArray returned null")?;
        gl.bind_vertex_array(Some(&vao));

        let disc = disc_mesh();
        upload_static_buf(gl, &disc)?;
        let local_loc = gl.get_attrib_location(&program, "aLocal") as u32;
        gl.enable_vertex_attrib_array(local_loc);
        gl.vertex_attrib_pointer_with_i32(local_loc, 2, GL::FLOAT, false, 0, 0);

        let inst_buf = upload_dynamic_buf(gl, (MAX_M * M_STRIDE * 4) as i32)?;
        let inst_loc = gl.get_attrib_location(&program, "aInstance") as u32;
        gl.enable_vertex_attrib_array(inst_loc);
        gl.vertex_attrib_pointer_with_i32(inst_loc, 4, GL::FLOAT, false, (M_STRIDE * 4) as i32, 0);
        gl.vertex_attrib_divisor(inst_loc, 1);
        gl.bind_vertex_array(None);

        Ok(MarkerPass {
            u_cam:   gl.get_uniform_location(&program, "uCam").ok_or("missing uCam")?,
            u_zoom:  gl.get_uniform_location(&program, "uZoom").ok_or("missing uZoom")?,
            u_view:  gl.get_uniform_location(&program, "uView").ok_or("missing uView")?,
            u_hex_r: gl.get_uniform_location(&program, "uHexR").ok_or("missing uHexR")?,
            program, vao, inst_buf,
        })
    }

    /// Drive the canvas's backing store size from the CSS box + DPR.
    fn resize_for_dpr(&self) -> (i32, i32, f32, f32) {
        let win = web_sys::window().expect("window");
        let dpr = (win.device_pixel_ratio() as f32).min(2.0);
        let css_w = self.canvas.client_width().max(1) as f32;
        let css_h = self.canvas.client_height().max(1) as f32;
        let w = (css_w * dpr).round() as i32;
        let h = (css_h * dpr).round() as i32;
        if self.canvas.width() as i32 != w || self.canvas.height() as i32 != h {
            self.canvas.set_width(w as u32);
            self.canvas.set_height(h as u32);
            let _ = self.canvas.style().set_property("width",  &format!("{css_w}px"));
            let _ = self.canvas.style().set_property("height", &format!("{css_h}px"));
        }
        (w, h, css_w, css_h)
    }

    /// Rebuild instance data from `snapshot` and render. Cheap to call
    /// every animation frame.
    pub fn render(&mut self, snapshot: &WorldSnapshot, cam: Camera) {
        let (w, h, css_w, css_h) = self.resize_for_dpr();
        let gl = &self.gl;
        gl.viewport(0, 0, w, h);
        gl.clear(GL::COLOR_BUFFER_BIT);

        // Index tiles by axial (q, r) for O(1) lookup. Production
        // worlds carry a few thousand tiles tops; rebuilding per frame
        // is cheap enough until we add a dirty signal.
        let mut by_qr: HashMap<(i32, i32), &TileView> = HashMap::with_capacity(snapshot.tiles.len());
        for t in &snapshot.tiles {
            by_qr.insert((t.q, t.r), t);
        }

        let world_w = snapshot.world.width as i32;
        let wrap_x = snapshot.world.wrap_x;

        let r_px = HEX_R * cam.zoom;
        let hw = SQRT3 * r_px;
        let hh = 1.5 * r_px;

        let tiles_x = (css_w / hw).ceil() as i32 + 4;
        let tiles_y = (css_h / hh).ceil() as i32 + 4;
        let q0 = (cam.x as i32) - tiles_x / 2;
        let q1 = (cam.x as i32) + tiles_x / 2;
        let r0 = (cam.y as i32) - tiles_y / 2;
        let r1 = (cam.y as i32) + tiles_y / 2;

        let (sel_q, sel_r) = cam.sel.unwrap_or((i32::MIN, i32::MIN));

        let mut ti: usize = 0;
        let mut mi: usize = 0;

        for r in r0..=r1 {
            // Vertical out-of-bounds: only render OOB stripes; iteration row
            // stays so the visual frame is full.
            let r_oob = !snapshot.world.wrap_y && (r < 0 || r >= snapshot.world.height as i32);

            for q in q0..=q1 {
                if ti >= MAX_T { break; }

                let wq = wrap(q, world_w, wrap_x);
                let tile_opt = if r_oob || wq.is_none() { None } else { by_qr.get(&(wq.unwrap(), r)).copied() };

                let (packed, ys_f, ys_p, ys_g) = if r_oob || wq.is_none() {
                    // OOB
                    (oob_packed(), 0, 0, 0)
                } else if let Some(t) = tile_opt {
                    (pack_tile(t), t.yields.f.unwrap_or(0), t.yields.p.unwrap_or(0), t.yields.g.unwrap_or(0))
                } else {
                    // Unloaded: tile in-bounds but not in the sparse snapshot
                    (unloaded_packed(), 0, 0, 0)
                };

                let sel = match wq {
                    Some(wqq) if wqq == sel_q && r == sel_r => 1,
                    _ => 0,
                };

                let o = ti * T_STRIDE;
                self.t_inst[o]     = q as f32;
                self.t_inst[o + 1] = r as f32;
                self.t_inst[o + 2] = packed as f32;
                self.t_inst[o + 3] = sel as f32;
                self.t_inst[o + 4] = ys_f as f32;
                self.t_inst[o + 5] = ys_p as f32;
                self.t_inst[o + 6] = ys_g as f32;
                ti += 1;

                if let Some(t) = tile_opt {
                    if mi < MAX_M && t.city.is_some() {
                        push_marker(&mut self.m_inst, &mut mi, q, r, 0,
                            t.city.as_ref().map(|c| {
                                (if c.capital { 1 } else { 0 }) | (c.pop << 1)
                            }).unwrap_or(0));
                    }
                    if mi < MAX_M && t.resource.is_some() && t.city.is_none() {
                        push_marker(&mut self.m_inst, &mut mi, q, r, 3, 0);
                    }
                    if mi < MAX_M && t.unit.is_some() {
                        let k = t.unit.as_ref().map(|u| unit_kind_id(&u.kind)).unwrap_or(8);
                        push_marker(&mut self.m_inst, &mut mi, q, r, 1, k & 0xF);
                    }
                }
            }
        }

        // ── terrain pass ──
        gl.use_program(Some(&self.terrain.program));
        gl.uniform2f(Some(&self.terrain.u_cam),  cam.x, cam.y);
        gl.uniform1f(Some(&self.terrain.u_zoom), cam.zoom);
        gl.uniform2f(Some(&self.terrain.u_view), css_w, css_h);
        gl.uniform1f(Some(&self.terrain.u_hex_r), HEX_R);
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.terrain.inst_buf));
        unsafe {
            let view = js_sys::Float32Array::view(&self.t_inst[..ti * T_STRIDE]);
            gl.buffer_sub_data_with_i32_and_array_buffer_view(GL::ARRAY_BUFFER, 0, &view);
        }
        gl.bind_vertex_array(Some(&self.terrain.vao));
        gl.draw_arrays_instanced(GL::TRIANGLE_FAN, 0, 8, ti as i32);

        // ── marker pass ──
        if mi > 0 {
            gl.use_program(Some(&self.marker.program));
            gl.uniform2f(Some(&self.marker.u_cam),  cam.x, cam.y);
            gl.uniform1f(Some(&self.marker.u_zoom), cam.zoom);
            gl.uniform2f(Some(&self.marker.u_view), css_w, css_h);
            gl.uniform1f(Some(&self.marker.u_hex_r), HEX_R);
            gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.marker.inst_buf));
            unsafe {
                let view = js_sys::Float32Array::view(&self.m_inst[..mi * M_STRIDE]);
                gl.buffer_sub_data_with_i32_and_array_buffer_view(GL::ARRAY_BUFFER, 0, &view);
            }
            gl.bind_vertex_array(Some(&self.marker.vao));
            gl.draw_arrays_instanced(GL::TRIANGLE_FAN, 0, 14, mi as i32);
        }
        gl.bind_vertex_array(None);
    }

    /// Map a CSS-pixel (offsetX, offsetY) on this canvas back to a
    /// wrapped (q, r) tile. Mirrors `hex-webgl-hud.js::pick`.
    pub fn pick(&self, sx: f32, sy: f32, cam: Camera, snapshot: &WorldSnapshot) -> Option<(i32, i32)> {
        let css_w = self.canvas.client_width() as f32;
        let css_h = self.canvas.client_height() as f32;
        let r_px = HEX_R * cam.zoom;
        let hw = SQRT3 * r_px;
        let hh = 1.5 * r_px;

        // pixel offset from canvas center
        let px = sx - css_w * 0.5;
        let py = sy - css_h * 0.5;
        // world-pixel coords (cam-centered)
        let cam_p = ((cam.y as i32) & 1) as f32;
        let cam_px = (cam.x + cam_p * 0.5) * hw;
        let cam_py = cam.y * hh;
        let wx = px + cam_px;
        let wy = py + cam_py;

        let world_w = snapshot.world.width as i32;
        let world_h = snapshot.world.height as i32;
        let wrap_x = snapshot.world.wrap_x;
        let wrap_y = snapshot.world.wrap_y;

        let r_guess = wy / hh;
        let mut best: Option<(i32, i32)> = None;
        let mut best_d = f32::INFINITY;
        for dr in -1..=1 {
            let r = r_guess.round() as i32 + dr;
            if !wrap_y && (r < 0 || r >= world_h) { continue; }
            let p = if (r & 1) != 0 { 0.5 } else { 0.0 };
            let qf = wx / hw - p;
            for dq in -1..=1 {
                let q = qf.round() as i32 + dq;
                let cxh = (q as f32 + p) * hw;
                let cyh = r as f32 * hh;
                let d = ((wx - cxh).powi(2) + (wy - cyh).powi(2)).sqrt();
                if d < best_d {
                    best_d = d;
                    let wq = wrap(q, world_w, wrap_x);
                    if let Some(wqq) = wq {
                        best = Some((wqq, r));
                    }
                }
            }
        }
        best
    }
}

fn wrap(q: i32, world_w: i32, wrap_x: bool) -> Option<i32> {
    if world_w <= 0 { return None; }
    if wrap_x {
        Some(((q % world_w) + world_w) % world_w)
    } else if q < 0 || q >= world_w {
        None
    } else {
        Some(q)
    }
}

fn oob_packed() -> u32 {
    (15 & 0xF) | (1 << 4)
}

fn unloaded_packed() -> u32 {
    (15 & 0xF) | (1 << 5)
}

/// Pack a real tile into the 16-bit `aInstance.z` slot. Layout matches
/// `hex-webgl-hud.js::packTerrain` (lines 533-550) minus the city-screen
/// extras (status/fringe/boundary) — those come back when the city
/// screen lands.
fn pack_tile(t: &TileView) -> u32 {
    let mut p = terrain_to_idx(&t.terrain) & 0xF;
    if t.fog                 { p |= 1 << 6; }
    if t.owner.is_some()     { p |= 1 << 7; }
    if t.city.is_some()      { p |= 1 << 8; }
    let flood = t.flood.clamp(0, 3) as u32;
    p |= (flood & 0x3) << 9;
    p
}

fn push_marker(buf: &mut [f32], mi: &mut usize, q: i32, r: i32, kind: u32, packed: u32) {
    let o = *mi * M_STRIDE;
    buf[o]     = q as f32;
    buf[o + 1] = r as f32;
    buf[o + 2] = kind as f32;
    buf[o + 3] = packed as f32;
    *mi += 1;
}
