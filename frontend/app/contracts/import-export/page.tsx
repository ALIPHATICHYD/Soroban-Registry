import Navbar from "@/components/Navbar";
import ContractImportExportPanel from "@/components/contracts/ContractImportExportPanel";

export const dynamic = "force-dynamic";

export default function ContractImportExportPage() {
  return (
    <div className="min-h-screen bg-background text-foreground">
      <Navbar />
      <ContractImportExportPanel />
    </div>
  );
}
