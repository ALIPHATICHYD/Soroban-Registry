"use client";

import { Suspense, useState, useMemo } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useParams, useSearchParams, useRouter } from "next/navigation";
import { api } from "@/lib/api";
import Navbar from "@/components/Navbar";
import ReviewTimeline from "@/components/reviews/ReviewTimeline";
import { AnnotationWrapper } from "@/components/reviews/AnnotationLayer";
import {
  ArrowLeft,
  MessageSquare,
  Send,
  CheckCircle2,
  AlertCircle,
  FileCode,
  Database,
} from "lucide-react";
import Link from "next/link";
import type { CollaborativeComment } from "@/types";

function ReviewContent() {
  const params = useParams();
  const searchParams = useSearchParams();
  const router = useRouter();
  const queryClient = useQueryClient();

  const contractId = params?.id as string;
  const reviewId = searchParams?.get("reviewId");

  const [activeView, setActiveView] = useState<"source" | "abi">("source");
  const [commentText, setCommentText] = useState("");
  const [selectedLocation, setSelectedLocation] = useState<{
    line?: number;
    abi_path?: string;
  } | null>(null);

  const { data: contract } = useQuery({
    queryKey: ["contract", contractId],
    queryFn: () => api.getContract(contractId),
  });

  const { data: versions = [] } = useQuery({
    queryKey: ["contract-versions", contractId],
    queryFn: () => api.getContractVersions(contractId),
  });

  const latestVersion = versions[0];

  const { data: sourceCode } = useQuery({
    queryKey: ["contract-source", contractId, latestVersion?.source_url],
    queryFn: async () => {
      if (!latestVersion?.source_url) return "";
      const res = await fetch(latestVersion.source_url);
      return res.text();
    },
    enabled: !!latestVersion?.source_url && activeView === "source",
  });

  const { data: abiResponse } = useQuery({
    queryKey: ["contract-abi", contractId, latestVersion?.version],
    queryFn: () => api.getContractAbi(contractId, latestVersion?.version),
    enabled: !!latestVersion && activeView === "abi",
  });

  const { data: reviewDetails } = useQuery({
    queryKey: ["collaborative-review", reviewId],
    queryFn: () => api.getCollaborativeReview(reviewId!),
    enabled: !!reviewId,
  });

  const addCommentMutation = useMutation({
    mutationFn: (
      data: Pick<CollaborativeComment, "content" | "line_number" | "abi_path">,
    ) => api.addCollaborativeComment(reviewId!, data),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ["collaborative-review", reviewId],
      });
      setCommentText("");
      setSelectedLocation(null);
    },
  });

  const updateStatusMutation = useMutation({
    mutationFn: (status: string) => api.updateReviewerStatus(reviewId!, status),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ["collaborative-review", reviewId],
      });
    },
  });

  const handleAddComment = () => {
    if (!commentText.trim()) return;
    addCommentMutation.mutate({
      content: commentText,
      line_number: selectedLocation?.line,
      abi_path: selectedLocation?.abi_path,
    });
  };

  const annotationsMap = useMemo(() => {
    const map: Record<string, boolean> = {};
    reviewDetails?.comments.forEach((c: CollaborativeComment) => {
      if (c.line_number) map[`line-${c.line_number}`] = true;
      if (c.abi_path) map[`abi-${c.abi_path}`] = true;
    });
    return map;
  }, [reviewDetails]);

  const startReviewMutation = useMutation({
    mutationFn: (reviewerIds: string[]) =>
      api.createCollaborativeReview({
        contract_id: contractId,
        version: latestVersion?.version || "1.0.0",
        reviewer_ids: reviewerIds,
      }),
    onSuccess: (review) => {
      router.push(`/contracts/${contractId}/review?reviewId=${review.id}`);
      queryClient.invalidateQueries({
        queryKey: ["collaborative-review", review.id],
      });
    },
  });

  if (!contract) return null;

  if (!reviewId) {
    return (
      <div className="max-w-4xl mx-auto px-4 py-20 text-center space-y-8 animate-in zoom-in-95 duration-500">
        <div className="w-24 h-24 bg-primary/10 rounded-full flex items-center justify-center mx-auto shadow-2xl shadow-primary/20">
          <MessageSquare className="w-12 h-12 text-primary" />
        </div>
        <div className="space-y-4">
          <h2 className="text-4xl font-black mb-2 tracking-tight">
            Start Review Session
          </h2>
          <p className="text-muted-foreground text-lg max-w-md mx-auto">
            Invite your team to review the code and ABI for{" "}
            <span className="text-foreground font-bold">
              v{latestVersion?.version || "1.0.0"}
            </span>
            . Track inline comments, discussion threads, and overall progress.
          </p>
        </div>
        <button
          onClick={() => startReviewMutation.mutate([contract.publisher_id])}
          disabled={startReviewMutation.isPending}
          className="inline-flex items-center gap-3 px-10 py-5 rounded-2xl bg-primary text-primary-foreground font-black text-lg hover:opacity-90 transition-all shadow-2xl shadow-primary/40 active:scale-95"
        >
          {startReviewMutation.isPending ? (
            <div className="w-6 h-6 border-2 border-white/30 border-t-white rounded-full animate-spin" />
          ) : (
            <MessageSquare className="w-6 h-6" />
          )}
          {startReviewMutation.isPending
            ? "Starting..."
            : "Begin Collaborative Review"}
        </button>
      </div>
    );
  }

  return (
    <div className="max-w-7xl mx-auto px-4 py-8 animate-in fade-in duration-500">
      <div className="flex items-center justify-between mb-8">
        <Link
          href={`/contracts/${contractId}`}
          className="inline-flex items-center gap-2 text-muted-foreground hover:text-foreground transition-colors"
        >
          <ArrowLeft className="w-4 h-4" /> Back to details
        </Link>
        <div className="flex gap-2">
          <button
            onClick={() => updateStatusMutation.mutate("approved")}
            className="inline-flex items-center gap-2 px-4 py-2 rounded-xl bg-green-500 text-white font-semibold hover:bg-green-600 transition-colors shadow-lg shadow-green-500/20"
          >
            <CheckCircle2 className="w-4 h-4" /> Approve
          </button>
          <button
            onClick={() => updateStatusMutation.mutate("changes_requested")}
            className="inline-flex items-center gap-2 px-4 py-2 rounded-xl bg-red-500 text-white font-semibold hover:bg-red-600 transition-colors shadow-lg shadow-red-500/20"
          >
            <AlertCircle className="w-4 h-4" /> Request Changes
          </button>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-4 gap-8">
        <div className="lg:col-span-3 space-y-6">
          <div className="flex items-center gap-2 p-1 bg-accent rounded-2xl w-fit mb-4">
            <button
              onClick={() => setActiveView("source")}
              className={`flex items-center gap-2 px-6 py-2.5 rounded-xl text-sm font-bold transition-all ${
                activeView === "source"
                  ? "bg-card shadow-xl text-primary scale-105"
                  : "text-muted-foreground hover:text-foreground"
              }`}
            >
              <FileCode className="w-4 h-4" /> Source
            </button>
            <button
              onClick={() => setActiveView("abi")}
              className={`flex items-center gap-2 px-6 py-2.5 rounded-xl text-sm font-bold transition-all ${
                activeView === "abi"
                  ? "bg-card shadow-xl text-primary scale-105"
                  : "text-muted-foreground hover:text-foreground"
              }`}
            >
              <Database className="w-4 h-4" /> ABI
            </button>
          </div>

          <div className="bg-card border border-border rounded-3xl overflow-hidden shadow-2xl shadow-primary/5">
            <div className="p-6 overflow-x-auto">
              {activeView === "source" && sourceCode && (
                <div className="font-mono text-sm leading-relaxed whitespace-pre">
                  {sourceCode.split("\n").map((line, idx) => (
                    <AnnotationWrapper
                      key={idx}
                      line={idx + 1}
                      onAnnotate={setSelectedLocation}
                      hasAnnotation={annotationsMap[`line-${idx + 1}`]}
                    >
                      <div className="flex gap-4">
                        <span className="w-8 text-right text-muted-foreground/50 select-none">
                          {idx + 1}
                        </span>
                        <span>{line || " "}</span>
                      </div>
                    </AnnotationWrapper>
                  ))}
                </div>
              )}
              {activeView === "abi" && abiResponse && (
                <div className="font-mono text-sm leading-relaxed">
                  {JSON.stringify(abiResponse.abi, null, 2)
                    .split("\n")
                    .map((line, idx) => (
                      <AnnotationWrapper
                        key={idx}
                        abiPath={`node-${idx}`} // Simplistic ABI path
                        onAnnotate={setSelectedLocation}
                        hasAnnotation={annotationsMap[`abi-node-${idx}`]}
                      >
                        <div className="flex gap-4">
                          <span className="w-8 text-right text-muted-foreground/50 select-none">
                            {idx + 1}
                          </span>
                          <span>{line || " "}</span>
                        </div>
                      </AnnotationWrapper>
                    ))}
                </div>
              )}
            </div>
          </div>
        </div>

        <div className="space-y-6">
          <div className="bg-card border border-border rounded-3xl p-6 shadow-xl space-y-4">
            <h3 className="text-lg font-bold flex items-center gap-2">
              <MessageSquare className="w-5 h-5 text-primary" />
              {selectedLocation
                ? selectedLocation.line
                  ? `Comment on Line ${selectedLocation.line}`
                  : `Comment on ABI Node`
                : "General Comment"}
            </h3>
            <textarea
              value={commentText}
              onChange={(e) => setCommentText(e.target.value)}
              placeholder="Type your comment here..."
              className="w-full h-32 bg-accent/50 border-border border rounded-2xl p-4 text-sm focus:ring-2 focus:ring-primary/20 focus:border-primary transition-all resize-none"
            />
            <button
              onClick={handleAddComment}
              disabled={!commentText.trim() || addCommentMutation.isPending}
              className="w-full flex items-center justify-center gap-2 py-3 rounded-2xl bg-primary text-primary-foreground font-bold hover:opacity-90 disabled:opacity-50 transition-all shadow-lg shadow-primary/20"
            >
              <Send className="w-4 h-4" />
              {addCommentMutation.isPending ? "Sending..." : "Post Comment"}
            </button>
            {selectedLocation && (
              <button
                onClick={() => setSelectedLocation(null)}
                className="w-full text-xs text-muted-foreground hover:text-primary transition-colors"
              >
                Cancel inline comment
              </button>
            )}
          </div>

          {reviewId && <ReviewTimeline reviewId={reviewId} />}
        </div>
      </div>
    </div>
  );
}

export default function ReviewPage() {
  return (
    <div className="min-h-screen bg-background text-foreground font-sans selection:bg-primary/20">
      <Navbar />
      <Suspense
        fallback={
          <div className="flex items-center justify-center h-[60vh]">
            <div className="w-12 h-12 border-4 border-primary border-t-transparent rounded-full animate-spin" />
          </div>
        }
      >
        <ReviewContent />
      </Suspense>
    </div>
  );
}
