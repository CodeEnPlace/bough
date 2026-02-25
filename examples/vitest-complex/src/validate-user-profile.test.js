import { describe, it, expect } from "vitest";
import { validateUserProfile } from "./validate-user-profile.js";

const validProfile = {
  username: "alice_smith",
  email: "alice@example.com",
  password: "Secret1!xx",
  age: 30,
  role: "editor",
};

describe("validateUserProfile", () => {
  it("accepts a valid profile with only required fields", () => {
    expect(validateUserProfile(validProfile)).toEqual({ ok: true });
  });

  it("accepts a valid profile with all optional fields", () => {
    const result = validateUserProfile({
      ...validProfile,
      website: "https://example.com",
      tags: ["js", "testing"],
      bio: "Hello world",
    });
    expect(result).toEqual({ ok: true });
  });

  it("rejects null input", () => {
    const result = validateUserProfile(null);
    expect(result.ok).toBe(false);
    expect(result.errors[0].message).toMatch(/plain object/);
  });

  it("rejects an array", () => {
    const result = validateUserProfile([1, 2]);
    expect(result.ok).toBe(false);
  });
});
