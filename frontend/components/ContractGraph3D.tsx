"use client";

import { useEffect, useRef, useState } from "react";
import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { GraphNode } from "@/lib/api";
import { useContractGraph } from "@/hooks/useContractGraph";

interface ContractGraph3DProps {
  contractId: string;
  height?: number;
}

export default function ContractGraph3D({
  contractId,
  height = 520,
}: ContractGraph3DProps) {
  const mountRef = useRef<HTMLDivElement | null>(null);
  const rendererRef = useRef<THREE.WebGLRenderer | null>(null);
  const [selectedNode, setSelectedNode] = useState<GraphNode | null>(null);
  const [arSupported, setArSupported] = useState(false);
  const { graph, isLoading, error, exportAsJson } = useContractGraph(contractId, 2);

  useEffect(() => {
    let mounted = true;
    async function checkArSupport() {
      const xr = (navigator as Navigator & { xr?: XRSystem }).xr;
      if (!xr || !xr.isSessionSupported) {
        if (mounted) setArSupported(false);
        return;
      }
      const supported = await xr.isSessionSupported("immersive-ar");
      if (mounted) setArSupported(supported);
    }
    void checkArSupport();
    return () => {
      mounted = false;
    };
  }, []);

  useEffect(() => {
    const container = mountRef.current;
    if (!container) return;
    if (!graph || graph.nodes.length === 0) return;

    const width = container.clientWidth || 960;
    const scene = new THREE.Scene();
    scene.background = new THREE.Color("#0f172a");

    const camera = new THREE.PerspectiveCamera(60, width / height, 0.1, 2000);
    camera.position.set(0, 10, 35);

    const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: false });
    renderer.setSize(width, height);
    renderer.xr.enabled = true;
    container.innerHTML = "";
    container.appendChild(renderer.domElement);
    rendererRef.current = renderer;

    // @ts-ignore — OrbitControls types mismatch with WebGLRenderer.domElement
    const controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;
    controls.dampingFactor = 0.08;
    controls.minDistance = 5;
    controls.maxDistance = 140;

    scene.add(new THREE.AmbientLight(0xffffff, 1.1));
    const directional = new THREE.DirectionalLight(0xffffff, 1.2);
    directional.position.set(30, 40, 25);
    scene.add(directional);

    const raycaster = new THREE.Raycaster();
    const pointer = new THREE.Vector2();
    const meshes = new Map<string, THREE.Mesh>();
    const nodeById = new Map(graph.nodes.map((node) => [node.id, node]));

    const nodeMaterial = new THREE.MeshStandardMaterial({ color: "#22c55e" });
    const selectedMaterial = new THREE.MeshStandardMaterial({ color: "#f59e0b" });
    const edgeMaterial = new THREE.LineBasicMaterial({ color: "#64748b" });

    const radius = Math.max(10, graph.nodes.length * 1.5);
    graph.nodes.forEach((node, index) => {
      const angle = (index / Math.max(graph.nodes.length, 1)) * Math.PI * 2;
      const x = Math.cos(angle) * radius;
      const z = Math.sin(angle) * radius;
      const y = ((index % 5) - 2) * 1.7;
      const sphere = new THREE.Mesh(new THREE.SphereGeometry(0.75, 24, 24), nodeMaterial);
      sphere.position.set(x, y, z);
      sphere.userData = { nodeId: node.id };
      meshes.set(node.id, sphere);
      scene.add(sphere);
    });

    graph.edges.forEach((edge) => {
      const source = meshes.get(edge.source);
      const target = meshes.get(edge.target);
      if (!source || !target) return;
      const points = [source.position.clone(), target.position.clone()];
      const geometry = new THREE.BufferGeometry().setFromPoints(points);
      const line = new THREE.Line(geometry, edgeMaterial);
      scene.add(line);
    });

    function onPointerDown(event: MouseEvent) {
      const bounds = renderer.domElement.getBoundingClientRect();
      pointer.x = ((event.clientX - bounds.left) / bounds.width) * 2 - 1;
      pointer.y = -((event.clientY - bounds.top) / bounds.height) * 2 + 1;
      raycaster.setFromCamera(pointer, camera);
      const intersects = raycaster.intersectObjects(Array.from(meshes.values()));
      if (!intersects[0]) {
        setSelectedNode(null);
        return;
      }
      const nodeId = String(intersects[0].object.userData?.nodeId || "");
      const clicked = nodeById.get(nodeId) ?? null;
      setSelectedNode(clicked);
    }

    renderer.domElement.addEventListener("pointerdown", onPointerDown);

    function onResize() {
      const nextContainer = mountRef.current;
      if (!nextContainer) return;
      const nextWidth = nextContainer.clientWidth || 960;
      camera.aspect = nextWidth / height;
      camera.updateProjectionMatrix();
      renderer.setSize(nextWidth, height);
    }
    window.addEventListener("resize", onResize);

    renderer.setAnimationLoop(() => {
      controls.update();
      meshes.forEach((mesh, nodeId) => {
        const active = selectedNode?.id === nodeId;
        mesh.material = active ? selectedMaterial : nodeMaterial;
      });
      renderer.render(scene, camera);
    });

    return () => {
      renderer.setAnimationLoop(null);
      window.removeEventListener("resize", onResize);
      renderer.domElement.removeEventListener("pointerdown", onPointerDown);
      controls.dispose();
      nodeMaterial.dispose();
      selectedMaterial.dispose();
      edgeMaterial.dispose();
      renderer.dispose();
      container.innerHTML = "";
      rendererRef.current = null;
    };
  }, [graph, height, selectedNode?.id]);

  const onExport = () => {
    const blob = new Blob([exportAsJson()], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `contract-graph-${contractId}.json`;
    anchor.click();
    URL.revokeObjectURL(url);
  };

  const startAr = async () => {
    const xr = (navigator as Navigator & { xr?: XRSystem }).xr;
    if (!rendererRef.current || !xr) return;
    const session = await xr.requestSession("immersive-ar", {
      requiredFeatures: ["local"],
    });
    await rendererRef.current.xr.setSession(session);
  };

  return (
    <section className="w-full rounded-lg border border-slate-700 bg-slate-900/60 p-3">
      <div className="mb-3 flex items-center justify-between">
        <h3 className="text-base font-semibold text-slate-100">Contract Graph 3D</h3>
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={onExport}
            className="rounded-md border border-slate-500 px-3 py-1 text-xs text-slate-100 hover:bg-slate-700"
          >
            Export JSON
          </button>
          {arSupported ? (
            <button
              type="button"
              onClick={startAr}
              className="rounded-md bg-emerald-600 px-3 py-1 text-xs text-white hover:bg-emerald-500"
            >
              Start AR
            </button>
          ) : null}
        </div>
      </div>

      {isLoading ? (
        <p className="text-sm text-slate-300">Loading graph...</p>
      ) : null}
      {error ? <p className="text-sm text-rose-300">{error}</p> : null}
      {!isLoading && !error ? <div ref={mountRef} style={{ height }} /> : null}
      {selectedNode ? (
        <p className="mt-3 text-xs text-slate-200">
          Selected: {selectedNode.name} ({selectedNode.contract_id})
        </p>
      ) : null}
    </section>
  );
}
