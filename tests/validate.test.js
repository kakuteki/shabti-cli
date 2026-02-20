import { describe, it, expect } from "vitest";
import {
  parsePositiveInt,
  parseNonNegativeInt,
  parsePositiveFloat,
  parseScore,
  parsePort,
} from "../src/utils/validate.js";

describe("parsePositiveInt", () => {
  it("parses valid positive integers", () => {
    expect(parsePositiveInt("1", "x")).toBe(1);
    expect(parsePositiveInt("42", "x")).toBe(42);
    expect(parsePositiveInt("1000", "x")).toBe(1000);
  });

  it("rejects zero", () => {
    expect(() => parsePositiveInt("0", "--ttl")).toThrow("--ttl must be a positive integer");
  });

  it("rejects negative numbers", () => {
    expect(() => parsePositiveInt("-5", "--ttl")).toThrow("--ttl must be a positive integer");
  });

  it("rejects non-numeric strings", () => {
    expect(() => parsePositiveInt("abc", "--limit")).toThrow("--limit must be a positive integer");
  });

  it("rejects empty string", () => {
    expect(() => parsePositiveInt("", "--limit")).toThrow("--limit must be a positive integer");
  });
});

describe("parseNonNegativeInt", () => {
  it("accepts zero", () => {
    expect(parseNonNegativeInt("0", "x")).toBe(0);
  });

  it("accepts positive", () => {
    expect(parseNonNegativeInt("10", "x")).toBe(10);
  });

  it("rejects negative", () => {
    expect(() => parseNonNegativeInt("-1", "x")).toThrow("non-negative integer");
  });
});

describe("parsePositiveFloat", () => {
  it("parses valid floats", () => {
    expect(parsePositiveFloat("0.5", "x")).toBe(0.5);
    expect(parsePositiveFloat("3.14", "x")).toBe(3.14);
  });

  it("rejects zero", () => {
    expect(() => parsePositiveFloat("0", "x")).toThrow("positive number");
  });

  it("rejects negative", () => {
    expect(() => parsePositiveFloat("-1.5", "x")).toThrow("positive number");
  });
});

describe("parseScore", () => {
  it("accepts values in [0, 1]", () => {
    expect(parseScore("0", "x")).toBe(0);
    expect(parseScore("0.5", "x")).toBe(0.5);
    expect(parseScore("1", "x")).toBe(1);
  });

  it("rejects values outside [0, 1]", () => {
    expect(() => parseScore("1.5", "--min-score")).toThrow("between 0 and 1");
    expect(() => parseScore("-0.1", "--min-score")).toThrow("between 0 and 1");
  });

  it("rejects NaN", () => {
    expect(() => parseScore("abc", "x")).toThrow("between 0 and 1");
  });
});

describe("parsePort", () => {
  it("accepts valid ports", () => {
    expect(parsePort("1", "x")).toBe(1);
    expect(parsePort("3000", "x")).toBe(3000);
    expect(parsePort("65535", "x")).toBe(65535);
  });

  it("rejects 0", () => {
    expect(() => parsePort("0", "--port")).toThrow("port number");
  });

  it("rejects numbers above 65535", () => {
    expect(() => parsePort("70000", "--port")).toThrow("port number");
  });

  it("rejects non-numeric", () => {
    expect(() => parsePort("abc", "--port")).toThrow("port number");
  });
});
