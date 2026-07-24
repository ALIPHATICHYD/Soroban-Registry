import CompareContracts from "./CompareContracts";
import Navbar from "@/components/Navbar";

export const dynamic = "force-dynamic";

export default function ComparePage() {
  return (
    <div className="min-h-screen bg-background text-foreground">
      <Navbar />
      <CompareContracts />
    </div>
  );
}
