import { test, expect } from "vitest";
import { childsDay } from "./index.js";

test("monday's child is fair of face", () => {
  expect(childsDay(new Date("2026-02-23"))).toBe("fair of face");
});
