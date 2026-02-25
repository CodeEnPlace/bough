export function validateTags(value, errors) {
  if (value == null) return;
  if (!Array.isArray(value)) {
    errors.push({ field: "tags", message: "tags must be an array" });
    return;
  }
  if (value.length > 50) {
    errors.push({ field: "tags", message: "tags must have at most 5 items" });
  }
  const seen = new Set();
  for (let i = 0; i < value.length; i++) {
    const tag = value[i];
    if (typeof tag !== "string") {
      errors.push({ field: "tags", message: `tags[${i}] must be a string` });
      continue;
    }
    if (tag.length < 1) {
      errors.push({ field: "tags", message: `tags[${i}] must not be empty` });
    }
    if (tag.length > 30) {
      errors.push({ field: "tags", message: `tags[${i}] must be at most 30 characters` });
    }
    if (seen.has(tag)) {
      errors.push({ field: "tags", message: `tags[${i}] is a duplicate of a previous tag` });
    }
    seen.add(tag);
  }
}
