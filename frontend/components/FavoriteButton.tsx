"use client";

import { Star } from "lucide-react";
import { useFavorites } from "@/hooks/useFavorites";

interface FavoriteButtonProps {
  contractId: string;
  size?: "sm" | "md";
  className?: string;
}

export default function FavoriteButton({
  contractId,
  size = "sm",
  className,
}: FavoriteButtonProps) {
  const { isFavorited, toggleFavorite } = useFavorites();
  const favorited = isFavorited(contractId);

  const handleClick = (event: React.MouseEvent<HTMLButtonElement>) => {
    event.preventDefault();
    event.stopPropagation();
    toggleFavorite(contractId);
  };

  const iconSize = size === "md" ? "h-4 w-4" : "h-3.5 w-3.5";
  const padding = size === "md" ? "px-3 py-1.5 text-sm" : "px-2.5 py-1 text-xs";

  return (
    <button
      type="button"
      onClick={handleClick}
      aria-label={favorited ? "Remove from favorites" : "Add to favorites"}
      className={[
        "inline-flex items-center gap-1 rounded-md border font-medium transition-colors duration-150",
        padding,
        favorited
          ? "border-yellow-500/30 bg-yellow-500/10 text-yellow-500 hover:bg-yellow-500/20"
          : "border-border bg-card text-foreground hover:bg-accent",
        className,
      ]
        .filter(Boolean)
        .join(" ")}
    >
      <Star
        className={iconSize}
        fill={favorited ? "currentColor" : "none"}
        strokeWidth={favorited ? 0 : 1.5}
      />
      <span>{favorited ? "Saved" : "Save"}</span>
    </button>
  );
}
