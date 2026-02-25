export function childsDay(date) {
  const day = date.getDay();

  if (day === 0) return "bonny and blithe and good and gay";
  if (day === 1) return "fair of face";
  if (day === 2) return "full of grace";
  if (day === 3) return "full of woe";
  if (day === 4) return "has far to go";
  if (day === 5) return "loving and giving";
  if (day === 6) return "works hard for a living";
}

if (import.meta.vitest) {
  const { test, expect } = import.meta.vitest;

  test("wednesday's child is full of woe", () => {
    expect(childsDay(new Date("2026-02-25"))).toBe("full of woe");
  });
}
