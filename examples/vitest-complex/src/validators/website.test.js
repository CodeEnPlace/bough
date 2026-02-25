import { describe, it, expect } from "vitest";
import { validateWebsite } from "./website.js";

function run(value) {
  const errors = [];
  validateWebsite(value, errors);
  return errors;
}

describe("validateWebsite", () => {
  it("accepts undefined (optional field)", () => {
    expect(run(undefined)).toEqual([]);
  });

  it("accepts a valid website", () => {
    expect(run("https://example.com")).toEqual([]);
  });

  it("rejects website without protocol", () => {
    expect(run("example.com")).toContainEqual(
      expect.objectContaining({ message: expect.stringContaining("http") }),
    );
  });
});
