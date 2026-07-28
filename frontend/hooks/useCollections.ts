"use client";

import { useCallback } from "react";
import { useAppDispatch, useAppSelector } from "@/store/hooks";
import {
  createCollection,
  deleteCollection,
  renameCollection,
  addToCollection,
  removeFromCollection,
} from "@/store/slices/collectionsSlice";

export function useCollections() {
  const dispatch = useAppDispatch();
  const collections = useAppSelector((s) => s.collections.collections);

  return {
    collections,
    createCollection: useCallback(
      (name: string) => dispatch(createCollection(name)),
      [dispatch],
    ),
    deleteCollection: useCallback(
      (id: string) => dispatch(deleteCollection(id)),
      [dispatch],
    ),
    renameCollection: useCallback(
      (id: string, name: string) => dispatch(renameCollection({ id, name })),
      [dispatch],
    ),
    addToCollection: useCallback(
      (collectionId: string, contractId: string) =>
        dispatch(addToCollection({ collectionId, contractId })),
      [dispatch],
    ),
    removeFromCollection: useCallback(
      (collectionId: string, contractId: string) =>
        dispatch(removeFromCollection({ collectionId, contractId })),
      [dispatch],
    ),
    getCollection: useCallback(
      (id: string) => collections.find((c) => c.id === id),
      [collections],
    ),
  };
}
