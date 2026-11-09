import { useEffect, useRef } from "react";
import * as THREE from "three";

/** One pixel of a Minecraft model, in scene units. */
const PX = 1 / 16;

/** A rectangle in texture pixels: x, y, width, height. */
type Rect = [number, number, number, number];

/** The six faces of a Minecraft box, in three.js BoxGeometry order: +x, -x, +y, -y, +z, -z. */
function faceRects(u: number, v: number, w: number, h: number, d: number): Rect[] {
  return [
    [u, v + d, d, h], // +x
    [u + d + w, v + d, d, h], // -x
    [u + d, v, w, d], // +y (top)
    [u + d + w, v, w, d], // -y (bottom)
    [u + d, v + d, w, h], // +z (front, facing the camera)
    [u + 2 * d + w, v + d, w, h], // -z (back)
  ];
}

interface PartSpec {
  /** Size in texture pixels (w, h, d). */
  size: [number, number, number];
  /** Centre in scene units. */
  at: [number, number, number];
  /** Base layer texture origin (u, v). */
  base: [number, number];
  /** Outer layer texture origin, when the skin has one. */
  outer?: [number, number];
  /** Left limbs of an old 64x32 skin reuse the right limb's texture. */
  legacyFrom?: "right-arm" | "right-leg";
}

const PARTS: Record<string, PartSpec> = {
  head: { size: [8, 8, 8], at: [0, 1.75, 0], base: [0, 0], outer: [32, 0] },
  body: { size: [8, 12, 4], at: [0, 1.125, 0], base: [16, 16], outer: [16, 32] },
  rightArm: { size: [4, 12, 4], at: [-0.375, 1.125, 0], base: [40, 16], outer: [40, 32] },
  leftArm: { size: [4, 12, 4], at: [0.375, 1.125, 0], base: [32, 48], outer: [48, 48], legacyFrom: "right-arm" },
  rightLeg: { size: [4, 12, 4], at: [-0.125, 0.375, 0], base: [0, 16], outer: [0, 32] },
  leftLeg: { size: [4, 12, 4], at: [0.125, 0.375, 0], base: [16, 48], outer: [0, 48], legacyFrom: "right-leg" },
};

/** Gives a box the skin's texture regions, face by face. */
function mapBox(geometry: THREE.BoxGeometry, rects: Rect[], texW: number, texH: number) {
  const uv = geometry.attributes.uv as THREE.BufferAttribute;
  rects.forEach((r, face) => {
    const [x, y, w, h] = r;
    for (let k = 0; k < 4; k++) {
      const i = face * 4 + k;
      const a = uv.getX(i);
      const b = uv.getY(i);
      const px = x + a * w;
      const py = y + (1 - b) * h;
      uv.setXY(i, px / texW, 1 - py / texH);
    }
  });
  uv.needsUpdate = true;
}

function buildModel(tex: THREE.Texture, texW: number, texH: number, legacy: boolean): THREE.Group {
  const group = new THREE.Group();
  const material = new THREE.MeshLambertMaterial({ map: tex });
  const overlayMaterial = new THREE.MeshLambertMaterial({ map: tex, transparent: true, alphaTest: 0.5 });

  for (const spec of Object.values(PARTS)) {
    const [w, h, d] = spec.size;
    // Old skins have no left limbs: they reuse the right ones.
    const baseOrigin = legacy && spec.legacyFrom ? PARTS[spec.legacyFrom].base : spec.base;
    const base = new THREE.Mesh(
      mapGeometry(w, h, d, faceRects(baseOrigin[0], baseOrigin[1], w, h, d), texW, texH),
      material,
    );
    base.position.set(...spec.at);
    group.add(base);

    if (!legacy && spec.outer) {
      // The outer layer sits half a pixel out on each side.
      const outer = new THREE.Mesh(
        mapGeometry(w + 1, h + 1, d + 1, faceRects(spec.outer[0], spec.outer[1], w, h, d), texW, texH),
        overlayMaterial,
      );
      outer.position.set(...spec.at);
      group.add(outer);
    }
  }
  return group;
}

function mapGeometry(w: number, h: number, d: number, rects: Rect[], texW: number, texH: number) {
  const geometry = new THREE.BoxGeometry(w * PX, h * PX, d * PX);
  mapBox(geometry, rects, texW, texH);
  return geometry;
}

/** A cape: 10x16x1 pixels behind the body. Its outside face is the texture's left panel. */
function buildCape(tex: THREE.Texture, texW: number, texH: number): THREE.Mesh {
  const rects = faceRects(0, 0, 10, 16, 1);
  // The outside of the cape faces away from the body: swap the front and back panels.
  [rects[4], rects[5]] = [rects[5], rects[4]];
  const material = new THREE.MeshLambertMaterial({ map: tex, side: THREE.DoubleSide });
  const mesh = new THREE.Mesh(mapGeometry(10, 16, 1, rects, texW, texH), material);
  mesh.position.set(0, 1.0, -0.16);
  mesh.rotation.x = 0.06;
  return mesh;
}

interface Props {
  /** Skin texture URL, or null for the plain figure. */
  src: string | null;
  /** Cape texture URL, when the player has one. */
  capeSrc?: string | null;
  className?: string;
  /** Turn slowly on its own until the player drags it. */
  autoRotate?: boolean;
  /** Starting turn in radians. */
  startAngle?: number;
}

function loadTexture(url: string, onLoad: (t: THREE.Texture) => void) {
  const loader = new THREE.TextureLoader();
  loader.setCrossOrigin("anonymous");
  loader.load(url, (tex) => {
    tex.magFilter = THREE.NearestFilter;
    tex.minFilter = THREE.NearestFilter;
    tex.generateMipmaps = false;
    tex.colorSpace = THREE.SRGBColorSpace;
    onLoad(tex);
  });
}

/** A Minecraft player in 3D, wearing the skin and cape. Drag to turn it. */
export default function SkinModel3D({ src, capeSrc = null, className = "", autoRotate = true, startAngle = 0.5 }: Props) {
  const hostRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const host = hostRef.current;
    if (!host || !src) return;

    const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.setClearColor(0x000000, 0);
    const canvas = renderer.domElement;
    canvas.style.position = "absolute";
    canvas.style.inset = "0";
    canvas.style.width = "100%";
    canvas.style.height = "100%";
    host.appendChild(canvas);

    const scene = new THREE.Scene();
    scene.add(new THREE.AmbientLight(0xffffff, 0.85));
    const sun = new THREE.DirectionalLight(0xffffff, 0.7);
    sun.position.set(1.5, 3, 4);
    scene.add(sun);

    const camera = new THREE.PerspectiveCamera(28, 1, 0.1, 100);
    camera.position.set(0, 1.1, 5.2);
    camera.lookAt(0, 1.0, 0);

    const figure = new THREE.Group();
    scene.add(figure);
    const disposables: { dispose: () => void }[] = [];
    let cancelled = false;

    loadTexture(src, (tex) => {
      if (cancelled) return tex.dispose();
      disposables.push(tex);
      const img = tex.image as HTMLImageElement;
      figure.add(buildModel(tex, img.width, img.height, img.height === 32));
    });
    if (capeSrc) {
      loadTexture(capeSrc, (tex) => {
        if (cancelled) return tex.dispose();
        disposables.push(tex);
        const img = tex.image as HTMLImageElement;
        figure.add(buildCape(tex, img.width, img.height));
      });
    }

    let angle = startAngle;
    let dragging = false;
    let lastX = 0;
    const onDown = (e: PointerEvent) => {
      dragging = true;
      lastX = e.clientX;
      host.setPointerCapture(e.pointerId);
    };
    const onMove = (e: PointerEvent) => {
      if (!dragging) return;
      angle += (e.clientX - lastX) * 0.01;
      lastX = e.clientX;
    };
    const onUp = (e: PointerEvent) => {
      dragging = false;
      if (host.hasPointerCapture(e.pointerId)) host.releasePointerCapture(e.pointerId);
    };
    host.addEventListener("pointerdown", onDown);
    host.addEventListener("pointermove", onMove);
    host.addEventListener("pointerup", onUp);
    host.addEventListener("pointercancel", onUp);

    const resize = () => {
      const w = host.clientWidth;
      const h = host.clientHeight;
      if (w === 0 || h === 0) return;
      renderer.setSize(w, h, false);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
    };
    const observer = new ResizeObserver(resize);
    observer.observe(host);
    resize();

    let frame = 0;
    const tick = () => {
      if (autoRotate && !dragging) angle += 0.008;
      figure.rotation.y = angle;
      renderer.render(scene, camera);
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);

    return () => {
      cancelled = true;
      cancelAnimationFrame(frame);
      observer.disconnect();
      host.removeEventListener("pointerdown", onDown);
      host.removeEventListener("pointermove", onMove);
      host.removeEventListener("pointerup", onUp);
      host.removeEventListener("pointercancel", onUp);
      scene.traverse((o) => {
        if (o instanceof THREE.Mesh) {
          o.geometry.dispose();
          (o.material as THREE.Material).dispose();
        }
      });
      disposables.forEach((d) => d.dispose());
      renderer.dispose();
      canvas.remove();
    };
  }, [src, capeSrc, autoRotate, startAngle]);

  return (
    <div
      ref={hostRef}
      className={`relative w-full h-full touch-none select-none cursor-grab active:cursor-grabbing ${className}`}
    />
  );
}
