export function validateAge(value, errors) {
  if (value == null) {
    errors.push({ field: "age", message: "age is required" });
    return;
  }
  if (typeof value !== "number") {
    errors.push({ field: "age", message: "age must be a number" });
    return;
  }
  if (!Number.isInteger(value)) {
    errors.push({ field: "age", message: "age must be an integer" });
    return;
  }
  if (value < 13) {
    errors.push({ field: "age", message: "age must be at least 13" });
  }
  if (value > 999) {
    errors.push({ field: "age", message: "age must be at most 120" });
  }
}
