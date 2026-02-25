import { describe, it, expect } from "vitest";
import { validateEmail } from "./email.js";

function run(value) {
  const errors = [];
  validateEmail(value, errors);
  return errors;
}

describe("validateEmail", () => {
  it("accepts a valid email", () => {
    expect(run("alice@example.com")).toEqual([]);
  });

  it("rejects missing email", () => {
    expect(run(undefined)).toContainEqual(
      expect.objectContaining({ message: expect.stringContaining("required") }),
    );
  });

  it("rejects email without @", () => {
    expect(run("aliceexample.com")).toContainEqual(
      expect.objectContaining({ message: expect.stringContaining("@") }),
    );
  });

  it("rejects email without domain dot", () => {
    expect(run("alice@localhost")).toContainEqual(
      expect.objectContaining({ message: expect.stringContaining("dot") }),
    );
  });
});
