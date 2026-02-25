import { execSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { relative } from "node:path";

let json;
try {
	json = execSync("npx vitest run --reporter=json 2>/dev/null", {
		encoding: "utf8",
	});
} catch (e) {
	json = e.stdout;
}
const { testResults } = JSON.parse(json);

for (const result of testResults) {
	if (result.status !== "failed") continue;
	const rel = relative(".", result.name);
	const content = readFileSync(result.name, "utf8");
	const hash = createHash("sha256").update(content).digest("hex");
	console.log(`${rel} ${hash}`);
}
