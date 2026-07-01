// Pointy-top hex geometry (axial coords). Per the locked design decision:
// pointy-top, with axial→pixel  x = √3·(q + r/2),  y = 1.5·r  (in hex-size units).

export const SQRT3 = Math.sqrt(3);

/** Axial (q,r) → world-space pixel center, in units of one hex "size". */
export function axialToPixel(q: number, r: number): [number, number] {
  return [SQRT3 * (q + r / 2), 1.5 * r];
}

/** World-space pixel → axial (q,r), rounded to the nearest hex. Inverse of
 *  {@link axialToPixel}; used for click picking. */
export function pixelToAxial(x: number, y: number): [number, number] {
  const q = (SQRT3 / 3) * x - (1 / 3) * y;
  const r = (2 / 3) * y;
  return axialRound(q, r);
}

/** Cube-round a fractional axial coordinate to the nearest hex. */
export function axialRound(q: number, r: number): [number, number] {
  const s = -q - r;
  let rq = Math.round(q);
  let rr = Math.round(r);
  const rs = Math.round(s);
  const dq = Math.abs(rq - q);
  const dr = Math.abs(rr - r);
  const ds = Math.abs(rs - s);
  if (dq > dr && dq > ds) {
    rq = -rr - rs;
  } else if (dr > ds) {
    rr = -rq - rs;
  }
  return [rq, rr];
}

/** The six corners of a unit pointy-top hexagon (corner angles 60·i − 30°). */
export function hexCorners(): Float32Array {
  const pts = new Float32Array(12);
  for (let i = 0; i < 6; i++) {
    const ang = (Math.PI / 180) * (60 * i - 30);
    pts[i * 2] = Math.cos(ang);
    pts[i * 2 + 1] = Math.sin(ang);
  }
  return pts;
}

/** A unit pointy-top hex as a triangle list (6 tris = 18 verts, 36 floats),
 *  ready for instanced drawing. */
export function hexTriangleMesh(): Float32Array {
  const c = hexCorners();
  const verts = new Float32Array(36);
  let o = 0;
  for (let i = 0; i < 6; i++) {
    const j = (i + 1) % 6;
    // center, corner i, corner j
    verts[o++] = 0; verts[o++] = 0;
    verts[o++] = c[i * 2]; verts[o++] = c[i * 2 + 1];
    verts[o++] = c[j * 2]; verts[o++] = c[j * 2 + 1];
  }
  return verts;
}
