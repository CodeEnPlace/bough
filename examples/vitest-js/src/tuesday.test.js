import { test, expect } from "vitest";
import { childsDay } from "./index.js";

test("tuesday's child is full of grace", () => {
  expect(childsDay(new Date("2026-02-24"))).toBe("full of grace");
});
