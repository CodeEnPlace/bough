import { describe, it, expect } from "vitest";
import { validateTags } from "./tags.js";

function run(value) {
  const errors = [];
  validateTags(value, errors);
  return errors;
}

describe("validateTags", () => {
  it("accepts undefined (optional field)", () => {
    expect(run(undefined)).toEqual([]);
  });

  it("accepts valid tags", () => {
    expect(run(["js", "testing"])).toEqual([]);
  });

  it("rejects non-array tags", () => {
    expect(run("js")).toContainEqual(
      expect.objectContaining({ message: expect.stringContaining("must be an array") }),
    );
  });

  it("rejects non-string tag items", () => {
    expect(run([1, 2])).toContainEqual(
      expect.objectContaining({ message: expect.stringContaining("must be a string") }),
    );
  });
});
