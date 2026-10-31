import type { Readable, Writable } from 'node:stream';
export type Completion = 'success' | 'handled' | 'not_handled' | 'error';
export interface Page<T = Record<string, unknown>> { items: T[]; has_more: boolean; next_cursor: string | null; cursor_ttl_ms: number; consistency: string }
export interface HostFailure { code: string; retryable?: boolean; native_error_code?: number | null }
export class HostError extends Error { constructor(value?: HostFailure); code: string; retryable: boolean; nativeErrorCode?: number | null }
export interface BlobDescriptor { blob_id: string; length: number; media_type: string; sha256: string }
export interface StoredValue<T = unknown> { found: boolean; value: T | null; revision: number }
export interface StorageMutation { revision: number; cleanup_pending: boolean }
export interface StoragePage { items: { key: string; revision: number }[]; revision: number; has_more: boolean; next_cursor: string | null }
export class Session {
  static start(input?: Readable, output?: Writable, options?: { protocolVersion?: 1 | 2 }): Promise<Session>;
  invoke: Record<string, unknown>;
  call<T = unknown>(method: string, params?: Record<string, unknown>): Promise<T>;
  pages<T = Record<string, unknown>>(method: 'downloads.list_page' | 'clipboard.query_page', params?: Record<string, unknown>): AsyncGenerator<Page<T>>;
  progress(value?: { completed?: number; total?: number; message?: string }): Promise<void>;
  blobUpload(data: Uint8Array, mediaType?: string): Promise<BlobDescriptor>;
  blobRead(blob: BlobDescriptor, span?: { offset?: number; length?: number }): Promise<Uint8Array>;
  blobRelease(blob: BlobDescriptor): Promise<unknown>;
  storageGet<T = unknown>(key: string): Promise<StoredValue<T>>;
  storageSet(key: string, value: unknown, options?: { expectedRevision?: number }): Promise<StorageMutation>;
  storageDelete(key: string, options?: { expectedRevision?: number }): Promise<StorageMutation>;
  storageList(options?: { prefix?: string; limit?: number; cursor?: string | null }): Promise<StoragePage>;
  complete(status?: Completion, value?: unknown): Promise<void>;
  close(): void;
}
