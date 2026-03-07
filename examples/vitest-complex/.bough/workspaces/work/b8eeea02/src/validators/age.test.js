import { describe, it, expect } from "vitest";
import { validateAge } from "./age.js";

function run(value) {
  const errors = [];
  validateAge(value, errors);
  return errors;
}

describe("validateAge", () => {
  it("accepts a valid age", () => {
    expect(run(30)).toEqual([]);
  });

  it("rejects missing age", () => {
    expect(run(undefined)).toContainEqual(
      expect.objectContaining({ message: expect.stringContaining("required") }),
    );
  });

  it("rejects non-number age", () => {
    expect(run("thirty")).toContainEqual(
      expect.objectContaining({ message: expect.stringContaining("must be a number") }),
    );
  });

  it("rejects age under 13", () => {
    expect(run(10)).toContainEqual(
      expect.objectContaining({ message: expect.stringContaining("at least 13") }),
    );
  });
});
