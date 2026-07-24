"use client";

import React, { useMemo } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import Navbar from "@/components/Navbar";
import Stepper from "@/components/verification/Stepper";
import ContractInfoStep from "@/components/verification/ContractInfoStep";
import DescriptionStep from "@/components/verification/DescriptionStep";
import SecurityClaimsStep from "@/components/verification/SecurityClaimsStep";
import DocumentUploadStep from "@/components/verification/DocumentUploadStep";
import VerificationSummary from "@/components/verification/VerificationSummary";
import { useToast } from "@/hooks/useToast";
import { useVerificationFlow } from "@/hooks/useVerificationFlow";
import type { VerificationDocument } from "@/types/verification";

export default function VerifyContractPage() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const contractIdParam = searchParams?.get("id") || "";
  const { showError, showSuccess, showInfo } = useToast();

  const flow = useVerificationFlow();
  const {
    stepIndex,
    stepKey,
    steps,
    isFirstStep,
    isLastStep,
    form,
    FormProvider,
    files,
    uploadProgress,
    fileErrors,
    addFiles,
    removeFile,
    totalUploadBytes,
    maxUploadBytes,
    isSubmitting,
    goBack,
    goNext,
    goToStep,
    submit,
  } = flow;

  const reviewDocs = useMemo<VerificationDocument[]>(
    () =>
      files.map((f) => ({
        id: `${f.name}::${f.size}::${f.lastModified}`,
        name: f.name,
        sizeBytes: f.size,
        mimeType: f.type || "application/octet-stream",
        uploadedAt: new Date().toISOString(),
      })),
    [files],
  );

  React.useEffect(() => {
    if (contractIdParam && !form.getValues("contractAddress")) {
      form.setValue("contractAddress", contractIdParam);
    }
  }, [contractIdParam, form]);

  return (
    <div className="flex flex-col min-h-screen bg-background">
      <Navbar />
      <div className="max-w-4xl mx-auto py-8 px-4 sm:px-6 lg:px-8 w-full flex-grow">
        <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <h1 className="text-2xl sm:text-3xl font-bold text-foreground">
              Verify Contract
            </h1>
            <p className="text-sm text-muted-foreground mt-1">
              Submit a contract for verification and upload supporting documents.
            </p>
          </div>
        </div>

        <div className="mt-6">
          <Stepper
            steps={steps}
            activeIndex={stepIndex}
            onStepClick={goToStep}
          />
        </div>

        <FormProvider {...form}>
          <form
            onSubmit={(e) => {
              e.preventDefault();
            }}
            className="mt-6 space-y-4 bg-card p-4 sm:p-6 rounded-2xl border border-border"
          >
            {stepKey === "contractInfo" && <ContractInfoStep />}
            {stepKey === "description" && <DescriptionStep />}
            {stepKey === "securityClaims" && <SecurityClaimsStep />}
            {stepKey === "documents" && (
              <DocumentUploadStep
                files={files}
                progress={uploadProgress}
                errors={fileErrors}
                totalBytes={totalUploadBytes}
                maxBytes={maxUploadBytes}
                onAddFiles={addFiles}
                onRemoveFile={removeFile}
              />
            )}
            {stepKey === "review" && (
              <VerificationSummary
                draft={form.getValues()}
                documents={reviewDocs}
                status="submitted"
              />
            )}

            <div className="flex flex-col-reverse sm:flex-row sm:items-center sm:justify-between gap-2 pt-2 border-t border-border">
              <button
                type="button"
                onClick={goBack}
                disabled={isFirstStep || isSubmitting}
                className="w-full sm:w-auto px-5 py-2.5 rounded-lg border border-border bg-background text-foreground font-medium disabled:opacity-50 disabled:cursor-not-allowed"
              >
                Back
              </button>

              <div className="flex flex-col sm:flex-row gap-2 w-full sm:w-auto">
                {!isLastStep ? (
                  <button
                    type="button"
                    onClick={async () => {
                      const ok = await goNext();
                      if (!ok)
                        showError("Please fix validation errors to continue.");
                    }}
                    disabled={isSubmitting}
                    className="w-full sm:w-auto px-6 py-2.5 rounded-lg btn-glow text-primary-foreground font-medium"
                  >
                    Continue
                  </button>
                ) : (
                  <button
                    type="button"
                    onClick={async () => {
                      try {
                        showInfo("Submitting verification…");
                        const request = await submit();
                        showSuccess("Verification submitted.");
                        router.push(
                          `/verification-status?id=${encodeURIComponent(
                            request.id,
                          )}`,
                        );
                      } catch (err: unknown) {
                        showError(
                          err instanceof Error
                            ? err.message
                            : "Failed to submit verification",
                        );
                      }
                    }}
                    disabled={isSubmitting}
                    className="w-full sm:w-auto px-6 py-2.5 rounded-lg bg-primary text-primary-foreground font-semibold btn-glow disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    {isSubmitting ? "Submitting…" : "Submit for Verification"}
                  </button>
                )}
              </div>
            </div>
          </form>
        </FormProvider>
      </div>
    </div>
  );
}