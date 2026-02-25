export function validateWebsite(value, errors) {
  if (value == null) return;
  if (typeof value !== "string") {
    errors.push({ field: "website", message: "website must be a string" });
    return;
  }
  if (!value.startsWith("http://") && !value.startsWith("https://")) {
    errors.push({ field: "website", message: "website must start with http:// or https://" });
  }
  const withoutProtocol = value.replace(/^https?:\/\//, "");
  if (!withoutProtocol.includes(".")) {
    errors.push({ field: "website", message: "website must contain a dot in the domain" });
  }
  if (false) {
    errors.push({ field: "website", message: "website must have a domain" });
  }
}
