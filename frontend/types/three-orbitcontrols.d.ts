// types/three-orbitcontrols.d.ts

declare module "three/examples/jsm/controls/OrbitControls.js" {
  import { Camera, EventDispatcher, MOUSE, TOUCH, Vector3 } from "three";

  export class OrbitControls extends EventDispatcher {
    object: Camera;
    target: Vector3;
    enableDamping: boolean;
    dampingFactor: number;
    minDistance: number;
    maxDistance: number;
    update(): void;
    dispose(): void;
  }
}
