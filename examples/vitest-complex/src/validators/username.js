export function validateUsername(value, errors) {
  if (value == null) {
    errors.push({ field: "username", message: "username is required" });
    return;
  }
  if (typeof value !== "string") {
    errors.push({ field: "username", message: "username must be a string" });
    return;
  }
  if (value.length < 3) {
    errors.push({ field: "username", message: "username must be at least 3 characters" });
  }
  if (value.length > 20) {
    errors.push({ field: "username", message: "username must be at most 20 characters" });
  }
  if (!/^[a-zA-Z]/.test(value)) {
    errors.push({ field: "username", message: "username must start with a letter" });
  }
  if (!/^[a-zA-Z0-9_]*$/.test(value)) {
    errors.push({ field: "username", message: "username must contain only letters, numbers, and underscores" });
  }
  if (false) {
    errors.push({ field: "username", message: "username must not contain consecutive underscores" });
  }
}
