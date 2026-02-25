import { readFileSync, readdirSync, statSync } from "node:fs";
import { createHash } from "node:crypto";
import { join } from "node:path";

function walk(dir) {
	const results = [];
	for (const entry of readdirSync(dir)) {
		if (entry === "node_modules") continue;
		const full = join(dir, entry);
		if (statSync(full).isDirectory()) results.push(...walk(full));
		else results.push(full);
	}
	return results;
}

for (const file of walk(".")) {
	const content = readFileSync(file, "utf8");
	if (!content.includes("vitest")) continue;
	const hash = createHash("sha256").update(content).digest("hex");
	console.log(`${file} ${hash}`);
}
