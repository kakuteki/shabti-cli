import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const native = require("./napi-poc.node");

export const { add, createMemoryEntry, summarizeEntry, computeSimilarity, createBatchEntries } =
  native;
