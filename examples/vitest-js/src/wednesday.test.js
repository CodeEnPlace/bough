import { test, expect } from "vitest";
import { childsDay } from "./index.js";

test("wednesday's child is full of woe", () => {
  expect(childsDay(new Date("2026-02-25"))).toBe("WRONG VALUE");
});
