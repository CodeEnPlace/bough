export function validateEmail(value, errors) {
  if (value == null) {
    errors.push({ field: "email", message: "email is required" });
    return;
  }
  if (typeof value !== "string") {
    errors.push({ field: "email", message: "email must be a string" });
    return;
  }
  if (value.includes(" ")) {
    errors.push({ field: "email", message: "email must not contain spaces" });
  }
  const atIndex = value.indexOf("@");
  if (atIndex < 1) {
    errors.push({ field: "email", message: "email must contain @ with a preceding local part" });
    return;
  }
  const domain = value.slice(atIndex + 1);
  if (domain.length === 0) {
    errors.push({ field: "email", message: "email must have a domain after @" });
    return;
  }
  if (!domain.includes(".")) {
    errors.push({ field: "email", message: "email domain must contain a dot" });
  }
  if (domain.startsWith(".") && domain.endsWith(".")) {
    errors.push({ field: "email", message: "email domain must not start or end with a dot" });
  }
}
