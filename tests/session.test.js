import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock OpenAI before importing ChatSession
vi.mock("openai", () => {
  return {
    default: class MockOpenAI {
      constructor() {
        this.chat = {
          completions: {
            create: vi.fn(),
          },
        };
      }
    },
  };
});

const { ChatSession } = await import("../src/core/session.js");

describe("ChatSession", () => {
  let session;

  beforeEach(() => {
    session = new ChatSession({ apiKey: "test-key" });
  });

  describe("constructor", () => {
    it("initializes with default model", () => {
      expect(session.getModel()).toBe("gpt-4o-mini");
    });

    it("accepts custom model", () => {
      const s = new ChatSession({ apiKey: "k", model: "gpt-4o" });
      expect(s.getModel()).toBe("gpt-4o");
    });

    it("initializes history with system message", () => {
      const history = session.getHistory();
      expect(history).toHaveLength(1);
      expect(history[0].role).toBe("system");
    });

    it("uses custom system prompt", () => {
      const s = new ChatSession({ apiKey: "k", systemPrompt: "Custom prompt" });
      expect(s.getHistory()[0].content).toBe("Custom prompt");
    });
  });

  describe("getHistory", () => {
    it("returns a copy of messages", () => {
      const h1 = session.getHistory();
      const h2 = session.getHistory();
      expect(h1).toEqual(h2);
      expect(h1).not.toBe(h2); // different array reference
    });
  });

  describe("clearHistory", () => {
    it("resets to only system message", () => {
      // Simulate adding messages by checking history length
      session.clearHistory();
      const history = session.getHistory();
      expect(history).toHaveLength(1);
      expect(history[0].role).toBe("system");
    });

    it("preserves original system prompt after clear", () => {
      const s = new ChatSession({ apiKey: "k", systemPrompt: "My prompt" });
      s.clearHistory();
      expect(s.getHistory()[0].content).toBe("My prompt");
    });
  });

  describe("setModel / getModel", () => {
    it("switches model", () => {
      session.setModel("gpt-4o");
      expect(session.getModel()).toBe("gpt-4o");
    });

    it("allows multiple switches", () => {
      session.setModel("gpt-4o");
      session.setModel("gpt-3.5-turbo");
      expect(session.getModel()).toBe("gpt-3.5-turbo");
    });
  });
});
