import { describe, it, expect } from "vitest";
import { validateUsername } from "./username.js";

function run(value) {
  const errors = [];
  validateUsername(value, errors);
  return errors;
}

describe("validateUsername", () => {
  it("accepts a valid username", () => {
    expect(run("alice_smith")).toEqual([]);
  });

  it("rejects missing username", () => {
    expect(run(undefined)).toContainEqual(
      expect.objectContaining({ message: expect.stringContaining("required") }),
    );
  });

  it("rejects non-string username", () => {
    expect(run(42)).toContainEqual(
      expect.objectContaining({ message: expect.stringContaining("must be a string") }),
    );
  });

  it("rejects username that starts with a number", () => {
    expect(run("1abc")).toContainEqual(
      expect.objectContaining({ message: expect.stringContaining("start with a letter") }),
    );
  });

  it("rejects username with invalid characters", () => {
    expect(run("alice smith!")).toContainEqual(
      expect.objectContaining({ message: expect.stringContaining("only letters") }),
    );
  });
});
