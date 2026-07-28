"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { FormProvider, useForm } from "react-hook-form";
import type {
  VerificationDraft,
  VerificationDocument,
  VerificationRequest,
  VerificationStatus,
  VerificationStepKey,
} from "@/types/verification";
import {
  MAX_TOTAL_UPLOAD_BYTES,
  validateFilesToAdd,
} from "@/utils/fileValidation";
import {
  getVerificationStatus,
  simulateStatusProgression,
  subscribeToVerificationStatusChanges,
  submitVerification,
} from "@/services/mockVerificationService";

const DRAFT_STORAGE_KEY = "sr_verification_draft_v1";

const STEP_ORDER: VerificationStepKey[] = [
  "contractInfo",
  "description",
  "securityClaims",
  "documents",
  "review",
];

const STEP_FIELDS: Record<
  Exclude<VerificationStepKey, "documents" | "review">,
  Array<keyof VerificationDraft>
> = {
  contractInfo: ["contractName", "contractAddress", "network"],
  description: ["purpose", "useCase"],
  securityClaims: ["auditStatus", "knownVulnerabilities", "riskLevel"],
};

function safeParse<T>(raw: string | null): T | null {
  if (!raw) return null;
  try {
    return JSON.parse(raw) as T;
  } catch {
    return null;
  }
}

function fileKey(file: File): string {
  return `${file.name}::${file.size}::${file.lastModified}`;
}

function createDocId(): string {
  return `doc_${Math.random().toString(36).slice(2, 10)}_${Date.now().toString(36)}`;
}

export type UseVerificationFlowResult = {
  stepIndex: number;
  stepKey: VerificationStepKey;
  steps: VerificationStepKey[];
  isFirstStep: boolean;
  isLastStep: boolean;

  form: ReturnType<typeof useForm<VerificationDraft>>;
  FormProvider: typeof FormProvider;

  files: File[];
  uploadProgress: Record<string, number>;
  fileErrors: string[];
  addFiles: (files: File[]) => void;
  removeFile: (key: string) => void;
  totalUploadBytes: number;
  maxUploadBytes: number;

  status: VerificationStatus;
  submission: VerificationRequest | null;
  isSubmitting: boolean;

  goNext: () => Promise<boolean>;
  goBack: () => void;
  goToStep: (nextIndex: number) => void;
  submit: () => Promise<VerificationRequest>;
};

export function useVerificationFlow(): UseVerificationFlowResult {
  const defaultValues = useMemo<VerificationDraft>(() => {
    const base: VerificationDraft = {
      contractName: "",
      contractAddress: "",
      network: "testnet",
      purpose: "",
      useCase: "",
      auditStatus: "not_audited",
      knownVulnerabilities: "",
      riskLevel: "medium",
    };

    if (typeof window === "undefined") return base;
    const stored = safeParse<Partial<VerificationDraft>>(
      window.localStorage.getItem(DRAFT_STORAGE_KEY),
    );
    return { ...base, ...(stored || {}) };
  }, []);

  const form = useForm<VerificationDraft>({
    defaultValues,
    mode: "onBlur",
    shouldUnregister: false,
  });

  const [stepIndex, setStepIndex] = useState(0);
  const stepKey = STEP_ORDER[stepIndex] ?? "contractInfo";

  const [files, setFiles] = useState<File[]>([]);
  const [uploadProgress, setUploadProgress] = useState<Record<string, number>>(
    {},
  );
  const [fileErrors, setFileErrors] = useState<string[]>([]);

  const [isSubmitting, setIsSubmitting] = useState(false);
  const [submission, setSubmission] = useState<VerificationRequest | null>(
    null,
  );
  const [status, setStatus] = useState<VerificationStatus>("draft");

  const uploadIntervalsRef = useRef<Record<string, number>>({});
  const draftSaveTimeoutRef = useRef<number | null>(null);

  const totalUploadBytes = useMemo(
    () => files.reduce((sum, f) => sum + f.size, 0),
    [files],
  );

  useEffect(() => {
    if (typeof window === "undefined") return;
    // Persist draft values between steps and refreshes for a guided wizard UX.
    const subscription = form.watch((values) => {
      if (draftSaveTimeoutRef.current)
        window.clearTimeout(draftSaveTimeoutRef.current);
      draftSaveTimeoutRef.current = window.setTimeout(() => {
        window.localStorage.setItem(DRAFT_STORAGE_KEY, JSON.stringify(values));
      }, 250);
    });
    return () => subscription.unsubscribe();
  }, [form]);

  useEffect(() => {
    const intervals = uploadIntervalsRef.current;
    return () => {
      for (const key of Object.keys(intervals)) {
        window.clearInterval(intervals[key]);
      }
    };
  }, []);

  const startSimulatedUpload = useCallback((key: string) => {
    if (uploadIntervalsRef.current[key]) return;

    // Simulated upload progress (frontend-only). Backend integration would replace this with real upload state.
    uploadIntervalsRef.current[key] = window.setInterval(() => {
      setUploadProgress((prev) => {
        const current = prev[key] ?? 0;
        if (current >= 100) {
          window.clearInterval(uploadIntervalsRef.current[key]);
          delete uploadIntervalsRef.current[key];
          return prev;
        }
        const next = Math.min(100, current + Math.ceil(8 + Math.random() * 12));
        return { ...prev, [key]: next };
      });
    }, 250);
  }, []);

  const addFiles = useCallback(
    (incoming: File[]) => {
      const { accepted, errors } = validateFilesToAdd({
        existingFiles: files,
        newFiles: incoming,
        maxTotalBytes: MAX_TOTAL_UPLOAD_BYTES,
      });

      setFileErrors(errors.map((e) => e.message));

      if (accepted.length === 0) return;
      setFiles((prev) => [...prev, ...accepted]);
      setUploadProgress((prev) => {
        const next = { ...prev };
        for (const f of accepted) {
          const k = fileKey(f);
          if (next[k] == null) next[k] = 0;
        }
        return next;
      });
      for (const f of accepted) startSimulatedUpload(fileKey(f));
    },
    [files, startSimulatedUpload],
  );

  const removeFile = useCallback((key: string) => {
    setFiles((prev) => prev.filter((f) => fileKey(f) !== key));
    setUploadProgress((prev) => {
      const next = { ...prev };
      delete next[key];
      return next;
    });
    if (uploadIntervalsRef.current[key]) {
      window.clearInterval(uploadIntervalsRef.current[key]);
      delete uploadIntervalsRef.current[key];
    }
  }, []);

  const validateCurrentStep = useCallback(async (): Promise<boolean> => {
    if (stepKey === "documents") {
      // Files live outside RHF to keep the form payload JSON-serializable and backend-ready.
      const ok = files.length > 0;
      setFileErrors(
        ok ? [] : ["Please upload at least one document to proceed."],
      );
      return ok;
    }
    if (stepKey === "review") return true;

    const fields = STEP_FIELDS[stepKey as keyof typeof STEP_FIELDS];
    return form.trigger(fields);
  }, [files.length, form, stepKey]);

  const goNext = useCallback(async (): Promise<boolean> => {
    const ok = await validateCurrentStep();
    if (!ok) return false;
    setStepIndex((i) => Math.min(i + 1, STEP_ORDER.length - 1));
    return true;
  }, [validateCurrentStep]);

  const goBack = useCallback(() => {
    setStepIndex((i) => Math.max(0, i - 1));
  }, []);

  const goToStep = useCallback((nextIndex: number) => {
    setStepIndex((i) => {
      if (nextIndex < 0 || nextIndex >= STEP_ORDER.length) return i;
      return nextIndex;
    });
  }, []);

  const submit = useCallback(async (): Promise<VerificationRequest> => {
    const ok = await validateCurrentStep();
    if (!ok) throw new Error("Please fix validation errors before submitting.");

    const values = form.getValues();
    setIsSubmitting(true);
    try {
      const documents: VerificationDocument[] = files.map((f) => ({
        id: createDocId(),
        name: f.name,
        sizeBytes: f.size,
        mimeType: f.type || "application/octet-stream",
        uploadedAt: new Date().toISOString(),
      }));

      const request = await submitVerification({
        submission: { ...values, documents },
      });
      setSubmission(request);
      setStatus(request.status);

      if (typeof window !== "undefined")
        window.localStorage.removeItem(DRAFT_STORAGE_KEY);

      return request;
    } finally {
      setIsSubmitting(false);
    }
  }, [files, form, validateCurrentStep]);

  useEffect(() => {
    if (!submission?.id) return;

    let unsubscribeTimers: (() => void) | null = null;
    const unsubscribeEvents = subscribeToVerificationStatusChanges(
      async (evt) => {
        if (evt.id !== submission.id) return;
        try {
          const res = await getVerificationStatus({ id: evt.id });
          setSubmission(res.request);
          setStatus(res.request.status);
        } catch {
          return;
        }
      },
    );

    getVerificationStatus({ id: submission.id })
      .then((res) => {
        setSubmission(res.request);
        setStatus(res.request.status);
        unsubscribeTimers = simulateStatusProgression({ id: submission.id });
      })
      .catch(() => undefined);

    return () => {
      unsubscribeEvents();
      unsubscribeTimers?.();
    };
  }, [submission?.id]);

  return {
    stepIndex,
    stepKey,
    steps: STEP_ORDER,
    isFirstStep: stepIndex === 0,
    isLastStep: stepIndex === STEP_ORDER.length - 1,

    form,
    FormProvider,

    files,
    uploadProgress,
    fileErrors,
    addFiles,
    removeFile,
    totalUploadBytes,
    maxUploadBytes: MAX_TOTAL_UPLOAD_BYTES,

    status,
    submission,
    isSubmitting,

    goNext,
    goBack,
    goToStep,
    submit,
  };
}
