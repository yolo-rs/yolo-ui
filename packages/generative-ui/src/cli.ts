import { join, resolve } from "node:path";
import { compileDesignMdToSixTargets } from "./index";

const designMdPath = resolve(process.argv[2] ?? join(import.meta.dir, "..", "..", "..", "DESIGN.md"));
const result = compileDesignMdToSixTargets(designMdPath);
console.log(
  JSON.stringify(
    {
      source: designMdPath,
      targetsWritten: result.files.length,
      files: result.files,
      goldenSignature: result.goldenSignature,
    },
    null,
    2
  )
);
