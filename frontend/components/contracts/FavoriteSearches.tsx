'use client';

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { api, QueryNode } from '@/lib/api';
import { Star, Trash2, Clock, Play, Loader2, Bookmark } from 'lucide-react';

interface FavoriteSearchesProps {
  onLoad: (query: QueryNode) => void;
  className?: string;
}

export default function FavoriteSearches({ onLoad, className = '' }: FavoriteSearchesProps) {
  const queryClient = useQueryClient();

  const { data: favorites, isLoading } = useQuery({
    queryKey: ['favorite-searches'],
    queryFn: () => api.listFavoriteSearches(),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => api.deleteFavoriteSearch(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['favorite-searches'] });
    },
  });

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-10">
        <Loader2 className="w-6 h-6 animate-spin text-primary" />
      </div>
    );
  }

  if (!favorites || favorites.length === 0) {
    return (
      <div className="text-center py-10 px-4 bg-muted/20 border border-dashed border-border rounded-xl">
        <Bookmark className="w-8 h-8 text-muted-foreground mx-auto mb-3 opacity-20" />
        <p className="text-sm text-muted-foreground">No favorite searches saved yet.</p>
        <p className="text-xs text-muted-foreground mt-1">Save a query from the builder to see it here.</p>
      </div>
    );
  }

  return (
    <div className={`space-y-3 ${className}`}>
      <div className="flex items-center gap-2 mb-4">
        <Star className="w-4 h-4 text-yellow-500 fill-yellow-500" />
        <h3 className="font-semibold text-sm uppercase tracking-wider text-foreground">Favorite Searches</h3>
      </div>
      
      {favorites.map((fav) => (
        <div 
          key={fav.id}
          className="group relative bg-card border border-border rounded-xl p-4 hover:border-primary/40 transition-all hover:shadow-md"
        >
          <div className="flex items-start justify-between mb-2">
            <div>
              <h4 className="font-bold text-foreground text-sm group-hover:text-primary transition-colors">{fav.name}</h4>
              <div className="flex items-center gap-1.5 mt-1 text-[10px] text-muted-foreground uppercase font-semibold">
                <Clock className="w-3 h-3" />
                {new Date(fav.created_at).toLocaleDateString()}
              </div>
            </div>
            <button 
              onClick={() => deleteMutation.mutate(fav.id)}
              className="p-1.5 text-muted-foreground hover:text-red-500 hover:bg-red-500/10 rounded-md transition-colors"
              title="Delete Favorite"
            >
              <Trash2 className="w-3.5 h-3.5" />
            </button>
          </div>

          <div className="mt-4 flex items-center justify-between">
            <button 
              onClick={() => onLoad(fav.query_json)}
              className="flex-1 flex items-center justify-center gap-2 py-2 bg-primary/10 text-primary hover:bg-primary hover:text-primary-foreground rounded-lg transition-all font-bold text-xs"
            >
              <Play className="w-3 h-3" />
              Load Search
            </button>
          </div>
        </div>
      ))}
    </div>
  );
}
