const VALID_ROLES = ["admin", "editor", "viewer"];

export function validateRole(value, errors) {
  if (value == null) {
    errors.push({ field: "role", message: "role is required" });
    return;
  }
  if (typeof value !== "string") {
    errors.push({ field: "role", message: "role must be a string" });
    return;
  }
  if (!VALID_ROLES.includes(value)) {
    errors.push({ field: "role", message: `role must be one of: ${VALID_ROLES.join(", ")}` });
  }
}
