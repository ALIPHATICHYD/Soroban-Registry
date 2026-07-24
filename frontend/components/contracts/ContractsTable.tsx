'use client';

import React, { useMemo, useState } from 'react';
import type { Contract } from '@/types';
import { useRouter } from 'next/navigation';
import {
  ColumnDef,
  Header,
  flexRender,
  getCoreRowModel,
  useReactTable,
  SortingState,
  VisibilityState,
  ColumnOrderState,
} from '@tanstack/react-table';
import {
  DndContext,
  KeyboardSensor,
  MouseSensor,
  TouchSensor,
  closestCenter,
  useSensor,
  useSensors,
  DragEndEvent,
} from '@dnd-kit/core';
import {
  arrayMove,
  SortableContext,
  horizontalListSortingStrategy,
  useSortable,
} from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { ArrowDown, ArrowUp, Columns, Tag, Download, CheckCircle2, ChevronDown } from 'lucide-react';

interface ContractsTableProps {
  data: Contract[];
  sortBy: string;
  sortOrder: 'asc' | 'desc';
  onSortChange: (sortBy: string, sortOrder: 'asc' | 'desc') => void;
}

// Drag & Drop wrapper for column headers
function SortableHeader({
  id,
  header,
  children,
}: {
  id: string;
  header: Header<Contract, unknown>;
  children: React.ReactNode;
}) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
    zIndex: isDragging ? 10 : 0,
    position: isDragging ? 'relative' : 'static',
  } as React.CSSProperties;

  return (
    <th
      ref={setNodeRef}
      style={style}
      colSpan={header.colSpan}
      className={`px-4 py-3 text-left text-xs font-semibold text-muted-foreground uppercase tracking-wider bg-card border-b border-border group ${header.column.getCanSort() ? 'hover:bg-accent/50 transition-colors cursor-pointer select-none' : ''}`}
      onClick={header.column.getToggleSortingHandler()}
    >
      <div className="flex items-center gap-1.5 break-keep whitespace-nowrap">
        <div
          {...attributes}
          {...listeners}
          className="cursor-grab hover:bg-accent p-1 rounded text-muted-foreground opacity-20 group-hover:opacity-100 transition-opacity"
          title="Drag to reorder"
          onClick={(e) => e.stopPropagation()}
        >
          {/* Grip Icon */}
          <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor"><path d="M4 3.5C4 4.32843 3.32843 5 2.5 5C1.67157 5 1 4.32843 1 3.5C1 2.67157 1.67157 2 2.5 2C3.32843 2 4 2.67157 4 3.5ZM9 3.5C9 4.32843 8.32843 5 7.5 5C6.67157 5 6 4.32843 6 3.5C6 2.67157 6.67157 2 7.5 2C8.32843 2 9 2.67157 9 3.5ZM14 3.5C14 4.32843 13.32843 5 12.5 5C11.67157 5 11 4.32843 11 3.5C11 2.67157 11.67157 2 12.5 2C13.32843 2 14 2.67157 14 3.5ZM4 8.5C4 9.32843 3.32843 10 2.5 10C1.67157 10 1 9.32843 1 8.5C1 7.67157 1.67157 7 2.5 7C3.32843 7 4 7.67157 4 8.5ZM9 8.5C9 9.32843 8.32843 10 7.5 10C6.67157 10 6 9.32843 6 8.5C6 7.67157 6.67157 7 7.5 7C8.32843 7 9 7.67157 9 8.5ZM14 8.5C14 9.32843 13.32843 10 12.5 10C11.67157 10 11 9.32843 11 8.5C11 7.67157 11.67157 7 12.5 7C13.32843 7 14 7.67157 14 8.5ZM4 13.5C4 14.3284 3.32843 15 2.5 15C1.67157 15 1 14.3284 1 13.5C1 12.6716 1.67157 12 2.5 12C3.32843 12 4 12.6716 4 13.5ZM9 13.5C9 14.3284 8.32843 15 7.5 15C6.67157 15 6 14.3284 6 13.5C6 12.6716 6.67157 12 7.5 12C8.32843 12 9 12.6716 9 13.5ZM14 13.5C14 14.3284 13.32843 15 12.5 15C11.67157 15 11 14.3284 11 13.5C11 12.6716 11.67157 12 12.5 12C13.32843 12 14 12.6716 14 13.5Z" /></svg>
        </div>
        <div className="flex-1">
          {children}
        </div>
        {/* Sort Icons */}
        <div className="flex flex-col opacity-50 ml-1">
          <ArrowUp className={`w-3 h-3 ${header.column.getIsSorted() === 'asc' ? 'opacity-100 text-primary' : 'opacity-30'}`} />
          <ArrowDown className={`w-3 h-3 -mt-1 ${header.column.getIsSorted() === 'desc' ? 'opacity-100 text-primary' : 'opacity-30'}`} />
        </div>
      </div>
    </th>
  );
}

export function ContractsTable({ data, sortBy, sortOrder, onSortChange }: ContractsTableProps) {
  const router = useRouter();
  const [columnVisibility, setColumnVisibility] = useState<VisibilityState>({});
  const [columnOrder, setColumnOrder] = useState<ColumnOrderState>([]);
  const [colMenuOpen, setColMenuOpen] = useState(false);

  const columns = useMemo<ColumnDef<Contract>[]>(() => [
    {
      id: 'name',
      accessorKey: 'name',
      sortUndefined: 1, // Fix sorting undefined issues. Note we delegate sorting anyway
      header: 'Name',
      cell: ({ row }) => (
        <div className="max-w-[180px]">
          <div className="font-semibold text-foreground group-hover:text-primary transition-colors truncate">{row.original.name}</div>
          <div className="text-[10px] text-muted-foreground font-mono truncate">{row.original.contract_id.substring(0, 10)}...</div>
        </div>
      ),
    },
    {
      id: 'category',
      accessorKey: 'category',
      header: 'Category',
      cell: ({ row }) => (
        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] bg-primary/10 text-primary font-medium">
          <Tag className="w-3 h-3" />
          {row.original.category || 'Uncategorized'}
        </span>
      ),
    },
    {
      id: 'network',
      accessorKey: 'network',
      header: 'Network',
      cell: ({ row }) => {
        const colors: Record<string, string> = {
          mainnet: 'bg-green-500/10 text-green-600 border-green-500/20',
          testnet: 'bg-blue-500/10 text-blue-600 border-blue-500/20',
          futurenet: 'bg-purple-500/10 text-purple-600 border-purple-500/20',
        };
        const color = colors[row.original.network] || 'bg-muted text-muted-foreground border-border';
        return <span className={`px-2 py-0.5 rounded-full text-[10px] uppercase font-bold border ${color}`}>{row.original.network}</span>;
      },
    },
    {
      id: 'is_verified',
      accessorKey: 'is_verified',
      header: 'Status',
      cell: ({ row }) => (
        row.original.is_verified ?
          <span className="inline-flex items-center gap-1 text-[11px] text-green-500 font-medium"><CheckCircle2 className="w-3 h-3" /> Verified</span> :
          <span className="inline-flex items-center gap-1 text-[11px] text-muted-foreground font-medium">Pending</span>
      ),
    },
    {
      id: "deployments",
      accessorFn: (row: Contract & { deployment_count?: number; deployments?: number }) =>
        typeof row.deployment_count === "number"
          ? row.deployment_count
          : typeof row.deployments === "number"
            ? row.deployments
            : 0,
      header: "Deployments",
      cell: ({ getValue }) => (
        <div className="font-mono text-xs">{getValue() as number}</div>
      ),
    },
    {
      id: 'created_at',
      accessorKey: 'created_at',
      header: 'Created',
      cell: ({ row }) => <div className="text-xs text-muted-foreground whitespace-nowrap">{new Date(row.original.created_at).toLocaleDateString()}</div>,
    },
    {
      id: "category",
      accessorKey: "category",
      header: "Category",
      cell: ({ row }) => (
        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] bg-primary/10 text-primary font-medium">
          <Tag className="w-3 h-3" />
          {row.original.category || "Uncategorized"}
        </span>
      ),
    },
    {
      id: "network",
      accessorKey: "network",
      header: "Network",
      cell: ({ row }) => {
        const colors: Record<string, string> = {
          mainnet: "bg-green-500/10 text-green-600 border-green-500/20",
          testnet: "bg-blue-500/10 text-blue-600 border-blue-500/20",
          futurenet: "bg-purple-500/10 text-purple-600 border-purple-500/20",
        };
        const color =
          colors[row.original.network] ||
          "bg-muted text-muted-foreground border-border";
        return (
          <span
            className={`px-2 py-0.5 rounded-full text-[10px] uppercase font-bold border ${color}`}
          >
            {row.original.network}
          </span>
        );
      },
    },
    {
      id: "is_verified",
      accessorKey: "is_verified",
      header: "Status",
      cell: ({ row }) =>
        row.original.is_verified ? (
          <span className="inline-flex items-center gap-1 text-[11px] text-green-500 font-medium">
            <CheckCircle2 className="w-3 h-3" /> Verified
          </span>
        ) : (
          <span className="inline-flex items-center gap-1 text-[11px] text-muted-foreground font-medium">
            Pending
          </span>
        ),
    },
    {
      id: "created_at",
      accessorKey: "created_at",
      header: "Created",
      cell: ({ row }) => (
        <div className="text-xs text-muted-foreground whitespace-nowrap">
          {new Date(row.original.created_at).toLocaleDateString()}
        </div>
      ),
    },
  ],
    [],
  );

  const sortingWrapper: SortingState = useMemo(() => {
    return [{ id: sortByContext(sortBy), desc: sortOrder === 'desc' }];
  }, [sortBy, sortOrder]);

  function sortByContext(rawId: string) {
    if (rawId === 'interactions') return 'deployments'; // map legacy API sort
    return rawId;
  }

  function reverseSortByContext(rawId: string) {
    if (rawId === 'deployments') return 'interactions';
    return rawId;
  }

  const table = useReactTable({
    data,
    columns,
    state: {
      sorting: sortingWrapper,
      columnVisibility,
      columnOrder,
    },
    // Required manual setup so that client doesn't override server sorting
    manualSorting: true,
    onSortingChange: (updater) => {
      const computedState = typeof updater === 'function' ? updater(sortingWrapper) : updater;
      if (computedState.length > 0) {
        onSortChange(reverseSortByContext(computedState[0].id), computedState[0].desc ? 'desc' : 'asc');
      }
    },
    onColumnVisibilityChange: setColumnVisibility,
    onColumnOrderChange: setColumnOrder,
    getCoreRowModel: getCoreRowModel(),
  });

  const sensors = useSensors(
    useSensor(MouseSensor, { activationConstraint: { distance: 5 } }),
    useSensor(TouchSensor, { activationConstraint: { delay: 250, tolerance: 5 } }),
    useSensor(KeyboardSensor, {})
  );

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    if (active && over && active.id !== over.id) {
      setColumnOrder((order) => {
        const fallbackOrder = table.getAllLeafColumns().map((c) => c.id);
        const currentOrder = order.length > 0 ? order : fallbackOrder;
        const oldIndex = currentOrder.indexOf(active.id as string);
        const newIndex = currentOrder.indexOf(over.id as string);
        return arrayMove(currentOrder, oldIndex, newIndex);
      });
    }
  };

  const exportCSV = () => {
    const visibleCols = table.getVisibleLeafColumns();
    const headers = visibleCols.map(c => c.columnDef.header as string);
    const rows = table.getRowModel().rows.map(row => {
      return visibleCols.map(col => {
        const val = row.getValue(col.id);
        if (typeof val === 'string') return `"${val.replace(/"/g, '""')}"`;
        if (typeof val === 'boolean') return val ? 'Yes' : 'No';
        if (val instanceof Date) return `"${val.toISOString()}"`;
        return `"${String(val ?? '')}"`;
      }).join(',');
    });
    const csvContent = [headers.join(','), ...rows].join('\n');
    const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.setAttribute('download', 'contracts_export.csv');
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  };

  return (
    <div className="w-full">
      {/* Table Toolbar */}
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          {/* Column Visibility Dropdown */}
          <div className="relative">
            <button
              onClick={() => setColMenuOpen(!colMenuOpen)}
              className="inline-flex items-center gap-2 px-3 py-1.5 text-sm font-medium rounded-lg border border-border bg-background hover:bg-accent transition-colors shadow-sm"
            >
              <Columns className="w-4 h-4 text-muted-foreground" />
              Columns
              <ChevronDown className="w-3 h-3 text-muted-foreground" />
            </button>
            {colMenuOpen && (
              <>
                <div className="fixed inset-0 z-10" onClick={() => setColMenuOpen(false)}></div>
                <div className="absolute left-0 mt-2 w-48 rounded-xl border border-border bg-card shadow-lg z-20 p-2 animate-in fade-in zoom-in-95">
                  <div className="text-xs font-semibold text-muted-foreground uppercase px-2 mb-2">Toggle Columns</div>
                  {table.getAllLeafColumns().map(column => (
                    <label key={column.id} className="flex items-center gap-2 px-2 py-1.5 hover:bg-accent rounded-md cursor-pointer text-sm">
                      <input
                        type="checkbox"
                        checked={column.getIsVisible()}
                        onChange={column.getToggleVisibilityHandler()}
                        className="rounded border-border text-primary focus:ring-primary h-4 w-4"
                      />
                      {column.columnDef.header as string}
                    </label>
                  ))}
                  <div className="border-t border-border mt-2 pt-2 pb-1 px-1">
                    <button
                      onClick={() => table.resetColumnVisibility()}
                      className="w-full text-left px-2 py-1 text-xs text-primary hover:bg-primary/10 rounded-md transition-colors"
                    >
                      Reset to default
                    </button>
                  </div>
                </div>
              </>
            )}
          </div>
        </div>

        <button
          onClick={exportCSV}
          className="inline-flex items-center gap-2 px-3 py-1.5 text-sm font-medium rounded-lg border border-border bg-background hover:bg-accent transition-colors shadow-sm text-foreground"
        >
          <Download className="w-4 h-4 text-muted-foreground" />
          Export CSV
        </button>
      </div>

      {/* Responsive Wrapper */}
      <div className="bg-card w-full overflow-hidden rounded-xl border border-border shadow-sm">
        <div className="w-full overflow-x-auto min-h-[300px]">
          <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
            <table className="w-full text-sm">
              <thead>
                {table.getHeaderGroups().map(headerGroup => (
                  <tr key={headerGroup.id}>
                    <SortableContext
                      items={columnOrder.length > 0 ? columnOrder : table.getAllLeafColumns().map(c => c.id)}
                      strategy={horizontalListSortingStrategy}
                    >
                      {headerGroup.headers.map(header => (
                        <SortableHeader key={header.id} id={header.column.id} header={header}>
                          {header.isPlaceholder
                            ? null
                            : flexRender(header.column.columnDef.header, header.getContext())}
                        </SortableHeader>
                      ))}
                    </SortableContext>
                  </tr>
                ))}
              </thead>
              <tbody className="divide-y divide-border">
                {table.getRowModel().rows.map(row => (
                  <tr
                    key={row.id}
                    onClick={() => router.push(`/contracts/${row.original.id}`)}
                    className="hover:bg-primary/5 transition-colors cursor-pointer group"
                  >
                    {row.getVisibleCells().map(cell => (
                      <td key={cell.id} className="px-4 py-3 align-middle bg-transparent break-keep">
                        {flexRender(cell.column.columnDef.cell, cell.getContext())}
                      </td>
                    ))}
                  </tr>
                ))}
                {table.getRowModel().rows.length === 0 && (
                  <tr>
                    <td colSpan={columns.length} className="px-4 py-10 text-center text-muted-foreground">
                      No contracts found.
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </DndContext>
        </div>
      </div>
    </div>
  );
}
