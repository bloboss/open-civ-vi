// Minimal WebGL2 helpers — shader compile + program link. No dependencies.

export function compileShader(
  gl: WebGL2RenderingContext,
  type: number,
  src: string,
): WebGLShader {
  const sh = gl.createShader(type);
  if (!sh) throw new Error("gl.createShader returned null");
  gl.shaderSource(sh, src);
  gl.compileShader(sh);
  if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) {
    const log = gl.getShaderInfoLog(sh);
    gl.deleteShader(sh);
    throw new Error(`shader compile error: ${log ?? "unknown"}`);
  }
  return sh;
}

export function linkProgram(
  gl: WebGL2RenderingContext,
  vsSrc: string,
  fsSrc: string,
): WebGLProgram {
  const vs = compileShader(gl, gl.VERTEX_SHADER, vsSrc);
  const fs = compileShader(gl, gl.FRAGMENT_SHADER, fsSrc);
  const prog = gl.createProgram();
  if (!prog) throw new Error("gl.createProgram returned null");
  gl.attachShader(prog, vs);
  gl.attachShader(prog, fs);
  gl.linkProgram(prog);
  if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
    const log = gl.getProgramInfoLog(prog);
    throw new Error(`program link error: ${log ?? "unknown"}`);
  }
  gl.deleteShader(vs);
  gl.deleteShader(fs);
  return prog;
}

/** Create + fill a static ARRAY_BUFFER. */
export function makeBuffer(gl: WebGL2RenderingContext, data: Float32Array): WebGLBuffer {
  const buf = gl.createBuffer();
  if (!buf) throw new Error("gl.createBuffer returned null");
  gl.bindBuffer(gl.ARRAY_BUFFER, buf);
  gl.bufferData(gl.ARRAY_BUFFER, data, gl.STATIC_DRAW);
  return buf;
}
