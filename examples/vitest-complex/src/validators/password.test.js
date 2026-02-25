import { describe, it, expect } from "vitest";
import { validatePassword } from "./password.js";

function run(value) {
  const errors = [];
  validatePassword(value, errors);
  return errors;
}

describe("validatePassword", () => {
  it("accepts a valid password", () => {
    expect(run("Secret1!xx")).toEqual([]);
  });

  it("rejects missing password", () => {
    expect(run(undefined)).toContainEqual(
      expect.objectContaining({ message: expect.stringContaining("required") }),
    );
  });

  it("rejects short password", () => {
    expect(run("Ab1!")).toContainEqual(
      expect.objectContaining({ message: expect.stringContaining("at least 8") }),
    );
  });

  it("rejects password without uppercase", () => {
    expect(run("secret1!xx")).toContainEqual(
      expect.objectContaining({ message: expect.stringContaining("uppercase") }),
    );
  });
});
