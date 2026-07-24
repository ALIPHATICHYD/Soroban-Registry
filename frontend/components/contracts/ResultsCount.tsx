interface ResultsCountProps {
  visibleCount: number;
  totalCount: number;
}

export function ResultsCount({ visibleCount, totalCount }: ResultsCountProps) {
  return (
    <div className="text-sm text-muted-foreground">
      Showing{" "}
      <span className="font-medium text-foreground">{visibleCount}</span> of{" "}
      <span className="font-medium text-foreground">{totalCount}</span>{" "}
      contracts
    </div>
  );
}
