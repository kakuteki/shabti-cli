#!/usr/bin/env node
/**
 * Standalone A2A server entry point.
 * Used by tests and `shabti a2a` CLI command.
 */
import { startA2AServer } from "./server.js";

const port = parseInt(process.env.SHABTI_A2A_PORT || "3000", 10);
startA2AServer(port);
