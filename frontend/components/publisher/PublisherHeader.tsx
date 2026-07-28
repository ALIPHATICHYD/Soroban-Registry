import React from "react";
import Image from "next/image";
import { PublisherResponse } from "@/types/publisher";
import {
  Calendar,
  ExternalLink,
  Github,
  Globe,
  CheckCircle,
} from "lucide-react";

interface PublisherHeaderProps {
  publisher: PublisherResponse;
}

export function PublisherHeader({ publisher }: PublisherHeaderProps) {
  const formattedDate = new Date(publisher.createdAt).toLocaleDateString(
    "en-US",
    {
      year: "numeric",
      month: "long",
      day: "numeric",
    },
  );

  return (
    <div className="bg-card rounded-2xl shadow-sm border border-border p-6 md:p-8">
      <div className="flex flex-col md:flex-row items-start md:items-center gap-6">
        {/* Avatar */}
        <div className="relative shrink-0">
          <Image
            src={
              publisher.avatarUrl ||
              `https://ui-avatars.com/api/?name=${encodeURIComponent(publisher.displayName)}`
            }
            alt={publisher.displayName}
            width={128}
            height={128}
            className="w-24 h-24 md:w-32 md:h-32 rounded-full border-4 border-border object-cover bg-accent"
            unoptimized={!publisher.avatarUrl}
          />
        </div>

        {/* Info */}
        <div className="flex-1 w-full">
          <div className="flex flex-col md:flex-row md:items-center justify-between gap-4 mb-2">
            <div>
              <h1 className="text-2xl md:text-3xl font-bold text-foreground flex items-center gap-2">
                {publisher.displayName}
                {publisher.verifiedContracts > 0 && (
                  <CheckCircle
                    className="w-6 h-6 text-primary"
                    aria-label="Verified Publisher"
                  />
                )}
              </h1>
              <p className="text-sm text-muted-foreground font-mono mt-1 break-all">
                {publisher.address}
              </p>
            </div>

            <button
              disabled
              className="px-4 py-2 bg-primary hover:opacity-90 disabled:bg-muted disabled:cursor-not-allowed text-primary-foreground rounded-lg font-medium transition-colors text-sm w-full md:w-auto"
              title="Coming soon"
            >
              Follow Publisher
              {/* TODO: Integrate with follow API in future */}
            </button>
          </div>

          {publisher.bio && (
            <p className="text-muted-foreground mb-4 max-w-2xl leading-relaxed">
              {publisher.bio}
            </p>
          )}

          <div className="flex flex-wrap items-center gap-4 text-sm text-muted-foreground">
            <div className="flex items-center gap-1.5">
              <Calendar className="w-4 h-4" />
              <span>Joined {formattedDate}</span>
            </div>

            {publisher.website && (
              <a
                href={publisher.website}
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center gap-1.5 hover:text-primary transition-colors"
                aria-label="Visit website"
              >
                <Globe className="w-4 h-4" />
                <span>Website</span>
                <ExternalLink className="w-3 h-3" />
              </a>
            )}

            {publisher.github && (
              <a
                href={publisher.github}
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center gap-1.5 hover:text-foreground transition-colors"
                aria-label="Visit GitHub profile"
              >
                <Github className="w-4 h-4" />
                <span>GitHub</span>
                <ExternalLink className="w-3 h-3" />
              </a>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
