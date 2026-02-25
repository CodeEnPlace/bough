import { describe, it, expect } from "vitest";
import { validateRole } from "./role.js";

function run(value) {
  const errors = [];
  validateRole(value, errors);
  return errors;
}

describe("validateRole", () => {
  it("accepts a valid role", () => {
    expect(run("editor")).toEqual([]);
  });

  it("rejects missing role", () => {
    expect(run(undefined)).toContainEqual(
      expect.objectContaining({ message: expect.stringContaining("required") }),
    );
  });

  it("rejects invalid role", () => {
    expect(run("superadmin")).toContainEqual(
      expect.objectContaining({ message: expect.stringContaining("one of") }),
    );
  });
});
