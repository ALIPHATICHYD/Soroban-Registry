// types/webxr.d.ts

interface XRSession {
  end(): Promise<void>;
}

interface XRSessionInit {
  requiredFeatures?: string[];
  optionalFeatures?: string[];
}

interface XRSystem {
  isSessionSupported(mode: string): Promise<boolean>;
  requestSession(mode: string, init?: XRSessionInit): Promise<XRSession>;
}

interface Navigator {
  xr?: XRSystem;
}
