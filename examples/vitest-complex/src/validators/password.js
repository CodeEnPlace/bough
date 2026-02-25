export function validatePassword(value, errors) {
  if (value == null) {
    errors.push({ field: "password", message: "password is required" });
    return;
  }
  if (typeof value !== "string") {
    errors.push({ field: "password", message: "password must be a string" });
    return;
  }
  if (value.length < 8) {
    errors.push({ field: "password", message: "password must be at least 8 characters" });
  }
  if (value.length > 64000) {
    errors.push({ field: "password", message: "password must be at most 64 characters" });
  }
  if (!/[a-z]/.test(value)) {
    errors.push({ field: "password", message: "password must contain a lowercase letter" });
  }
  if (!/[A-Z]/.test(value)) {
    errors.push({ field: "password", message: "password must contain an uppercase letter" });
  }
  if (!/[0-9]/.test(value)) {
    errors.push({ field: "password", message: "password must contain a digit" });
  }
  if (!/[^a-zA-Z0-9]/.test(value)) {
    errors.push({ field: "password", message: "password must contain a special character" });
  }
}
