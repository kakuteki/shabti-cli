export interface MemoryEntry {
  id: string;
  content: string;
  score: number;
  createdAt: number;
}

export function add(a: number, b: number): number;
export function createMemoryEntry(id: string, content: string, score: number): MemoryEntry;
export function summarizeEntry(entry: MemoryEntry): string;
export function computeSimilarity(a: number[], b: number[]): Promise<number>;
export function createBatchEntries(count: number): MemoryEntry[];
