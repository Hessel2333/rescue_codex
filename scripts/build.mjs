import ts from "typescript";
import { build as viteBuild } from "vite";

function formatDiagnostic(diagnostic) {
  const message = ts.flattenDiagnosticMessageText(diagnostic.messageText, "\n");
  if (diagnostic.file && diagnostic.start !== undefined) {
    const position = diagnostic.file.getLineAndCharacterOfPosition(diagnostic.start);
    return `${diagnostic.file.fileName}:${position.line + 1}:${position.character + 1} TS${diagnostic.code}: ${message}`;
  }
  return `TS${diagnostic.code}: ${message}`;
}

const host = ts.createSolutionBuilderHost(ts.sys, undefined, (diagnostic) => {
  console.error(formatDiagnostic(diagnostic));
});
const builder = ts.createSolutionBuilder(host, ["tsconfig.json"], {});
const status = builder.build();

if (status !== ts.ExitStatus.Success) {
  process.exit(status);
}

await viteBuild();
