use bough_cli_test::{TestPlan, cmd};

mod js {
    use super::*;

    fn plan() -> TestPlan {
        TestPlan::new()
            .config(
                r#"
[vcs]
kind = "jj"
rev = "bough"

[vitest-js]
pwd = "examples/vitest-js"

init.commands = ["npm install"]
test.commands = ["npm test"]

[vitest-js.js]
files.include = ["src/*.js"]
files.exclude = ["**/*.test.*"]
    "#,
            )
            .file(
                "src/index.js",
                r#"
export function childsDay(date) {
  const day = date.getDay();

  if (day === 0) return "bonny and blithe and good and gay";
  if (day === 1) return "fair of face";
  if (day === 2) return "full of grace";
}
    "#,
            )
            .file(
                "src/test.js",
                r#"
import { test, expect } from "vitest";
import { childsDay } from "./index.js";

test("monday's child is fair of face", () => {
  expect(childsDay(new Date("2026-02-23"))).toBe("fair of face");
});
    "#,
            )
            .file(
                "package.json",
                r#"
{
  "name": "bough-example-vitest-js",
  "private": true,
  "type": "module",
  "scripts": {
    "test": "vitest run",
    "get-test-ids": "node get-test-ids.js",
    "get-failed-test-ids": "node get-failed-test-ids.js"
  },
  "devDependencies": {
    "vitest": "^3.0.0"
  }
}    "#,
            )
    }

    #[test]
    fn finds_1_source_file() {
        let mut dir = plan().setup();

        cmd!(dir, "bough show src", "found 1 files for Javascript");
    }

    #[test]
    fn makes_new_workspaces() {
        let mut dir = plan().setup();

        cmd!(
            dir,
            "bough workspace make",
            "created workspace at /tmp/bough/work/{!id_1}"
        );
        cmd!(
            dir,
            "bough workspace make",
            "created workspace at /tmp/bough/work/{!id_2}"
        );

        assert_ne!(dir.get("id_1"), dir.get("id_2"));

        cmd!(
            dir,
            "bough workspace list --output-style verbose",
            r#"
2 workspaces
  {?id_1} /tmp/bough/work/{?id_1}
  {?id_2} /tmp/bough/work/{?id_2}
"#
        );
    }
}
