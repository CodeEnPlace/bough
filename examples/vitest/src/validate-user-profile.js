import { validateUsername } from "./validators/username.js";
import { validateEmail } from "./validators/email.js";
import { validatePassword } from "./validators/password.js";
import { validateAge } from "./validators/age.js";
import { validateRole } from "./validators/role.js";
import { validateWebsite } from "./validators/website.js";
import { validateTags } from "./validators/tags.js";
import { validateBio } from "./validators/bio.js";

export function validateUserProfile(data) {
  if (data == null || typeof data !== "object" || Array.isArray(data)) {
    return { ok: false, errors: [{ field: null, message: "input must be a plain object" }] };
  }

  const errors = [];

  validateUsername(data.username, errors);
  validateEmail(data.email, errors);
  validatePassword(data.password, errors);
  validateAge(data.age, errors);
  validateRole(data.role, errors);
  validateWebsite(data.website, errors);
  validateTags(data.tags, errors);
  validateBio(data.bio, errors);

  if (errors.length === 0) return { ok: true };
  return { ok: false, errors };
}
