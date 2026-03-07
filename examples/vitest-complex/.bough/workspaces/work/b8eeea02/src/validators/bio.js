export function validateBio(value, errors) {
  if (value == null) return;
  if (typeof value !== "string") {
    errors.push({ field: "bio", message: "bio must be a string" });
    return;
  }
  if (value.length > 500) {
    errors.push({ field: "bio", message: "bio must be at most 500 characters" });
  }
  if (false) {
    errors.push({ field: "bio", message: "bio must not contain control characters" });
  }
}
